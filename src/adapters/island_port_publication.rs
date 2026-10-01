//! Strict outbound island-port adapter for immutable knowledge publication operations.

use std::{fmt, sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
  adapters::island_port::{IslandPortClientError, IslandPortTransport},
  domain::{
    canonical::{CanonicalReleasePin, EvidenceConfidence, LexicalPartOfSpeech},
    graph::{GraphNodeKind, RelationshipVerificationState},
    knowledge_projection::{
      EdgeProjection, EmbeddingCompatibilityEntry, EmbeddingCompatibilityRegistry, NodeProjection,
      NodeProjectionPayload,
    },
    knowledge_publication::{
      BuildExecutionReceipt, DenseExecutionReceipt, LexicalExecutionReceipt,
      PersistedCollectionHash, PublicationBuildId, PublicationBuildState,
      PublicationCollectionFamily, PublicationManifestHash, PublicationRequestFingerprint,
    },
    knowledge_release::{
      DenseEmbeddingVersion, EdgeCollectionId, EdgeCollectionManifest, KnowledgeReleaseFailure,
      KnowledgeReleaseTrio, NodeCollectionId, NodeCollectionManifest, ProjectionCollectionState,
      SparseEmbeddingVersion,
    },
  },
  ports::knowledge_publication::{
    BeginPublication, EdgePublicationBatch, FreezeEdges, FreezeNodes, FrozenEdgePublication,
    FrozenNodePublication, KnowledgePublicationContext, KnowledgePublicationPort,
    NodePublicationBatch, PublicationActivationCandidate, PublicationBatchWireAdmission,
    PublicationStatus, ReconcilePublication, MAX_PUBLICATION_BATCH_POINTS,
    MAX_PUBLICATION_BATCH_REQUEST_BYTES, MAX_PUBLICATION_DEADLINE_BYTES,
    MAX_PUBLICATION_REQUEST_ID_BYTES,
  },
};

const SCHEMA_VERSION: &str = "knowledge-publication-v1";
const MAX_CONTROL_REQUEST_BYTES: usize = 256 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const PLANNING_DEADLINE_AT: &str =
  "9999-12-31T23:59:59.99999999999999999999999999999999999999+23:59";

const BEGIN_PATH: &str = "/api/v1/knowledge-publications/begin";
const NODE_BATCH_PATH: &str = "/api/v1/knowledge-publications/nodes/batch";
const NODE_FREEZE_PATH: &str = "/api/v1/knowledge-publications/nodes/freeze";
const EDGE_BATCH_PATH: &str = "/api/v1/knowledge-publications/edges/batch";
const EDGE_FREEZE_PATH: &str = "/api/v1/knowledge-publications/edges/freeze";
const RECONCILE_PATH: &str = "/api/v1/knowledge-publications/reconcile";
const STATUS_PATH: &str = "/api/v1/knowledge-publications/status";
const ABORT_PATH: &str = "/api/v1/knowledge-publications/abort";

/// Strict outbound publication client with no Qdrant, MySQL, or activation capability.
pub struct IslandPortPublicationClient {
  transport: Arc<dyn IslandPortTransport>,
}

impl IslandPortPublicationClient {
  /// Creates a publication client over the shared bounded island-port transport.
  pub fn new(transport: Arc<dyn IslandPortTransport>) -> Self {
    Self { transport }
  }

  async fn call<T: Serialize, R: DeserializeOwned>(
    &self,
    path: &'static str,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    input: T,
    request_limit: usize,
  ) -> Result<PublicationResponseDto<R>, KnowledgeReleaseFailure> {
    let context_dto = PublicationContextDto::new(context, canonical)?;
    let body = serialize_request(context_dto, input)?;
    if body.len() > request_limit {
      return Err(KnowledgeReleaseFailure::SchemaIncompatible);
    }
    let timeout = effective_timeout(context)?;
    let bytes = self
      .transport
      .post_json(path, body, timeout)
      .await
      .map_err(map_transport_error)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
      return Err(KnowledgeReleaseFailure::SchemaIncompatible);
    }
    let response: PublicationResponseDto<R> =
      serde_json::from_slice(&bytes).map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)?;
    response.validate_context(context, canonical)?;
    Ok(response)
  }
}

impl fmt::Debug for IslandPortPublicationClient {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortPublicationClient([redacted])")
  }
}

