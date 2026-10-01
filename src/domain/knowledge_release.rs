//! Immutable canonical and Qdrant release-trio contract invariants.

use thiserror::Error;

use crate::domain::canonical::{CanonicalId, CanonicalReleasePin, ReleaseId};

const MAX_VERSION_LENGTH: usize = 128;
const MAX_HASH_LENGTH: usize = 256;

/// Opaque immutable identifier of one Qdrant knowledge-node collection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeCollectionId(CanonicalId);

impl NodeCollectionId {
  /// Creates a typed node-collection identifier.
  ///
  /// # Errors
  ///
  /// Returns an error when the identifier violates the canonical opaque-ID policy.
  pub fn parse(value: impl AsRef<str>) -> Result<Self, KnowledgeReleaseValidationError> {
    CanonicalId::new(value)
      .map(Self)
      .map_err(|_| KnowledgeReleaseValidationError::InvalidCollectionId)
  }

  /// Returns the opaque collection identifier.
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

/// Opaque immutable identifier of one Qdrant knowledge-edge collection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeCollectionId(CanonicalId);

impl EdgeCollectionId {
  /// Creates a typed edge-collection identifier.
  ///
  /// # Errors
  ///
  /// Returns an error when the identifier violates the canonical opaque-ID policy.
  pub fn parse(value: impl AsRef<str>) -> Result<Self, KnowledgeReleaseValidationError> {
    CanonicalId::new(value)
      .map(Self)
      .map_err(|_| KnowledgeReleaseValidationError::InvalidCollectionId)
  }

  /// Returns the opaque collection identifier.
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

/// Lifecycle state of one immutable projection collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionCollectionState {
  /// The collection is still being built or checked and cannot be activated.
  Building,
  /// The collection passed all deterministic validation for its manifest.
  Verified,
}

/// Versioned dense-vector configuration shared by one node/edge release pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenseEmbeddingVersion {
  /// Stable dense model family selected by the execution contract.
  pub model_family: String,
  /// Exact immutable artifact revision; floating aliases are invalid.
  pub artifact_revision: String,
  /// Exact dense-vector dimensions produced by the model revision.
  pub dimensions: u32,
}

/// Versioned sparse-vector configuration shared by one node/edge release pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseEmbeddingVersion {
  /// Stable deterministic lexical encoder identity.
  pub encoder_identity: String,
  /// Exact immutable encoder contract revision.
  pub encoder_revision: String,
}

/// Deterministic manifest for one immutable knowledge-node collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCollectionManifest {
  /// Canonical release from which every node was projected.
  pub release_id: ReleaseId,
  /// Immutable physical collection identifier, never an active alias.
  pub collection_id: NodeCollectionId,
  /// Closed node payload-schema version.
  pub payload_schema_version: String,
  /// Deterministic hash of the complete ordered node projection.
  pub content_hash: String,
  /// Number of projected node points covered by `content_hash`.
  pub point_count: u64,
  /// Current immutable-build verification state.
  pub state: ProjectionCollectionState,
}

/// Deterministic manifest for one immutable knowledge-edge collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeCollectionManifest {
  /// Canonical release from which every edge was projected.
  pub release_id: ReleaseId,
  /// Immutable physical collection identifier, never an active alias.
  pub collection_id: EdgeCollectionId,
  /// Closed edge payload-schema version.
  pub payload_schema_version: String,
  /// Deterministic hash of the complete ordered edge projection.
  pub content_hash: String,
  /// Number of projected edge points covered by `content_hash`.
  pub point_count: u64,
  /// Hash of the verified node collection against which endpoints were checked.
  pub verified_node_content_hash: String,
  /// Number of edge endpoints expected during reconciliation.
  pub expected_endpoint_count: u64,
  /// Number of endpoints resolved in the pinned node collection.
  pub resolved_endpoint_count: u64,
  /// Current immutable-build verification state.
  pub state: ProjectionCollectionState,
}

