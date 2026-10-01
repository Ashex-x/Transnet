//! Deterministic pre-publication knowledge-node and relationship-edge projections.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::canonical::{
  CanonicalId, CanonicalStatus, EvidenceConfidence, LexicalPartOfSpeech, ReleaseId,
};
use super::embedding_input::{
  AuthoritativeEmbeddingMaterial, CanonicalEmbeddingInput, EmbeddingInputFamily,
  EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION, NODE_DENSE_INPUT_VERSION,
  NODE_LEXICAL_INPUT_VERSION,
};
use super::graph::{
  validate_published_relationships, GraphNodeKey, GraphNodeKind, GraphScope, GraphValidationError,
  PublishedEdgeIdentity, PublishedRelationship, RelationshipVerificationState,
};
use super::knowledge_release::{DenseEmbeddingVersion, SparseEmbeddingVersion};

/// Fixed dense-vector name in the authoritative Qdrant contract.
pub const DENSE_VECTOR_NAME: &str = "semantic";
/// Fixed sparse-vector name in the authoritative Qdrant contract.
pub const SPARSE_VECTOR_NAME: &str = "lexical";
/// Frozen dense model family for the first controlled execution contract.
pub const DENSE_MODEL_FAMILY: &str = "Qwen/Qwen3-Embedding-0.6B";
/// Frozen dense dimensions for the first controlled execution contract.
pub const DENSE_DIMENSIONS: u32 = 1024;
/// Frozen deterministic lexical encoder identity.
pub const LEXICAL_ENCODER_IDENTITY: &str = "transnet-lexical-bm25";
/// Frozen deterministic lexical encoder revision.
pub const LEXICAL_ENCODER_REVISION: &str = "v1";
/// Frozen lexical execution contract identity.
pub const LEXICAL_CONTRACT_IDENTITY: &str = "transnet-lexical-bm25-v1";
/// Version of the canonical binary serialization used for projection hashes.
pub const PROJECTION_HASH_VERSION: &str = "knowledge-projection-hash-v1";

const MAX_VERSION_LENGTH: usize = 128;

/// One closed compatibility entry configured by the controlled embedding authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingCompatibilityEntry {
  /// Stable registry-entry identity used by execution receipts.
  pub entry_id: String,
  /// Approved dense model family.
  pub dense_model_family: String,
  /// Exact immutable dense artifact revision.
  pub dense_artifact_revision: String,
  /// Only accepted dimensions for that dense revision.
  pub dense_dimensions: u32,
  /// Closed dense named-vector slot.
  pub dense_vector_name: String,
  /// Dense node input specification accepted by the revision.
  pub node_dense_input_specification: String,
  /// Dense edge input specification accepted by the revision.
  pub edge_dense_input_specification: String,
  /// Stable deterministic lexical encoder identity.
  pub lexical_encoder_identity: String,
  /// Exact deterministic lexical encoder identity and revision.
  pub lexical_encoder_revision: String,
  /// Stable lexical contract identity.
  pub lexical_contract_identity: String,
  /// Closed lexical named-vector slot.
  pub lexical_vector_name: String,
  /// Lexical node input specification accepted by the encoder.
  pub node_lexical_input_specification: String,
  /// Lexical edge input specification accepted by the encoder.
  pub edge_lexical_input_specification: String,
}

/// Closed model/encoder compatibility registry; absence is never treated as a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingCompatibilityRegistry {
  entries: Vec<EmbeddingCompatibilityEntry>,
}

impl EmbeddingCompatibilityRegistry {
  /// Creates a registry whose entries must be unique by dense and lexical revision pair.
  pub fn new(
    mut entries: Vec<EmbeddingCompatibilityEntry>,
  ) -> Result<Self, ProjectionValidationError> {
    entries.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    if entries
      .windows(2)
      .any(|pair| pair[0].entry_id == pair[1].entry_id)
    {
      return Err(ProjectionValidationError::InvalidEmbeddingMetadata);
    }
    for entry in &entries {
      validate_version(&entry.entry_id)?;
      validate_version(&entry.dense_model_family)?;
      validate_exact_revision(&entry.dense_artifact_revision)?;
      validate_version(&entry.lexical_encoder_revision)?;
      validate_version(&entry.dense_vector_name)?;
      validate_version(&entry.node_dense_input_specification)?;
      validate_version(&entry.edge_dense_input_specification)?;
      validate_version(&entry.lexical_encoder_identity)?;
      validate_version(&entry.lexical_contract_identity)?;
      validate_version(&entry.lexical_vector_name)?;
      validate_version(&entry.node_lexical_input_specification)?;
      validate_version(&entry.edge_lexical_input_specification)?;
      if entry.dense_dimensions == 0
        || entry.dense_model_family != DENSE_MODEL_FAMILY
        || entry.dense_dimensions != DENSE_DIMENSIONS
        || entry.dense_vector_name != DENSE_VECTOR_NAME
        || entry.node_dense_input_specification != NODE_DENSE_INPUT_VERSION
        || entry.edge_dense_input_specification != EDGE_DENSE_INPUT_VERSION
        || entry.lexical_encoder_identity != LEXICAL_ENCODER_IDENTITY
        || entry.lexical_encoder_revision != LEXICAL_ENCODER_REVISION
        || entry.lexical_contract_identity != LEXICAL_CONTRACT_IDENTITY
        || entry.lexical_vector_name != SPARSE_VECTOR_NAME
        || entry.node_lexical_input_specification != NODE_LEXICAL_INPUT_VERSION
        || entry.edge_lexical_input_specification != EDGE_LEXICAL_INPUT_VERSION
      {
        return Err(ProjectionValidationError::InvalidEmbeddingMetadata);
      }
    }
    Ok(Self { entries })
  }

  /// Fails closed unless an exact model revision, dimensions, encoder, and input-spec tuple exists.
  pub fn validate(
    &self,
    projection: &ProjectionEmbeddingSpec,
  ) -> Result<(), ProjectionValidationError> {
    self
      .entries
      .iter()
      .any(|entry| {
        entry.dense_model_family == projection.dense.model_family
          && entry.dense_artifact_revision == projection.dense.artifact_revision
          && entry.dense_dimensions == projection.dense.dimensions
          && entry.dense_vector_name == projection.dense_vector_name
          && entry.lexical_encoder_identity == projection.sparse.encoder_identity
          && entry.lexical_encoder_revision == projection.sparse.encoder_revision
          && entry.lexical_vector_name == projection.sparse_vector_name
          && entry.node_dense_input_specification
            == super::embedding_input::NODE_DENSE_INPUT_VERSION
          && entry.edge_dense_input_specification
            == super::embedding_input::EDGE_DENSE_INPUT_VERSION
          && entry.node_lexical_input_specification
            == super::embedding_input::NODE_LEXICAL_INPUT_VERSION
          && entry.edge_lexical_input_specification
            == super::embedding_input::EDGE_LEXICAL_INPUT_VERSION
      })
      .then_some(())
      .ok_or(ProjectionValidationError::EmbeddingCompatibilityMissing)
  }

