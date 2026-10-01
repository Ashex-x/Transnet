//! Resumable application orchestration for immutable knowledge-publication builds.

use std::{future::Future, sync::Arc};

use crate::{
  domain::{
    canonical::CanonicalReleasePin,
    knowledge_projection::{
      EdgeProjection, EdgeProjectionBuild, EmbeddingCompatibilityEntry,
      EmbeddingCompatibilityRegistry, NodeProjection, NodeProjectionBuild,
    },
    knowledge_publication::{
      LexicalDictionaryManifest, PersistedCollectionHash, PublicationBatchContentHash,
      PublicationBatchIdentity, PublicationBatchOrdinal, PublicationBuildId, PublicationBuildState,
      PublicationCollectionFamily, PublicationFinalizeIdentity, PublicationIdempotencyKey,
      PublicationManifestHash, PublicationReconcileIdentity, PublicationRequestFingerprint,
    },
    knowledge_release::KnowledgeReleaseFailure,
  },
  ports::knowledge_publication::{
    BeginPublication, EdgePublicationBatch, FreezeEdges, FreezeNodes, KnowledgePublicationContext,
    KnowledgePublicationPort, NodePublicationBatch, PublicationActivationCandidate,
    PublicationBatchWireAdmission, PublicationStatus, ReconcilePublication,
    MAX_PUBLICATION_BATCH_POINTS,
  },
};

/// Immutable inputs needed to execute or resume one publication build.
pub struct KnowledgePublicationPlan {
  /// Canonical release and schema shared by every publication operation.
  pub canonical: CanonicalReleasePin,
  /// Storage-neutral hash of all canonical rows admitted to the immutable release.
  pub canonical_content_hash: String,
  /// Complete deterministic node projection.
  pub nodes: NodeProjectionBuild,
  /// Complete deterministic edge projection bound to `nodes`.
  pub edges: EdgeProjectionBuild,
  /// Exact execution compatibility entry approved for both collections.
  pub compatibility: EmbeddingCompatibilityEntry,
  /// Frozen release-local dictionary used by node lexical encoding.
  pub node_dictionary: LexicalDictionaryManifest,
  /// Frozen release-local dictionary used by edge lexical encoding.
  pub edge_dictionary: LexicalDictionaryManifest,
  /// Caller-controlled idempotency identity for the immutable build intent.
  pub idempotency_key: PublicationIdempotencyKey,
}

/// Drives the outbound publication capability without owning mutation credentials or activation.
#[derive(Clone)]
pub struct KnowledgePublicationService {
  publication: Arc<dyn KnowledgePublicationPort>,
}

impl KnowledgePublicationService {
  /// Creates an application service over one explicit outbound publication port.
  pub fn new(publication: Arc<dyn KnowledgePublicationPort>) -> Self {
    Self { publication }
  }

  /// Reads the authoritative state of one offline publication build.
  ///
  /// This operation is stateless and never infers progress from local files or prior calls.
  ///
  /// # Errors
  ///
  /// Returns a closed publication failure when the authority is unavailable or echoes a different
  /// build identity.
  pub async fn status(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    let status = self
      .publication
      .status(context, canonical, build_id)
      .await?;
    if status.build_id != *build_id {
      return Err(KnowledgeReleaseFailure::IdempotencyConflict);
    }
    Ok(status)
  }

  /// Requests an idempotent legal abort of one non-activated offline publication build.
  ///
  /// Activation candidates cannot be aborted, and this operation never deletes immutable
  /// collections or selects an active or rollback release.
  ///
  /// # Errors
  ///
  /// Returns a closed publication failure when the authority rejects the lifecycle transition,
  /// cannot complete the request, or echoes a different build identity.
  pub async fn abort(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
    idempotency_key: &PublicationIdempotencyKey,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    let fingerprint = PublicationRequestFingerprint::derive("abort", &[build_id.as_str()])
      .map_err(KnowledgeReleaseFailure::from)?;
    let status = self
      .publication
      .abort(context, canonical, build_id, idempotency_key, &fingerprint)
      .await?;
    if status.build_id != *build_id {
      return Err(KnowledgeReleaseFailure::IdempotencyConflict);
    }
    if !matches!(
      status.state,
      PublicationBuildState::Aborting | PublicationBuildState::Abandoned
    ) {
      return Err(KnowledgeReleaseFailure::InvalidLifecycleTransition);
    }
    Ok(status)
  }

