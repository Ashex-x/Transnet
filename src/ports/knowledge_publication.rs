//! Transport-independent outbound publication capability for immutable knowledge releases.

use std::time::Duration;

use async_trait::async_trait;

use crate::domain::{
  canonical::CanonicalReleasePin,
  knowledge_projection::{EdgeProjection, EmbeddingCompatibilityEntry, NodeProjection},
  knowledge_publication::{
    BuildExecutionReceipt, LexicalDictionaryManifest, PersistedCollectionHash,
    PublicationBatchIdentity, PublicationBuildId, PublicationBuildState,
    PublicationFinalizeIdentity, PublicationIdempotencyKey, PublicationManifestHash,
    PublicationReconcileIdentity, PublicationRequestFingerprint,
  },
  knowledge_release::{
    EdgeCollectionManifest, KnowledgeReleaseFailure, KnowledgeReleaseTrio, NodeCollectionManifest,
  },
};

/// Maximum number of points admitted to one publication batch.
pub const MAX_PUBLICATION_BATCH_POINTS: usize = 256;
/// Maximum serialized bytes admitted for one publication batch request.
pub const MAX_PUBLICATION_BATCH_REQUEST_BYTES: usize = 1_048_576;
/// Maximum request-correlation identifier bytes accepted by publication transport.
pub const MAX_PUBLICATION_REQUEST_ID_BYTES: usize = 128;
/// Maximum RFC 3339 deadline bytes accepted by publication transport.
pub const MAX_PUBLICATION_DEADLINE_BYTES: usize = 64;

/// Pure wire-boundary inspection result for one prospective publication batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationBatchWireAdmission {
  /// The batch fits the frozen worst-case transport context budget.
  Fits {
    /// Exact serialized request bytes under the deterministic planning envelope.
    serialized_bytes: usize,
  },
  /// The batch exceeds the frozen worst-case transport context budget.
  TooLarge {
    /// Exact serialized request bytes under the deterministic planning envelope.
    serialized_bytes: usize,
  },
}

impl PublicationBatchWireAdmission {
  /// Returns the exact deterministic planning-envelope byte count.
  pub const fn serialized_bytes(self) -> usize {
    match self {
      Self::Fits { serialized_bytes } | Self::TooLarge { serialized_bytes } => serialized_bytes,
    }
  }

  /// Returns whether the prospective request fits the frozen body limit.
  pub const fn fits(self) -> bool {
    matches!(self, Self::Fits { .. })
  }
}

/// Request-scoped correlation and deadline propagated to every publication operation.
#[derive(Clone)]
pub struct KnowledgePublicationContext {
  /// Opaque request correlation identifier containing no canonical content.
  pub request_id: String,
  /// Absolute RFC 3339 deadline shared by the operation.
  pub deadline_at: String,
  /// Local per-call maximum, further bounded by the absolute deadline.
  pub timeout: Duration,
}

/// Immutable build intent accepted by the publication authority.
pub struct BeginPublication {
  /// Stable identity derived from immutable build inputs.
  pub build_id: PublicationBuildId,
  /// Canonical release and schema pin.
  pub canonical: CanonicalReleasePin,
  /// Projection payload schema shared by node and edge artifacts.
  pub projection_schema_version: String,
  /// Complete deterministic node projection hash.
  pub node_projection_hash: String,
  /// Complete deterministic edge projection hash.
  pub edge_projection_hash: String,
  /// Exact compatibility registry entry required for execution.
  pub compatibility: EmbeddingCompatibilityEntry,
  /// Expected unique node point count.
  pub expected_node_count: u64,
  /// Expected unique edge point count.
  pub expected_edge_count: u64,
  /// Expected edge endpoint references.
  pub expected_endpoint_count: u64,
  /// Caller idempotency key.
  pub idempotency_key: PublicationIdempotencyKey,
  /// Canonical begin-request fingerprint.
  pub request_fingerprint: PublicationRequestFingerprint,
}