  /// Returns one exact approved entry, or `None` when deployment has not registered it.
  pub fn entry(&self, entry_id: &str) -> Option<&EmbeddingCompatibilityEntry> {
    self.entries.iter().find(|entry| entry.entry_id == entry_id)
  }
}

/// Stable deterministic point identifier for a future Qdrant projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionPointId(String);

impl ProjectionPointId {
  /// Returns the deterministic point identifier.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Dense and sparse embedding requirements shared by one projection build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionEmbeddingSpec {
  payload_schema_version: String,
  dense_vector_name: String,
  dense: DenseEmbeddingVersion,
  sparse_vector_name: String,
  sparse: SparseEmbeddingVersion,
}

impl ProjectionEmbeddingSpec {
  /// Creates one complete, contract-compatible embedding requirement.
  ///
  /// # Errors
  ///
  /// Returns an error when a schema/model version is invalid or dense dimensions are zero.
  pub fn new(
    payload_schema_version: impl Into<String>,
    dense: DenseEmbeddingVersion,
    sparse: SparseEmbeddingVersion,
  ) -> Result<Self, ProjectionValidationError> {
    let payload_schema_version = payload_schema_version.into();
    validate_version(&payload_schema_version)?;
    validate_version(&dense.model_family)?;
    validate_exact_revision(&dense.artifact_revision)?;
    validate_version(&sparse.encoder_identity)?;
    validate_version(&sparse.encoder_revision)?;
    if dense.dimensions == 0 {
      return Err(ProjectionValidationError::InvalidEmbeddingMetadata);
    }
    Ok(Self {
      payload_schema_version,
      dense_vector_name: DENSE_VECTOR_NAME.to_string(),
      dense,
      sparse_vector_name: SPARSE_VECTOR_NAME.to_string(),
      sparse,
    })
  }

  /// Returns the closed projection payload schema.
  pub fn payload_schema_version(&self) -> &str {
    &self.payload_schema_version
  }

  /// Returns the authoritative named dense-vector slot.
  pub fn dense_vector_name(&self) -> &str {
    &self.dense_vector_name
  }

  /// Returns the required dense model revision and dimensions.
  pub fn dense(&self) -> &DenseEmbeddingVersion {
    &self.dense
  }

  /// Returns the authoritative named sparse-vector slot.
  pub fn sparse_vector_name(&self) -> &str {
    &self.sparse_vector_name
  }

  /// Returns the required sparse model revision.
  pub fn sparse(&self) -> &SparseEmbeddingVersion {
    &self.sparse
  }
}

/// Authoritative canonical input accepted by the current node projector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalNodeProjectionInput {
  /// One active canonical lexeme with fully resolved authoritative embedding material.
  Lexeme(AuthoritativeEmbeddingMaterial),
  /// One active sense with fully resolved authoritative embedding material.
  Sense(AuthoritativeEmbeddingMaterial),
  /// A graph family for which no authoritative publisher source is currently frozen.
  Unresolved(GraphNodeKey),
}

/// Deterministic payload for an admitted canonical node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeProjectionPayload {
  /// Canonical lexeme payload.
  Lexeme {
    /// Language-specific source spelling.
    lemma: String,
    /// Explicit normalized lookup form.
    normalized_lemma: String,
    /// Canonical BCP-47 language tag.
    language: String,
    /// Canonical part of speech.
    part_of_speech: LexicalPartOfSpeech,
  },
  /// Canonical sense payload.
  Sense {
    /// Owning lexeme identity.
    lexeme_id: CanonicalId,
    /// Source-stable key within the lexeme.
    sense_key: String,
    /// Canonical definition.
    definition: String,
    /// Owning lexeme's source spelling.
    lemma: String,
    /// Canonical BCP-47 language tag.
    language: String,
    /// Canonical part of speech.
    part_of_speech: LexicalPartOfSpeech,
    /// Ordered evidence identities supporting the definition.
    definition_evidence_ids: Vec<CanonicalId>,
  },
}

/// One deterministic node point ready for a later embedding and Qdrant write step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeProjection {
  /// Stable point identity derived from release, schema, and canonical node identity.
  pub point_id: ProjectionPointId,
  /// Typed canonical node identity.
  pub node: GraphNodeKey,
  /// Immutable release owning the node.
  pub release_id: ReleaseId,
  /// Deterministic payload free of runtime metadata.
  pub payload: NodeProjectionPayload,
  /// Payload schema used by this point.
  pub payload_schema_version: String,
  /// Dense and sparse embedding requirements; no vectors are generated here.
  pub embedding: ProjectionEmbeddingSpec,
  /// Frozen semantic input; no vector is generated by this artifact.
  pub dense_input: CanonicalEmbeddingInput,
  /// Frozen deterministic lexical-encoder input.
  pub lexical_input: CanonicalEmbeddingInput,
  /// Hash of the canonical point serialization.
  pub content_hash: String,
}

/// Ordered, deterministic node pre-publication artifact without a collection identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeProjectionBuild {
  /// Immutable canonical release represented by every point.
  pub release_id: ReleaseId,
  /// Embedding and payload-schema requirements.
  pub embedding: ProjectionEmbeddingSpec,
  /// Points ordered by typed canonical identity.
  pub points: Vec<NodeProjection>,
  /// Hash of the complete ordered projection.
  pub content_hash: String,
  /// Number of unique points covered by the build hash.
  pub point_count: u64,
}

/// Evidence and source identities retained by an edge projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectionEvidenceReference {
  /// Canonical evidence fragment identity.
  pub evidence_id: CanonicalId,
  /// Canonical source-policy identity.
  pub source_id: CanonicalId,
  /// Authoritative immutable fragment content hash.
  pub content_hash: String,
}

/// Verification metadata retained from the admitted relationship revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeVerificationMetadata {
  /// Positive immutable relationship revision.
  pub relationship_revision: u32,
  /// Closed evidence confidence category.
  pub evidence_confidence: EvidenceConfidence,
  /// Verified lifecycle state; exploratory relationships never reach this type.
  pub verification_state: RelationshipVerificationState,
}