/// Complete immutable MySQL/Qdrant release trio eligible for later activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeReleaseTrio {
  canonical: CanonicalReleasePin,
  nodes: NodeCollectionManifest,
  edges: EdgeCollectionManifest,
  dense: DenseEmbeddingVersion,
  sparse: SparseEmbeddingVersion,
}

impl KnowledgeReleaseTrio {
  /// Validates a complete canonical, node-collection, and edge-collection release trio.
  ///
  /// This constructor accepts optional projection members so an incomplete wire or repository
  /// result is rejected structurally rather than promoted with a placeholder collection.
  ///
  /// # Errors
  ///
  /// Returns an error when a member is absent, belongs to another release, has incompatible
  /// schema or embedding metadata, is unverified, or failed endpoint reconciliation.
  pub fn try_from_parts(
    canonical: CanonicalReleasePin,
    nodes: Option<NodeCollectionManifest>,
    edges: Option<EdgeCollectionManifest>,
    dense: DenseEmbeddingVersion,
    sparse: SparseEmbeddingVersion,
  ) -> Result<Self, KnowledgeReleaseValidationError> {
    let nodes = nodes.ok_or(KnowledgeReleaseValidationError::IncompleteTrio)?;
    let edges = edges.ok_or(KnowledgeReleaseValidationError::IncompleteTrio)?;
    validate_version(&canonical.canonical_schema_version)?;
    validate_version(&nodes.payload_schema_version)?;
    validate_version(&edges.payload_schema_version)?;
    validate_hash(&nodes.content_hash)?;
    validate_hash(&edges.content_hash)?;
    validate_hash(&edges.verified_node_content_hash)?;
    validate_version(&dense.model_family)?;
    validate_version(&dense.artifact_revision)?;
    validate_version(&sparse.encoder_identity)?;
    validate_version(&sparse.encoder_revision)?;

    if dense.dimensions == 0 {
      return Err(KnowledgeReleaseValidationError::InvalidDenseDimensions);
    }
    if nodes.point_count == 0 || edges.point_count == 0 {
      return Err(KnowledgeReleaseValidationError::InvalidPointCount);
    }
    if nodes.release_id != canonical.release_id || edges.release_id != canonical.release_id {
      return Err(KnowledgeReleaseValidationError::ReleaseMismatch);
    }
    if nodes.payload_schema_version != edges.payload_schema_version {
      return Err(KnowledgeReleaseValidationError::ProjectionSchemaMismatch);
    }
    if nodes.state != ProjectionCollectionState::Verified
      || edges.state != ProjectionCollectionState::Verified
    {
      return Err(KnowledgeReleaseValidationError::UnverifiedCollection);
    }
    if edges.verified_node_content_hash != nodes.content_hash {
      return Err(KnowledgeReleaseValidationError::NodeBuildMismatch);
    }
    let projected_endpoint_count = edges
      .point_count
      .checked_mul(2)
      .ok_or(KnowledgeReleaseValidationError::InvalidPointCount)?;
    if edges.expected_endpoint_count != edges.resolved_endpoint_count {
      return Err(KnowledgeReleaseValidationError::EndpointCoverageMismatch);
    }
    if edges.expected_endpoint_count != projected_endpoint_count {
      return Err(KnowledgeReleaseValidationError::EndpointCoverageMismatch);
    }

    Ok(Self {
      canonical,
      nodes,
      edges,
      dense,
      sparse,
    })
  }

  /// Returns the authoritative canonical release and schema pin.
  pub fn canonical(&self) -> &CanonicalReleasePin {
    &self.canonical
  }

  /// Returns the verified immutable node-collection manifest.
  pub fn nodes(&self) -> &NodeCollectionManifest {
    &self.nodes
  }

  /// Returns the verified immutable edge-collection manifest.
  pub fn edges(&self) -> &EdgeCollectionManifest {
    &self.edges
  }

  /// Returns the dense embedding revision shared by both collections.
  pub fn dense_embedding(&self) -> &DenseEmbeddingVersion {
    &self.dense
  }

  /// Returns the sparse embedding revision shared by both collections.
  pub fn sparse_embedding(&self) -> &SparseEmbeddingVersion {
    &self.sparse
  }
}