/// One bounded node batch with its canonical retry identity.
pub struct NodePublicationBatch {
  /// Batch identity binding build, ordinal, fingerprint, and content hash.
  pub identity: PublicationBatchIdentity,
  /// Deterministically ordered node projection points.
  pub points: Vec<NodeProjection>,
}

/// One bounded edge batch with its canonical retry identity.
pub struct EdgePublicationBatch {
  /// Batch identity binding build, ordinal, fingerprint, and content hash.
  pub identity: PublicationBatchIdentity,
  /// Deterministically ordered edge projection points.
  pub points: Vec<EdgeProjection>,
}

/// Freeze request for one fully submitted node collection.
pub struct FreezeNodes {
  /// Stable build being frozen.
  pub build_id: PublicationBuildId,
  /// Stable finalize identity.
  pub finalize_id: PublicationFinalizeIdentity,
  /// Expected persisted collection proof computed from the complete artifact.
  pub persisted_hash: PersistedCollectionHash,
  /// Complete node projection hash.
  pub projection_hash: String,
  /// Projection payload schema of every submitted node point.
  pub projection_schema_version: String,
  /// Expected processed node count.
  pub point_count: u64,
  /// Frozen release-local lexical dictionary.
  pub dictionary: LexicalDictionaryManifest,
  /// Exact execution compatibility entry.
  pub compatibility: EmbeddingCompatibilityEntry,
}

/// Freeze request for one fully submitted edge collection.
pub struct FreezeEdges {
  /// Stable build being frozen.
  pub build_id: PublicationBuildId,
  /// Stable finalize identity.
  pub finalize_id: PublicationFinalizeIdentity,
  /// Expected persisted collection proof computed from the complete artifact.
  pub persisted_hash: PersistedCollectionHash,
  /// Complete edge projection hash.
  pub projection_hash: String,
  /// Projection payload schema of every submitted edge point.
  pub projection_schema_version: String,
  /// Exact node projection hash against which endpoints were resolved.
  pub verified_node_projection_hash: String,
  /// Expected processed edge count.
  pub point_count: u64,
  /// Expected endpoint references.
  pub expected_endpoint_count: u64,
  /// Resolved endpoint references; must equal the expected value.
  pub resolved_endpoint_count: u64,
  /// Frozen release-local lexical dictionary.
  pub dictionary: LexicalDictionaryManifest,
  /// Exact execution compatibility entry.
  pub compatibility: EmbeddingCompatibilityEntry,
}

/// Verified immutable node artifact returned after node freeze.
#[derive(Debug, Clone)]
pub struct FrozenNodePublication {
  /// Verified physical collection manifest.
  pub manifest: NodeCollectionManifest,
  /// Persisted proof independent of raw vector bytes.
  pub persisted_hash: PersistedCollectionHash,
  /// Observed execution receipt validated against the requested compatibility entry.
  pub receipt: BuildExecutionReceipt,
  /// Frozen dictionary against which the lexical receipt was validated.
  pub dictionary: LexicalDictionaryManifest,
}

/// Verified immutable edge artifact returned after edge freeze.
#[derive(Debug, Clone)]
pub struct FrozenEdgePublication {
  /// Verified physical collection manifest.
  pub manifest: EdgeCollectionManifest,
  /// Persisted proof independent of raw vector bytes.
  pub persisted_hash: PersistedCollectionHash,
  /// Observed execution receipt validated against the requested compatibility entry.
  pub receipt: BuildExecutionReceipt,
  /// Frozen dictionary against which the lexical receipt was validated.
  pub dictionary: LexicalDictionaryManifest,
}