/// One deterministic edge point ready for a later embedding and Qdrant write step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeProjection {
  /// Stable point identity derived without runtime or insertion-order inputs.
  pub point_id: ProjectionPointId,
  /// Stage 2 canonical relationship identity.
  pub identity: PublishedEdgeIdentity,
  /// Immutable release owning the relationship and both endpoints.
  pub release_id: ReleaseId,
  /// Resolved source node point identity.
  pub source_point_id: ProjectionPointId,
  /// Resolved target node point identity.
  pub target_point_id: ProjectionPointId,
  /// Exact relationship name frozen by the Qdrant contract.
  pub wire_relation: String,
  /// Admitted bounded scope.
  pub scope: GraphScope,
  /// Ordered evidence and provenance references.
  pub evidence: Vec<ProjectionEvidenceReference>,
  /// Immutable review metadata.
  pub verification: EdgeVerificationMetadata,
  /// Structured edge input bound to both endpoint semantic inputs.
  pub dense_input: CanonicalEmbeddingInput,
  /// Structured edge input bound to both endpoint lexical inputs.
  pub lexical_input: CanonicalEmbeddingInput,
  /// Hash of the canonical point serialization.
  pub content_hash: String,
}

/// Ordered deterministic edge artifact bound to one exact node build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeProjectionBuild {
  /// Immutable canonical release represented by every point.
  pub release_id: ReleaseId,
  /// Embedding and payload-schema requirements.
  pub embedding: ProjectionEmbeddingSpec,
  /// Exact node build hash against which endpoints were checked.
  pub verified_node_content_hash: String,
  /// Points ordered by admitted relationship identity.
  pub points: Vec<EdgeProjection>,
  /// Hash of the complete ordered edge projection.
  pub content_hash: String,
  /// Number of unique edge points covered by the build hash.
  pub point_count: u64,
  /// Number of endpoints required by all projected relationships.
  pub expected_endpoint_count: u64,
  /// Number of endpoints resolved in the pinned node build.
  pub resolved_endpoint_count: u64,
}