/// Closed M3 publication, reconciliation, and activation failure category.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum KnowledgeReleaseFailure {
  /// The selected immutable canonical release cannot be read.
  #[error("canonical release unavailable")]
  CanonicalReleaseUnavailable,
  /// The immutable node collection could not be built or verified.
  #[error("node collection build unavailable")]
  NodeBuildUnavailable,
  /// The immutable edge collection could not be built or verified.
  #[error("edge collection build unavailable")]
  EdgeBuildUnavailable,
  /// Canonical or projection schemas are not compatible with the publisher.
  #[error("release schema incompatible")]
  SchemaIncompatible,
  /// Dense or sparse embedding metadata is incompatible with the release manifest.
  #[error("embedding metadata incompatible")]
  EmbeddingMetadataIncompatible,
  /// A publication operation attempted an illegal lifecycle transition.
  #[error("publication lifecycle transition is invalid")]
  InvalidLifecycleTransition,
  /// An idempotency identity was reused with different canonical content.
  #[error("publication idempotency identity conflicts")]
  IdempotencyConflict,
  /// The executed dense artifact differed from the approved immutable revision.
  #[error("observed dense artifact revision mismatched the registry")]
  ArtifactRevisionMismatch,
  /// The executed lexical encoder differed from the approved revision.
  #[error("observed lexical encoder revision mismatched the registry")]
  LexicalEncoderMismatch,
  /// The release-local lexical dictionary differed from its frozen manifest.
  #[error("lexical dictionary mismatched the publication manifest")]
  DictionaryMismatch,
  /// One or more edge endpoints were absent from the pinned node collection.
  #[error("endpoint reconciliation failed")]
  EndpointReconciliationFailed,
  /// Projected counts or content hashes did not match the release manifest.
  #[error("hash or count reconciliation failed")]
  HashOrCountReconciliationFailed,
  /// At least one mandatory release-trio member is absent or unverified.
  #[error("release trio incomplete")]
  IncompleteTrio,
  /// The active release changed from the caller's expected immutable predecessor.
  #[error("release activation conflict")]
  ActivationConflict,
  /// A selected immutable release is no longer retained and addressable.
  #[error("immutable release unavailable")]
  ImmutableReleaseUnavailable,
  /// The bounded operation exceeded its deadline.
  #[error("release operation timed out")]
  Timeout,
  /// A required external publication dependency is unavailable.
  #[error("release dependency unavailable")]
  DependencyUnavailable,
}

impl KnowledgeReleaseFailure {
  /// Parses one closed internal publication failure code without inspecting human text.
  ///
  /// # Errors
  ///
  /// Returns an error for an unknown code so incompatible island-port responses fail closed.
  pub fn parse_code(value: &str) -> Result<Self, KnowledgeReleaseValidationError> {
    match value {
      "canonical_release_unavailable" => Ok(Self::CanonicalReleaseUnavailable),
      "node_build_unavailable" => Ok(Self::NodeBuildUnavailable),
      "edge_build_unavailable" => Ok(Self::EdgeBuildUnavailable),
      "schema_incompatible" => Ok(Self::SchemaIncompatible),
      "embedding_metadata_incompatible" => Ok(Self::EmbeddingMetadataIncompatible),
      "invalid_lifecycle_transition" => Ok(Self::InvalidLifecycleTransition),
      "idempotency_conflict" => Ok(Self::IdempotencyConflict),
      "artifact_revision_mismatch" => Ok(Self::ArtifactRevisionMismatch),
      "lexical_encoder_mismatch" => Ok(Self::LexicalEncoderMismatch),
      "dictionary_mismatch" => Ok(Self::DictionaryMismatch),
      "endpoint_reconciliation_failed" => Ok(Self::EndpointReconciliationFailed),
      "hash_or_count_reconciliation_failed" => Ok(Self::HashOrCountReconciliationFailed),
      "incomplete_trio" => Ok(Self::IncompleteTrio),
      "activation_conflict" => Ok(Self::ActivationConflict),
      "immutable_release_unavailable" => Ok(Self::ImmutableReleaseUnavailable),
      "timeout" => Ok(Self::Timeout),
      "dependency_unavailable" => Ok(Self::DependencyUnavailable),
      _ => Err(KnowledgeReleaseValidationError::UnknownFailureCode),
    }
  }
}

