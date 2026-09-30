//! Publication lifecycle, idempotency, manifest hashes, and execution-receipt invariants.

use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
  canonical::ReleaseId,
  embedding_input::{
    EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION, NODE_DENSE_INPUT_VERSION,
    NODE_LEXICAL_INPUT_VERSION,
  },
  knowledge_projection::{
    EdgeProjectionBuild, EmbeddingCompatibilityEntry, NodeProjectionBuild, DENSE_VECTOR_NAME,
    SPARSE_VECTOR_NAME,
  },
  knowledge_release::KnowledgeReleaseFailure,
};

const MAX_ID_LENGTH: usize = 128;
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 256;
const HASH_DOMAIN: &str = "sha256:";

/// Closed publication collection family used in hashes, receipts, and lifecycle checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationCollectionFamily {
  /// Canonical knowledge-node projection.
  Nodes,
  /// Typed relationship-edge projection.
  Edges,
}

impl PublicationCollectionFamily {
  fn label(self) -> &'static str {
    match self {
      Self::Nodes => "nodes",
      Self::Edges => "edges",
    }
  }
}

/// Closed build lifecycle; only explicit transitions can approach activation eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationBuildState {
  /// Node batches may be admitted.
  AcceptingNodes,
  /// The node artifact and collection proof are frozen.
  NodesFrozen,
  /// Edge batches may be admitted against the frozen node proof.
  AcceptingEdges,
  /// The edge artifact and collection proof are frozen.
  EdgesFrozen,
  /// Cross-store and cross-collection reconciliation is running.
  Reconciling,
  /// Reconciliation succeeded and produced an immutable activation candidate.
  ActivationCandidate,
  /// The build failed closed and cannot resume in place.
  Failed,
  /// An explicit abort is being completed.
  Aborting,
  /// The build is abandoned and can never activate.
  Abandoned,
  /// Retention policy permits cleanup of unverified build artifacts.
  GcEligible,
}

impl PublicationBuildState {
  /// Applies one legal lifecycle transition.
  ///
  /// # Errors
  ///
  /// Returns an error when the transition skips a prerequisite or leaves a terminal state.
  pub fn transition(self, next: Self) -> Result<Self, PublicationValidationError> {
    let legal = matches!(
      (self, next),
      (Self::AcceptingNodes, Self::NodesFrozen)
        | (Self::NodesFrozen, Self::AcceptingEdges)
        | (Self::AcceptingEdges, Self::EdgesFrozen)
        | (Self::EdgesFrozen, Self::Reconciling)
        | (Self::Reconciling, Self::ActivationCandidate)
        | (Self::AcceptingNodes, Self::Failed)
        | (Self::NodesFrozen, Self::Failed)
        | (Self::AcceptingEdges, Self::Failed)
        | (Self::EdgesFrozen, Self::Failed)
        | (Self::Reconciling, Self::Failed)
        | (Self::AcceptingNodes, Self::Aborting)
        | (Self::NodesFrozen, Self::Aborting)
        | (Self::AcceptingEdges, Self::Aborting)
        | (Self::EdgesFrozen, Self::Aborting)
        | (Self::Reconciling, Self::Aborting)
        | (Self::Aborting, Self::Abandoned)
        | (Self::Abandoned, Self::GcEligible)
        | (Self::Failed, Self::GcEligible)
    );
    legal
      .then_some(next)
      .ok_or(PublicationValidationError::InvalidLifecycleTransition)
  }
}

/// Stable build identity derived only from immutable publication inputs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicationBuildId(String);

impl PublicationBuildId {
  /// Parses a previously derived build identity received from the publication authority.
  pub fn parse(value: impl Into<String>) -> Result<Self, PublicationValidationError> {
    let value = value.into();
    validate_hash(&value)?;
    Ok(Self(value))
  }