/// Reconciliation request over two already frozen immutable artifacts.
pub struct ReconcilePublication {
  /// Stable build being reconciled.
  pub build_id: PublicationBuildId,
  /// Stable reconcile identity over the ordered persisted proofs.
  pub reconcile_id: PublicationReconcileIdentity,
  /// Canonical release and schema pin.
  pub canonical: CanonicalReleasePin,
  /// Storage-neutral hash of the complete canonical release content.
  pub canonical_content_hash: String,
  /// Frozen node artifact.
  pub nodes: FrozenNodePublication,
  /// Frozen edge artifact.
  pub edges: FrozenEdgePublication,
  /// Expected publication manifest proof.
  pub manifest_hash: PublicationManifestHash,
  /// Exact dense and lexical versions used by both collections.
  pub compatibility: EmbeddingCompatibilityEntry,
}

/// Complete verified trio that is eligible for a later, separate activation operation.
pub struct PublicationActivationCandidate {
  /// Stable publication build that produced this candidate.
  pub build_id: PublicationBuildId,
  /// Stable reconciliation operation that verified the complete trio.
  pub reconcile_id: PublicationReconcileIdentity,
  /// Validated canonical, node, and edge release trio.
  pub trio: KnowledgeReleaseTrio,
  /// Storage-neutral hash of the complete canonical release content.
  pub canonical_content_hash: String,
  /// Reconciled publication manifest proof.
  pub manifest_hash: PublicationManifestHash,
  /// Persisted node collection proof.
  pub node_persisted_hash: PersistedCollectionHash,
  /// Persisted edge collection proof.
  pub edge_persisted_hash: PersistedCollectionHash,
}

/// Closed status snapshot containing no canonical text or vector data.
#[derive(Debug)]
pub struct PublicationStatus {
  /// Stable build queried by the caller.
  pub build_id: PublicationBuildId,
  /// Current closed lifecycle state.
  pub state: PublicationBuildState,
  /// Next node ordinal expected by island-port.
  pub next_node_ordinal: u32,
  /// Next edge ordinal expected by island-port.
  pub next_edge_ordinal: u32,
}

/// Outbound-only publication capability independent of HTTP, UDS, Qdrant, and provider details.
#[async_trait]
pub trait KnowledgePublicationPort: Send + Sync {
  /// Inspects one node batch under the deterministic worst-case transport context without I/O.
  fn inspect_node_batch(
    &self,
    canonical: &CanonicalReleasePin,
    request: &NodePublicationBatch,
  ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure>;

  /// Inspects one edge batch under the deterministic worst-case transport context without I/O.
  fn inspect_edge_batch(
    &self,
    canonical: &CanonicalReleasePin,
    request: &EdgePublicationBatch,
  ) -> Result<PublicationBatchWireAdmission, KnowledgeReleaseFailure>;

  /// Begins or safely replays one immutable publication build.
  async fn begin(
    &self,
    context: &KnowledgePublicationContext,
    request: &BeginPublication,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure>;

  /// Submits or safely replays one bounded node batch.
  async fn submit_nodes(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &NodePublicationBatch,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure>;

  /// Freezes and verifies the complete node artifact.
  async fn freeze_nodes(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &FreezeNodes,
  ) -> Result<FrozenNodePublication, KnowledgeReleaseFailure>;

  /// Submits or safely replays one bounded edge batch after node freeze.
  async fn submit_edges(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &EdgePublicationBatch,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure>;

  /// Freezes and verifies the complete edge artifact.
  async fn freeze_edges(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    request: &FreezeEdges,
  ) -> Result<FrozenEdgePublication, KnowledgeReleaseFailure>;

  /// Reconciles a complete trio and returns an activation candidate without activating it.
  async fn reconcile(
    &self,
    context: &KnowledgePublicationContext,
    request: &ReconcilePublication,
  ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure>;

  /// Reads one build state without mutation.
  async fn status(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure>;

  /// Requests a legal abort transition; activation candidates cannot be aborted.
  async fn abort(
    &self,
    context: &KnowledgePublicationContext,
    canonical: &CanonicalReleasePin,
    build_id: &PublicationBuildId,
    idempotency_key: &PublicationIdempotencyKey,
    fingerprint: &PublicationRequestFingerprint,
  ) -> Result<PublicationStatus, KnowledgeReleaseFailure>;
}