/// Validation failure for a candidate M3 release trio.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum KnowledgeReleaseValidationError {
  /// A physical collection identifier violated the opaque-ID policy.
  #[error("invalid immutable collection identifier")]
  InvalidCollectionId,
  /// A required canonical, node, or edge trio member was absent.
  #[error("release trio is incomplete")]
  IncompleteTrio,
  /// A release or collection schema/model version was blank or oversized.
  #[error("release manifest version is invalid")]
  InvalidVersion,
  /// A deterministic manifest hash was blank or oversized.
  #[error("release manifest hash is invalid")]
  InvalidHash,
  /// Dense embedding dimensions were zero.
  #[error("dense embedding dimensions must be positive")]
  InvalidDenseDimensions,
  /// A node or edge collection contained no published projection points.
  #[error("projection collection point counts must be positive")]
  InvalidPointCount,
  /// A projection collection belonged to a different canonical release.
  #[error("release trio members do not share one release")]
  ReleaseMismatch,
  /// Node and edge collections used different payload-schema versions.
  #[error("node and edge payload schemas are incompatible")]
  ProjectionSchemaMismatch,
  /// At least one collection did not reach the verified immutable state.
  #[error("release trio contains an unverified collection")]
  UnverifiedCollection,
  /// The edge build was validated against a different node projection hash.
  #[error("edge collection was not built against the pinned node collection")]
  NodeBuildMismatch,
  /// Not every edge endpoint resolved within the pinned node collection.
  #[error("edge endpoint coverage is incomplete")]
  EndpointCoverageMismatch,
  /// An internal publication response contained an unknown structured failure code.
  #[error("unknown release publication failure code")]
  UnknownFailureCode,
}

fn validate_version(value: &str) -> Result<(), KnowledgeReleaseValidationError> {
  if value.trim().is_empty() || value.len() > MAX_VERSION_LENGTH {
    Err(KnowledgeReleaseValidationError::InvalidVersion)
  } else {
    Ok(())
  }
}