  /// Derives a build identity without request IDs, clocks, randomness, or storage-generated values.
  pub fn derive(
    release: &ReleaseId,
    projection_schema: &str,
    node_projection_hash: &str,
    registry_entry_id: &str,
  ) -> Result<Self, PublicationValidationError> {
    validate_id(projection_schema)?;
    validate_hash(node_projection_hash)?;
    validate_id(registry_entry_id)?;
    let mut bytes = CanonicalBytes::new("transnet-publication-build-v1");
    bytes.field(release.as_str());
    bytes.field(projection_schema);
    bytes.field(node_projection_hash);
    bytes.field(registry_entry_id);
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the stable opaque build identity.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Bounded caller idempotency key whose debug representation is always redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct PublicationIdempotencyKey(String);

impl PublicationIdempotencyKey {
  /// Validates one nonempty printable ASCII key.
  pub fn parse(value: impl Into<String>) -> Result<Self, PublicationValidationError> {
    let value = value.into();
    if value.is_empty()
      || value.len() > MAX_IDEMPOTENCY_KEY_LENGTH
      || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
      return Err(PublicationValidationError::InvalidIdentity);
    }
    Ok(Self(value))
  }

  /// Returns the opaque caller-supplied key for strict wire propagation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for PublicationIdempotencyKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("PublicationIdempotencyKey([redacted])")
  }
}

/// Canonical fingerprint of one mutation request, independent of transport metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationRequestFingerprint(String);