  /// Executes or safely resumes one node-first build through reconciliation.
  ///
  /// The returned value is only an activation candidate. This service never activates or rolls
  /// back a release and never acquires Qdrant, MySQL, or embedding credentials.
  ///
  /// # Errors
  ///
  /// Returns a closed publication failure when immutable inputs conflict, a remote status cannot
  /// be resumed safely, any batch/freeze step fails, or reconciliation does not complete.
  pub async fn publish(
    &self,
    context: &KnowledgePublicationContext,
    plan: &KnowledgePublicationPlan,
  ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure> {
    validate_plan(plan)?;
    let build_id = PublicationBuildId::derive(
      &plan.canonical.release_id,
      plan.nodes.embedding.payload_schema_version(),
      &plan.nodes.content_hash,
      &plan.compatibility.entry_id,
    )
    .map_err(KnowledgeReleaseFailure::from)?;
    let node_batches = self.plan_node_batches(plan, &build_id)?;
    let edge_batches = self.plan_edge_batches(plan, &build_id)?;
    let begin = begin_request(plan, build_id.clone())?;
    let begin_status = self.publication.begin(context, &begin).await?;
    if begin_status.build_id != build_id {
      return Err(KnowledgeReleaseFailure::IdempotencyConflict);
    }
    let initial = self
      .publication
      .status(context, &plan.canonical, &build_id)
      .await?;
    validate_status(
      &initial,
      &build_id,
      batch_count(&node_batches)?,
      batch_count(&edge_batches)?,
    )?;

    if terminal_failure(initial.state) {
      return Err(KnowledgeReleaseFailure::InvalidLifecycleTransition);
    }

    if initial.state == PublicationBuildState::AcceptingNodes {
      self
        .submit_node_batches(
          context,
          plan,
          &build_id,
          &node_batches,
          batch_count(&edge_batches)?,
          initial.next_node_ordinal,
        )
        .await?;
    }

    let node_hash =
      PersistedCollectionHash::for_nodes(&plan.nodes, &plan.compatibility, &plan.node_dictionary)
        .map_err(KnowledgeReleaseFailure::from)?;
    let frozen_nodes = self
      .publication
      .freeze_nodes(
        context,
        &plan.canonical,
        &FreezeNodes {
          build_id: build_id.clone(),
          finalize_id: PublicationFinalizeIdentity::derive(
            &build_id,
            PublicationCollectionFamily::Nodes,
            &node_hash,
          ),
          persisted_hash: node_hash.clone(),
          projection_hash: plan.nodes.content_hash.clone(),
          projection_schema_version: plan.nodes.embedding.payload_schema_version().to_string(),
          point_count: plan.nodes.point_count,
          dictionary: plan.node_dictionary.clone(),
          compatibility: plan.compatibility.clone(),
        },
      )
      .await?;

    if matches!(
      initial.state,
      PublicationBuildState::AcceptingNodes
        | PublicationBuildState::NodesFrozen
        | PublicationBuildState::AcceptingEdges
    ) {
      self
        .submit_edge_batches(
          context,
          plan,
          &build_id,
          &edge_batches,
          batch_count(&node_batches)?,
          initial.next_edge_ordinal,
        )
        .await?;
    }

    let edge_hash =
      PersistedCollectionHash::for_edges(&plan.edges, &plan.compatibility, &plan.edge_dictionary)
        .map_err(KnowledgeReleaseFailure::from)?;
    let frozen_edges = self
      .publication
      .freeze_edges(
        context,
        &plan.canonical,
        &FreezeEdges {
          build_id: build_id.clone(),
          finalize_id: PublicationFinalizeIdentity::derive(
            &build_id,
            PublicationCollectionFamily::Edges,
            &edge_hash,
          ),
          persisted_hash: edge_hash.clone(),
          projection_hash: plan.edges.content_hash.clone(),
          projection_schema_version: plan.edges.embedding.payload_schema_version().to_string(),
          verified_node_projection_hash: plan.edges.verified_node_content_hash.clone(),
          point_count: plan.edges.point_count,
          expected_endpoint_count: plan.edges.expected_endpoint_count,
          resolved_endpoint_count: plan.edges.resolved_endpoint_count,
          dictionary: plan.edge_dictionary.clone(),
          compatibility: plan.compatibility.clone(),
        },
      )
      .await?;

    let manifest_hash = PublicationManifestHash::derive(
      &plan.canonical.release_id,
      &plan.canonical.canonical_schema_version,
      &node_hash,
      &edge_hash,
    )
    .map_err(KnowledgeReleaseFailure::from)?;
    let reconcile_id = PublicationReconcileIdentity::derive(&build_id, &node_hash, &edge_hash)
      .map_err(KnowledgeReleaseFailure::from)?;
    self
      .publication
      .reconcile(
        context,
        &ReconcilePublication {
          build_id,
          reconcile_id,
          canonical: plan.canonical.clone(),
          canonical_content_hash: plan.canonical_content_hash.clone(),
          nodes: frozen_nodes,
          edges: frozen_edges,
          manifest_hash,
          compatibility: plan.compatibility.clone(),
        },
      )
      .await
  }

  async fn submit_node_batches(
    &self,
    context: &KnowledgePublicationContext,
    plan: &KnowledgePublicationPlan,
    build_id: &PublicationBuildId,
    batches: &[PlannedBatch<NodeProjection>],
    edge_batch_count: u32,
    next_ordinal: u32,
  ) -> Result<(), KnowledgeReleaseFailure> {
    submit_planned_batches(
      batches,
      next_ordinal,
      PublicationCollectionFamily::Nodes,
      build_id,
      batch_count(batches)?,
      edge_batch_count,
      |identity, points| async move {
        self
          .publication
          .submit_nodes(
            context,
            &plan.canonical,
            &NodePublicationBatch { identity, points },
          )
          .await
      },
    )
    .await
  }

  async fn submit_edge_batches(
    &self,
    context: &KnowledgePublicationContext,
    plan: &KnowledgePublicationPlan,
    build_id: &PublicationBuildId,
    batches: &[PlannedBatch<EdgeProjection>],
    node_batch_count: u32,
    next_ordinal: u32,
  ) -> Result<(), KnowledgeReleaseFailure> {
    submit_planned_batches(
      batches,
      next_ordinal,
      PublicationCollectionFamily::Edges,
      build_id,
      node_batch_count,
      batch_count(batches)?,
      |identity, points| async move {
        self
          .publication
          .submit_edges(
            context,
            &plan.canonical,
            &EdgePublicationBatch { identity, points },
          )
          .await
      },
    )
    .await
  }

  fn plan_node_batches(
    &self,
    plan: &KnowledgePublicationPlan,
    build_id: &PublicationBuildId,
  ) -> Result<Vec<PlannedBatch<NodeProjection>>, KnowledgeReleaseFailure> {
    plan_batches(
      &plan.nodes.points,
      PublicationCollectionFamily::Nodes,
      build_id,
      |identity, points| {
        self
          .publication
          .inspect_node_batch(&plan.canonical, &NodePublicationBatch { identity, points })
      },
    )
  }

  fn plan_edge_batches(
    &self,
    plan: &KnowledgePublicationPlan,
    build_id: &PublicationBuildId,
  ) -> Result<Vec<PlannedBatch<EdgeProjection>>, KnowledgeReleaseFailure> {
    plan_batches(
      &plan.edges.points,
      PublicationCollectionFamily::Edges,
      build_id,
      |identity, points| {
        self
          .publication
          .inspect_edge_batch(&plan.canonical, &EdgePublicationBatch { identity, points })
      },
    )
  }
}

fn validate_plan(plan: &KnowledgePublicationPlan) -> Result<(), KnowledgeReleaseFailure> {
  if PublicationManifestHash::parse(plan.canonical_content_hash.clone()).is_err()
    || plan.nodes.release_id != plan.canonical.release_id
    || plan.edges.release_id != plan.canonical.release_id
    || plan.nodes.embedding != plan.edges.embedding
    || plan.edges.verified_node_content_hash != plan.nodes.content_hash
    || plan.nodes.point_count == 0
    || plan.edges.point_count == 0
    || plan.nodes.point_count != plan.nodes.points.len() as u64
    || plan.edges.point_count != plan.edges.points.len() as u64
    || plan.edges.expected_endpoint_count != plan.edges.resolved_endpoint_count
    || plan.edges.expected_endpoint_count != plan.edges.point_count.saturating_mul(2)
  {
    return Err(KnowledgeReleaseFailure::IncompleteTrio);
  }
  EmbeddingCompatibilityRegistry::new(vec![plan.compatibility.clone()])
    .and_then(|registry| registry.validate(&plan.nodes.embedding))
    .map_err(|_| KnowledgeReleaseFailure::EmbeddingMetadataIncompatible)
}

fn begin_request(
  plan: &KnowledgePublicationPlan,
  build_id: PublicationBuildId,
) -> Result<BeginPublication, KnowledgeReleaseFailure> {
  let node_count = plan.nodes.point_count.to_string();
  let edge_count = plan.edges.point_count.to_string();
  let endpoint_count = plan.edges.expected_endpoint_count.to_string();
  let fingerprint = PublicationRequestFingerprint::derive(
    "begin",
    &[
      build_id.as_str(),
      plan.canonical.release_id.as_str(),
      &plan.canonical.canonical_schema_version,
      plan.nodes.embedding.payload_schema_version(),
      &plan.nodes.content_hash,
      &plan.edges.content_hash,
      &plan.compatibility.entry_id,
      &plan.compatibility.dense_model_family,
      &plan.compatibility.dense_artifact_revision,
      &plan.compatibility.dense_dimensions.to_string(),
      &plan.compatibility.dense_vector_name,
      &plan.compatibility.node_dense_input_specification,
      &plan.compatibility.edge_dense_input_specification,
      &plan.compatibility.lexical_encoder_identity,
      &plan.compatibility.lexical_encoder_revision,
      &plan.compatibility.lexical_contract_identity,
      &plan.compatibility.lexical_vector_name,
      &plan.compatibility.node_lexical_input_specification,
      &plan.compatibility.edge_lexical_input_specification,
      &node_count,
      &edge_count,
      &endpoint_count,
    ],
  )
  .map_err(KnowledgeReleaseFailure::from)?;
  Ok(BeginPublication {
    build_id,
    canonical: plan.canonical.clone(),
    projection_schema_version: plan.nodes.embedding.payload_schema_version().to_string(),
    node_projection_hash: plan.nodes.content_hash.clone(),
    edge_projection_hash: plan.edges.content_hash.clone(),
    compatibility: plan.compatibility.clone(),
    expected_node_count: plan.nodes.point_count,
    expected_edge_count: plan.edges.point_count,
    expected_endpoint_count: plan.edges.expected_endpoint_count,
    idempotency_key: plan.idempotency_key.clone(),
    request_fingerprint: fingerprint,
  })
}

fn validate_status(
  status: &PublicationStatus,
  build_id: &PublicationBuildId,
  node_batches: u32,
  edge_batches: u32,
) -> Result<(), KnowledgeReleaseFailure> {
  validate_status_progress(status, build_id, node_batches, edge_batches)
}

fn validate_status_progress(
  status: &PublicationStatus,
  build_id: &PublicationBuildId,
  node_batches: u32,
  edge_batches: u32,
) -> Result<(), KnowledgeReleaseFailure> {
  if &status.build_id != build_id
    || status.next_node_ordinal > node_batches
    || status.next_edge_ordinal > edge_batches
  {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  let consistent = match status.state {
    PublicationBuildState::AcceptingNodes => status.next_edge_ordinal == 0,
    PublicationBuildState::NodesFrozen => {
      status.next_node_ordinal == node_batches && status.next_edge_ordinal == 0
    }
    PublicationBuildState::AcceptingEdges => status.next_node_ordinal == node_batches,
    PublicationBuildState::EdgesFrozen
    | PublicationBuildState::Reconciling
    | PublicationBuildState::ActivationCandidate => {
      status.next_node_ordinal == node_batches && status.next_edge_ordinal == edge_batches
    }
    PublicationBuildState::Failed
    | PublicationBuildState::Aborting
    | PublicationBuildState::Abandoned
    | PublicationBuildState::GcEligible => false,
  };
  if consistent {
    Ok(())
  } else if terminal_failure(status.state) {
    Err(KnowledgeReleaseFailure::InvalidLifecycleTransition)
  } else {
    Err(KnowledgeReleaseFailure::IdempotencyConflict)
  }
}

fn batch_count<T>(batches: &[PlannedBatch<T>]) -> Result<u32, KnowledgeReleaseFailure> {
  u32::try_from(batches.len()).map_err(|_| KnowledgeReleaseFailure::IncompleteTrio)
}

fn terminal_failure(state: PublicationBuildState) -> bool {
  matches!(
    state,
    PublicationBuildState::Failed
      | PublicationBuildState::Aborting
      | PublicationBuildState::Abandoned
      | PublicationBuildState::GcEligible
  )
}

#[derive(Clone)]
struct PlannedBatch<T> {
  identity: PublicationBatchIdentity,
  points: Vec<T>,
}

fn plan_batches<T, Inspect>(
  points: &[T],
  family: PublicationCollectionFamily,
  build_id: &PublicationBuildId,
  mut inspect: Inspect,
) -> Result<Vec<PlannedBatch<T>>, KnowledgeReleaseFailure>
where
  T: Clone + PointContentHash,
  Inspect: FnMut(
    PublicationBatchIdentity,
    Vec<T>,
  ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure>,
{
  let mut batches = Vec::new();
  let mut start = 0;
  while start < points.len() {
    let ordinal =
      u32::try_from(batches.len()).map_err(|_| KnowledgeReleaseFailure::IncompleteTrio)?;
    let max_end = (start + MAX_PUBLICATION_BATCH_POINTS).min(points.len());
    let mut accepted = None;
    for end in (start + 1)..=max_end {
      let candidate = points[start..end].to_vec();
      let identity = batch_identity(build_id, family, ordinal, &candidate)?;
      match inspect(identity.clone(), candidate.clone())? {
        PublicationBatchWireAdmission::Fits { .. } => accepted = Some((end, identity, candidate)),
        PublicationBatchWireAdmission::TooLarge { .. } => break,
      }
    }
    let Some((end, identity, batch_points)) = accepted else {
      return Err(KnowledgeReleaseFailure::SchemaIncompatible);
    };
    batches.push(PlannedBatch {
      identity,
      points: batch_points,
    });
    start = end;
  }
  Ok(batches)
}

fn batch_identity<T: PointContentHash>(
  build_id: &PublicationBuildId,
  family: PublicationCollectionFamily,
  ordinal: u32,
  points: &[T],
) -> Result<PublicationBatchIdentity, KnowledgeReleaseFailure> {
  let point_hashes = points
    .iter()
    .map(PointContentHash::content_hash)
    .collect::<Vec<_>>();
  let content_hash = PublicationBatchContentHash::derive(family, &point_hashes)
    .map_err(KnowledgeReleaseFailure::from)?;
  let ordinal_value = ordinal.to_string();
  let family_name = match family {
    PublicationCollectionFamily::Nodes => "nodes",
    PublicationCollectionFamily::Edges => "edges",
  };
  let fingerprint = PublicationRequestFingerprint::derive(
    "batch",
    &[
      build_id.as_str(),
      family_name,
      &ordinal_value,
      content_hash.as_str(),
    ],
  )
  .map_err(KnowledgeReleaseFailure::from)?;
  Ok(PublicationBatchIdentity {
    build_id: build_id.clone(),
    ordinal: PublicationBatchOrdinal::new(ordinal),
    request_fingerprint: fingerprint,
    content_hash,
  })
}

async fn submit_planned_batches<T, Fut, Submit>(
  batches: &[PlannedBatch<T>],
  next_ordinal: u32,
  family: PublicationCollectionFamily,
  build_id: &PublicationBuildId,
  node_batches: u32,
  edge_batches: u32,
  mut submit: Submit,
) -> Result<(), KnowledgeReleaseFailure>
where
  T: Clone + PointContentHash,
  Submit: FnMut(PublicationBatchIdentity, Vec<T>) -> Fut,
  Fut: Future<Output = Result<PublicationStatus, KnowledgeReleaseFailure>>,
{
  let total = batch_count(batches)?;
  if next_ordinal > total {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  for planned in batches.iter().skip(next_ordinal as usize) {
    let ordinal = planned.identity.ordinal.get();
    let status = submit(planned.identity.clone(), planned.points.clone()).await?;
    validate_status_progress(&status, build_id, node_batches, edge_batches)?;
    let observed = match family {
      PublicationCollectionFamily::Nodes => status.next_node_ordinal,
      PublicationCollectionFamily::Edges => status.next_edge_ordinal,
    };
    let expected_state = match family {
      PublicationCollectionFamily::Nodes => PublicationBuildState::AcceptingNodes,
      PublicationCollectionFamily::Edges => PublicationBuildState::AcceptingEdges,
    };
    if observed != ordinal + 1 || status.state != expected_state {
      return Err(KnowledgeReleaseFailure::IdempotencyConflict);
    }
  }
  Ok(())
}

trait PointContentHash {
  fn content_hash(&self) -> &str;
}

impl PointContentHash for NodeProjection {
  fn content_hash(&self) -> &str {
    &self.content_hash
  }
}

impl PointContentHash for EdgeProjection {
  fn content_hash(&self) -> &str {
    &self.content_hash
  }
}

#[cfg(test)]
mod tests {
  use std::{collections::BTreeSet, sync::Mutex, time::Duration};

  use async_trait::async_trait;

  use super::*;
  use crate::domain::{
    canonical::{
      CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind,
      LanguageTag, Lexeme, LexicalPartOfSpeech, LexicalSource, Sense, SourcePermissions,
    },
    canonical_content::{CanonicalEvidenceLineage, CanonicalEvidenceOrigin},
    embedding_input::AuthoritativeEmbeddingMaterial,
    graph::{
      GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphNodeKey, GraphNodeKind,
      GraphRanking, GraphRelationType, GraphScope, GraphScore, GraphScoreComponents,
      PublishedRelationship, RelationVersion, RelationshipVerificationState, StoredGraphRelation,
    },
    knowledge_projection::{
      build_edge_projection, build_node_projection, CanonicalNodeProjectionInput,
      ProjectionEmbeddingSpec, DENSE_DIMENSIONS, DENSE_MODEL_FAMILY, DENSE_VECTOR_NAME,
      LEXICAL_CONTRACT_IDENTITY, LEXICAL_ENCODER_IDENTITY, LEXICAL_ENCODER_REVISION,
      SPARSE_VECTOR_NAME,
    },
    knowledge_publication::{
      BuildExecutionReceipt, DenseExecutionReceipt, LexicalExecutionReceipt,
    },
    knowledge_release::{
      DenseEmbeddingVersion, EdgeCollectionId, EdgeCollectionManifest, KnowledgeReleaseTrio,
      NodeCollectionId, NodeCollectionManifest, ProjectionCollectionState, SparseEmbeddingVersion,
    },
  };

  #[derive(Default)]
  struct FakeState {
    events: Vec<String>,
    status_calls: usize,
    next_node: u32,
    next_edge: u32,
    nodes_frozen: bool,
    edges_frozen: bool,
    candidate_count: usize,
    fail_node_once_at: Option<u32>,
    failed_once: bool,
    fail_reconcile: bool,
    conflict_begin: bool,
    status_build_mismatch: bool,
    status_node_ordinal_override: Option<u32>,
    status_edge_ordinal_override: Option<u32>,
    status_state_override: Option<PublicationBuildState>,
    node_response_edge_ordinal_override: Option<u32>,
    edge_response_node_ordinal_override: Option<u32>,
    node_inspection_point_limit: Option<usize>,
    edge_inspection_point_limit: Option<usize>,
    node_inspections: usize,
    edge_inspections: usize,
    abort_build_mismatch: bool,
    abort_state_override: Option<PublicationBuildState>,
    node_batches: Vec<(u32, String, String, usize)>,
    edge_batches: Vec<(u32, String, String, usize)>,
  }

  struct FakePublicationPort {
    state: Mutex<FakeState>,
  }

  impl FakePublicationPort {
    fn new(state: FakeState) -> Self {
      Self {
        state: Mutex::new(state),
      }
    }

    fn snapshot(&self) -> FakeState {
      let state = self.state.lock().unwrap();
      FakeState {
        events: state.events.clone(),
        status_calls: state.status_calls,
        next_node: state.next_node,
        next_edge: state.next_edge,
        nodes_frozen: state.nodes_frozen,
        edges_frozen: state.edges_frozen,
        candidate_count: state.candidate_count,
        fail_node_once_at: state.fail_node_once_at,
        failed_once: state.failed_once,
        fail_reconcile: state.fail_reconcile,
        conflict_begin: state.conflict_begin,
        status_build_mismatch: state.status_build_mismatch,
        status_node_ordinal_override: state.status_node_ordinal_override,
        status_edge_ordinal_override: state.status_edge_ordinal_override,
        status_state_override: state.status_state_override,
        node_response_edge_ordinal_override: state.node_response_edge_ordinal_override,
        edge_response_node_ordinal_override: state.edge_response_node_ordinal_override,
        node_inspection_point_limit: state.node_inspection_point_limit,
        edge_inspection_point_limit: state.edge_inspection_point_limit,
        node_inspections: state.node_inspections,
        edge_inspections: state.edge_inspections,
        abort_build_mismatch: state.abort_build_mismatch,
        abort_state_override: state.abort_state_override,
        node_batches: state.node_batches.clone(),
        edge_batches: state.edge_batches.clone(),
      }
    }
  }

  #[async_trait]
  impl KnowledgePublicationPort for FakePublicationPort {
    fn inspect_node_batch(
      &self,
      _canonical: &CanonicalReleasePin,
      request: &NodePublicationBatch,
    ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      state.node_inspections += 1;
      let serialized_bytes = request.points.len();
      if request.points.len() <= state.node_inspection_point_limit.unwrap_or(256) {
        Ok(PublicationBatchWireAdmission::Fits { serialized_bytes })
      } else {
        Ok(PublicationBatchWireAdmission::TooLarge { serialized_bytes })
      }
    }

    fn inspect_edge_batch(
      &self,
      _canonical: &CanonicalReleasePin,
      request: &EdgePublicationBatch,
    ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      state.edge_inspections += 1;
      let serialized_bytes = request.points.len();
      if request.points.len() <= state.edge_inspection_point_limit.unwrap_or(256) {
        Ok(PublicationBatchWireAdmission::Fits { serialized_bytes })
      } else {
        Ok(PublicationBatchWireAdmission::TooLarge { serialized_bytes })
      }
    }

    async fn begin(
      &self,
      _context: &KnowledgePublicationContext,
      request: &BeginPublication,
    ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      state.events.push("begin".into());
      if state.conflict_begin {
        return Err(KnowledgeReleaseFailure::IdempotencyConflict);
      }
      Ok(PublicationStatus {
        build_id: request.build_id.clone(),
        state: PublicationBuildState::AcceptingNodes,
        next_node_ordinal: 0,
        next_edge_ordinal: 0,
      })
    }

    async fn submit_nodes(
      &self,
      _context: &KnowledgePublicationContext,
      _canonical: &CanonicalReleasePin,
      request: &NodePublicationBatch,
    ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      assert!(!state.nodes_frozen);
      assert_eq!(request.identity.ordinal.get(), state.next_node);
      if state.fail_node_once_at == Some(state.next_node) && !state.failed_once {
        state.failed_once = true;
        return Err(KnowledgeReleaseFailure::DependencyUnavailable);
      }
      let ordinal = state.next_node;
      state.events.push(format!("nodes:{ordinal}"));
      state.node_batches.push((
        request.identity.ordinal.get(),
        request.identity.content_hash.as_str().to_string(),
        request.identity.request_fingerprint.as_str().to_string(),
        request.points.len(),
      ));
      state.next_node += 1;
      Ok(PublicationStatus {
        build_id: request.identity.build_id.clone(),
        state: PublicationBuildState::AcceptingNodes,
        next_node_ordinal: state.next_node,
        next_edge_ordinal: state
          .node_response_edge_ordinal_override
          .unwrap_or(state.next_edge),
      })
    }

    async fn freeze_nodes(
      &self,
      _context: &KnowledgePublicationContext,
      canonical: &CanonicalReleasePin,
      request: &FreezeNodes,
    ) -> Result<crate::ports::knowledge_publication::FrozenNodePublication, KnowledgeReleaseFailure>
    {
      let mut state = self.state.lock().unwrap();
      state.events.push("freeze_nodes".into());
      state.nodes_frozen = true;
      Ok(crate::ports::knowledge_publication::FrozenNodePublication {
        manifest: NodeCollectionManifest {
          release_id: canonical.release_id.clone(),
          collection_id: NodeCollectionId::parse("nodes-r1").unwrap(),
          payload_schema_version: request.projection_schema_version.clone(),
          content_hash: request.projection_hash.clone(),
          point_count: request.point_count,
          state: ProjectionCollectionState::Verified,
        },
        persisted_hash: request.persisted_hash.clone(),
        receipt: receipt(
          &request.build_id,
          PublicationCollectionFamily::Nodes,
          &request.compatibility,
          &request.dictionary,
          request.point_count,
        ),
        dictionary: request.dictionary.clone(),
      })
    }

    async fn submit_edges(
      &self,
      _context: &KnowledgePublicationContext,
      _canonical: &CanonicalReleasePin,
      request: &EdgePublicationBatch,
    ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      if !state.nodes_frozen {
        return Err(KnowledgeReleaseFailure::InvalidLifecycleTransition);
      }
      assert_eq!(request.identity.ordinal.get(), state.next_edge);
      let ordinal = state.next_edge;
      state.events.push(format!("edges:{ordinal}"));
      state.edge_batches.push((
        request.identity.ordinal.get(),
        request.identity.content_hash.as_str().to_string(),
        request.identity.request_fingerprint.as_str().to_string(),
        request.points.len(),
      ));
      state.next_edge += 1;
      Ok(PublicationStatus {
        build_id: request.identity.build_id.clone(),
        state: PublicationBuildState::AcceptingEdges,
        next_node_ordinal: state
          .edge_response_node_ordinal_override
          .unwrap_or(state.next_node),
        next_edge_ordinal: state.next_edge,
      })
    }

    async fn freeze_edges(
      &self,
      _context: &KnowledgePublicationContext,
      canonical: &CanonicalReleasePin,
      request: &FreezeEdges,
    ) -> Result<crate::ports::knowledge_publication::FrozenEdgePublication, KnowledgeReleaseFailure>
    {
      let mut state = self.state.lock().unwrap();
      if !state.nodes_frozen {
        return Err(KnowledgeReleaseFailure::InvalidLifecycleTransition);
      }
      state.events.push("freeze_edges".into());
      state.edges_frozen = true;
      Ok(crate::ports::knowledge_publication::FrozenEdgePublication {
        manifest: EdgeCollectionManifest {
          release_id: canonical.release_id.clone(),
          collection_id: EdgeCollectionId::parse("edges-r1").unwrap(),
          payload_schema_version: request.projection_schema_version.clone(),
          content_hash: request.projection_hash.clone(),
          point_count: request.point_count,
          verified_node_content_hash: request.verified_node_projection_hash.clone(),
          expected_endpoint_count: request.expected_endpoint_count,
          resolved_endpoint_count: request.resolved_endpoint_count,
          state: ProjectionCollectionState::Verified,
        },
        persisted_hash: request.persisted_hash.clone(),
        receipt: receipt(
          &request.build_id,
          PublicationCollectionFamily::Edges,
          &request.compatibility,
          &request.dictionary,
          request.point_count,
        ),
        dictionary: request.dictionary.clone(),
      })
    }

    async fn reconcile(
      &self,
      _context: &KnowledgePublicationContext,
      request: &ReconcilePublication,
    ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      if !state.nodes_frozen || !state.edges_frozen {
        return Err(KnowledgeReleaseFailure::IncompleteTrio);
      }
      state.events.push("reconcile".into());
      if state.fail_reconcile {
        return Err(KnowledgeReleaseFailure::HashOrCountReconciliationFailed);
      }
      let trio = KnowledgeReleaseTrio::try_from_parts(
        request.canonical.clone(),
        Some(request.nodes.manifest.clone()),
        Some(request.edges.manifest.clone()),
        DenseEmbeddingVersion {
          model_family: request.compatibility.dense_model_family.clone(),
          artifact_revision: request.compatibility.dense_artifact_revision.clone(),
          dimensions: request.compatibility.dense_dimensions,
        },
        SparseEmbeddingVersion {
          encoder_identity: request.compatibility.lexical_encoder_identity.clone(),
          encoder_revision: request.compatibility.lexical_encoder_revision.clone(),
        },
      )
      .map_err(|_| KnowledgeReleaseFailure::IncompleteTrio)?;
      state.candidate_count += 1;
      Ok(PublicationActivationCandidate {
        build_id: request.build_id.clone(),
        reconcile_id: request.reconcile_id.clone(),
        trio,
        canonical_content_hash: request.canonical_content_hash.clone(),
        manifest_hash: request.manifest_hash.clone(),
        node_persisted_hash: request.nodes.persisted_hash.clone(),
        edge_persisted_hash: request.edges.persisted_hash.clone(),
      })
    }

    async fn status(
      &self,
      _context: &KnowledgePublicationContext,
      _canonical: &CanonicalReleasePin,
      build_id: &PublicationBuildId,
    ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      state.status_calls += 1;
      state.events.push("status".into());
      let inferred_lifecycle = if state.candidate_count > 0 {
        PublicationBuildState::ActivationCandidate
      } else if state.edges_frozen {
        PublicationBuildState::EdgesFrozen
      } else if state.nodes_frozen {
        PublicationBuildState::AcceptingEdges
      } else {
        PublicationBuildState::AcceptingNodes
      };
      let lifecycle = state.status_state_override.unwrap_or(inferred_lifecycle);
      let returned_build = if state.status_build_mismatch {
        PublicationBuildId::parse(
          "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .unwrap()
      } else {
        build_id.clone()
      };
      Ok(PublicationStatus {
        build_id: returned_build,
        state: lifecycle,
        next_node_ordinal: state
          .status_node_ordinal_override
          .unwrap_or(state.next_node),
        next_edge_ordinal: state
          .status_edge_ordinal_override
          .unwrap_or(state.next_edge),
      })
    }

    async fn abort(
      &self,
      _context: &KnowledgePublicationContext,
      _canonical: &CanonicalReleasePin,
      build_id: &PublicationBuildId,
      _idempotency_key: &PublicationIdempotencyKey,
      fingerprint: &PublicationRequestFingerprint,
    ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
      let mut state = self.state.lock().unwrap();
      state.events.push(format!("abort:{}", fingerprint.as_str()));
      let returned_build_id = if state.abort_build_mismatch {
        PublicationBuildId::parse(
          "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .unwrap()
      } else {
        build_id.clone()
      };
      Ok(PublicationStatus {
        build_id: returned_build_id,
        state: state
          .abort_state_override
          .unwrap_or(PublicationBuildState::Abandoned),
        next_node_ordinal: state.next_node,
        next_edge_ordinal: state.next_edge,
      })
    }
  }

  fn id(value: impl AsRef<str>) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn permissions() -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding: true,
      model_processing: false,
      api_redistribution: true,
    }
  }

  fn material(index: usize, release: &str) -> AuthoritativeEmbeddingMaterial {
    let evidence_id = id(format!("evidence-{index}"));
    let definition_evidence_id = id(format!("definition-evidence-{index}"));
    let source_id = id(format!("source-{index}"));
    let lexeme = Lexeme {
      id: id(format!("lexeme-{index}")),
      release_id: id(release),
      language: LanguageTag::parse("en").unwrap(),
      lemma: format!("term{index}"),
      lemma_evidence_ids: vec![evidence_id.clone()],
      normalized_lemma: format!("term{index}"),
      part_of_speech: LexicalPartOfSpeech::Noun,
      status: CanonicalStatus::Active,
    };
    let sense = Sense {
      id: id(format!("sense-{index}")),
      lexeme_id: lexeme.id.clone(),
      release_id: id(release),
      sense_key: "1".into(),
      definition: format!("Definition {index}"),
      definition_evidence_ids: vec![definition_evidence_id.clone()],
      status: CanonicalStatus::Active,
    };
    let evidence = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".into(),
        version: "2026-01".into(),
        license: "reviewed".into(),
        attribution: Some("Reviewed source".into()),
        permissions: permissions(),
      },
      EvidenceFragment {
        id: evidence_id,
        source_id,
        source_reference: "entry".into(),
        release_id: id(release),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Reviewed evidence".into(),
        content_hash: format!("sha256:evidence-{index}"),
        permissions: permissions(),
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap();
    let definition_evidence = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: id(format!("definition-source-{index}")),
        name: "Reviewed source".into(),
        version: "2026-01".into(),
        license: "reviewed".into(),
        attribution: Some("Reviewed source".into()),
        permissions: permissions(),
      },
      EvidenceFragment {
        id: definition_evidence_id,
        source_id: id(format!("definition-source-{index}")),
        source_reference: "definition".into(),
        release_id: id(release),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Reviewed definition".into(),
        content_hash: format!("sha256:definition-{index}"),
        permissions: permissions(),
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap();
    AuthoritativeEmbeddingMaterial::new(
      lexeme,
      Some(sense),
      vec![],
      vec![],
      vec![],
      vec![evidence, definition_evidence],
    )
    .unwrap()
  }

  fn embedding() -> ProjectionEmbeddingSpec {
    ProjectionEmbeddingSpec::new(
      "knowledge-graph-v1",
      DenseEmbeddingVersion {
        model_family: DENSE_MODEL_FAMILY.into(),
        artifact_revision: "sha256:test-artifact-r1".into(),
        dimensions: DENSE_DIMENSIONS,
      },
      SparseEmbeddingVersion {
        encoder_identity: LEXICAL_ENCODER_IDENTITY.into(),
        encoder_revision: LEXICAL_ENCODER_REVISION.into(),
      },
    )
    .unwrap()
  }

  fn compatibility() -> EmbeddingCompatibilityEntry {
    EmbeddingCompatibilityEntry {
      entry_id: "registry-test-r1".into(),
      dense_model_family: DENSE_MODEL_FAMILY.into(),
      dense_artifact_revision: "sha256:test-artifact-r1".into(),
      dense_dimensions: DENSE_DIMENSIONS,
      dense_vector_name: DENSE_VECTOR_NAME.into(),
      node_dense_input_specification: "node-dense-input-v1".into(),
      edge_dense_input_specification: "edge-dense-input-v1".into(),
      lexical_encoder_identity: LEXICAL_ENCODER_IDENTITY.into(),
      lexical_encoder_revision: LEXICAL_ENCODER_REVISION.into(),
      lexical_contract_identity: LEXICAL_CONTRACT_IDENTITY.into(),
      lexical_vector_name: SPARSE_VECTOR_NAME.into(),
      node_lexical_input_specification: "node-lexical-input-v1".into(),
      edge_lexical_input_specification: "edge-lexical-input-v1".into(),
    }
  }

  fn relationship() -> PublishedRelationship {
    relationship_to(1)
  }

  fn relationship_to(target_index: usize) -> PublishedRelationship {
    let release = id("release-r1");
    let evidence_id = id(format!("edge-evidence-{target_index}"));
    let source_id = id(format!("edge-source-{target_index}"));
    let relation_type = GraphRelationType::Hypernym;
    let rule = relation_type.rule();
    PublishedRelationship {
      relation: StoredGraphRelation {
        edge_id: GraphEdgeId::stored(id(format!("edge-{target_index}"))),
        relation_version: RelationVersion::new(1).unwrap(),
        source: GraphNodeKey::new(GraphNodeKind::Sense, id("sense-0")),
        target: GraphNodeKey::new(GraphNodeKind::Sense, id(format!("sense-{target_index}"))),
        relation_type,
        evidence: GraphEvidence::new(vec![evidence_id.clone()], EvidenceConfidence::High).unwrap(),
        scope: GraphScope::default(),
        feedback_capabilities: BTreeSet::from([GraphFeedbackCapability::Accuracy]),
        ranking: GraphRanking {
          display_rank: GraphScore::new(9000).unwrap(),
          components: GraphScoreComponents {
            evidence: GraphScore::new(9000).unwrap(),
            community: None,
            pedagogical: None,
          },
          ranking_version: "graph-rank-v1".into(),
        },
      },
      relation_release_id: release.clone(),
      source_release_id: release.clone(),
      target_release_id: release.clone(),
      declared_wire_relation: rule.require_qdrant_wire_name().unwrap().into(),
      declared_inverse: rule.inverse,
      evidence_lineage: vec![CanonicalEvidenceLineage::new(
        LexicalSource {
          id: source_id.clone(),
          name: "Reviewed source".into(),
          version: "2026-01".into(),
          license: "reviewed".into(),
          attribution: Some("Reviewed source".into()),
          permissions: permissions(),
        },
        EvidenceFragment {
          id: evidence_id,
          source_id,
          source_reference: format!("edge-{target_index}"),
          release_id: release,
          language: LanguageTag::parse("en").unwrap(),
          kind: EvidenceKind::Definition,
          confidence: EvidenceConfidence::High,
          text: "Reviewed edge evidence".into(),
          content_hash: "sha256:edge-evidence".into(),
          permissions: permissions(),
          status: CanonicalStatus::Active,
        },
        CanonicalEvidenceOrigin::LicensedSource,
      )
      .unwrap()],
      verification_state: RelationshipVerificationState::Verified,
    }
  }

  fn plan(node_count: usize) -> KnowledgePublicationPlan {
    let release = id("release-r1");
    let nodes = build_node_projection(
      release.clone(),
      embedding(),
      (0..node_count)
        .map(|index| CanonicalNodeProjectionInput::Sense(material(index, "release-r1")))
        .collect(),
    )
    .unwrap();
    let edges = build_edge_projection(&nodes, embedding(), vec![relationship()]).unwrap();
    KnowledgePublicationPlan {
      canonical: CanonicalReleasePin::new(release, "canonical-v1".into()).unwrap(),
      canonical_content_hash: format!("sha256:{}", "9".repeat(64)),
      nodes,
      edges,
      compatibility: compatibility(),
      node_dictionary: LexicalDictionaryManifest::new(vec![]).unwrap(),
      edge_dictionary: LexicalDictionaryManifest::new(vec![]).unwrap(),
      idempotency_key: PublicationIdempotencyKey::parse("publication-r1").unwrap(),
    }
  }

  fn plan_with_edge_count(node_count: usize, edge_count: usize) -> KnowledgePublicationPlan {
    let mut plan = plan(node_count.max(edge_count + 1));
    plan.edges = build_edge_projection(
      &plan.nodes,
      embedding(),
      (1..=edge_count).map(relationship_to).collect(),
    )
    .unwrap();
    plan
  }

  fn context() -> KnowledgePublicationContext {
    KnowledgePublicationContext {
      request_id: "request-r1".into(),
      deadline_at: "2099-01-01T00:00:00Z".into(),
      timeout: Duration::from_secs(5),
    }
  }

  fn receipt(
    build_id: &PublicationBuildId,
    family: PublicationCollectionFamily,
    compatibility: &EmbeddingCompatibilityEntry,
    dictionary: &LexicalDictionaryManifest,
    count: u64,
  ) -> BuildExecutionReceipt {
    let (dense_input, lexical_input) = match family {
      PublicationCollectionFamily::Nodes => ("node-dense-input-v1", "node-lexical-input-v1"),
      PublicationCollectionFamily::Edges => ("edge-dense-input-v1", "edge-lexical-input-v1"),
    };
    BuildExecutionReceipt {
      build_id: build_id.clone(),
      family,
      dense: DenseExecutionReceipt {
        registry_entry_id: compatibility.entry_id.clone(),
        observed_artifact_revision: compatibility.dense_artifact_revision.clone(),
        dimensions: compatibility.dense_dimensions,
        vector_name: DENSE_VECTOR_NAME.into(),
        input_specification: dense_input.into(),
        processed_point_count: count,
      },
      lexical: LexicalExecutionReceipt {
        encoder_identity: compatibility.lexical_encoder_identity.clone(),
        encoder_revision: compatibility.lexical_encoder_revision.clone(),
        vector_name: SPARSE_VECTOR_NAME.into(),
        input_specification: lexical_input.into(),
        dictionary_hash: dictionary.dictionary_hash().into(),
        processed_point_count: count,
      },
    }
  }

  async fn assert_publish_error(
    service: &KnowledgePublicationService,
    plan: &KnowledgePublicationPlan,
    expected: KnowledgeReleaseFailure,
  ) {
    match service.publish(&context(), plan).await {
      Ok(_) => panic!("publication unexpectedly produced an activation candidate"),
      Err(error) => assert_eq!(error, expected),
    }
  }

  #[tokio::test]
  async fn complete_flow_is_node_first_and_returns_candidate() {
    let port = Arc::new(FakePublicationPort::new(FakeState::default()));
    let service = KnowledgePublicationService::new(port.clone());
    let candidate = service.publish(&context(), &plan(2)).await.unwrap();
    assert_eq!(candidate.trio.canonical().release_id, id("release-r1"));
    assert_eq!(
      port.snapshot().events,
      [
        "begin",
        "status",
        "nodes:0",
        "freeze_nodes",
        "edges:0",
        "freeze_edges",
        "reconcile"
      ]
    );
  }

  #[tokio::test]
  async fn node_batches_are_bounded_and_ordered() {
    let port = Arc::new(FakePublicationPort::new(FakeState::default()));
    let service = KnowledgePublicationService::new(port.clone());
    service.publish(&context(), &plan(257)).await.unwrap();
    let state = port.snapshot();
    assert_eq!(state.next_node, 2);
    assert_eq!(
      state
        .node_batches
        .iter()
        .map(|batch| batch.3)
        .collect::<Vec<_>>(),
      [256, 1]
    );
    assert!(
      state
        .events
        .iter()
        .position(|event| event == "freeze_nodes")
        .unwrap()
        < state
          .events
          .iter()
          .position(|event| event == "edges:0")
          .unwrap()
    );
  }

  #[tokio::test]
  async fn exactly_256_small_points_form_one_batch() {
    let port = Arc::new(FakePublicationPort::new(FakeState::default()));
    let service = KnowledgePublicationService::new(port.clone());
    service.publish(&context(), &plan(256)).await.unwrap();
    assert_eq!(port.snapshot().node_batches[0].3, 256);
    assert_eq!(port.snapshot().node_batches.len(), 1);
  }

  #[tokio::test]
  async fn wire_size_admission_splits_node_and_edge_families() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      node_inspection_point_limit: Some(2),
      edge_inspection_point_limit: Some(2),
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    service
      .publish(&context(), &plan_with_edge_count(5, 5))
      .await
      .unwrap();
    let state = port.snapshot();
    assert_eq!(
      state
        .node_batches
        .iter()
        .map(|batch| batch.3)
        .collect::<Vec<_>>(),
      [2, 2, 2]
    );
    assert_eq!(
      state
        .edge_batches
        .iter()
        .map(|batch| batch.3)
        .collect::<Vec<_>>(),
      [2, 2, 1]
    );
  }

  #[tokio::test]
  async fn repeated_size_aware_planning_preserves_boundaries_and_identities() {
    let plan = plan_with_edge_count(5, 5);
    let first = Arc::new(FakePublicationPort::new(FakeState {
      node_inspection_point_limit: Some(2),
      edge_inspection_point_limit: Some(2),
      ..FakeState::default()
    }));
    let second = Arc::new(FakePublicationPort::new(FakeState {
      node_inspection_point_limit: Some(2),
      edge_inspection_point_limit: Some(2),
      ..FakeState::default()
    }));
    KnowledgePublicationService::new(first.clone())
      .publish(&context(), &plan)
      .await
      .unwrap();
    KnowledgePublicationService::new(second.clone())
      .publish(&context(), &plan)
      .await
      .unwrap();
    assert_eq!(
      first.snapshot().node_batches,
      second.snapshot().node_batches
    );
    assert_eq!(
      first.snapshot().edge_batches,
      second.snapshot().edge_batches
    );
  }

  #[tokio::test]
  async fn size_aware_resume_uses_the_final_plan_ordinal() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      next_node: 1,
      node_inspection_point_limit: Some(2),
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    service.publish(&context(), &plan(5)).await.unwrap();
    assert_eq!(
      port
        .snapshot()
        .node_batches
        .iter()
        .map(|batch| batch.0)
        .collect::<Vec<_>>(),
      [1, 2]
    );
  }

  #[tokio::test]
  async fn one_point_too_large_fails_before_any_mutation() {
    for state in [
      FakeState {
        node_inspection_point_limit: Some(0),
        ..FakeState::default()
      },
      FakeState {
        edge_inspection_point_limit: Some(0),
        ..FakeState::default()
      },
    ] {
      let port = Arc::new(FakePublicationPort::new(state));
      let service = KnowledgePublicationService::new(port.clone());
      assert_publish_error(
        &service,
        &plan(2),
        KnowledgeReleaseFailure::SchemaIncompatible,
      )
      .await;
      assert!(port.snapshot().events.is_empty());
    }
  }

  #[tokio::test]
  async fn interrupted_build_resumes_from_authoritative_ordinal() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      fail_node_once_at: Some(1),
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    let plan = plan(257);
    assert_publish_error(
      &service,
      &plan,
      KnowledgeReleaseFailure::DependencyUnavailable,
    )
    .await;
    service.publish(&context(), &plan).await.unwrap();
    let events = &port.snapshot().events;
    assert_eq!(port.snapshot().status_calls, 2);
    assert_eq!(events.iter().filter(|event| *event == "nodes:0").count(), 1);
    assert_eq!(events.iter().filter(|event| *event == "nodes:1").count(), 1);
  }

  #[tokio::test]
  async fn completed_duplicate_retry_replays_without_resubmitting_batches() {
    let port = Arc::new(FakePublicationPort::new(FakeState::default()));
    let service = KnowledgePublicationService::new(port.clone());
    let plan = plan(2);
    service.publish(&context(), &plan).await.unwrap();
    service.publish(&context(), &plan).await.unwrap();
    let events = &port.snapshot().events;
    assert_eq!(port.snapshot().status_calls, 2);
    assert_eq!(events.iter().filter(|event| *event == "nodes:0").count(), 1);
    assert_eq!(events.iter().filter(|event| *event == "edges:0").count(), 1);
  }

  #[tokio::test]
  async fn conflicting_retry_fails_closed_before_mutating_batches() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      conflict_begin: true,
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    assert_publish_error(
      &service,
      &plan(2),
      KnowledgeReleaseFailure::IdempotencyConflict,
    )
    .await;
    assert_eq!(port.snapshot().events, ["begin"]);
  }