fn validate_hash(value: &str) -> Result<(), KnowledgeReleaseValidationError> {
  if value.trim().is_empty() || value.len() > MAX_HASH_LENGTH {
    Err(KnowledgeReleaseValidationError::InvalidHash)
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn pin(release: &str) -> CanonicalReleasePin {
    CanonicalReleasePin::new(id(release), "canonical-v1".to_string()).unwrap()
  }

  fn nodes(release: &str) -> NodeCollectionManifest {
    NodeCollectionManifest {
      release_id: id(release),
      collection_id: NodeCollectionId::parse("knowledge-nodes-r1").unwrap(),
      payload_schema_version: "knowledge-graph-v1".to_string(),
      content_hash: "sha256:nodes".to_string(),
      point_count: 4,
      state: ProjectionCollectionState::Verified,
    }
  }

  fn edges(release: &str) -> EdgeCollectionManifest {
    EdgeCollectionManifest {
      release_id: id(release),
      collection_id: EdgeCollectionId::parse("knowledge-edges-r1").unwrap(),
      payload_schema_version: "knowledge-graph-v1".to_string(),
      content_hash: "sha256:edges".to_string(),
      point_count: 3,
      verified_node_content_hash: "sha256:nodes".to_string(),
      expected_endpoint_count: 6,
      resolved_endpoint_count: 6,
      state: ProjectionCollectionState::Verified,
    }
  }

  fn dense() -> DenseEmbeddingVersion {
    DenseEmbeddingVersion {
      model_family: "Qwen/Qwen3-Embedding-0.6B".to_string(),
      artifact_revision: "sha256:deployment-artifact-r1".to_string(),
      dimensions: 1024,
    }
  }

  fn sparse() -> SparseEmbeddingVersion {
    SparseEmbeddingVersion {
      encoder_identity: "transnet-lexical-bm25".to_string(),
      encoder_revision: "v1".to_string(),
    }
  }

  #[test]
  fn valid_release_trio_preserves_typed_members() {
    let trio = KnowledgeReleaseTrio::try_from_parts(
      pin("release-r1"),
      Some(nodes("release-r1")),
      Some(edges("release-r1")),
      dense(),
      sparse(),
    )
    .unwrap();

    assert_eq!(trio.nodes().collection_id.as_str(), "knowledge-nodes-r1");
    assert_eq!(trio.edges().collection_id.as_str(), "knowledge-edges-r1");
    assert_eq!(trio.canonical().release_id, id("release-r1"));
  }

  #[test]
  fn node_and_edge_collection_ids_are_distinct_types() {
    let node = NodeCollectionId::parse("same-opaque-value").unwrap();
    let edge = EdgeCollectionId::parse("same-opaque-value").unwrap();

    assert_eq!(node.as_str(), edge.as_str());
    // Their concrete Rust types intentionally cannot be compared or interchanged.
    assert_ne!(
      std::any::type_name_of_val(&node),
      std::any::type_name_of_val(&edge)
    );
  }

  #[test]
  fn release_mismatch_is_rejected() {
    let error = KnowledgeReleaseTrio::try_from_parts(
      pin("release-r1"),
      Some(nodes("release-r2")),
      Some(edges("release-r1")),
      dense(),
      sparse(),
    )
    .unwrap_err();

    assert_eq!(error, KnowledgeReleaseValidationError::ReleaseMismatch);
  }

  #[test]
  fn schema_mismatch_is_rejected() {
    let mut edge_manifest = edges("release-r1");
    edge_manifest.payload_schema_version = "knowledge-graph-v2".to_string();
    let error = KnowledgeReleaseTrio::try_from_parts(
      pin("release-r1"),
      Some(nodes("release-r1")),
      Some(edge_manifest),
      dense(),
      sparse(),
    )
    .unwrap_err();

    assert_eq!(
      error,
      KnowledgeReleaseValidationError::ProjectionSchemaMismatch
    );
  }

  #[test]
  fn incomplete_trio_is_rejected_without_placeholder() {
    let error = KnowledgeReleaseTrio::try_from_parts(
      pin("release-r1"),
      Some(nodes("release-r1")),
      None,
      dense(),
      sparse(),
    )
    .unwrap_err();

    assert_eq!(error, KnowledgeReleaseValidationError::IncompleteTrio);
  }

  #[test]
  fn edge_build_must_bind_and_cover_verified_nodes() {
    let mut wrong_hash = edges("release-r1");
    wrong_hash.verified_node_content_hash = "sha256:other".to_string();
    assert_eq!(
      KnowledgeReleaseTrio::try_from_parts(
        pin("release-r1"),
        Some(nodes("release-r1")),
        Some(wrong_hash),
        dense(),
        sparse(),
      )
      .unwrap_err(),
      KnowledgeReleaseValidationError::NodeBuildMismatch
    );

    let mut incomplete = edges("release-r1");
    incomplete.resolved_endpoint_count = 5;
    assert_eq!(
      KnowledgeReleaseTrio::try_from_parts(
        pin("release-r1"),
        Some(nodes("release-r1")),
        Some(incomplete),
        dense(),
        sparse(),
      )
      .unwrap_err(),
      KnowledgeReleaseValidationError::EndpointCoverageMismatch
    );
  }

  #[test]
  fn publication_failure_codes_are_closed() {
    assert_eq!(
      KnowledgeReleaseFailure::parse_code("schema_incompatible"),
      Ok(KnowledgeReleaseFailure::SchemaIncompatible)
    );
    assert_eq!(
      KnowledgeReleaseFailure::parse_code("new_unrecognized_failure"),
      Err(KnowledgeReleaseValidationError::UnknownFailureCode)
    );
  }
}