impl PublicationRequestFingerprint {
  /// Derives a fingerprint from fields already ordered by the owning operation contract.
  pub fn derive(operation: &str, fields: &[&str]) -> Result<Self, PublicationValidationError> {
    validate_id(operation)?;
    let mut bytes = CanonicalBytes::new("transnet-publication-request-v1");
    bytes.field(operation);
    for field in fields {
      bytes.field(field);
    }
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Zero-based bounded ordinal of one publication batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicationBatchOrdinal(u32);

impl PublicationBatchOrdinal {
  /// Creates an ordinal; the transport enforces contiguity against prior accepted batches.
  pub const fn new(value: u32) -> Self {
    Self(value)
  }

  /// Returns the zero-based ordinal.
  pub const fn get(self) -> u32 {
    self.0
  }

  /// Verifies that this ordinal is the exact next batch in one family-local sequence.
  pub fn require_next(self, expected: u32) -> Result<(), PublicationValidationError> {
    if self.0 == expected {
      Ok(())
    } else {
      Err(PublicationValidationError::NonConsecutiveBatch)
    }
  }
}

/// Hash of one canonical bounded batch payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBatchContentHash(String);

impl PublicationBatchContentHash {
  /// Derives a family-separated batch hash from ordered canonical point hashes.
  pub fn derive(
    family: PublicationCollectionFamily,
    point_hashes: &[&str],
  ) -> Result<Self, PublicationValidationError> {
    let mut bytes = CanonicalBytes::new("transnet-publication-batch-v1");
    bytes.field(family.label());
    for value in point_hashes {
      validate_hash(value)?;
      bytes.field(value);
    }
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Identity needed to classify an idempotent batch retry without inspecting its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBatchIdentity {
  /// Stable build being mutated.
  pub build_id: PublicationBuildId,
  /// Consecutive family-local batch ordinal.
  pub ordinal: PublicationBatchOrdinal,
  /// Canonical request fingerprint.
  pub request_fingerprint: PublicationRequestFingerprint,
  /// Canonical payload hash.
  pub content_hash: PublicationBatchContentHash,
}

/// Closed retry classification for a previously accepted batch identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationRetryDisposition {
  /// Every canonical identity member matches and the stored result can be replayed.
  Replay,
}

impl PublicationBatchIdentity {
  /// Classifies an identical retry or rejects a conflicting reuse of build and ordinal.
  pub fn classify_retry(
    &self,
    retry: &Self,
  ) -> Result<PublicationRetryDisposition, PublicationValidationError> {
    if self.build_id != retry.build_id || self.ordinal != retry.ordinal {
      return Err(PublicationValidationError::RetryTargetMismatch);
    }
    if self.request_fingerprint != retry.request_fingerprint
      || self.content_hash != retry.content_hash
    {
      return Err(PublicationValidationError::ConflictingRetry);
    }
    Ok(PublicationRetryDisposition::Replay)
  }
}

/// Stable identity of one node or edge finalize operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationFinalizeIdentity(String);

impl PublicationFinalizeIdentity {
  /// Derives finalize identity from the build, family, and persisted collection proof.
  pub fn derive(
    build: &PublicationBuildId,
    family: PublicationCollectionFamily,
    collection: &PersistedCollectionHash,
  ) -> Self {
    let mut bytes = CanonicalBytes::new("transnet-publication-finalize-v1");
    bytes.field(build.as_str());
    bytes.field(family.label());
    bytes.field(collection.as_str());
    Self(hash(bytes.finish()))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Stable identity of one reconciliation attempt over two frozen collection proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationReconcileIdentity(String);

impl PublicationReconcileIdentity {
  /// Derives reconciliation identity from one build and its ordered node/edge proofs.
  pub fn derive(
    build: &PublicationBuildId,
    nodes: &PersistedCollectionHash,
    edges: &PersistedCollectionHash,
  ) -> Result<Self, PublicationValidationError> {
    if nodes == edges {
      return Err(PublicationValidationError::InvalidCollectionProof);
    }
    let mut bytes = CanonicalBytes::new("transnet-publication-reconcile-v1");
    bytes.field(build.as_str());
    bytes.field(nodes.as_str());
    bytes.field(edges.as_str());
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Frozen collision-free release-local lexical term dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalDictionaryManifest {
  entries: Vec<(String, u32)>,
  dictionary_hash: String,
}

impl LexicalDictionaryManifest {
  /// Validates bytewise term ordering, unique consecutive indices, and computes its hash.
  pub fn new(entries: Vec<(String, u32)>) -> Result<Self, PublicationValidationError> {
    let mut prior: Option<&[u8]> = None;
    for (position, (term, index)) in entries.iter().enumerate() {
      if term.is_empty() || term.len() > 256 || *index != (position as u32).saturating_add(1) {
        return Err(PublicationValidationError::InvalidDictionary);
      }
      if prior.is_some_and(|value| value >= term.as_bytes()) {
        return Err(PublicationValidationError::InvalidDictionary);
      }
      prior = Some(term.as_bytes());
    }
    let mut bytes = CanonicalBytes::new("transnet-lexical-dictionary-v1");
    bytes.field("transnet-lexical-bm25");
    bytes.field("v1");
    bytes.u64(entries.len() as u64);
    for (term, index) in &entries {
      bytes.field(term);
      bytes.u32(*index);
    }
    Ok(Self {
      entries,
      dictionary_hash: hash(bytes.finish()),
    })
  }

  /// Returns the deterministic dictionary hash.
  pub fn dictionary_hash(&self) -> &str {
    &self.dictionary_hash
  }

  /// Returns the release-local dictionary cardinality.
  pub fn cardinality(&self) -> usize {
    self.entries.len()
  }

  /// Returns the frozen ordered term-to-index entries.
  pub fn entries(&self) -> &[(String, u32)] {
    &self.entries
  }
}

/// Persisted collection hash that excludes raw dense and sparse vector bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCollectionHash(String);

impl PersistedCollectionHash {
  /// Parses a persisted collection proof returned by the publication authority.
  pub fn parse(value: impl Into<String>) -> Result<Self, PublicationValidationError> {
    let value = value.into();
    validate_hash(&value)?;
    Ok(Self(value))
  }

  /// Hashes one node collection proof from deterministic projection and execution metadata.
  pub fn for_nodes(
    build: &NodeProjectionBuild,
    entry: &EmbeddingCompatibilityEntry,
    dictionary: &LexicalDictionaryManifest,
  ) -> Result<Self, PublicationValidationError> {
    let points = build.points.iter().map(|point| {
      (
        point.point_id.as_str(),
        point.content_hash.as_str(),
        point.dense_input.input_hash(),
        point.lexical_input.input_hash(),
      )
    });
    Self::derive(
      PublicationCollectionFamily::Nodes,
      &build.release_id,
      build.embedding.payload_schema_version(),
      &build.content_hash,
      build.point_count,
      points,
      entry,
      dictionary,
    )
  }

  /// Hashes one edge collection proof from deterministic projection and execution metadata.
  pub fn for_edges(
    build: &EdgeProjectionBuild,
    entry: &EmbeddingCompatibilityEntry,
    dictionary: &LexicalDictionaryManifest,
  ) -> Result<Self, PublicationValidationError> {
    let points = build.points.iter().map(|point| {
      (
        point.point_id.as_str(),
        point.content_hash.as_str(),
        point.dense_input.input_hash(),
        point.lexical_input.input_hash(),
      )
    });
    Self::derive(
      PublicationCollectionFamily::Edges,
      &build.release_id,
      build.embedding.payload_schema_version(),
      &build.content_hash,
      build.point_count,
      points,
      entry,
      dictionary,
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn derive<'a>(
    family: PublicationCollectionFamily,
    release: &ReleaseId,
    projection_schema: &str,
    projection_hash: &str,
    point_count: u64,
    points: impl Iterator<Item = (&'a str, &'a str, &'a str, &'a str)>,
    entry: &EmbeddingCompatibilityEntry,
    dictionary: &LexicalDictionaryManifest,
  ) -> Result<Self, PublicationValidationError> {
    validate_hash(projection_hash)?;
    let mut points = points.collect::<Vec<_>>();
    points.sort_unstable();
    if points.windows(2).any(|pair| pair[0].0 == pair[1].0) || points.len() as u64 != point_count {
      return Err(PublicationValidationError::InvalidCollectionProof);
    }
    let mut bytes = CanonicalBytes::new("transnet-persisted-collection-v1");
    bytes.field(family.label());
    bytes.field(release.as_str());
    bytes.field(projection_schema);
    bytes.field(projection_hash);
    bytes.field(&entry.entry_id);
    bytes.field(&entry.dense_model_family);
    bytes.field(&entry.dense_artifact_revision);
    bytes.u32(entry.dense_dimensions);
    bytes.field(&entry.dense_vector_name);
    bytes.field(&entry.node_dense_input_specification);
    bytes.field(&entry.edge_dense_input_specification);
    bytes.field(&entry.lexical_encoder_identity);
    bytes.field(&entry.lexical_encoder_revision);
    bytes.field(&entry.lexical_contract_identity);
    bytes.field(&entry.lexical_vector_name);
    bytes.field(&entry.node_lexical_input_specification);
    bytes.field(&entry.edge_lexical_input_specification);
    bytes.field(dictionary.dictionary_hash());
    bytes.u64(dictionary.cardinality() as u64);
    bytes.u64(point_count);
    for (point_id, content_hash, dense_hash, lexical_hash) in points {
      validate_hash(content_hash)?;
      validate_hash(dense_hash)?;
      validate_hash(lexical_hash)?;
      bytes.field(point_id);
      bytes.field(content_hash);
      bytes.field(dense_hash);
      bytes.field(lexical_hash);
    }
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Hash of the reconciled canonical release and paired persisted collection proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationManifestHash(String);

impl PublicationManifestHash {
  /// Parses a publication manifest proof returned by the publication authority.
  pub fn parse(value: impl Into<String>) -> Result<Self, PublicationValidationError> {
    let value = value.into();
    validate_hash(&value)?;
    Ok(Self(value))
  }

  /// Binds one immutable release to distinct node and edge collection proofs.
  pub fn derive(
    release: &ReleaseId,
    canonical_schema: &str,
    nodes: &PersistedCollectionHash,
    edges: &PersistedCollectionHash,
  ) -> Result<Self, PublicationValidationError> {
    validate_id(canonical_schema)?;
    if nodes == edges {
      return Err(PublicationValidationError::InvalidCollectionProof);
    }
    let mut bytes = CanonicalBytes::new("transnet-publication-manifest-v1");
    bytes.field(release.as_str());
    bytes.field(canonical_schema);
    bytes.field(nodes.as_str());
    bytes.field(edges.as_str());
    Ok(Self(hash(bytes.finish())))
  }

  /// Returns the canonical SHA-256 representation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Dense execution observation returned by the future controlled embedding authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenseExecutionReceipt {
  /// Registry entry requested by the publisher.
  pub registry_entry_id: String,
  /// Immutable artifact revision independently observed by island-port.
  pub observed_artifact_revision: String,
  /// Observed dense dimensions.
  pub dimensions: u32,
  /// Observed named-vector slot.
  pub vector_name: String,
  /// Applied canonical input specification.
  pub input_specification: String,
  /// Number of processed points.
  pub processed_point_count: u64,
}

/// Lexical execution observation returned by the future controlled encoding authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalExecutionReceipt {
  /// Deterministic encoder identity.
  pub encoder_identity: String,
  /// Exact encoder revision.
  pub encoder_revision: String,
  /// Observed named-vector slot.
  pub vector_name: String,
  /// Applied canonical input specification.
  pub input_specification: String,
  /// Frozen release-local dictionary hash.
  pub dictionary_hash: String,
  /// Number of processed points.
  pub processed_point_count: u64,
}

/// Paired execution proof for one node or edge collection build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildExecutionReceipt {
  /// Stable build receiving the execution result.
  pub build_id: PublicationBuildId,
  /// Collection family processed by the receipt.
  pub family: PublicationCollectionFamily,
  /// Dense execution observation.
  pub dense: DenseExecutionReceipt,
  /// Lexical execution observation.
  pub lexical: LexicalExecutionReceipt,
}

impl BuildExecutionReceipt {
  /// Validates observed execution against one exact registry entry and dictionary proof.
  ///
  /// # Errors
  ///
  /// Returns a closed mismatch without accepting request-echoed or drifting metadata.
  pub fn validate(
    &self,
    expected_build: &PublicationBuildId,
    entry: &EmbeddingCompatibilityEntry,
    dictionary: &LexicalDictionaryManifest,
    expected_count: u64,
  ) -> Result<(), PublicationValidationError> {
    if &self.build_id != expected_build {
      return Err(PublicationValidationError::ReceiptBuildMismatch);
    }
    let (dense_input, lexical_input) = match self.family {
      PublicationCollectionFamily::Nodes => (NODE_DENSE_INPUT_VERSION, NODE_LEXICAL_INPUT_VERSION),
      PublicationCollectionFamily::Edges => (EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION),
    };
    if self.dense.registry_entry_id != entry.entry_id
      || self.dense.observed_artifact_revision != entry.dense_artifact_revision
    {
      return Err(PublicationValidationError::ArtifactRevisionMismatch);
    }
    if self.dense.dimensions != entry.dense_dimensions
      || self.dense.vector_name != DENSE_VECTOR_NAME
      || self.dense.input_specification != dense_input
    {
      return Err(PublicationValidationError::EmbeddingMetadataMismatch);
    }
    if self.lexical.encoder_identity != entry.lexical_encoder_identity
      || self.lexical.encoder_revision != entry.lexical_encoder_revision
    {
      return Err(PublicationValidationError::LexicalEncoderMismatch);
    }
    if self.lexical.vector_name != SPARSE_VECTOR_NAME
      || self.lexical.input_specification != lexical_input
    {
      return Err(PublicationValidationError::EmbeddingMetadataMismatch);
    }
    if self.lexical.dictionary_hash != dictionary.dictionary_hash() {
      return Err(PublicationValidationError::DictionaryMismatch);
    }
    if self.dense.processed_point_count != expected_count
      || self.lexical.processed_point_count != expected_count
    {
      return Err(PublicationValidationError::IncompleteBuild);
    }
    Ok(())
  }
}

/// Closed validation failures for the publication foundation.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum PublicationValidationError {
  /// An identifier, idempotency key, or schema value is invalid.
  #[error("publication identity is invalid")]
  InvalidIdentity,
  /// A typed SHA-256 value is invalid.
  #[error("publication hash is invalid")]
  InvalidHash,
  /// The requested lifecycle transition is forbidden.
  #[error("publication lifecycle transition is invalid")]
  InvalidLifecycleTransition,
  /// A retry targeted another build or ordinal.
  #[error("publication retry target does not match")]
  RetryTargetMismatch,
  /// A retry reused an identity with different canonical content.
  #[error("publication retry conflicts with accepted content")]
  ConflictingRetry,
  /// A batch ordinal skipped or repeated the next expected position.
  #[error("publication batch ordinal is not consecutive")]
  NonConsecutiveBatch,
  /// A lexical dictionary is unordered, duplicated, oversized, or non-consecutive.
  #[error("lexical dictionary is invalid")]
  InvalidDictionary,
  /// Persisted point identities or counts do not form one closed collection proof.
  #[error("persisted collection proof is invalid")]
  InvalidCollectionProof,
  /// An execution receipt belongs to another build.
  #[error("execution receipt belongs to another build")]
  ReceiptBuildMismatch,
  /// The observed dense artifact revision differs from the approved registry entry.
  #[error("observed dense artifact revision mismatched the registry")]
  ArtifactRevisionMismatch,
  /// Dense dimensions, vector name, or input specification differs.
  #[error("execution embedding metadata is incompatible")]
  EmbeddingMetadataMismatch,
  /// The observed lexical encoder identity or revision differs.
  #[error("observed lexical encoder mismatched the registry")]
  LexicalEncoderMismatch,
  /// The observed lexical dictionary differs from the frozen dictionary.
  #[error("observed lexical dictionary mismatched the manifest")]
  DictionaryMismatch,
  /// Dense or lexical execution did not process the complete point set.
  #[error("execution receipt describes an incomplete build")]
  IncompleteBuild,
}

impl From<PublicationValidationError> for KnowledgeReleaseFailure {
  fn from(value: PublicationValidationError) -> Self {
    match value {
      PublicationValidationError::InvalidLifecycleTransition => Self::InvalidLifecycleTransition,
      PublicationValidationError::RetryTargetMismatch
      | PublicationValidationError::ConflictingRetry
      | PublicationValidationError::NonConsecutiveBatch => Self::IdempotencyConflict,
      PublicationValidationError::ArtifactRevisionMismatch => Self::ArtifactRevisionMismatch,
      PublicationValidationError::LexicalEncoderMismatch => Self::LexicalEncoderMismatch,
      PublicationValidationError::DictionaryMismatch
      | PublicationValidationError::InvalidDictionary => Self::DictionaryMismatch,
      PublicationValidationError::InvalidCollectionProof
      | PublicationValidationError::InvalidHash => Self::HashOrCountReconciliationFailed,
      PublicationValidationError::IncompleteBuild => Self::IncompleteTrio,
      PublicationValidationError::EmbeddingMetadataMismatch => Self::EmbeddingMetadataIncompatible,
      PublicationValidationError::InvalidIdentity
      | PublicationValidationError::ReceiptBuildMismatch => Self::SchemaIncompatible,
    }
  }
}

fn validate_id(value: &str) -> Result<(), PublicationValidationError> {
  if value.trim().is_empty() || value.len() > MAX_ID_LENGTH {
    Err(PublicationValidationError::InvalidIdentity)
  } else {
    Ok(())
  }
}

fn validate_hash(value: &str) -> Result<(), PublicationValidationError> {
  let Some(hex) = value.strip_prefix(HASH_DOMAIN) else {
    return Err(PublicationValidationError::InvalidHash);
  };
  if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
    Err(PublicationValidationError::InvalidHash)
  } else {
    Ok(())
  }
}

fn hash(bytes: Vec<u8>) -> String {
  format!("sha256:{:x}", Sha256::digest(bytes))
}

struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
  fn new(domain: &str) -> Self {
    let mut this = Self(Vec::new());
    this.field(domain);
    this
  }

  fn field(&mut self, value: &str) {
    self.u64(value.len() as u64);
    self.0.extend_from_slice(value.as_bytes());
  }

  fn u32(&mut self, value: u32) {
    self.0.extend_from_slice(&value.to_be_bytes());
  }

  fn u64(&mut self, value: u64) {
    self.0.extend_from_slice(&value.to_be_bytes());
  }

  fn finish(self) -> Vec<u8> {
    self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::knowledge_projection::{
    DENSE_DIMENSIONS, DENSE_MODEL_FAMILY, LEXICAL_CONTRACT_IDENTITY, LEXICAL_ENCODER_IDENTITY,
    LEXICAL_ENCODER_REVISION,
  };

  fn release(value: &str) -> ReleaseId {
    ReleaseId::new(value).unwrap()
  }

  fn fingerprint(value: &str) -> PublicationRequestFingerprint {
    PublicationRequestFingerprint::derive("append_nodes", &[value]).unwrap()
  }

  fn batch(value: &str) -> PublicationBatchContentHash {
    PublicationBatchContentHash::derive(
      PublicationCollectionFamily::Nodes,
      &[&hash(value.as_bytes().to_vec())],
    )
    .unwrap()
  }

  fn entry() -> EmbeddingCompatibilityEntry {
    EmbeddingCompatibilityEntry {
      entry_id: "registry-test-r1".into(),
      dense_model_family: DENSE_MODEL_FAMILY.into(),
      dense_artifact_revision: "sha256:immutable-artifact-r1".into(),
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

  fn build_id() -> PublicationBuildId {
    PublicationBuildId::derive(
      &release("release-r1"),
      "knowledge-graph-v1",
      &hash(b"nodes".to_vec()),
      "registry-test-r1",
    )
    .unwrap()
  }

  fn receipt(dictionary: &LexicalDictionaryManifest) -> BuildExecutionReceipt {
    BuildExecutionReceipt {
      build_id: build_id(),
      family: PublicationCollectionFamily::Nodes,
      dense: DenseExecutionReceipt {
        registry_entry_id: "registry-test-r1".into(),
        observed_artifact_revision: "sha256:immutable-artifact-r1".into(),
        dimensions: DENSE_DIMENSIONS,
        vector_name: DENSE_VECTOR_NAME.into(),
        input_specification: NODE_DENSE_INPUT_VERSION.into(),
        processed_point_count: 2,
      },
      lexical: LexicalExecutionReceipt {
        encoder_identity: LEXICAL_ENCODER_IDENTITY.into(),
        encoder_revision: LEXICAL_ENCODER_REVISION.into(),
        vector_name: SPARSE_VECTOR_NAME.into(),
        input_specification: NODE_LEXICAL_INPUT_VERSION.into(),
        dictionary_hash: dictionary.dictionary_hash().into(),
        processed_point_count: 2,
      },
    }
  }

  #[test]
  fn lifecycle_accepts_only_prerequisite_order() {
    let state = PublicationBuildState::AcceptingNodes
      .transition(PublicationBuildState::NodesFrozen)
      .unwrap()
      .transition(PublicationBuildState::AcceptingEdges)
      .unwrap()
      .transition(PublicationBuildState::EdgesFrozen)
      .unwrap()
      .transition(PublicationBuildState::Reconciling)
      .unwrap()
      .transition(PublicationBuildState::ActivationCandidate)
      .unwrap();
    assert_eq!(state, PublicationBuildState::ActivationCandidate);
    assert_eq!(
      PublicationBuildState::AcceptingNodes.transition(PublicationBuildState::AcceptingEdges),
      Err(PublicationValidationError::InvalidLifecycleTransition)
    );
  }

  #[test]
  fn failed_and_abandoned_builds_cannot_activate() {
    for state in [
      PublicationBuildState::Failed,
      PublicationBuildState::Abandoned,
    ] {
      assert_eq!(
        state.transition(PublicationBuildState::ActivationCandidate),
        Err(PublicationValidationError::InvalidLifecycleTransition)
      );
    }
  }

  #[test]
  fn identical_retry_replays_and_conflicting_retry_fails_closed() {
    let build = PublicationBuildId::derive(
      &release("release-r1"),
      "knowledge-graph-v1",
      &hash(b"nodes".to_vec()),
      "registry-r1",
    )
    .unwrap();
    let accepted = PublicationBatchIdentity {
      build_id: build,
      ordinal: PublicationBatchOrdinal::new(0),
      request_fingerprint: fingerprint("same"),
      content_hash: batch("same"),
    };
    assert_eq!(
      accepted.classify_retry(&accepted),
      Ok(PublicationRetryDisposition::Replay)
    );
    let mut conflict = accepted.clone();
    conflict.content_hash = batch("different");
    assert_eq!(
      accepted.classify_retry(&conflict),
      Err(PublicationValidationError::ConflictingRetry)
    );
  }

  #[test]
  fn batch_ordinals_are_consecutive_and_finalize_reconcile_identities_are_separate() {
    assert_eq!(PublicationBatchOrdinal::new(2).require_next(2), Ok(()));
    assert_eq!(
      PublicationBatchOrdinal::new(3).require_next(2),
      Err(PublicationValidationError::NonConsecutiveBatch)
    );
    let nodes = PersistedCollectionHash(hash(b"nodes".to_vec()));
    let edges = PersistedCollectionHash(hash(b"edges".to_vec()));
    let finalize =
      PublicationFinalizeIdentity::derive(&build_id(), PublicationCollectionFamily::Nodes, &nodes);
    let reconcile = PublicationReconcileIdentity::derive(&build_id(), &nodes, &edges).unwrap();
    assert_ne!(finalize.as_str(), reconcile.as_str());
  }

  #[test]
  fn dictionary_requires_stable_order_and_consecutive_collision_free_indices() {
    let first =
      LexicalDictionaryManifest::new(vec![("C".into(), 1), ("C#".into(), 2), ("C++".into(), 3)])
        .unwrap();
    let second =
      LexicalDictionaryManifest::new(vec![("C".into(), 1), ("C#".into(), 2), ("C++".into(), 3)])
        .unwrap();
    assert_eq!(first.dictionary_hash(), second.dictionary_hash());
    assert_eq!(
      LexicalDictionaryManifest::new(vec![("C#".into(), 1), ("C".into(), 2)]),
      Err(PublicationValidationError::InvalidDictionary)
    );
  }

  #[test]
  fn build_identity_excludes_request_and_clock_values() {
    let arguments = (
      &release("release-r1"),
      "knowledge-graph-v1",
      hash(b"nodes".to_vec()),
      "registry-r1",
    );
    assert_eq!(
      PublicationBuildId::derive(arguments.0, arguments.1, &arguments.2, arguments.3),
      PublicationBuildId::derive(arguments.0, arguments.1, &arguments.2, arguments.3)
    );
  }

  #[test]
  fn idempotency_debug_is_redacted() {
    let key = PublicationIdempotencyKey::parse("sensitive-publication-key").unwrap();
    assert!(!format!("{key:?}").contains("sensitive"));
  }

  #[test]
  fn execution_receipt_requires_exact_build_revision_dimensions_and_encoder() {
    let dictionary = LexicalDictionaryManifest::new(vec![("alpha".into(), 1)]).unwrap();
    let valid = receipt(&dictionary);
    assert_eq!(
      valid.validate(&build_id(), &entry(), &dictionary, 2),
      Ok(())
    );

    let mut drift = valid.clone();
    drift.dense.observed_artifact_revision = "sha256:drift".into();
    assert_eq!(
      drift.validate(&build_id(), &entry(), &dictionary, 2),
      Err(PublicationValidationError::ArtifactRevisionMismatch)
    );

    let mut dimensions = valid.clone();
    dimensions.dense.dimensions = 768;
    assert_eq!(
      dimensions.validate(&build_id(), &entry(), &dictionary, 2),
      Err(PublicationValidationError::EmbeddingMetadataMismatch)
    );

    let mut lexical = valid;
    lexical.lexical.encoder_revision = "v2".into();
    assert_eq!(
      lexical.validate(&build_id(), &entry(), &dictionary, 2),
      Err(PublicationValidationError::LexicalEncoderMismatch)
    );
  }

  #[test]
  fn execution_receipt_rejects_build_dictionary_and_count_mismatches() {
    let dictionary = LexicalDictionaryManifest::new(vec![("alpha".into(), 1)]).unwrap();
    let valid = receipt(&dictionary);
    let other_build = PublicationBuildId::derive(
      &release("release-r2"),
      "knowledge-graph-v1",
      &hash(b"nodes".to_vec()),
      "registry-test-r1",
    )
    .unwrap();
    assert_eq!(
      valid.validate(&other_build, &entry(), &dictionary, 2),
      Err(PublicationValidationError::ReceiptBuildMismatch)
    );

    let other_dictionary = LexicalDictionaryManifest::new(vec![("beta".into(), 1)]).unwrap();
    assert_eq!(
      valid.validate(&build_id(), &entry(), &other_dictionary, 2),
      Err(PublicationValidationError::DictionaryMismatch)
    );
    assert_eq!(
      valid.validate(&build_id(), &entry(), &dictionary, 3),
      Err(PublicationValidationError::IncompleteBuild)
    );
  }

  #[test]
  fn manifest_hash_separates_node_and_edge_collection_domains() {
    let node = PersistedCollectionHash(hash(b"node-proof".to_vec()));
    let edge = PersistedCollectionHash(hash(b"edge-proof".to_vec()));
    let manifest =
      PublicationManifestHash::derive(&release("release-r1"), "canonical-v1", &node, &edge)
        .unwrap();
    assert_ne!(node, edge);
    assert_ne!(manifest.as_str(), node.as_str());
    assert_ne!(manifest.as_str(), edge.as_str());
  }
}