  #[tokio::test]
  async fn resume_uses_authoritative_status_and_skips_two_accepted_batches() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      next_node: 2,
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    service.publish(&context(), &plan(769)).await.unwrap();
    let state = port.snapshot();
    assert_eq!(state.status_calls, 1);
    assert!(!state.events.iter().any(|event| event == "nodes:0"));
    assert!(!state.events.iter().any(|event| event == "nodes:1"));
    assert_eq!(
      state
        .events
        .iter()
        .filter(|event| event.starts_with("nodes:"))
        .cloned()
        .collect::<Vec<_>>(),
      ["nodes:2", "nodes:3"]
    );
  }

  #[tokio::test]
  async fn conflicting_authority_status_fails_closed_before_submission() {
    for state in [
      FakeState {
        status_build_mismatch: true,
        ..FakeState::default()
      },
      FakeState {
        status_node_ordinal_override: Some(2),
        ..FakeState::default()
      },
    ] {
      let port = Arc::new(FakePublicationPort::new(state));
      let service = KnowledgePublicationService::new(port.clone());
      assert_publish_error(
        &service,
        &plan(2),
        KnowledgeReleaseFailure::IdempotencyConflict,
      )
      .await;
      let snapshot = port.snapshot();
      assert_eq!(snapshot.status_calls, 1);
      assert!(!snapshot
        .events
        .iter()
        .any(|event| event.starts_with("nodes:")));
    }
  }

  #[tokio::test]
  async fn impossible_cross_family_progress_fails_before_every_mutation() {
    let challenge_states = [
      FakeState {
        status_state_override: Some(PublicationBuildState::AcceptingNodes),
        status_edge_ordinal_override: Some(1),
        ..FakeState::default()
      },
      FakeState {
        status_state_override: Some(PublicationBuildState::NodesFrozen),
        status_node_ordinal_override: Some(1),
        ..FakeState::default()
      },
      FakeState {
        status_state_override: Some(PublicationBuildState::AcceptingEdges),
        status_node_ordinal_override: Some(1),
        status_edge_ordinal_override: Some(1),
        ..FakeState::default()
      },
      FakeState {
        status_state_override: Some(PublicationBuildState::EdgesFrozen),
        status_node_ordinal_override: Some(2),
        status_edge_ordinal_override: Some(0),
        ..FakeState::default()
      },
      FakeState {
        status_state_override: Some(PublicationBuildState::Reconciling),
        status_node_ordinal_override: Some(1),
        status_edge_ordinal_override: Some(1),
        ..FakeState::default()
      },
      FakeState {
        status_state_override: Some(PublicationBuildState::Reconciling),
        status_node_ordinal_override: Some(2),
        status_edge_ordinal_override: Some(0),
        ..FakeState::default()
      },
    ];
    for state in challenge_states {
      let port = Arc::new(FakePublicationPort::new(state));
      let service = KnowledgePublicationService::new(port.clone());
      assert_publish_error(
        &service,
        &plan(257),
        KnowledgeReleaseFailure::IdempotencyConflict,
      )
      .await;
      assert_eq!(port.snapshot().events, ["begin", "status"]);
    }
  }

  #[tokio::test]
  async fn invalid_node_batch_response_stops_before_following_mutation() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      node_response_edge_ordinal_override: Some(1),
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    assert_publish_error(
      &service,
      &plan(257),
      KnowledgeReleaseFailure::IdempotencyConflict,
    )
    .await;
    assert_eq!(port.snapshot().events, ["begin", "status", "nodes:0"]);
  }

  #[tokio::test]
  async fn invalid_edge_batch_response_stops_before_following_mutation() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      next_node: 1,
      nodes_frozen: true,
      edge_response_node_ordinal_override: Some(0),
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    assert_publish_error(
      &service,
      &plan(2),
      KnowledgeReleaseFailure::IdempotencyConflict,
    )
    .await;
    assert_eq!(
      port.snapshot().events,
      ["begin", "status", "freeze_nodes", "edges:0"]
    );
  }

  #[tokio::test]
  async fn reconcile_failure_never_returns_partial_candidate() {
    let port = Arc::new(FakePublicationPort::new(FakeState {
      fail_reconcile: true,
      ..FakeState::default()
    }));
    let service = KnowledgePublicationService::new(port.clone());
    assert_publish_error(
      &service,
      &plan(2),
      KnowledgeReleaseFailure::HashOrCountReconciliationFailed,
    )
    .await;
    assert_eq!(port.snapshot().candidate_count, 0);
  }

  #[tokio::test]
  async fn explicit_status_and_abort_use_only_authoritative_state() {
    let port = Arc::new(FakePublicationPort::new(FakeState::default()));
    let service = KnowledgePublicationService::new(port.clone());
    let plan = plan(2);
    let build_id = PublicationBuildId::derive(
      &plan.canonical.release_id,
      plan.nodes.embedding.payload_schema_version(),
      &plan.nodes.content_hash,
      &plan.compatibility.entry_id,
    )
    .unwrap();
    let status = service
      .status(&context(), &plan.canonical, &build_id)
      .await
      .unwrap();
    assert_eq!(status.state, PublicationBuildState::AcceptingNodes);

    let aborted = service
      .abort(
        &context(),
        &plan.canonical,
        &build_id,
        &PublicationIdempotencyKey::parse("abort-offline-r1").unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(aborted.state, PublicationBuildState::Abandoned);
    let events = &port.snapshot().events;
    assert_eq!(events[0], "status");
    assert!(events[1].starts_with("abort:sha256:"));
  }

  #[tokio::test]
  async fn abort_rejects_wrong_identity_and_illegal_returned_state() {
    let plan = plan(2);
    let build_id = PublicationBuildId::derive(
      &plan.canonical.release_id,
      plan.nodes.embedding.payload_schema_version(),
      &plan.nodes.content_hash,
      &plan.compatibility.entry_id,
    )
    .unwrap();
    let key = PublicationIdempotencyKey::parse("abort-offline-r2").unwrap();

    let wrong_identity =
      KnowledgePublicationService::new(Arc::new(FakePublicationPort::new(FakeState {
        abort_build_mismatch: true,
        ..FakeState::default()
      })));
    assert_eq!(
      wrong_identity
        .abort(&context(), &plan.canonical, &build_id, &key)
        .await
        .unwrap_err(),
      KnowledgeReleaseFailure::IdempotencyConflict
    );

    let wrong_state =
      KnowledgePublicationService::new(Arc::new(FakePublicationPort::new(FakeState {
        abort_state_override: Some(PublicationBuildState::ActivationCandidate),
        ..FakeState::default()
      })));
    assert_eq!(
      wrong_state
        .abort(&context(), &plan.canonical, &build_id, &key)
        .await
        .unwrap_err(),
      KnowledgeReleaseFailure::InvalidLifecycleTransition
    );
  }
}