/// Closed, redacted failure categories for deterministic projection preparation.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ProjectionValidationError {
  /// A schema or model version is blank or oversized.
  #[error("projection version metadata is invalid")]
  InvalidVersion,
  /// Embedding dimensions or vector metadata are incompatible.
  #[error("projection embedding metadata is invalid")]
  InvalidEmbeddingMetadata,
  /// No exact controlled model/encoder compatibility registry entry exists.
  #[error("embedding compatibility registry has no exact match")]
  EmbeddingCompatibilityMissing,
  /// Canonical content belongs to another immutable release.
  #[error("projection input belongs to another release")]
  ReleaseMismatch,
  /// A canonical record is not active and publication-eligible.
  #[error("canonical projection input is not publication eligible")]
  IneligibleCanonicalRecord,
  /// The node family lacks a frozen authoritative projection source.
  #[error("canonical node family is not projection-ready")]
  UnresolvedNodeFamily,
  /// A sense did not match its owning lexeme.
  #[error("sense projection does not match its owning lexeme")]
  SenseLexemeMismatch,
  /// The same typed node identity was supplied with conflicting content.
  #[error("canonical node projection conflicts with a duplicate identity")]
  ConflictingDuplicateNode,
  /// A relationship source endpoint was absent from the verified node build.
  #[error("relationship source endpoint is missing")]
  MissingSourceEndpoint,
  /// A relationship target endpoint was absent from the verified node build.
  #[error("relationship target endpoint is missing")]
  MissingTargetEndpoint,
  /// An edge build used incompatible release, schema, or embedding metadata.
  #[error("edge projection is incompatible with the node build")]
  NodeBuildMismatch,
  /// A point or endpoint count exceeded its bounded integer representation.
  #[error("projection count overflow")]
  CountOverflow,
  /// Stage 2 relationship admission rejected the candidate batch.
  #[error("relationship admission rejected projection input")]
  RelationshipAdmission(#[source] GraphValidationError),
  /// Authoritative lexical material did not satisfy embedding admission.
  #[error("authoritative embedding material is invalid")]
  EmbeddingMaterial(#[source] super::embedding_input::EmbeddingMaterialError),
}

impl From<GraphValidationError> for ProjectionValidationError {
  fn from(value: GraphValidationError) -> Self {
    Self::RelationshipAdmission(value)
  }
}

impl From<super::embedding_input::EmbeddingMaterialError> for ProjectionValidationError {
  fn from(value: super::embedding_input::EmbeddingMaterialError) -> Self {
    Self::EmbeddingMaterial(value)
  }
}

/// Builds an ordered node artifact for one immutable release.
///
/// Exact duplicate inputs are deterministically collapsed; conflicting content for one identity
/// fails closed.
///
/// # Errors
///
/// Returns an error for cross-release, inactive, unresolved, or conflicting canonical input.
pub fn build_node_projection(
  release_id: ReleaseId,
  embedding: ProjectionEmbeddingSpec,
  inputs: Vec<CanonicalNodeProjectionInput>,
) -> Result<NodeProjectionBuild, ProjectionValidationError> {
  let mut points = BTreeMap::<GraphNodeKey, NodeProjection>::new();
  for input in inputs {
    let point = project_node(&release_id, &embedding, input)?;
    match points.get(&point.node) {
      Some(existing) if existing == &point => {}
      Some(_) => return Err(ProjectionValidationError::ConflictingDuplicateNode),
      None => {
        points.insert(point.node.clone(), point);
      }
    }
  }
  let points = points.into_values().collect::<Vec<_>>();
  let point_count =
    u64::try_from(points.len()).map_err(|_| ProjectionValidationError::CountOverflow)?;
  let content_hash = hash_node_build(&release_id, &embedding, &points);
  Ok(NodeProjectionBuild {
    release_id,
    embedding,
    points,
    content_hash,
    point_count,
  })
}

/// Builds an ordered edge artifact after resolving every endpoint against one node artifact.
///
/// # Errors
///
/// Returns an error when Stage 2 admission fails, build metadata differs, or any endpoint is
/// missing. No placeholder node or inverse relationship is synthesized.
pub fn build_edge_projection(
  nodes: &NodeProjectionBuild,
  embedding: ProjectionEmbeddingSpec,
  relationships: Vec<PublishedRelationship>,
) -> Result<EdgeProjectionBuild, ProjectionValidationError> {
  if nodes.embedding != embedding {
    return Err(ProjectionValidationError::NodeBuildMismatch);
  }
  validate_published_relationships(&relationships)?;
  let node_points = nodes
    .points
    .iter()
    .map(|point| (point.node.clone(), point))
    .collect::<BTreeMap<_, _>>();
  let mut ordered = relationships;
  ordered.sort_by_key(PublishedRelationship::identity);
  let mut points = Vec::with_capacity(ordered.len());
  for relationship in ordered {
    if relationship.relation_release_id != nodes.release_id {
      return Err(ProjectionValidationError::ReleaseMismatch);
    }
    let identity = relationship.identity();
    let source_point = node_points
      .get(&identity.source)
      .ok_or(ProjectionValidationError::MissingSourceEndpoint)?;
    let target_point = node_points
      .get(&identity.target)
      .ok_or(ProjectionValidationError::MissingTargetEndpoint)?;
    points.push(project_edge(
      &embedding,
      relationship,
      identity,
      source_point,
      target_point,
    )?);
  }
  let point_count =
    u64::try_from(points.len()).map_err(|_| ProjectionValidationError::CountOverflow)?;
  let expected_endpoint_count = point_count
    .checked_mul(2)
    .ok_or(ProjectionValidationError::CountOverflow)?;
  let resolved_endpoint_count = expected_endpoint_count;
  let content_hash = hash_edge_build(
    nodes,
    &embedding,
    &points,
    expected_endpoint_count,
    resolved_endpoint_count,
  );
  Ok(EdgeProjectionBuild {
    release_id: nodes.release_id.clone(),
    embedding,
    verified_node_content_hash: nodes.content_hash.clone(),
    points,
    content_hash,
    point_count,
    expected_endpoint_count,
    resolved_endpoint_count,
  })
}

fn project_node(
  release_id: &ReleaseId,
  embedding: &ProjectionEmbeddingSpec,
  input: CanonicalNodeProjectionInput,
) -> Result<NodeProjection, ProjectionValidationError> {
  let (node, payload, dense_input, lexical_input) = match input {
    CanonicalNodeProjectionInput::Lexeme(material) => {
      let lexeme = material.lexeme();
      if material.sense().is_some() {
        return Err(ProjectionValidationError::SenseLexemeMismatch);
      }
      require_active_release(&lexeme.release_id, lexeme.status, release_id)?;
      let node = GraphNodeKey::new(GraphNodeKind::Lexeme, lexeme.id.clone());
      let payload = NodeProjectionPayload::Lexeme {
        lemma: lexeme.lemma.clone(),
        normalized_lemma: lexeme.normalized_lemma.clone(),
        language: lexeme.language.as_str().to_string(),
        part_of_speech: lexeme.part_of_speech,
      };
      let dense = material.dense_input(&node.id)?;
      let lexical = material.lexical_input(&node.id)?;
      (node, payload, dense, lexical)
    }
    CanonicalNodeProjectionInput::Sense(material) => {
      let lexeme = material.lexeme();
      let sense = material
        .sense()
        .ok_or(ProjectionValidationError::SenseLexemeMismatch)?;
      require_active_release(&sense.release_id, sense.status, release_id)?;
      require_active_release(&lexeme.release_id, lexeme.status, release_id)?;
      if sense.lexeme_id != lexeme.id || sense.release_id != lexeme.release_id {
        return Err(ProjectionValidationError::SenseLexemeMismatch);
      }
      let node = GraphNodeKey::new(GraphNodeKind::Sense, sense.id.clone());
      let mut definition_evidence_ids = sense.definition_evidence_ids.clone();
      definition_evidence_ids.sort();
      definition_evidence_ids.dedup();
      let payload = NodeProjectionPayload::Sense {
        lexeme_id: lexeme.id.clone(),
        sense_key: sense.sense_key.clone(),
        definition: sense.definition.clone(),
        lemma: lexeme.lemma.clone(),
        language: lexeme.language.as_str().to_string(),
        part_of_speech: lexeme.part_of_speech,
        definition_evidence_ids,
      };
      let dense = material.dense_input(&node.id)?;
      let lexical = material.lexical_input(&node.id)?;
      (node, payload, dense, lexical)
    }
    CanonicalNodeProjectionInput::Unresolved(_) => {
      return Err(ProjectionValidationError::UnresolvedNodeFamily);
    }
  };
  let point_id = point_id(
    "node",
    release_id,
    &embedding.payload_schema_version,
    &node_key_bytes(&node),
  );
  let content_hash = hash_node_point(
    &point_id,
    release_id,
    embedding,
    &node,
    &payload,
    &dense_input,
    &lexical_input,
  );
  Ok(NodeProjection {
    point_id,
    node,
    release_id: release_id.clone(),
    payload,
    payload_schema_version: embedding.payload_schema_version.clone(),
    embedding: embedding.clone(),
    dense_input,
    lexical_input,
    content_hash,
  })
}

fn project_edge(
  embedding: &ProjectionEmbeddingSpec,
  relationship: PublishedRelationship,
  identity: PublishedEdgeIdentity,
  source_point: &NodeProjection,
  target_point: &NodeProjection,
) -> Result<EdgeProjection, ProjectionValidationError> {
  let mut evidence = relationship
    .evidence_lineage
    .iter()
    .map(|lineage| ProjectionEvidenceReference {
      evidence_id: lineage.fragment().id.clone(),
      source_id: lineage.source().id.clone(),
      content_hash: lineage.fragment().content_hash.clone(),
    })
    .collect::<Vec<_>>();
  evidence.sort();
  let verification = EdgeVerificationMetadata {
    relationship_revision: relationship.relation.relation_version.get(),
    evidence_confidence: relationship.relation.evidence.confidence,
    verification_state: relationship.verification_state,
  };
  let release_id = relationship.relation_release_id;
  let wire_relation = relationship.declared_wire_relation;
  let scope = relationship.relation.scope;
  let point_id = point_id(
    "edge",
    &release_id,
    &embedding.payload_schema_version,
    &edge_identity_bytes(&identity),
  );
  let dense_input = edge_input(
    EmbeddingInputFamily::Dense,
    EDGE_DENSE_INPUT_VERSION,
    &identity,
    &wire_relation,
    &scope,
    &source_point.dense_input,
    &target_point.dense_input,
    &evidence,
    &verification,
  )?;
  let lexical_input = edge_input(
    EmbeddingInputFamily::Lexical,
    EDGE_LEXICAL_INPUT_VERSION,
    &identity,
    &wire_relation,
    &scope,
    &source_point.lexical_input,
    &target_point.lexical_input,
    &evidence,
    &verification,
  )?;
  let mut projection = EdgeProjection {
    point_id,
    identity,
    release_id,
    source_point_id: source_point.point_id.clone(),
    target_point_id: target_point.point_id.clone(),
    wire_relation,
    scope,
    evidence,
    verification,
    dense_input,
    lexical_input,
    content_hash: String::new(),
  };
  projection.content_hash = hash_edge_point(embedding, &projection);
  Ok(projection)
}

fn require_active_release(
  actual: &ReleaseId,
  status: CanonicalStatus,
  expected: &ReleaseId,
) -> Result<(), ProjectionValidationError> {
  if actual != expected {
    return Err(ProjectionValidationError::ReleaseMismatch);
  }
  if status != CanonicalStatus::Active {
    return Err(ProjectionValidationError::IneligibleCanonicalRecord);
  }
  Ok(())
}

#[allow(clippy::too_many_arguments)]
fn edge_input(
  family: EmbeddingInputFamily,
  version: &'static str,
  identity: &PublishedEdgeIdentity,
  wire_relation: &str,
  scope: &GraphScope,
  source: &CanonicalEmbeddingInput,
  target: &CanonicalEmbeddingInput,
  evidence: &[ProjectionEvidenceReference],
  verification: &EdgeVerificationMetadata,
) -> Result<CanonicalEmbeddingInput, ProjectionValidationError> {
  CanonicalEmbeddingInput::from_fields(family, version, |out| {
    out.field("release", identity.release_id.as_str());
    out.field("relationship_id", identity.relationship_id.as_str());
    out.field(
      "relationship_revision",
      &identity.relationship_revision.get().to_string(),
    );
    out.field("source_node_input_hash", source.input_hash());
    out.field("wire_relation", wire_relation);
    out.field("target_node_input_hash", target.input_hash());
    out.optional(
      "scope_dialect",
      scope.dialect.as_ref().map(|value| value.as_str()),
    );
    out.optional("scope_domain", scope.domain.as_deref());
    out.optional("scope_register", scope.register.as_deref());
    out.optional("scope_note", scope.note.as_deref());
    out.list(
      "evidence_ids",
      evidence
        .iter()
        .map(|item| item.evidence_id.as_str().to_string()),
    );
    out.list(
      "evidence_source_ids",
      evidence
        .iter()
        .map(|item| item.source_id.as_str().to_string()),
    );
    out.list(
      "evidence_content_hashes",
      evidence.iter().map(|item| item.content_hash.clone()),
    );
    out.field(
      "evidence_confidence",
      confidence_name(verification.evidence_confidence),
    );
    out.field("verification_state", "verified");
  })
  .map_err(ProjectionValidationError::from)
}

fn validate_version(value: &str) -> Result<(), ProjectionValidationError> {
  if value.trim().is_empty() || value.len() > MAX_VERSION_LENGTH {
    Err(ProjectionValidationError::InvalidVersion)
  } else {
    Ok(())
  }
}

fn validate_exact_revision(value: &str) -> Result<(), ProjectionValidationError> {
  validate_version(value)?;
  if matches!(
    value.to_ascii_lowercase().as_str(),
    "latest" | "main" | "master" | "head"
  ) {
    Err(ProjectionValidationError::InvalidEmbeddingMetadata)
  } else {
    Ok(())
  }
}

fn point_id(kind: &str, release: &ReleaseId, schema: &str, identity: &[u8]) -> ProjectionPointId {
  let mut bytes = CanonicalBytes::new();
  bytes.string(PROJECTION_HASH_VERSION);
  bytes.string(kind);
  bytes.string(release.as_str());
  bytes.string(schema);
  bytes.bytes(identity);
  ProjectionPointId(format!("sha256:{}", hex_digest(bytes.finish())))
}

fn hash_node_point(
  point_id: &ProjectionPointId,
  release: &ReleaseId,
  embedding: &ProjectionEmbeddingSpec,
  node: &GraphNodeKey,
  payload: &NodeProjectionPayload,
  dense_input: &CanonicalEmbeddingInput,
  lexical_input: &CanonicalEmbeddingInput,
) -> String {
  let mut out = CanonicalBytes::new();
  out.string(PROJECTION_HASH_VERSION);
  out.string("node-point");
  out.string(point_id.as_str());
  out.string(release.as_str());
  embedding_bytes(&mut out, embedding);
  out.bytes(&node_key_bytes(node));
  payload_bytes(&mut out, payload);
  out.string(dense_input.input_hash());
  out.string(lexical_input.input_hash());
  format!("sha256:{}", hex_digest(out.finish()))
}

fn hash_node_build(
  release: &ReleaseId,
  embedding: &ProjectionEmbeddingSpec,
  points: &[NodeProjection],
) -> String {
  let mut out = CanonicalBytes::new();
  out.string(PROJECTION_HASH_VERSION);
  out.string("node-build");
  out.string(release.as_str());
  embedding_bytes(&mut out, embedding);
  out.usize(points.len());
  for point in points {
    out.string(point.point_id.as_str());
    out.string(&point.content_hash);
  }
  format!("sha256:{}", hex_digest(out.finish()))
}

fn hash_edge_point(embedding: &ProjectionEmbeddingSpec, projection: &EdgeProjection) -> String {
  let mut out = CanonicalBytes::new();
  out.string(PROJECTION_HASH_VERSION);
  out.string("edge-point");
  out.string(projection.point_id.as_str());
  out.string(projection.release_id.as_str());
  embedding_bytes(&mut out, embedding);
  out.bytes(&edge_identity_bytes(&projection.identity));
  out.string(projection.source_point_id.as_str());
  out.string(projection.target_point_id.as_str());
  out.string(&projection.wire_relation);
  scope_bytes(&mut out, &projection.scope);
  out.usize(projection.evidence.len());
  for item in &projection.evidence {
    out.string(item.evidence_id.as_str());
    out.string(item.source_id.as_str());
    out.string(&item.content_hash);
  }
  out.u32(projection.verification.relationship_revision);
  out.string(confidence_name(projection.verification.evidence_confidence));
  out.string("verified");
  out.string(projection.dense_input.input_hash());
  out.string(projection.lexical_input.input_hash());
  format!("sha256:{}", hex_digest(out.finish()))
}

fn hash_edge_build(
  nodes: &NodeProjectionBuild,
  embedding: &ProjectionEmbeddingSpec,
  points: &[EdgeProjection],
  expected: u64,
  resolved: u64,
) -> String {
  let mut out = CanonicalBytes::new();
  out.string(PROJECTION_HASH_VERSION);
  out.string("edge-build");
  out.string(nodes.release_id.as_str());
  embedding_bytes(&mut out, embedding);
  out.string(&nodes.content_hash);
  out.u64(expected);
  out.u64(resolved);
  out.usize(points.len());
  for point in points {
    out.string(point.point_id.as_str());
    out.string(&point.content_hash);
  }
  format!("sha256:{}", hex_digest(out.finish()))
}

fn embedding_bytes(out: &mut CanonicalBytes, spec: &ProjectionEmbeddingSpec) {
  out.string(&spec.payload_schema_version);
  out.string(&spec.dense_vector_name);
  out.string(&spec.dense.model_family);
  out.string(&spec.dense.artifact_revision);
  out.u32(spec.dense.dimensions);
  out.string(&spec.sparse_vector_name);
  out.string(&spec.sparse.encoder_identity);
  out.string(&spec.sparse.encoder_revision);
}
fn node_key_bytes(node: &GraphNodeKey) -> Vec<u8> {
  let mut out = CanonicalBytes::new();
  out.string(node_kind_name(node.kind));
  out.string(node.id.as_str());
  out.finish()
}
fn edge_identity_bytes(identity: &PublishedEdgeIdentity) -> Vec<u8> {
  let mut out = CanonicalBytes::new();
  out.string(identity.release_id.as_str());
  out.string(identity.relationship_id.as_str());
  out.u32(identity.relationship_revision.get());
  out.bytes(&node_key_bytes(&identity.source));
  out.bytes(&node_key_bytes(&identity.target));
  out.string(
    identity
      .relation_type
      .rule()
      .qdrant_wire_name
      .unwrap_or("unresolved"),
  );
  scope_bytes(&mut out, &identity.scope);
  out.finish()
}
fn payload_bytes(out: &mut CanonicalBytes, payload: &NodeProjectionPayload) {
  match payload {
    NodeProjectionPayload::Lexeme {
      lemma,
      normalized_lemma,
      language,
      part_of_speech,
    } => {
      out.string("lexeme");
      out.string(lemma);
      out.string(normalized_lemma);
      out.string(language);
      out.string(pos_name(*part_of_speech));
    }
    NodeProjectionPayload::Sense {
      lexeme_id,
      sense_key,
      definition,
      lemma,
      language,
      part_of_speech,
      definition_evidence_ids,
    } => {
      out.string("sense");
      out.string(lexeme_id.as_str());
      out.string(sense_key);
      out.string(definition);
      out.string(lemma);
      out.string(language);
      out.string(pos_name(*part_of_speech));
      out.usize(definition_evidence_ids.len());
      for id in definition_evidence_ids {
        out.string(id.as_str());
      }
    }
  }
}
fn scope_bytes(out: &mut CanonicalBytes, scope: &GraphScope) {
  out.optional(scope.dialect.as_ref().map(|value| value.as_str()));
  out.optional(scope.domain.as_deref());
  out.optional(scope.register.as_deref());
  out.optional(scope.note.as_deref());
}
fn node_kind_name(kind: GraphNodeKind) -> &'static str {
  match kind {
    GraphNodeKind::Sense => "sense",
    GraphNodeKind::Lexeme => "lexeme",
    GraphNodeKind::Construction => "construction",
    GraphNodeKind::Scale => "scale",
  }
}
fn pos_name(value: LexicalPartOfSpeech) -> &'static str {
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
fn confidence_name(value: EvidenceConfidence) -> &'static str {
  match value {
    EvidenceConfidence::High => "high",
    EvidenceConfidence::Medium => "medium",
    EvidenceConfidence::Low => "low",
  }
}
fn hex_digest(bytes: Vec<u8>) -> String {
  format!("{:x}", Sha256::digest(bytes))
}