#[async_trait]
impl KnowledgePublicationPort for IslandPortPublicationClient {
  fn inspect_node_batch(
    &self,
    canonical: &CanonicalReleasePin,
    request: &NodePublicationBatch,
  ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure> {
    validate_batch(&request.points)?;
    validate_batch_identity(&request.identity, "nodes")?;
    inspect_batch(canonical, BatchInputDto::nodes(request)?)
  }

  fn inspect_edge_batch(
    &self,
    canonical: &CanonicalReleasePin,
    request: &EdgePublicationBatch,
  ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure> {
    validate_batch(&request.points)?;
    validate_batch_identity(&request.identity, "edges")?;
    inspect_batch(canonical, BatchInputDto::edges(request)?)
  }

  async fn begin(
    &self,
    context: &KnowledgePublicationContext,
    request: &BeginPublication,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    validate_begin(request)?;
    let input = BeginInputDto::from_domain(request);
    let response = self
      .call(
        BEGIN_PATH,
        context,
        &request.canonical,
        input,
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    begin_value(response.value()?, request)
  }

  async fn submit_nodes(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &NodePublicationBatch,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    validate_batch(&request.points)?;
    validate_batch_identity(&request.identity, "nodes")?;
    let input = BatchInputDto::nodes(request)?;
    let response = self
      .call(
        NODE_BATCH_PATH,
        context,
        canonical,
        input,
        MAX_PUBLICATION_BATCH_REQUEST_BYTES,
      )
      .await?;
    batch_value(response.value()?, &request.identity)
  }

  async fn freeze_nodes(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &FreezeNodes,
  ) -> Result<FrozenNodePublication, KnowledgeReleaseFailure> {
    validate_compatibility(&request.compatibility)?;
    let input = FreezeInputDto::nodes(request);
    let response = self
      .call(
        NODE_FREEZE_PATH,
        context,
        canonical,
        input,
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    let value: FreezeValueDto = response.value()?;
    value.nodes(canonical, request)
  }

  async fn submit_edges(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &EdgePublicationBatch,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    validate_batch(&request.points)?;
    validate_batch_identity(&request.identity, "edges")?;
    let input = BatchInputDto::edges(request)?;
    let response = self
      .call(
        EDGE_BATCH_PATH,
        context,
        canonical,
        input,
        MAX_PUBLICATION_BATCH_REQUEST_BYTES,
      )
      .await?;
    batch_value(response.value()?, &request.identity)
  }

  async fn freeze_edges(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &FreezeEdges,
  ) -> Result<FrozenEdgePublication, KnowledgeReleaseFailure> {
    validate_compatibility(&request.compatibility)?;
    let input = FreezeInputDto::edges(request);
    let response = self
      .call(
        EDGE_FREEZE_PATH,
        context,
        canonical,
        input,
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    let value: FreezeValueDto = response.value()?;
    value.edges(canonical, request)
  }

  async fn reconcile(
    &self,
    context: &KnowledgePublicationContext,
    request: &ReconcilePublication,
  ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure> {
    let input = ReconcileInputDto::from_domain(request);
    let response = self
      .call(
        RECONCILE_PATH,
        context,
        &request.canonical,
        input,
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    let value: ReconcileValueDto = response.value()?;
    value.into_domain(request)
  }

  async fn status(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    let response = self
      .call(
        STATUS_PATH,
        context,
        canonical,
        BuildInputDto {
          build_id: build_id.as_str(),
        },
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    status_value(response.value()?, build_id)
  }

  async fn abort(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
    idempotency_key: &crate::domain::knowledge_publication::PublicationIdempotencyKey,
    fingerprint: &crate::domain::knowledge_publication::PublicationRequestFingerprint,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
    let input = AbortInputDto {
      build_id: build_id.as_str(),
      idempotency_key: idempotency_key.as_str(),
      request_fingerprint: fingerprint.as_str(),
    };
    let response = self
      .call(
        ABORT_PATH,
        context,
        canonical,
        input,
        MAX_CONTROL_REQUEST_BYTES,
      )
      .await?;
    mutation_status_value(response.value()?, build_id, fingerprint)
  }
}

fn validate_batch<T>(points: &[T]) -> Result<(), KnowledgeReleaseFailure> {
  if points.is_empty() || points.len() > MAX_PUBLICATION_BATCH_POINTS {
    Err(KnowledgeReleaseFailure::SchemaIncompatible)
  } else {
    Ok(())
  }
}

fn inspect_batch<T: Serialize>(
  canonical: &CanonicalReleasePin,
  input: T,
) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure> {
  let bytes = serialize_request(PublicationContextDto::planning(canonical), input)?.len();
  if bytes <= MAX_PUBLICATION_BATCH_REQUEST_BYTES {
    Ok(PublicationBatchWireAdmission::Fits {
      serialized_bytes: bytes,
    })
  } else {
    Ok(PublicationBatchWireAdmission::TooLarge {
      serialized_bytes: bytes,
    })
  }
}

fn serialize_request<T: Serialize>(
  context: PublicationContextDto,
  input: T,
) -> Result<Vec<u8>, KnowledgeReleaseFailure> {
  serde_json::to_vec(&PublicationRequestDto { context, input })
    .map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)
}

fn validate_compatibility(
  entry: &EmbeddingCompatibilityEntry,
) -> Result<(), KnowledgeReleaseFailure> {
  EmbeddingCompatibilityRegistry::new(vec![entry.clone()])
    .map(|_| ())
    .map_err(|_| KnowledgeReleaseFailure::EmbeddingMetadataIncompatible)
}

fn validate_begin(request: &BeginPublication) -> Result<(), KnowledgeReleaseFailure> {
  validate_compatibility(&request.compatibility)?;
  PublicationManifestHash::parse(request.canonical_content_hash.clone())
    .map_err(KnowledgeReleaseFailure::from)?;
  if request.expected_node_count == 0
    || request.expected_edge_count == 0
    || request.expected_endpoint_count != request.expected_edge_count.saturating_mul(2)
  {
    return Err(KnowledgeReleaseFailure::IncompleteTrio);
  }
  let node_count = request.expected_node_count.to_string();
  let edge_count = request.expected_edge_count.to_string();
  let endpoint_count = request.expected_endpoint_count.to_string();
  let expected = PublicationRequestFingerprint::derive(
    "begin",
    &[
      request.build_id.as_str(),
      request.canonical.release_id.as_str(),
      &request.canonical.canonical_schema_version,
      &request.canonical_content_hash,
      &request.projection_schema_version,
      &request.node_projection_hash,
      &request.edge_projection_hash,
      &request.compatibility.entry_id,
      &request.compatibility.dense_model_family,
      &request.compatibility.dense_artifact_revision,
      &request.compatibility.dense_dimensions.to_string(),
      &request.compatibility.dense_vector_name,
      &request.compatibility.node_dense_input_specification,
      &request.compatibility.edge_dense_input_specification,
      &request.compatibility.lexical_encoder_identity,
      &request.compatibility.lexical_encoder_revision,
      &request.compatibility.lexical_contract_identity,
      &request.compatibility.lexical_vector_name,
      &request.compatibility.node_lexical_input_specification,
      &request.compatibility.edge_lexical_input_specification,
      &node_count,
      &edge_count,
      &endpoint_count,
    ],
  )
  .map_err(KnowledgeReleaseFailure::from)?;
  if expected != request.request_fingerprint {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  Ok(())
}

fn validate_batch_identity(
  identity: &crate::domain::knowledge_publication::PublicationBatchIdentity,
  family: &str,
) -> Result<(), KnowledgeReleaseFailure> {
  let ordinal = identity.ordinal.get().to_string();
  let expected = PublicationRequestFingerprint::derive(
    "batch",
    &[
      identity.build_id.as_str(),
      family,
      &ordinal,
      identity.content_hash.as_str(),
    ],
  )
  .map_err(KnowledgeReleaseFailure::from)?;
  if expected != identity.request_fingerprint {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  Ok(())
}

fn effective_timeout(
  context: &KnowledgePublicationContext,
) -> Result<Duration, KnowledgeReleaseFailure> {
  if context.request_id.is_empty()
    || context.request_id.len() > MAX_PUBLICATION_REQUEST_ID_BYTES
    || !context
      .request_id
      .bytes()
      .all(|byte| byte.is_ascii_graphic())
    || context.deadline_at.len() > MAX_PUBLICATION_DEADLINE_BYTES
    || context.timeout.is_zero()
  {
    return Err(KnowledgeReleaseFailure::SchemaIncompatible);
  }
  let deadline = time::OffsetDateTime::parse(
    &context.deadline_at,
    &time::format_description::well_known::Rfc3339,
  )
  .map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)?;
  let remaining = Duration::try_from(deadline - time::OffsetDateTime::now_utc())
    .map_err(|_| KnowledgeReleaseFailure::Timeout)?;
  if remaining.is_zero() {
    return Err(KnowledgeReleaseFailure::Timeout);
  }
  Ok(context.timeout.min(remaining))
}

fn map_transport_error(error: IslandPortClientError) -> KnowledgeReleaseFailure {
  match error {
    IslandPortClientError::Timeout => KnowledgeReleaseFailure::Timeout,
    IslandPortClientError::Unavailable => KnowledgeReleaseFailure::DependencyUnavailable,
    _ => KnowledgeReleaseFailure::SchemaIncompatible,
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct PublicationRequestDto<T> {
  context: PublicationContextDto,
  input: T,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct PublicationContextDto {
  request_id: String,
  deadline_at: String,
  schema_version: &'static str,
  content_release: String,
}

impl PublicationContextDto {
  fn new(
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
  ) -> Result<Self, KnowledgeReleaseFailure> {
    effective_timeout(context)?;
    Ok(Self {
      request_id: context.request_id.clone(),
      deadline_at: context.deadline_at.clone(),
      schema_version: SCHEMA_VERSION,
      content_release: canonical.release_id.as_str().to_string(),
    })
  }

  fn planning(canonical: &CanonicalReleasePin) -> Self {
    Self {
      // Backslash is a legal ASCII-graphic request-ID byte and has the largest JSON encoding.
      request_id: "\\".repeat(MAX_PUBLICATION_REQUEST_ID_BYTES),
      // Maximum contract length, using parser-accepted fractional seconds and a numeric offset.
      deadline_at: PLANNING_DEADLINE_AT.into(),
      schema_version: SCHEMA_VERSION,
      content_release: canonical.release_id.as_str().to_string(),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct BeginInputDto<'a> {
  build_id: &'a str,
  canonical_schema_version: &'a str,
  canonical_content_hash: &'a str,
  projection_schema_version: &'a str,
  node_projection_hash: &'a str,
  edge_projection_hash: &'a str,
  compatibility: BeginCompatibilityDto<'a>,
  expected_node_count: u64,
  expected_edge_count: u64,
  expected_endpoint_count: u64,
  idempotency_key: &'a str,
  request_fingerprint: &'a str,
}

impl<'a> BeginInputDto<'a> {
  fn from_domain(value: &'a BeginPublication) -> Self {
    Self {
      build_id: value.build_id.as_str(),
      canonical_schema_version: &value.canonical.canonical_schema_version,
      canonical_content_hash: &value.canonical_content_hash,
      projection_schema_version: &value.projection_schema_version,
      node_projection_hash: &value.node_projection_hash,
      edge_projection_hash: &value.edge_projection_hash,
      compatibility: BeginCompatibilityDto::from(&value.compatibility),
      expected_node_count: value.expected_node_count,
      expected_edge_count: value.expected_edge_count,
      expected_endpoint_count: value.expected_endpoint_count,
      idempotency_key: value.idempotency_key.as_str(),
      request_fingerprint: value.request_fingerprint.as_str(),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityDto<'a> {
  entry_id: &'a str,
  dense_model_family: &'a str,
  dense_artifact_revision: &'a str,
  dense_dimensions: u32,
  dense_vector_name: &'a str,
  dense_input_specification: &'a str,
  lexical_encoder_identity: &'a str,
  lexical_encoder_revision: &'a str,
  lexical_contract_identity: &'a str,
  lexical_vector_name: &'a str,
  lexical_input_specification: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct BeginCompatibilityDto<'a> {
  entry_id: &'a str,
  dense_model_family: &'a str,
  dense_artifact_revision: &'a str,
  dense_dimensions: u32,
  dense_vector_name: &'a str,
  node_dense_input_specification: &'a str,
  edge_dense_input_specification: &'a str,
  lexical_encoder_identity: &'a str,
  lexical_encoder_revision: &'a str,
  lexical_contract_identity: &'a str,
  lexical_vector_name: &'a str,
  node_lexical_input_specification: &'a str,
  edge_lexical_input_specification: &'a str,
}

impl<'a> From<&'a EmbeddingCompatibilityEntry> for BeginCompatibilityDto<'a> {
  fn from(value: &'a EmbeddingCompatibilityEntry) -> Self {
    Self {
      entry_id: &value.entry_id,
      dense_model_family: &value.dense_model_family,
      dense_artifact_revision: &value.dense_artifact_revision,
      dense_dimensions: value.dense_dimensions,
      dense_vector_name: &value.dense_vector_name,
      node_dense_input_specification: &value.node_dense_input_specification,
      edge_dense_input_specification: &value.edge_dense_input_specification,
      lexical_encoder_identity: &value.lexical_encoder_identity,
      lexical_encoder_revision: &value.lexical_encoder_revision,
      lexical_contract_identity: &value.lexical_contract_identity,
      lexical_vector_name: &value.lexical_vector_name,
      node_lexical_input_specification: &value.node_lexical_input_specification,
      edge_lexical_input_specification: &value.edge_lexical_input_specification,
    }
  }
}

impl<'a> CompatibilityDto<'a> {
  fn from_family(value: &'a EmbeddingCompatibilityEntry, nodes: bool) -> Self {
    Self {
      entry_id: &value.entry_id,
      dense_model_family: &value.dense_model_family,
      dense_artifact_revision: &value.dense_artifact_revision,
      dense_dimensions: value.dense_dimensions,
      dense_vector_name: &value.dense_vector_name,
      dense_input_specification: if nodes {
        &value.node_dense_input_specification
      } else {
        &value.edge_dense_input_specification
      },
      lexical_encoder_identity: &value.lexical_encoder_identity,
      lexical_encoder_revision: &value.lexical_encoder_revision,
      lexical_contract_identity: &value.lexical_contract_identity,
      lexical_vector_name: &value.lexical_vector_name,
      lexical_input_specification: if nodes {
        &value.node_lexical_input_specification
      } else {
        &value.edge_lexical_input_specification
      },
    }
  }
}

#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct BatchInputDto {
  build_id: String,
  ordinal: u32,
  request_fingerprint: String,
  content_hash: String,
  points: Vec<Value>,
}

impl BatchInputDto {
  fn nodes(value: &NodePublicationBatch) -> Result<Self, KnowledgeReleaseFailure> {
    Ok(Self {
      build_id: value.identity.build_id.as_str().to_string(),
      ordinal: value.identity.ordinal.get(),
      request_fingerprint: value.identity.request_fingerprint.as_str().to_string(),
      content_hash: value.identity.content_hash.as_str().to_string(),
      points: value.points.iter().map(node_point).collect(),
    })
  }

  fn edges(value: &EdgePublicationBatch) -> Result<Self, KnowledgeReleaseFailure> {
    Ok(Self {
      build_id: value.identity.build_id.as_str().to_string(),
      ordinal: value.identity.ordinal.get(),
      request_fingerprint: value.identity.request_fingerprint.as_str().to_string(),
      content_hash: value.identity.content_hash.as_str().to_string(),
      points: value.points.iter().map(edge_point).collect(),
    })
  }
}

fn node_point(point: &NodeProjection) -> Value {
  let payload = match &point.payload {
    NodeProjectionPayload::Lexeme {
      lemma,
      normalized_lemma,
      language,
      part_of_speech,
    } => json!({
      "kind": "lexeme", "lemma": lemma, "normalized_lemma": normalized_lemma,
      "language": language, "part_of_speech": part_of_speech_name(*part_of_speech)
    }),
    NodeProjectionPayload::Sense {
      lexeme_id,
      sense_key,
      definition,
      lemma,
      language,
      part_of_speech,
      definition_evidence_ids,
    } => json!({
      "kind": "sense", "lexeme_id": lexeme_id.as_str(), "sense_key": sense_key,
      "definition": definition, "lemma": lemma, "language": language,
      "part_of_speech": part_of_speech_name(*part_of_speech),
      "definition_evidence_ids": definition_evidence_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()
    }),
  };
  json!({
    "point_id": point.point_id.as_str(),
    "node_kind": node_kind(point.node.kind),
    "canonical_id": point.node.id.as_str(),
    "payload_schema_version": point.payload_schema_version,
    "payload": payload,
    "dense_input": BASE64.encode(point.dense_input.canonical_bytes()),
    "dense_input_hash": point.dense_input.input_hash(),
    "lexical_input": BASE64.encode(point.lexical_input.canonical_bytes()),
    "lexical_input_hash": point.lexical_input.input_hash(),
    "projection_content_hash": point.content_hash
  })
}

fn edge_point(point: &EdgeProjection) -> Value {
  let mut value = json!({
    "point_id": point.point_id.as_str(),
    "relationship_id": point.identity.relationship_id.as_str(),
    "relationship_revision": point.identity.relationship_revision.get(),
    "source": {"kind": node_kind(point.identity.source.kind), "id": point.identity.source.id.as_str()},
    "target": {"kind": node_kind(point.identity.target.kind), "id": point.identity.target.id.as_str()},
    "source_point_id": point.source_point_id.as_str(),
    "target_point_id": point.target_point_id.as_str(),
    "wire_relation": point.wire_relation,
    "scope": {
      "dialect": point.scope.dialect.as_ref().map(|v| v.as_str()),
      "domain": point.scope.domain,
      "register": point.scope.register,
      "note": point.scope.note
    },
    "evidence": point.evidence.iter().map(|e| json!({
      "evidence_id": e.evidence_id.as_str(), "source_id": e.source_id.as_str(),
      "content_hash": e.content_hash
    })).collect::<Vec<_>>(),
    "verification": {
      "relationship_revision": point.verification.relationship_revision,
      "evidence_confidence": evidence_confidence(point.verification.evidence_confidence),
      "verification_state": verification_state(point.verification.verification_state)
    },
    "dense_input": BASE64.encode(point.dense_input.canonical_bytes()),
    "dense_input_hash": point.dense_input.input_hash(),
    "lexical_input": BASE64.encode(point.lexical_input.canonical_bytes()),
    "lexical_input_hash": point.lexical_input.input_hash(),
    "projection_content_hash": point.content_hash
  });
  if let (Some(assertion), Some(object)) = (point.assertion.as_ref(), value.as_object_mut()) {
    object.insert("fact_id".into(), json!(assertion.assertion_id.as_str()));
    object.insert("fact_revision".into(), json!(assertion.assertion_revision));
    object.insert(
      "relation_type_id".into(),
      json!(assertion.relation_type_id.as_str()),
    );
    object.insert(
      "relation_registry_version".into(),
      json!(assertion.relation_registry_revision),
    );
    object.insert(
      "traversal_id".into(),
      json!(assertion.traversal_id.as_str()),
    );
  }
  value
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FreezeInputDto<'a> {
  build_id: &'a str,
  finalize_id: &'a str,
  expected_persisted_hash: &'a str,
  projection_hash: &'a str,
  projection_schema_version: &'a str,
  point_count: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  verified_node_projection_hash: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  expected_endpoint_count: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  resolved_endpoint_count: Option<u64>,
  compatibility: CompatibilityDto<'a>,
  dictionary_hash: &'a str,
  dictionary_cardinality: usize,
  dictionary: Vec<DictionaryEntryDto<'a>>,
}

impl<'a> FreezeInputDto<'a> {
  fn nodes(value: &'a FreezeNodes) -> Self {
    Self {
      build_id: value.build_id.as_str(),
      finalize_id: value.finalize_id.as_str(),
      expected_persisted_hash: value.persisted_hash.as_str(),
      projection_hash: &value.projection_hash,
      projection_schema_version: &value.projection_schema_version,
      point_count: value.point_count,
      verified_node_projection_hash: None,
      expected_endpoint_count: None,
      resolved_endpoint_count: None,
      compatibility: CompatibilityDto::from_family(&value.compatibility, true),
      dictionary_hash: value.dictionary.dictionary_hash(),
      dictionary_cardinality: value.dictionary.cardinality(),
      dictionary: dictionary_entries(&value.dictionary),
    }
  }

  fn edges(value: &'a FreezeEdges) -> Self {
    Self {
      build_id: value.build_id.as_str(),
      finalize_id: value.finalize_id.as_str(),
      expected_persisted_hash: value.persisted_hash.as_str(),
      projection_hash: &value.projection_hash,
      projection_schema_version: &value.projection_schema_version,
      point_count: value.point_count,
      verified_node_projection_hash: Some(&value.verified_node_projection_hash),
      expected_endpoint_count: Some(value.expected_endpoint_count),
      resolved_endpoint_count: Some(value.resolved_endpoint_count),
      compatibility: CompatibilityDto::from_family(&value.compatibility, false),
      dictionary_hash: value.dictionary.dictionary_hash(),
      dictionary_cardinality: value.dictionary.cardinality(),
      dictionary: dictionary_entries(&value.dictionary),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct DictionaryEntryDto<'a> {
  term: &'a str,
  index: u32,
}

fn dictionary_entries(
  value: &crate::domain::knowledge_publication::LexicalDictionaryManifest,
) -> Vec<DictionaryEntryDto<'_>> {
  value
    .entries()
    .iter()
    .map(|(term, index)| DictionaryEntryDto {
      term,
      index: *index,
    })
    .collect()
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ReconcileInputDto<'a> {
  build_id: &'a str,
  reconcile_id: &'a str,
  canonical_schema_version: &'a str,
  canonical_content_hash: &'a str,
  node_collection_id: &'a str,
  edge_collection_id: &'a str,
  node_projection_hash: &'a str,
  edge_projection_hash: &'a str,
  node_persisted_hash: &'a str,
  edge_persisted_hash: &'a str,
  edge_verified_node_projection_hash: &'a str,
  node_point_count: u64,
  edge_point_count: u64,
  expected_endpoint_count: u64,
  resolved_endpoint_count: u64,
  publication_manifest_hash: &'a str,
  compatibility_entry_id: &'a str,
  node_receipt: ReceiptRequestDto<'a>,
  edge_receipt: ReceiptRequestDto<'a>,
}

impl<'a> ReconcileInputDto<'a> {
  fn from_domain(value: &'a ReconcilePublication) -> Self {
    Self {
      build_id: value.build_id.as_str(),
      reconcile_id: value.reconcile_id.as_str(),
      canonical_schema_version: &value.canonical.canonical_schema_version,
      canonical_content_hash: &value.canonical_content_hash,
      node_collection_id: value.nodes.manifest.collection_id.as_str(),
      edge_collection_id: value.edges.manifest.collection_id.as_str(),
      node_projection_hash: &value.nodes.manifest.content_hash,
      edge_projection_hash: &value.edges.manifest.content_hash,
      node_persisted_hash: value.nodes.persisted_hash.as_str(),
      edge_persisted_hash: value.edges.persisted_hash.as_str(),
      edge_verified_node_projection_hash: &value.edges.manifest.verified_node_content_hash,
      node_point_count: value.nodes.manifest.point_count,
      edge_point_count: value.edges.manifest.point_count,
      expected_endpoint_count: value.edges.manifest.expected_endpoint_count,
      resolved_endpoint_count: value.edges.manifest.resolved_endpoint_count,
      publication_manifest_hash: value.manifest_hash.as_str(),
      compatibility_entry_id: &value.compatibility.entry_id,
      node_receipt: ReceiptRequestDto::from(&value.nodes.receipt),
      edge_receipt: ReceiptRequestDto::from(&value.edges.receipt),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ReceiptRequestDto<'a> {
  dense: DenseReceiptRequestDto<'a>,
  lexical: LexicalReceiptRequestDto<'a>,
}

impl<'a> From<&'a BuildExecutionReceipt> for ReceiptRequestDto<'a> {
  fn from(value: &'a BuildExecutionReceipt) -> Self {
    Self {
      dense: DenseReceiptRequestDto {
        registry_entry_id: &value.dense.registry_entry_id,
        observed_artifact_revision: &value.dense.observed_artifact_revision,
        dimensions: value.dense.dimensions,
        vector_name: &value.dense.vector_name,
        input_specification: &value.dense.input_specification,
        processed_point_count: value.dense.processed_point_count,
      },
      lexical: LexicalReceiptRequestDto {
        encoder_identity: &value.lexical.encoder_identity,
        encoder_revision: &value.lexical.encoder_revision,
        vector_name: &value.lexical.vector_name,
        input_specification: &value.lexical.input_specification,
        dictionary_hash: &value.lexical.dictionary_hash,
        processed_point_count: value.lexical.processed_point_count,
      },
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct DenseReceiptRequestDto<'a> {
  registry_entry_id: &'a str,
  observed_artifact_revision: &'a str,
  dimensions: u32,
  vector_name: &'a str,
  input_specification: &'a str,
  processed_point_count: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct LexicalReceiptRequestDto<'a> {
  encoder_identity: &'a str,
  encoder_revision: &'a str,
  vector_name: &'a str,
  input_specification: &'a str,
  dictionary_hash: &'a str,
  processed_point_count: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct BuildInputDto<'a> {
  build_id: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AbortInputDto<'a> {
  build_id: &'a str,
  idempotency_key: &'a str,
  request_fingerprint: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicationResponseDto<T> {
  request_id: String,
  schema_version: String,
  content_release: String,
  outcome: OutcomeDto,
  value: Option<T>,
  error: Option<ErrorDto>,
}

impl<T> PublicationResponseDto<T> {
  fn validate_context(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
  ) -> Result<(), KnowledgeReleaseFailure> {
    if self.request_id != context.request_id
      || self.schema_version != SCHEMA_VERSION
      || self.content_release != canonical.release_id.as_str()
    {
      return Err(KnowledgeReleaseFailure::SchemaIncompatible);
    }
    Ok(())
  }

  fn value(self) -> Result<T, KnowledgeReleaseFailure> {
    match self.outcome {
      OutcomeDto::Ok if self.error.is_none() => self
        .value
        .ok_or(KnowledgeReleaseFailure::SchemaIncompatible),
      OutcomeDto::Ok => Err(KnowledgeReleaseFailure::SchemaIncompatible),
      outcome => {
        if self.value.is_some() {
          return Err(KnowledgeReleaseFailure::SchemaIncompatible);
        }
        let error = self
          .error
          .ok_or(KnowledgeReleaseFailure::SchemaIncompatible)?;
        let failure = KnowledgeReleaseFailure::parse_code(&error.code)
          .map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)?;
        if outcome.accepts(failure) {
          Err(failure)
        } else {
          Err(KnowledgeReleaseFailure::SchemaIncompatible)
        }
      }
    }
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeDto {
  Ok,
  Missing,
  InvalidPayload,
  VersionMismatch,
  Conflict,
  Unavailable,
  Timeout,
}

impl OutcomeDto {
  fn accepts(self, failure: KnowledgeReleaseFailure) -> bool {
    match self {
      Self::Missing => matches!(
        failure,
        KnowledgeReleaseFailure::CanonicalReleaseUnavailable
          | KnowledgeReleaseFailure::ImmutableReleaseUnavailable
      ),
      Self::InvalidPayload => matches!(
        failure,
        KnowledgeReleaseFailure::InvalidLifecycleTransition
          | KnowledgeReleaseFailure::IncompleteTrio
      ),
      Self::VersionMismatch => matches!(
        failure,
        KnowledgeReleaseFailure::SchemaIncompatible
          | KnowledgeReleaseFailure::EmbeddingMetadataIncompatible
          | KnowledgeReleaseFailure::ArtifactRevisionMismatch
          | KnowledgeReleaseFailure::LexicalEncoderMismatch
          | KnowledgeReleaseFailure::DictionaryMismatch
      ),
      Self::Conflict => matches!(
        failure,
        KnowledgeReleaseFailure::IdempotencyConflict
          | KnowledgeReleaseFailure::EndpointReconciliationFailed
          | KnowledgeReleaseFailure::HashOrCountReconciliationFailed
          | KnowledgeReleaseFailure::ActivationConflict
      ),
      Self::Unavailable => matches!(
        failure,
        KnowledgeReleaseFailure::DependencyUnavailable
          | KnowledgeReleaseFailure::NodeBuildUnavailable
          | KnowledgeReleaseFailure::EdgeBuildUnavailable
      ),
      Self::Timeout => failure == KnowledgeReleaseFailure::Timeout,
      Self::Ok => false,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorDto {
  code: String,
  #[allow(dead_code)]
  message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusValueDto {
  build_id: String,
  state: StateDto,
  next_node_ordinal: u32,
  next_edge_ordinal: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BeginValueDto {
  build_id: String,
  state: StateDto,
  next_node_ordinal: u32,
  next_edge_ordinal: u32,
  accepted_request_fingerprint: String,
}

fn begin_value(
  value: BeginValueDto,
  expected: &BeginPublication,
) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
  if value.accepted_request_fingerprint != expected.request_fingerprint.as_str() {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  status_value(
    StatusValueDto {
      build_id: value.build_id,
      state: value.state,
      next_node_ordinal: value.next_node_ordinal,
      next_edge_ordinal: value.next_edge_ordinal,
    },
    &expected.build_id,
  )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchValueDto {
  build_id: String,
  state: StateDto,
  next_node_ordinal: u32,
  next_edge_ordinal: u32,
  accepted_ordinal: u32,
  accepted_request_fingerprint: String,
  accepted_content_hash: String,
}

fn batch_value(
  value: BatchValueDto,
  expected: &crate::domain::knowledge_publication::PublicationBatchIdentity,
) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
  if value.accepted_ordinal != expected.ordinal.get()
    || value.accepted_request_fingerprint != expected.request_fingerprint.as_str()
    || value.accepted_content_hash != expected.content_hash.as_str()
  {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  status_value(
    StatusValueDto {
      build_id: value.build_id,
      state: value.state,
      next_node_ordinal: value.next_node_ordinal,
      next_edge_ordinal: value.next_edge_ordinal,
    },
    &expected.build_id,
  )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationStatusValueDto {
  build_id: String,
  state: StateDto,
  next_node_ordinal: u32,
  next_edge_ordinal: u32,
  accepted_request_fingerprint: String,
}

fn mutation_status_value(
  value: MutationStatusValueDto,
  build_id: &PublicationBuildId,
  fingerprint: &PublicationRequestFingerprint,
) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
  if value.accepted_request_fingerprint != fingerprint.as_str() {
    return Err(KnowledgeReleaseFailure::IdempotencyConflict);
  }
  status_value(
    StatusValueDto {
      build_id: value.build_id,
      state: value.state,
      next_node_ordinal: value.next_node_ordinal,
      next_edge_ordinal: value.next_edge_ordinal,
    },
    build_id,
  )
}

fn status_value(
  value: StatusValueDto,
  expected: &PublicationBuildId,
) -> Result<PublicationStatus, KnowledgeReleaseFailure> {
  if value.build_id != expected.as_str() {
    return Err(KnowledgeReleaseFailure::SchemaIncompatible);
  }
  Ok(PublicationStatus {
    build_id: expected.clone(),
    state: value.state.into_domain(),
    next_node_ordinal: value.next_node_ordinal,
    next_edge_ordinal: value.next_edge_ordinal,
  })
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StateDto {
  AcceptingNodes,
  NodesFrozen,
  AcceptingEdges,
  EdgesFrozen,
  Reconciling,
  ActivationCandidate,
  Failed,
  Aborting,
  Abandoned,
  GcEligible,
}

impl StateDto {
  fn into_domain(self) -> PublicationBuildState {
    match self {
      Self::AcceptingNodes => PublicationBuildState::AcceptingNodes,
      Self::NodesFrozen => PublicationBuildState::NodesFrozen,
      Self::AcceptingEdges => PublicationBuildState::AcceptingEdges,
      Self::EdgesFrozen => PublicationBuildState::EdgesFrozen,
      Self::Reconciling => PublicationBuildState::Reconciling,
      Self::ActivationCandidate => PublicationBuildState::ActivationCandidate,
      Self::Failed => PublicationBuildState::Failed,
      Self::Aborting => PublicationBuildState::Aborting,
      Self::Abandoned => PublicationBuildState::Abandoned,
      Self::GcEligible => PublicationBuildState::GcEligible,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FreezeValueDto {
  build_id: String,
  finalize_id: String,
  collection_id: String,
  persisted_collection_hash: String,
  projection_content_hash: String,
  point_count: u64,
  #[serde(default)]
  verified_node_projection_hash: Option<String>,
  #[serde(default)]
  expected_endpoint_count: Option<u64>,
  #[serde(default)]
  resolved_endpoint_count: Option<u64>,
  receipt: ReceiptDto,
}

impl FreezeValueDto {
  fn nodes(
    self,
    canonical: &CanonicalReleasePin,
    expected: &FreezeNodes,
  ) -> Result<FrozenNodePublication, KnowledgeReleaseFailure> {
    self.validate_common(
      &expected.build_id,
      expected.finalize_id.as_str(),
      &expected.persisted_hash,
      &expected.projection_hash,
      expected.point_count,
    )?;
    if self.verified_node_projection_hash.is_some()
      || self.expected_endpoint_count.is_some()
      || self.resolved_endpoint_count.is_some()
    {
      return Err(KnowledgeReleaseFailure::SchemaIncompatible);
    }
    let receipt = self.receipt.into_domain(
      expected.build_id.clone(),
      PublicationCollectionFamily::Nodes,
    );
    receipt.validate(
      &expected.build_id,
      &expected.compatibility,
      &expected.dictionary,
      expected.point_count,
    )?;
    Ok(FrozenNodePublication {
      manifest: NodeCollectionManifest {
        release_id: canonical.release_id.clone(),
        collection_id: NodeCollectionId::parse(self.collection_id)
          .map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)?,
        payload_schema_version: expected.projection_schema_version.clone(),
        content_hash: expected.projection_hash.clone(),
        point_count: expected.point_count,
        state: ProjectionCollectionState::Verified,
      },
      persisted_hash: expected.persisted_hash.clone(),
      receipt,
      dictionary: expected.dictionary.clone(),
    })
  }

  fn edges(
    self,
    canonical: &CanonicalReleasePin,
    expected: &FreezeEdges,
  ) -> Result<FrozenEdgePublication, KnowledgeReleaseFailure> {
    self.validate_common(
      &expected.build_id,
      expected.finalize_id.as_str(),
      &expected.persisted_hash,
      &expected.projection_hash,
      expected.point_count,
    )?;
    if self.verified_node_projection_hash.as_deref()
      != Some(expected.verified_node_projection_hash.as_str())
      || self.expected_endpoint_count != Some(expected.expected_endpoint_count)
      || self.resolved_endpoint_count != Some(expected.resolved_endpoint_count)
      || expected.expected_endpoint_count != expected.resolved_endpoint_count
    {
      return Err(KnowledgeReleaseFailure::EndpointReconciliationFailed);
    }
    let receipt = self.receipt.into_domain(
      expected.build_id.clone(),
      PublicationCollectionFamily::Edges,
    );
    receipt.validate(
      &expected.build_id,
      &expected.compatibility,
      &expected.dictionary,
      expected.point_count,
    )?;
    Ok(FrozenEdgePublication {
      manifest: EdgeCollectionManifest {
        release_id: canonical.release_id.clone(),
        collection_id: EdgeCollectionId::parse(self.collection_id)
          .map_err(|_| KnowledgeReleaseFailure::SchemaIncompatible)?,
        payload_schema_version: expected.projection_schema_version.clone(),
        content_hash: expected.projection_hash.clone(),
        point_count: expected.point_count,
        verified_node_content_hash: expected.verified_node_projection_hash.clone(),
        expected_endpoint_count: expected.expected_endpoint_count,
        resolved_endpoint_count: expected.resolved_endpoint_count,
        state: ProjectionCollectionState::Verified,
      },
      persisted_hash: expected.persisted_hash.clone(),
      receipt,
      dictionary: expected.dictionary.clone(),
    })
  }

  fn validate_common(
    &self,
    build: &PublicationBuildId,
    finalize_id: &str,
    persisted: &PersistedCollectionHash,
    projection: &str,
    count: u64,
  ) -> Result<(), KnowledgeReleaseFailure> {
    if self.build_id != build.as_str()
      || self.finalize_id != finalize_id
      || self.persisted_collection_hash != persisted.as_str()
      || self.projection_content_hash != projection
      || self.point_count != count
    {
      return Err(KnowledgeReleaseFailure::HashOrCountReconciliationFailed);
    }
    Ok(())
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptDto {
  dense: DenseReceiptDto,
  lexical: LexicalReceiptDto,
}

impl ReceiptDto {
  fn into_domain(
    self,
    build_id: PublicationBuildId,
    family: PublicationCollectionFamily,
  ) -> BuildExecutionReceipt {
    BuildExecutionReceipt {
      build_id,
      family,
      dense: DenseExecutionReceipt {
        registry_entry_id: self.dense.registry_entry_id,
        observed_artifact_revision: self.dense.observed_artifact_revision,
        dimensions: self.dense.dimensions,
        vector_name: self.dense.vector_name,
        input_specification: self.dense.input_specification,
        processed_point_count: self.dense.processed_point_count,
      },
      lexical: LexicalExecutionReceipt {
        encoder_identity: self.lexical.encoder_identity,
        encoder_revision: self.lexical.encoder_revision,
        vector_name: self.lexical.vector_name,
        input_specification: self.lexical.input_specification,
        dictionary_hash: self.lexical.dictionary_hash,
        processed_point_count: self.lexical.processed_point_count,
      },
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DenseReceiptDto {
  registry_entry_id: String,
  observed_artifact_revision: String,
  dimensions: u32,
  vector_name: String,
  input_specification: String,
  processed_point_count: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LexicalReceiptDto {
  encoder_identity: String,
  encoder_revision: String,
  vector_name: String,
  input_specification: String,
  dictionary_hash: String,
  processed_point_count: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconcileValueDto {
  build_id: String,
  reconcile_id: String,
  node_persisted_hash: String,
  edge_persisted_hash: String,
  publication_manifest_hash: String,
  canonical_content_hash: String,
  state: StateDto,
}

impl ReconcileValueDto {
  fn into_domain(
    self,
    expected: &ReconcilePublication,
  ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure> {
    if self.build_id != expected.build_id.as_str()
      || self.reconcile_id != expected.reconcile_id.as_str()
      || self.node_persisted_hash != expected.nodes.persisted_hash.as_str()
      || self.edge_persisted_hash != expected.edges.persisted_hash.as_str()
      || self.publication_manifest_hash != expected.manifest_hash.as_str()
      || self.canonical_content_hash != expected.canonical_content_hash
      || !matches!(self.state, StateDto::ActivationCandidate)
    {
      return Err(KnowledgeReleaseFailure::HashOrCountReconciliationFailed);
    }
    expected.nodes.receipt.validate(
      &expected.build_id,
      &expected.compatibility,
      &expected.nodes.dictionary,
      expected.nodes.manifest.point_count,
    )?;
    expected.edges.receipt.validate(
      &expected.build_id,
      &expected.compatibility,
      &expected.edges.dictionary,
      expected.edges.manifest.point_count,
    )?;
    let trio = KnowledgeReleaseTrio::try_from_parts(
      expected.canonical.clone(),
      Some(expected.nodes.manifest.clone()),
      Some(expected.edges.manifest.clone()),
      DenseEmbeddingVersion {
        model_family: expected.compatibility.dense_model_family.clone(),
        artifact_revision: expected.compatibility.dense_artifact_revision.clone(),
        dimensions: expected.compatibility.dense_dimensions,
      },
      SparseEmbeddingVersion {
        encoder_identity: expected.compatibility.lexical_encoder_identity.clone(),
        encoder_revision: expected.compatibility.lexical_encoder_revision.clone(),
      },
    )
    .map_err(|_| KnowledgeReleaseFailure::IncompleteTrio)?;
    Ok(PublicationActivationCandidate {
      build_id: expected.build_id.clone(),
      reconcile_id: expected.reconcile_id.clone(),
      trio,
      canonical_content_hash: expected.canonical_content_hash.clone(),
      manifest_hash: expected.manifest_hash.clone(),
      node_persisted_hash: expected.nodes.persisted_hash.clone(),
      edge_persisted_hash: expected.edges.persisted_hash.clone(),
    })
  }
}

const fn node_kind(value: GraphNodeKind) -> &'static str {
  match value {
    GraphNodeKind::Sense => "sense",
    GraphNodeKind::Lexeme => "lexeme",
    GraphNodeKind::Construction => "construction",
    GraphNodeKind::Scale => "scale",
  }
}

const fn part_of_speech_name(value: LexicalPartOfSpeech) -> &'static str {
  match value {
    LexicalPartOfSpeech::Noun => "noun",
    LexicalPartOfSpeech::Verb => "verb",
    LexicalPartOfSpeech::Adjective => "adjective",
    LexicalPartOfSpeech::Adverb => "adverb",
    LexicalPartOfSpeech::Pronoun => "pronoun",
    LexicalPartOfSpeech::Preposition => "preposition",
    LexicalPartOfSpeech::Conjunction => "conjunction",
    LexicalPartOfSpeech::Determiner => "determiner",
    LexicalPartOfSpeech::Interjection => "interjection",
    LexicalPartOfSpeech::Numeral => "numeral",
    LexicalPartOfSpeech::Other => "other",
  }
}

const fn evidence_confidence(value: EvidenceConfidence) -> &'static str {
  match value {
    EvidenceConfidence::High => "high",
    EvidenceConfidence::Medium => "medium",
    EvidenceConfidence::Low => "low",
  }
}

const fn verification_state(value: RelationshipVerificationState) -> &'static str {
  match value {
    RelationshipVerificationState::Verified => "verified",
    RelationshipVerificationState::Exploratory => "exploratory",
  }
}

#[cfg(test)]
mod tests {
  use std::{collections::VecDeque, sync::Mutex};

  use serde_json::json;

  use super::*;
  use crate::domain::{
    canonical::{CanonicalId, CanonicalReleasePin},
    embedding_input::{
      EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION, NODE_DENSE_INPUT_VERSION,
      NODE_LEXICAL_INPUT_VERSION,
    },
    knowledge_projection::{
      DENSE_DIMENSIONS, DENSE_MODEL_FAMILY, DENSE_VECTOR_NAME, LEXICAL_CONTRACT_IDENTITY,
      LEXICAL_ENCODER_IDENTITY, LEXICAL_ENCODER_REVISION, SPARSE_VECTOR_NAME,
    },
    knowledge_publication::{
      LexicalDictionaryManifest, PublicationIdempotencyKey, PublicationManifestHash,
      PublicationRequestFingerprint,
    },
  };

  #[derive(Default)]
  struct FakeTransport {
    responses: Mutex<VecDeque<Result<Vec<u8>, IslandPortClientError>>>,
    requests: Mutex<Vec<(&'static str, Value, Duration)>>,
  }

  impl FakeTransport {
    fn with_responses(responses: Vec<Value>) -> Self {
      Self {
        responses: Mutex::new(
          responses
            .into_iter()
            .map(|value| Ok(serde_json::to_vec(&value).unwrap()))
            .collect(),
        ),
        requests: Mutex::new(Vec::new()),
      }
    }
  }

  #[async_trait]
  impl IslandPortTransport for FakeTransport {
    async fn post_json(
      &self,
      path: &'static str,
      body: Vec<u8>,
      timeout: Duration,
    ) -> Result<Vec<u8>, IslandPortClientError> {
      let value =
        serde_json::from_slice(&body).map_err(|_| IslandPortClientError::InvalidRequest)?;
      self.requests.lock().unwrap().push((path, value, timeout));
      self
        .responses
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or(Err(IslandPortClientError::Unavailable))
    }
  }

  fn hash(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
  }

  fn pin() -> CanonicalReleasePin {
    CanonicalReleasePin::new(
      CanonicalId::new("release-r1").unwrap(),
      "canonical-v1".into(),
    )
    .unwrap()
  }

  fn context() -> KnowledgePublicationContext {
    let deadline = time::OffsetDateTime::now_utc() + time::Duration::minutes(2);
    KnowledgePublicationContext {
      request_id: "req-publication-test".into(),
      deadline_at: deadline
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap(),
      timeout: Duration::from_secs(30),
    }
  }

  fn compatibility() -> EmbeddingCompatibilityEntry {
    EmbeddingCompatibilityEntry {
      entry_id: "fixture-qwen-r1".into(),
      dense_model_family: DENSE_MODEL_FAMILY.into(),
      dense_artifact_revision: "sha256:test-immutable-artifact-r1".into(),
      dense_dimensions: DENSE_DIMENSIONS,
      dense_vector_name: DENSE_VECTOR_NAME.into(),
      node_dense_input_specification: NODE_DENSE_INPUT_VERSION.into(),
      edge_dense_input_specification: EDGE_DENSE_INPUT_VERSION.into(),
      lexical_encoder_identity: LEXICAL_ENCODER_IDENTITY.into(),
      lexical_encoder_revision: LEXICAL_ENCODER_REVISION.into(),
      lexical_contract_identity: LEXICAL_CONTRACT_IDENTITY.into(),
      lexical_vector_name: SPARSE_VECTOR_NAME.into(),
      node_lexical_input_specification: NODE_LEXICAL_INPUT_VERSION.into(),
      edge_lexical_input_specification: EDGE_LEXICAL_INPUT_VERSION.into(),
    }
  }

  fn begin_request() -> BeginPublication {
    let canonical = pin();
    let build_id = PublicationBuildId::parse(hash('a')).unwrap();
    let node_projection_hash = hash('b');
    let edge_projection_hash = hash('c');
    let canonical_content_hash = hash('9');
    let node_count = "2";
    let edge_count = "1";
    let endpoint_count = "2";
    let request_fingerprint = PublicationRequestFingerprint::derive(
      "begin",
      &[
        build_id.as_str(),
        canonical.release_id.as_str(),
        &canonical.canonical_schema_version,
        &canonical_content_hash,
        "knowledge-projection-v1",
        &node_projection_hash,
        &edge_projection_hash,
        "fixture-qwen-r1",
        DENSE_MODEL_FAMILY,
        "sha256:test-immutable-artifact-r1",
        "1024",
        DENSE_VECTOR_NAME,
        NODE_DENSE_INPUT_VERSION,
        EDGE_DENSE_INPUT_VERSION,
        LEXICAL_ENCODER_IDENTITY,
        LEXICAL_ENCODER_REVISION,
        LEXICAL_CONTRACT_IDENTITY,
        SPARSE_VECTOR_NAME,
        NODE_LEXICAL_INPUT_VERSION,
        EDGE_LEXICAL_INPUT_VERSION,
        node_count,
        edge_count,
        endpoint_count,
      ],
    )
    .unwrap();
    BeginPublication {
      build_id,
      canonical,
      canonical_content_hash,
      projection_schema_version: "knowledge-projection-v1".into(),
      node_projection_hash,
      edge_projection_hash,
      compatibility: compatibility(),
      expected_node_count: 2,
      expected_edge_count: 1,
      expected_endpoint_count: 2,
      idempotency_key: PublicationIdempotencyKey::parse("publish-release-r1").unwrap(),
      request_fingerprint,
    }
  }

  fn status_response(state: &str) -> Value {
    json!({
      "request_id": "req-publication-test",
      "schema_version": SCHEMA_VERSION,
      "content_release": "release-r1",
      "outcome": "ok",
      "value": {
        "build_id": hash('a'), "state": state,
        "next_node_ordinal": 0, "next_edge_ordinal": 0
      }
    })
  }

  fn begin_response(request: &BeginPublication) -> Value {
    json!({
      "request_id": "req-publication-test",
      "schema_version": SCHEMA_VERSION,
      "content_release": "release-r1",
      "outcome": "ok",
      "value": {
        "build_id": hash('a'), "state": "accepting_nodes",
        "next_node_ordinal": 0, "next_edge_ordinal": 0,
        "accepted_request_fingerprint": request.request_fingerprint.as_str()
      }
    })
  }

  fn batch_response(fingerprint: &str, content_hash: &str) -> Value {
    json!({
      "request_id": "req-publication-test",
      "schema_version": SCHEMA_VERSION,
      "content_release": "release-r1",
      "outcome": "ok",
      "value": {
        "build_id": hash('a'), "state": "accepting_nodes",
        "next_node_ordinal": 1, "next_edge_ordinal": 0,
        "accepted_ordinal": 0,
        "accepted_request_fingerprint": fingerprint,
        "accepted_content_hash": content_hash
      }
    })
  }

  fn execution_receipt(
    build_id: &PublicationBuildId,
    family: PublicationCollectionFamily,
    dictionary: &LexicalDictionaryManifest,
    count: u64,
  ) -> BuildExecutionReceipt {
    let (dense_input, lexical_input) = match family {
      PublicationCollectionFamily::Nodes => (NODE_DENSE_INPUT_VERSION, NODE_LEXICAL_INPUT_VERSION),
      PublicationCollectionFamily::Edges => (EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION),
    };
    BuildExecutionReceipt {
      build_id: build_id.clone(),
      family,
      dense: DenseExecutionReceipt {
        registry_entry_id: "fixture-qwen-r1".into(),
        observed_artifact_revision: "sha256:test-immutable-artifact-r1".into(),
        dimensions: 1024,
        vector_name: "semantic".into(),
        input_specification: dense_input.into(),
        processed_point_count: count,
      },
      lexical: LexicalExecutionReceipt {
        encoder_identity: "transnet-lexical-bm25".into(),
        encoder_revision: "v1".into(),
        vector_name: "lexical".into(),
        input_specification: lexical_input.into(),
        dictionary_hash: dictionary.dictionary_hash().into(),
        processed_point_count: count,
      },
    }
  }

  #[tokio::test]
  async fn begin_is_strict_replay_safe_and_propagates_context() {
    let request = begin_request();
    let transport = Arc::new(FakeTransport::with_responses(vec![
      begin_response(&request),
      begin_response(&request),
    ]));
    let client = IslandPortPublicationClient::new(transport.clone());
    let first = client.begin(&context(), &request).await.unwrap();
    let second = client.begin(&context(), &request).await.unwrap();
    assert_eq!(first.state, PublicationBuildState::AcceptingNodes);
    assert_eq!(second.build_id, request.build_id);
    let calls = transport.requests.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|(path, body, timeout)| {
      *path == BEGIN_PATH
        && body["context"]["request_id"] == "req-publication-test"
        && body["context"]["schema_version"] == SCHEMA_VERSION
        && body["context"]["content_release"] == "release-r1"
        && timeout <= &Duration::from_secs(30)
    }));
    assert_eq!(calls[0].1["input"], calls[1].1["input"]);
  }

  #[tokio::test]
  async fn begin_rejects_local_fingerprint_and_registry_conflicts_before_transport() {
    let transport = Arc::new(FakeTransport::default());
    let client = IslandPortPublicationClient::new(transport.clone());
    let mut request = begin_request();
    request.request_fingerprint =
      PublicationRequestFingerprint::derive("begin", &["wrong"]).unwrap();
    assert_eq!(
      client.begin(&context(), &request).await.unwrap_err(),
      KnowledgeReleaseFailure::IdempotencyConflict
    );
    let mut request = begin_request();
    request.compatibility.dense_artifact_revision = "latest".into();
    assert_eq!(
      client.begin(&context(), &request).await.unwrap_err(),
      KnowledgeReleaseFailure::EmbeddingMetadataIncompatible
    );
    assert!(transport.requests.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn closed_outcome_and_error_codes_never_depend_on_message_text() {
    let cases = [
      (
        "conflict",
        "idempotency_conflict",
        KnowledgeReleaseFailure::IdempotencyConflict,
      ),
      (
        "invalid_payload",
        "invalid_lifecycle_transition",
        KnowledgeReleaseFailure::InvalidLifecycleTransition,
      ),
      (
        "version_mismatch",
        "artifact_revision_mismatch",
        KnowledgeReleaseFailure::ArtifactRevisionMismatch,
      ),
      (
        "version_mismatch",
        "lexical_encoder_mismatch",
        KnowledgeReleaseFailure::LexicalEncoderMismatch,
      ),
      (
        "version_mismatch",
        "dictionary_mismatch",
        KnowledgeReleaseFailure::DictionaryMismatch,
      ),
      (
        "conflict",
        "hash_or_count_reconciliation_failed",
        KnowledgeReleaseFailure::HashOrCountReconciliationFailed,
      ),
      (
        "conflict",
        "endpoint_reconciliation_failed",
        KnowledgeReleaseFailure::EndpointReconciliationFailed,
      ),
      (
        "unavailable",
        "dependency_unavailable",
        KnowledgeReleaseFailure::DependencyUnavailable,
      ),
      ("timeout", "timeout", KnowledgeReleaseFailure::Timeout),
    ];
    for (outcome, code, expected) in cases {
      let response = json!({
        "request_id": "req-publication-test", "schema_version": SCHEMA_VERSION,
        "content_release": "release-r1", "outcome": outcome,
        "error": {"code": code, "message": "secret socket / credential / evidence"}
      });
      let client =
        IslandPortPublicationClient::new(Arc::new(FakeTransport::with_responses(vec![response])));
      let error = client
        .begin(&context(), &begin_request())
        .await
        .unwrap_err();
      assert_eq!(error, expected);
      assert!(!format!("{error:?}").contains("secret"));
    }
  }

  #[tokio::test]
  async fn unknown_contradictory_malformed_and_oversized_responses_fail_closed() {
    let responses = vec![
      serde_json::to_vec(&json!({
        "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
        "content_release":"release-r1", "outcome":"future_outcome"
      })).unwrap(),
      serde_json::to_vec(&json!({
        "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
        "content_release":"release-r1", "outcome":"conflict",
        "value": {"build_id":hash('a'),"state":"accepting_nodes","next_node_ordinal":0,"next_edge_ordinal":0},
        "error":{"code":"idempotency_conflict","message":"x"}
      })).unwrap(),
      br#"{"#.to_vec(),
      vec![b'x'; MAX_RESPONSE_BYTES + 1],
      serde_json::to_vec(&json!({
        "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
        "content_release":"release-r1", "outcome":"ok",
        "value":{"build_id":hash('a'),"state":"accepting_nodes","next_node_ordinal":0,"next_edge_ordinal":0},
        "unknown":true
      })).unwrap(),
    ];
    for bytes in responses {
      let transport = Arc::new(FakeTransport {
        responses: Mutex::new(VecDeque::from([Ok(bytes)])),
        requests: Mutex::new(Vec::new()),
      });
      let error = IslandPortPublicationClient::new(transport)
        .begin(&context(), &begin_request())
        .await
        .unwrap_err();
      assert_eq!(error, KnowledgeReleaseFailure::SchemaIncompatible);
    }
  }

  #[tokio::test]
  async fn status_maps_every_closed_state_and_abort_respects_server_transition() {
    let states = [
      ("accepting_nodes", PublicationBuildState::AcceptingNodes),
      ("nodes_frozen", PublicationBuildState::NodesFrozen),
      ("accepting_edges", PublicationBuildState::AcceptingEdges),
      ("edges_frozen", PublicationBuildState::EdgesFrozen),
      ("reconciling", PublicationBuildState::Reconciling),
      (
        "activation_candidate",
        PublicationBuildState::ActivationCandidate,
      ),
      ("failed", PublicationBuildState::Failed),
      ("aborting", PublicationBuildState::Aborting),
      ("abandoned", PublicationBuildState::Abandoned),
      ("gc_eligible", PublicationBuildState::GcEligible),
    ];
    for (wire, expected) in states {
      let transport = Arc::new(FakeTransport::with_responses(vec![status_response(wire)]));
      let status = IslandPortPublicationClient::new(transport)
        .status(
          &context(),
          &pin(),
          &PublicationBuildId::parse(hash('a')).unwrap(),
        )
        .await
        .unwrap();
      assert_eq!(status.state, expected);
    }
    let response = json!({
      "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
      "content_release":"release-r1", "outcome":"invalid_payload",
      "error":{"code":"invalid_lifecycle_transition","message":"closed"}
    });
    let client =
      IslandPortPublicationClient::new(Arc::new(FakeTransport::with_responses(vec![response])));
    assert_eq!(
      client
        .abort(
          &context(),
          &pin(),
          &PublicationBuildId::parse(hash('a')).unwrap(),
          &PublicationIdempotencyKey::parse("abort-r1").unwrap(),
          &PublicationRequestFingerprint::derive("abort", &["r1"]).unwrap(),
        )
        .await
        .unwrap_err(),
      KnowledgeReleaseFailure::InvalidLifecycleTransition
    );
  }

  #[tokio::test]
  async fn batch_paths_preserve_identity_and_reject_edge_before_node_freeze() {
    let transport = Arc::new(FakeTransport::with_responses(vec![
      batch_response(&hash('b'), &hash('c')),
      json!({
        "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
        "content_release":"release-r1", "outcome":"invalid_payload",
        "error":{"code":"invalid_lifecycle_transition","message":"closed"}
      }),
    ]));
    let client = IslandPortPublicationClient::new(transport.clone());
    let node_input = BatchInputDto {
      build_id: hash('a'),
      ordinal: 0,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![json!({"point_id":"node-1"})],
    };
    let response: PublicationResponseDto<BatchValueDto> = client
      .call(
        NODE_BATCH_PATH,
        &context(),
        &pin(),
        node_input,
        MAX_PUBLICATION_BATCH_REQUEST_BYTES,
      )
      .await
      .unwrap();
    // The direct DTO test uses the exact wire values independently of the domain-derived fixture.
    let value = response.value().unwrap();
    assert_eq!(value.accepted_ordinal, 0);
    assert_eq!(value.accepted_request_fingerprint, hash('b'));
    assert_eq!(value.accepted_content_hash, hash('c'));
    let edge_input = BatchInputDto {
      build_id: hash('a'),
      ordinal: 0,
      request_fingerprint: hash('d'),
      content_hash: hash('e'),
      points: vec![json!({"point_id":"edge-1"})],
    };
    let response: PublicationResponseDto<BatchValueDto> = client
      .call(
        EDGE_BATCH_PATH,
        &context(),
        &pin(),
        edge_input,
        MAX_PUBLICATION_BATCH_REQUEST_BYTES,
      )
      .await
      .unwrap();
    assert!(matches!(
      response.value(),
      Err(KnowledgeReleaseFailure::InvalidLifecycleTransition)
    ));
    let calls = transport.requests.lock().unwrap();
    assert_eq!(calls[0].0, NODE_BATCH_PATH);
    assert_eq!(calls[1].0, EDGE_BATCH_PATH);
    assert_eq!(calls[0].1["input"]["ordinal"], 0);
  }

  #[tokio::test]
  async fn request_and_response_bounds_fail_before_accepting_oversized_content() {
    let transport = Arc::new(FakeTransport::default());
    let client = IslandPortPublicationClient::new(transport.clone());
    let input = BatchInputDto {
      build_id: hash('a'),
      ordinal: 0,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![json!({"payload":"x".repeat(MAX_PUBLICATION_BATCH_REQUEST_BYTES)})],
    };
    let result: Result<PublicationResponseDto<StatusValueDto>, _> = client
      .call(
        NODE_BATCH_PATH,
        &context(),
        &pin(),
        input,
        MAX_PUBLICATION_BATCH_REQUEST_BYTES,
      )
      .await;
    assert!(matches!(
      result,
      Err(KnowledgeReleaseFailure::SchemaIncompatible)
    ));
    assert!(transport.requests.lock().unwrap().is_empty());
  }

  #[test]
  fn batch_inspection_is_io_free_and_uses_the_send_serializer() {
    let transport = Arc::new(FakeTransport::default());
    let _client = IslandPortPublicationClient::new(transport.clone());
    let input = BatchInputDto {
      build_id: hash('a'),
      ordinal: 17,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![json!({"point_id":"node-1","dense_input":"Qysr"})],
    };
    let admission = inspect_batch(&pin(), input.clone()).unwrap();
    let planning_bytes = serialize_request(PublicationContextDto::planning(&pin()), input).unwrap();
    assert_eq!(admission.serialized_bytes(), planning_bytes.len());
    assert!(admission.fits());
    assert!(transport.requests.lock().unwrap().is_empty());
  }

  #[test]
  fn wire_admission_rejects_a_large_batch_below_the_point_limit() {
    let point = json!({"payload":"x".repeat(140_000)});
    let fitting = BatchInputDto {
      build_id: hash('a'),
      ordinal: 0,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![point.clone(); 7],
    };
    let oversized = BatchInputDto {
      points: vec![point; 8],
      ..fitting.clone()
    };

    let fitting_admission = inspect_batch(&pin(), fitting).unwrap();
    let oversized_admission = inspect_batch(&pin(), oversized).unwrap();

    assert!(fitting_admission.fits());
    assert!(matches!(
      oversized_admission,
      PublicationBatchWireAdmission::TooLarge { .. }
    ));
  }

  #[test]
  fn planning_context_is_the_maximum_legal_serialized_context() {
    let input = BatchInputDto {
      build_id: hash('a'),
      ordinal: u32::MAX,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![json!({"point_id":"node-1"})],
    };
    let planned = serialize_request(PublicationContextDto::planning(&pin()), input.clone())
      .unwrap()
      .len();
    for request_id in [
      "x".repeat(MAX_PUBLICATION_REQUEST_ID_BYTES),
      "\\".repeat(MAX_PUBLICATION_REQUEST_ID_BYTES),
      "\"".repeat(MAX_PUBLICATION_REQUEST_ID_BYTES),
    ] {
      for deadline_at in [
        PLANNING_DEADLINE_AT,
        "9999-12-31T23:59:59.999999999Z",
        "9999-12-31T23:59:59+00:00",
      ] {
        assert!(deadline_at.len() <= MAX_PUBLICATION_DEADLINE_BYTES);
        time::OffsetDateTime::parse(deadline_at, &time::format_description::well_known::Rfc3339)
          .unwrap();
        let actual = serialize_request(
          PublicationContextDto {
            request_id: request_id.clone(),
            deadline_at: deadline_at.into(),
            schema_version: SCHEMA_VERSION,
            content_release: pin().release_id.as_str().to_string(),
          },
          input.clone(),
        )
        .unwrap()
        .len();
        assert!(actual <= planned);
      }
    }
  }

  #[test]
  fn planning_fit_remains_within_the_limit_at_the_runtime_deadline_boundary() {
    assert_eq!(PLANNING_DEADLINE_AT.len(), MAX_PUBLICATION_DEADLINE_BYTES);
    time::OffsetDateTime::parse(
      PLANNING_DEADLINE_AT,
      &time::format_description::well_known::Rfc3339,
    )
    .unwrap();

    let input_for = |payload_bytes| BatchInputDto {
      build_id: hash('a'),
      ordinal: u32::MAX,
      request_fingerprint: hash('b'),
      content_hash: hash('c'),
      points: vec![json!({"payload":"x".repeat(payload_bytes)})],
    };
    let mut low = 0;
    let mut high = MAX_PUBLICATION_BATCH_REQUEST_BYTES;
    while low < high {
      let midpoint = low + (high - low).div_ceil(2);
      if inspect_batch(&pin(), input_for(midpoint)).unwrap().fits() {
        low = midpoint;
      } else {
        high = midpoint - 1;
      }
    }

    let input = input_for(low);
    let planning_bytes = inspect_batch(&pin(), input.clone())
      .unwrap()
      .serialized_bytes();
    let runtime_bytes = serialize_request(
      PublicationContextDto {
        request_id: "\\".repeat(MAX_PUBLICATION_REQUEST_ID_BYTES),
        deadline_at: PLANNING_DEADLINE_AT.into(),
        schema_version: SCHEMA_VERSION,
        content_release: pin().release_id.as_str().to_string(),
      },
      input,
    )
    .unwrap()
    .len();

    assert!(planning_bytes >= runtime_bytes);
    assert!(runtime_bytes <= MAX_PUBLICATION_BATCH_REQUEST_BYTES);
    assert!(MAX_PUBLICATION_BATCH_REQUEST_BYTES - planning_bytes < 8);
    assert!(!inspect_batch(&pin(), input_for(low + 1)).unwrap().fits());
  }

  #[tokio::test]
  async fn reconciliation_produces_candidate_without_activation() {
    let build_id = PublicationBuildId::parse(hash('a')).unwrap();
    let dictionary = LexicalDictionaryManifest::new(vec![("C".into(), 1)]).unwrap();
    let nodes = FrozenNodePublication {
      manifest: NodeCollectionManifest {
        release_id: pin().release_id,
        collection_id: NodeCollectionId::parse("nodes-r1").unwrap(),
        payload_schema_version: "knowledge-graph-v1".into(),
        content_hash: hash('b'),
        point_count: 2,
        state: ProjectionCollectionState::Verified,
      },
      persisted_hash: PersistedCollectionHash::parse(hash('d')).unwrap(),
      receipt: execution_receipt(
        &build_id,
        PublicationCollectionFamily::Nodes,
        &dictionary,
        2,
      ),
      dictionary: dictionary.clone(),
    };
    let edges = FrozenEdgePublication {
      manifest: EdgeCollectionManifest {
        release_id: pin().release_id,
        collection_id: EdgeCollectionId::parse("edges-r1").unwrap(),
        payload_schema_version: "knowledge-graph-v1".into(),
        content_hash: hash('c'),
        point_count: 1,
        verified_node_content_hash: hash('b'),
        expected_endpoint_count: 2,
        resolved_endpoint_count: 2,
        state: ProjectionCollectionState::Verified,
      },
      persisted_hash: PersistedCollectionHash::parse(hash('e')).unwrap(),
      receipt: execution_receipt(
        &build_id,
        PublicationCollectionFamily::Edges,
        &dictionary,
        1,
      ),
      dictionary,
    };
    let manifest_hash = PublicationManifestHash::parse(hash('f')).unwrap();
    let reconcile_id = crate::domain::knowledge_publication::PublicationReconcileIdentity::derive(
      &build_id,
      &hash('9'),
      &nodes.persisted_hash,
      &nodes.manifest.collection_id,
      &edges.persisted_hash,
      &edges.manifest.collection_id,
    )
    .unwrap();
    let request = ReconcilePublication {
      build_id: build_id.clone(),
      reconcile_id,
      canonical: pin(),
      canonical_content_hash: hash('9'),
      nodes,
      edges,
      manifest_hash: manifest_hash.clone(),
      compatibility: compatibility(),
    };
    let response = json!({
      "request_id":"req-publication-test", "schema_version":SCHEMA_VERSION,
      "content_release":"release-r1", "outcome":"ok",
      "value":{
        "build_id":hash('a'), "reconcile_id":request.reconcile_id.as_str(), "node_persisted_hash":hash('d'),
        "edge_persisted_hash":hash('e'), "publication_manifest_hash":hash('f'),
        "canonical_content_hash":hash('9'),
        "state":"activation_candidate"
      }
    });
    let candidate =
      IslandPortPublicationClient::new(Arc::new(FakeTransport::with_responses(vec![response])))
        .reconcile(&context(), &request)
        .await
        .unwrap();
    assert_eq!(candidate.manifest_hash, manifest_hash);
    assert_eq!(candidate.trio.canonical(), &request.canonical);
  }

  #[test]
  fn freeze_receipt_mapping_rejects_revision_dimensions_encoder_dictionary_and_counts() {
    let build = PublicationBuildId::parse(hash('a')).unwrap();
    let dictionary =
      LexicalDictionaryManifest::new(vec![("C".into(), 1), ("C#".into(), 2), ("C++".into(), 3)])
        .unwrap();
    let expected = FreezeNodes {
      build_id: build.clone(),
      finalize_id: crate::domain::knowledge_publication::PublicationFinalizeIdentity::derive(
        &build,
        PublicationCollectionFamily::Nodes,
        &PersistedCollectionHash::parse(hash('d')).unwrap(),
      ),
      persisted_hash: PersistedCollectionHash::parse(hash('d')).unwrap(),
      projection_hash: hash('b'),
      projection_schema_version: "knowledge-graph-v1".into(),
      point_count: 3,
      dictionary,
      compatibility: compatibility(),
    };
    let base = json!({
      "build_id": hash('a'), "finalize_id":expected.finalize_id.as_str(), "collection_id":"nodes-r1",
      "persisted_collection_hash":hash('d'), "projection_content_hash":hash('b'),
      "point_count":3,
      "receipt":{
        "dense":{"registry_entry_id":"fixture-qwen-r1","observed_artifact_revision":"sha256:test-immutable-artifact-r1","dimensions":1024,"vector_name":"semantic","input_specification":NODE_DENSE_INPUT_VERSION,"processed_point_count":3},
        "lexical":{"encoder_identity":"transnet-lexical-bm25","encoder_revision":"v1","vector_name":"lexical","input_specification":NODE_LEXICAL_INPUT_VERSION,"dictionary_hash":expected.dictionary.dictionary_hash(),"processed_point_count":3}
      }
    });
    let value: FreezeValueDto = serde_json::from_value(base.clone()).unwrap();
    assert!(value.nodes(&pin(), &expected).is_ok());
    let mutations = [
      (
        "/receipt/dense/observed_artifact_revision",
        json!("drift"),
        KnowledgeReleaseFailure::ArtifactRevisionMismatch,
      ),
      (
        "/receipt/dense/dimensions",
        json!(768),
        KnowledgeReleaseFailure::EmbeddingMetadataIncompatible,
      ),
      (
        "/receipt/lexical/encoder_revision",
        json!("v2"),
        KnowledgeReleaseFailure::LexicalEncoderMismatch,
      ),
      (
        "/receipt/lexical/dictionary_hash",
        json!(hash('e')),
        KnowledgeReleaseFailure::DictionaryMismatch,
      ),
      (
        "/receipt/dense/processed_point_count",
        json!(2),
        KnowledgeReleaseFailure::IncompleteTrio,
      ),
    ];
    for (pointer, replacement, failure) in mutations {
      let mut changed = base.clone();
      *changed.pointer_mut(pointer).unwrap() = replacement;
      let value: FreezeValueDto = serde_json::from_value(changed).unwrap();
      assert_eq!(value.nodes(&pin(), &expected).unwrap_err(), failure);
    }
  }

  #[test]
  fn debug_output_redacts_transport_and_idempotency_details() {
    let client = IslandPortPublicationClient::new(Arc::new(FakeTransport::default()));
    assert_eq!(
      format!("{client:?}"),
      "IslandPortPublicationClient([redacted])"
    );
    let key = PublicationIdempotencyKey::parse("credential-like-secret").unwrap();
    assert!(!format!("{key:?}").contains("credential-like-secret"));
  }
}