struct CanonicalBytes(Vec<u8>);
impl CanonicalBytes {
  fn new() -> Self {
    Self(Vec::new())
  }
  fn string(&mut self, value: &str) {
    self.bytes(value.as_bytes());
  }
  fn bytes(&mut self, value: &[u8]) {
    self.u64(value.len() as u64);
    self.0.extend_from_slice(value);
  }
  fn optional(&mut self, value: Option<&str>) {
    match value {
      Some(value) => {
        self.0.push(1);
        self.string(value);
      }
      None => self.0.push(0),
    }
  }
  fn u32(&mut self, value: u32) {
    self.0.extend_from_slice(&value.to_be_bytes());
  }
  fn u64(&mut self, value: u64) {
    self.0.extend_from_slice(&value.to_be_bytes());
  }
  fn usize(&mut self, value: usize) {
    self.u64(value as u64);
  }
  fn finish(self) -> Vec<u8> {
    self.0
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeSet;

  use super::*;
  use crate::domain::canonical::{
    EvidenceFragment, EvidenceKind, LanguageTag, Lexeme, LexicalSource, Sense, SourcePermissions,
  };
  use crate::domain::canonical_content::{CanonicalEvidenceLineage, CanonicalEvidenceOrigin};
  use crate::domain::graph::{
    GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphRanking, GraphRelationType,
    GraphScore, GraphScoreComponents, RelationVersion, StoredGraphRelation,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn embedding(schema: &str) -> ProjectionEmbeddingSpec {
    ProjectionEmbeddingSpec::new(
      schema,
      DenseEmbeddingVersion {
        model_family: "Qwen/Qwen3-Embedding-0.6B".to_string(),
        artifact_revision: "sha256:test-artifact-r1".to_string(),
        dimensions: 1024,
      },
      SparseEmbeddingVersion {
        encoder_identity: "transnet-lexical-bm25".to_string(),
        encoder_revision: "v1".to_string(),
      },
    )
    .unwrap()
  }

  fn compatibility_entry() -> EmbeddingCompatibilityEntry {
    EmbeddingCompatibilityEntry {
      entry_id: "qwen3-embedding-test-r1".into(),
      dense_model_family: DENSE_MODEL_FAMILY.into(),
      dense_artifact_revision: "sha256:test-artifact-r1".into(),
      dense_dimensions: DENSE_DIMENSIONS,
      dense_vector_name: DENSE_VECTOR_NAME.into(),
      node_dense_input_specification: super::super::embedding_input::NODE_DENSE_INPUT_VERSION
        .into(),
      edge_dense_input_specification: super::super::embedding_input::EDGE_DENSE_INPUT_VERSION
        .into(),
      lexical_encoder_identity: LEXICAL_ENCODER_IDENTITY.into(),
      lexical_encoder_revision: LEXICAL_ENCODER_REVISION.into(),
      lexical_contract_identity: LEXICAL_CONTRACT_IDENTITY.into(),
      lexical_vector_name: SPARSE_VECTOR_NAME.into(),
      node_lexical_input_specification: super::super::embedding_input::NODE_LEXICAL_INPUT_VERSION
        .into(),
      edge_lexical_input_specification: super::super::embedding_input::EDGE_LEXICAL_INPUT_VERSION
        .into(),
    }
  }

  fn lexeme(name: &str, release: &str) -> Lexeme {
    Lexeme {
      id: id(&format!("lexeme-{name}")),
      release_id: id(release),
      language: LanguageTag::parse("en").unwrap(),
      lemma: name.to_string(),
      lemma_evidence_ids: vec![id(&format!("evidence-lemma-{name}"))],
      normalized_lemma: name.to_lowercase(),
      part_of_speech: LexicalPartOfSpeech::Noun,
      status: CanonicalStatus::Active,
    }
  }

  fn sense(name: &str, release: &str) -> Sense {
    Sense {
      id: id(&format!("sense-{name}")),
      lexeme_id: id(&format!("lexeme-{name}")),
      release_id: id(release),
      sense_key: "1".to_string(),
      definition: format!("Definition of {name}"),
      definition_evidence_ids: vec![id(&format!("evidence-{name}"))],
      status: CanonicalStatus::Active,
    }
  }

  fn node_inputs(release: &str) -> Vec<CanonicalNodeProjectionInput> {
    let alpha = lexeme("alpha", release);
    let beta = lexeme("beta", release);
    vec![
      CanonicalNodeProjectionInput::Lexeme(material(alpha.clone(), None)),
      CanonicalNodeProjectionInput::Sense(material(alpha, Some(sense("alpha", release)))),
      CanonicalNodeProjectionInput::Lexeme(material(beta.clone(), None)),
      CanonicalNodeProjectionInput::Sense(material(beta, Some(sense("beta", release)))),
    ]
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

  fn material(lexeme: Lexeme, sense: Option<Sense>) -> AuthoritativeEmbeddingMaterial {
    let release = lexeme.release_id.clone();
    let mut ids = lexeme.lemma_evidence_ids.clone();
    if let Some(item) = &sense {
      ids.extend(item.definition_evidence_ids.clone());
    }
    ids.sort();
    ids.dedup();
    let evidence = ids
      .into_iter()
      .map(|evidence_id| {
        let source_id = id(&format!("source-{}", evidence_id.as_str()));
        CanonicalEvidenceLineage::new(
          LexicalSource {
            id: source_id.clone(),
            name: "Reviewed source".into(),
            version: "2026-01".into(),
            license: "reviewed".into(),
            attribution: Some("Reviewed source".into()),
            permissions: permissions(),
          },
          EvidenceFragment {
            id: evidence_id.clone(),
            source_id,
            source_reference: "entry".into(),
            release_id: release.clone(),
            language: LanguageTag::parse("en").unwrap(),
            kind: EvidenceKind::Definition,
            confidence: EvidenceConfidence::High,
            text: "Reviewed evidence".into(),
            content_hash: format!("sha256:{}", evidence_id.as_str()),
            permissions: permissions(),
            status: CanonicalStatus::Active,
          },
          CanonicalEvidenceOrigin::LicensedSource,
        )
        .unwrap()
      })
      .collect();
    AuthoritativeEmbeddingMaterial::new(lexeme, sense, vec![], vec![], vec![], evidence).unwrap()
  }

  fn relationship(edge: &str, source: &str, target: &str) -> PublishedRelationship {
    let release = id("release-1");
    let source_id = id("source-1");
    let evidence_id = id(&format!("evidence-{edge}"));
    let permissions = permissions();
    let lineage = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".to_string(),
        version: "2026-01".to_string(),
        license: "reviewed".to_string(),
        attribution: Some("Reviewed source".to_string()),
        permissions,
      },
      EvidenceFragment {
        id: evidence_id.clone(),
        source_id,
        source_reference: "entry".to_string(),
        release_id: release.clone(),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Reviewed evidence".to_string(),
        content_hash: format!("sha256:{edge}"),
        permissions,
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap();
    let relation_type = GraphRelationType::Hypernym;
    let rule = relation_type.rule();
    PublishedRelationship {
      relation: StoredGraphRelation {
        edge_id: GraphEdgeId::stored(id(edge)),
        relation_version: RelationVersion::new(1).unwrap(),
        source: GraphNodeKey::new(GraphNodeKind::Sense, id(&format!("sense-{source}"))),
        target: GraphNodeKey::new(GraphNodeKind::Sense, id(&format!("sense-{target}"))),
        relation_type,
        evidence: GraphEvidence::new(vec![evidence_id], EvidenceConfidence::High).unwrap(),
        scope: GraphScope::default(),
        feedback_capabilities: BTreeSet::from([GraphFeedbackCapability::Accuracy]),
        ranking: GraphRanking {
          display_rank: GraphScore::new(9000).unwrap(),
          components: GraphScoreComponents {
            evidence: GraphScore::new(9000).unwrap(),
            community: None,
            pedagogical: None,
          },
          ranking_version: "graph-rank-v1".to_string(),
        },
      },
      relation_release_id: release.clone(),
      source_release_id: release.clone(),
      target_release_id: release,
      declared_wire_relation: rule.require_qdrant_wire_name().unwrap().to_string(),
      declared_inverse: rule.inverse,
      evidence_lineage: vec![lineage],
      verification_state: RelationshipVerificationState::Verified,
    }
  }

  #[test]
  fn node_build_is_stable_under_input_reordering() {
    let mut reversed = node_inputs("release-1");
    let original = node_inputs("release-1");
    reversed.reverse();

    let left =
      build_node_projection(id("release-1"), embedding("knowledge-graph-v1"), original).unwrap();
    let right =
      build_node_projection(id("release-1"), embedding("knowledge-graph-v1"), reversed).unwrap();

    assert_eq!(left, right);
    assert_eq!(left.point_count, 4);
  }

  #[test]
  fn exact_duplicate_nodes_deduplicate_but_conflicts_fail_closed() {
    let item = CanonicalNodeProjectionInput::Lexeme(material(lexeme("alpha", "release-1"), None));
    let build = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      vec![item.clone(), item],
    )
    .unwrap();
    assert_eq!(build.point_count, 1);

    let mut conflict = lexeme("alpha", "release-1");
    conflict.lemma = "ALPHA".to_string();
    assert_eq!(
      build_node_projection(
        id("release-1"),
        embedding("knowledge-graph-v1"),
        vec![
          CanonicalNodeProjectionInput::Lexeme(material(lexeme("alpha", "release-1"), None)),
          CanonicalNodeProjectionInput::Lexeme(material(conflict, None))
        ],
      ),
      Err(ProjectionValidationError::ConflictingDuplicateNode)
    );
  }

  #[test]
  fn release_and_schema_change_point_identity_and_hash() {
    let r1 = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let r2 = build_node_projection(
      id("release-2"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-2"),
    )
    .unwrap();
    let schema = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v2"),
      node_inputs("release-1"),
    )
    .unwrap();
    assert_ne!(r1.points[0].point_id, r2.points[0].point_id);
    assert_ne!(r1.content_hash, r2.content_hash);
    assert_ne!(r1.points[0].point_id, schema.points[0].point_id);
    assert_ne!(r1.content_hash, schema.content_hash);
  }

  #[test]
  fn unresolved_and_ineligible_node_families_fail_closed() {
    assert_eq!(
      build_node_projection(
        id("release-1"),
        embedding("knowledge-graph-v1"),
        vec![CanonicalNodeProjectionInput::Unresolved(GraphNodeKey::new(
          GraphNodeKind::Construction,
          id("construction-1")
        ))],
      ),
      Err(ProjectionValidationError::UnresolvedNodeFamily)
    );
    let mut draft = lexeme("alpha", "release-1");
    draft.status = CanonicalStatus::Draft;
    assert_eq!(
      AuthoritativeEmbeddingMaterial::new(draft, None, vec![], vec![], vec![], vec![]),
      Err(super::super::embedding_input::EmbeddingMaterialError::IneligibleRecord)
    );
  }

  #[test]
  fn edge_build_is_order_independent_and_binds_node_hash() {
    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let first = relationship("edge-1", "alpha", "beta");
    let second = relationship("edge-2", "beta", "alpha");
    let left = build_edge_projection(
      &nodes,
      embedding("knowledge-graph-v1"),
      vec![first.clone(), second.clone()],
    )
    .unwrap();
    let right =
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![second, first]).unwrap();
    assert_eq!(left, right);
    assert_eq!(left.verified_node_content_hash, nodes.content_hash);
    assert_eq!(left.expected_endpoint_count, 4);
    assert_eq!(left.resolved_endpoint_count, 4);
    assert_eq!(left.points.len(), 2);
  }

  #[test]
  fn missing_endpoints_and_embedding_mismatch_fail_closed() {
    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    assert_eq!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v2"), vec![]),
      Err(ProjectionValidationError::NodeBuildMismatch)
    );
    let missing = relationship("edge-1", "alpha", "missing");
    assert_eq!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![missing]),
      Err(ProjectionValidationError::MissingTargetEndpoint)
    );
    let missing = relationship("edge-2", "missing", "beta");
    assert_eq!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![missing]),
      Err(ProjectionValidationError::MissingSourceEndpoint)
    );
  }

  #[test]
  fn unadmitted_and_duplicate_relationships_are_rejected() {
    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let mut unverified = relationship("edge-1", "alpha", "beta");
    unverified.verification_state = RelationshipVerificationState::Exploratory;
    assert!(matches!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![unverified]),
      Err(ProjectionValidationError::RelationshipAdmission(
        GraphValidationError::UnverifiedRelationship
      ))
    ));

    let left = relationship("edge-1", "alpha", "beta");
    let right = relationship("edge-other", "alpha", "beta");
    assert!(matches!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![left, right]),
      Err(ProjectionValidationError::RelationshipAdmission(
        GraphValidationError::DuplicateTypedRelationship
      ))
    ));
  }

  #[test]
  fn unresolved_wire_mapping_never_enters_projection() {
    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let mut unresolved = relationship("edge-1", "alpha", "beta");
    unresolved.relation.relation_type = GraphRelationType::Synonym;
    unresolved.declared_inverse = GraphRelationType::Synonym;
    unresolved.declared_wire_relation = "synonym".to_string();
    assert!(matches!(
      build_edge_projection(&nodes, embedding("knowledge-graph-v1"), vec![unresolved]),
      Err(ProjectionValidationError::RelationshipAdmission(
        GraphValidationError::UnresolvedQdrantRelation
      ))
    ));
  }

  #[test]
  fn invalid_embedding_metadata_fails_closed_without_guessing() {
    assert_eq!(
      ProjectionEmbeddingSpec::new(
        "knowledge-graph-v1",
        DenseEmbeddingVersion {
          model_family: "Qwen/Qwen3-Embedding-0.6B".to_string(),
          artifact_revision: "sha256:test-artifact-r1".to_string(),
          dimensions: 0
        },
        SparseEmbeddingVersion {
          encoder_identity: "transnet-lexical-bm25".to_string(),
          encoder_revision: "v1".to_string()
        },
      ),
      Err(ProjectionValidationError::InvalidEmbeddingMetadata)
    );
  }

  #[test]
  fn compatibility_registry_requires_exact_revision_dimensions_and_input_specs() {
    let registry = EmbeddingCompatibilityRegistry::new(vec![compatibility_entry()]).unwrap();
    assert_eq!(registry.validate(&embedding("knowledge-graph-v1")), Ok(()));
    let incompatible = ProjectionEmbeddingSpec::new(
      "knowledge-graph-v1",
      DenseEmbeddingVersion {
        model_family: "Qwen/Qwen3-Embedding-0.6B".into(),
        artifact_revision: "sha256:test-artifact-r1".into(),
        dimensions: 768,
      },
      SparseEmbeddingVersion {
        encoder_identity: "transnet-lexical-bm25".into(),
        encoder_revision: "v1".into(),
      },
    )
    .unwrap();
    assert_eq!(
      registry.validate(&incompatible),
      Err(ProjectionValidationError::EmbeddingCompatibilityMissing)
    );

    assert!(EmbeddingCompatibilityRegistry::new(Vec::new()).is_ok());
    let mut floating = compatibility_entry();
    floating.dense_artifact_revision = "latest".into();
    assert_eq!(
      EmbeddingCompatibilityRegistry::new(vec![floating]),
      Err(ProjectionValidationError::InvalidEmbeddingMetadata)
    );
  }

  #[test]
  fn persisted_hashes_exclude_raw_vectors_and_separate_node_and_edge_domains() {
    use super::super::knowledge_publication::{LexicalDictionaryManifest, PersistedCollectionHash};

    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let edges = build_edge_projection(
      &nodes,
      embedding("knowledge-graph-v1"),
      vec![relationship("edge-1", "alpha", "beta")],
    )
    .unwrap();
    let dictionary = LexicalDictionaryManifest::new(vec![("alpha".into(), 1)]).unwrap();
    let entry = compatibility_entry();
    let first = PersistedCollectionHash::for_nodes(&nodes, &entry, &dictionary).unwrap();
    let hypothetical_raw_vectors = [0.125_f32, -0.75_f32, 1.0_f32];
    let second = PersistedCollectionHash::for_nodes(&nodes, &entry, &dictionary).unwrap();
    assert_eq!(first, second);
    assert_eq!(hypothetical_raw_vectors.len(), 3);
    assert_ne!(
      first,
      PersistedCollectionHash::for_edges(&edges, &entry, &dictionary).unwrap()
    );
  }

  #[test]
  fn edge_inputs_are_structured_and_bound_to_frozen_endpoint_inputs() {
    let nodes = build_node_projection(
      id("release-1"),
      embedding("knowledge-graph-v1"),
      node_inputs("release-1"),
    )
    .unwrap();
    let edges = build_edge_projection(
      &nodes,
      embedding("knowledge-graph-v1"),
      vec![relationship("edge-1", "alpha", "beta")],
    )
    .unwrap();
    let edge = &edges.points[0];
    assert_eq!(
      edge.dense_input.specification_version(),
      EDGE_DENSE_INPUT_VERSION
    );
    assert_eq!(
      edge.lexical_input.specification_version(),
      EDGE_LEXICAL_INPUT_VERSION
    );
    assert_ne!(
      edge.dense_input.input_hash(),
      edge.lexical_input.input_hash()
    );
    assert!(!format!("{:?}", edge.dense_input).contains("Reviewed evidence"));
  }

  #[test]
  fn projection_errors_are_redacted() {
    let error = ProjectionValidationError::MissingSourceEndpoint;
    let debug = format!("{error:?}");
    let display = error.to_string();
    assert!(!debug.contains("Reviewed evidence"));
    assert!(!display.contains("Reviewed evidence"));
    assert!(!display.contains("release-1"));
  }
}
