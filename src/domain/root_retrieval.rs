//! Bounded, request-local root retrieval outcomes and ranking metadata.

use std::fmt;

use thiserror::Error;

use super::{
  canonical::{
    normalize_lookup_key, CanonicalId, CanonicalReleasePin, EvidenceId, LanguageTag, ReleaseId,
    SenseId,
  },
  model_runtime::ModelVersion,
  retrieval_data::{
    NodeMatchMechanism, NodeProjectionExecutionExpectation, RetrievalNodeType,
    DENSE_QUERY_VECTOR_DIMENSIONS,
  },
};

/// Frozen query-side lexical encoder input contract.
pub const QUERY_LEXICAL_INPUT_VERSION: &str = "query-lexical-input-v1";
/// Frozen query-side lexical encoder identity.
pub const QUERY_LEXICAL_ENCODER_IDENTITY: &str = "transnet-lexical-bm25";
/// Frozen query-side lexical encoder revision.
pub const QUERY_LEXICAL_ENCODER_REVISION: &str = "v1";
/// Maximum number of vector nominations considered after canonical resolution.
pub const MAX_ROOT_NOMINATIONS: usize = 12;
/// Maximum canonical label size admitted during root hydration.
pub const MAX_ROOT_LABEL_BYTES: usize = 512;

/// Release-pinned dense and sparse execution contract for one service instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootRetrievalExecutionSpec {
  content: CanonicalReleasePin,
  node_collection_id: CanonicalId,
  node_collection_content_hash: String,
  dense_artifact_revision: ModelVersion,
  dense_dimensions: usize,
  sparse_encoder_identity: String,
  sparse_encoder_revision: String,
  sparse_input_version: String,
}

impl RootRetrievalExecutionSpec {
  /// Creates the exact supported root-query execution contract.
  ///
  /// # Errors
  ///
  /// Returns an error for dimension drift or a sparse encoder/input contract mismatch.
  pub fn new(
    content: CanonicalReleasePin,
    node_collection_id: CanonicalId,
    node_collection_content_hash: impl Into<String>,
    execution: NodeProjectionExecutionExpectation,
  ) -> Result<Self, RootRetrievalValidationError> {
    let node_collection_content_hash = node_collection_content_hash.into();
    if execution.validate().is_err()
      || node_collection_content_hash.is_empty()
      || node_collection_content_hash.len() > 128
      || node_collection_content_hash.chars().any(char::is_control)
    {
      return Err(RootRetrievalValidationError::InvalidExecutionSpec);
    }
    let dense_artifact_revision = ModelVersion::new(execution.dense_artifact_revision)
      .map_err(|_| RootRetrievalValidationError::InvalidExecutionSpec)?;
    Ok(Self {
      content,
      node_collection_id,
      node_collection_content_hash,
      dense_artifact_revision,
      dense_dimensions: DENSE_QUERY_VECTOR_DIMENSIONS,
      sparse_encoder_identity: QUERY_LEXICAL_ENCODER_IDENTITY.to_owned(),
      sparse_encoder_revision: QUERY_LEXICAL_ENCODER_REVISION.to_owned(),
      sparse_input_version: QUERY_LEXICAL_INPUT_VERSION.to_owned(),
    })
  }

  /// Returns the immutable release whose dictionary and dense artifact are configured.
  pub fn release_id(&self) -> &ReleaseId {
    &self.content.release_id
  }

  /// Returns the full immutable canonical release and schema pin.
  pub fn content(&self) -> &CanonicalReleasePin {
    &self.content
  }

  /// Returns the immutable node collection selected with the release.
  pub fn node_collection_id(&self) -> &CanonicalId {
    &self.node_collection_id
  }

  /// Returns the exact persisted node collection hash.
  pub fn node_collection_content_hash(&self) -> &str {
    &self.node_collection_content_hash
  }

  /// Returns the exact dense artifact revision.
  pub fn dense_artifact_revision(&self) -> &ModelVersion {
    &self.dense_artifact_revision
  }

  /// Returns the exact projection-side execution contract sent to retrieval-data.
  pub fn projection_expectation(&self) -> NodeProjectionExecutionExpectation {
    NodeProjectionExecutionExpectation::v1(self.dense_artifact_revision.as_str())
      .expect("validated root retrieval execution specification")
  }

  /// Returns the fixed dense dimensions.
  pub const fn dense_dimensions(&self) -> usize {
    self.dense_dimensions
  }

  /// Returns the frozen sparse encoder identity.
  pub fn sparse_encoder_identity(&self) -> &str {
    &self.sparse_encoder_identity
  }

  /// Returns the frozen sparse encoder revision.
  pub fn sparse_encoder_revision(&self) -> &str {
    &self.sparse_encoder_revision
  }

  /// Returns the frozen query input version.
  pub fn sparse_input_version(&self) -> &str {
    &self.sparse_input_version
  }
}

/// Dense execution metadata validated before any candidate is ranked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseQueryExecutionReceipt {
  /// Release pin carried by the model operation context.
  pub release_id: ReleaseId,
  /// Exact immutable artifact revision reported by the embedding adapter.
  pub artifact_revision: String,
  /// Exact vector dimensions observed by Transnet.
  pub dimensions: usize,
}

/// Sparse execution metadata returned by the trusted request-local encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseQueryExecutionReceipt {
  /// Release-local dictionary used for token indexes.
  pub release_id: ReleaseId,
  /// Frozen encoder identity.
  pub encoder_identity: String,
  /// Frozen encoder revision.
  pub encoder_revision: String,
  /// Frozen query input contract.
  pub input_version: String,
}

/// Complete validated signal execution proof retained in a resolved result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootQueryExecutionReceipts {
  /// Dense embedding execution metadata.
  pub dense: DenseQueryExecutionReceipt,
  /// Sparse lexical encoding execution metadata.
  pub sparse: SparseQueryExecutionReceipt,
}

/// A private normalized query retained only for one retrieval operation.
#[derive(Clone, PartialEq, Eq)]
pub struct RootQuery(String);

impl RootQuery {
  /// Creates one bounded, nonblank request-local query.
  ///
  /// # Errors
  ///
  /// Returns an error when the query is blank or exceeds the embedding boundary.
  pub fn new(value: impl Into<String>) -> Result<Self, RootRetrievalValidationError> {
    let value = value.into();
    if value.len() > crate::domain::model_runtime::MAX_EMBEDDING_INPUT_BYTES {
      return Err(RootRetrievalValidationError::InvalidQuery);
    }
    let value = normalize_lookup_key(&value);
    if value.is_empty() || value.len() > crate::domain::model_runtime::MAX_EMBEDDING_INPUT_BYTES {
      return Err(RootRetrievalValidationError::InvalidQuery);
    }
    Ok(Self(value))
  }

  /// Borrows the request-local value for canonical resolution or signal generation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for RootQuery {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("RootQuery(REDACTED)")
  }
}

/// Input accepted by bounded root retrieval.
#[derive(Clone)]
pub struct RootRetrievalRequest {
  /// Private normalized source query.
  pub query: RootQuery,
  /// Source language used by canonical resolution and projection filters.
  pub source_language: LanguageTag,
  /// Explanation language used for authoritative hydration.
  pub explanation_language: LanguageTag,
  /// Maximum number of exploratory nominations returned.
  pub limit: usize,
}

impl fmt::Debug for RootRetrievalRequest {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("RootRetrievalRequest")
      .field("query", &"[redacted]")
      .field("source_language", &self.source_language)
      .field("explanation_language", &self.explanation_language)
      .field("limit", &self.limit)
      .finish()
  }
}

impl RootRetrievalRequest {
  /// Validates one request without retaining or hashing its content.
  ///
  /// # Errors
  ///
  /// Returns an error when the nomination limit is outside the closed bound.
  pub fn new(
    query: RootQuery,
    source_language: LanguageTag,
    explanation_language: LanguageTag,
    limit: usize,
  ) -> Result<Self, RootRetrievalValidationError> {
    if !(1..=MAX_ROOT_NOMINATIONS).contains(&limit) {
      return Err(RootRetrievalValidationError::InvalidLimit);
    }
    Ok(Self {
      query,
      source_language,
      explanation_language,
      limit,
    })
  }
}

/// Closed local validation failures for root retrieval.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RootRetrievalValidationError {
  /// The private query was blank or oversized.
  #[error("root retrieval query is invalid")]
  InvalidQuery,
  /// The result limit was zero or exceeded the root-retrieval bound.
  #[error("root retrieval limit is invalid")]
  InvalidLimit,
  /// Dense or sparse execution metadata did not match the frozen release contract.
  #[error("root retrieval execution specification is invalid")]
  InvalidExecutionSpec,
}

/// One authoritative canonical root selected before vector retrieval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRoot {
  /// Full immutable release and canonical-schema pin owning the root.
  pub content: CanonicalReleasePin,
  /// Stable projection node identity supplied by canonical-data.
  pub node_id: CanonicalId,
  /// Positive immutable canonical node revision.
  pub revision: u32,
  /// Closed canonical node family supplied by canonical-data.
  pub node_family: RetrievalNodeType,
  /// Stable lexical sense identity resolved by canonical-data.
  pub sense_id: SenseId,
  /// Canonical display label, never generated by similarity retrieval.
  pub canonical_label: String,
  /// Sorted authoritative evidence identities supporting the root values.
  pub evidence_ids: Vec<EvidenceId>,
}

/// Closed canonical resolution outcome evaluated before retrieval-data is contacted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalRootResolution {
  /// No eligible canonical root exists.
  NotFound,
  /// Multiple same-precedence roots remain and must not be guessed.
  Ambiguous(Vec<CanonicalRoot>),
  /// Exactly one authoritative root may seed retrieval.
  Resolved(CanonicalRoot),
}

/// Evidence class used by the root-retrieval superset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootEvidenceState {
  /// Canonical-data established the root identity.
  Verified,
  /// Evidence-grounded request-local synthesis; unused by similarity alone.
  Inferred,
  /// A retrieval signal nominated a hydrated canonical node without proving a fact.
  Exploratory,
}

/// One ranked item whose identity was authoritatively hydrated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankedRootItem {
  /// Stable canonical node identity.
  pub node_id: CanonicalId,
  /// Positive immutable canonical node revision.
  pub revision: u32,
  /// Closed authoritative node family.
  pub node_type: RetrievalNodeType,
  /// Stable sense identity when available.
  pub sense_id: Option<SenseId>,
  /// Authority-owned label.
  pub canonical_label: String,
  /// Sorted evidence identities supporting hydrated display values.
  pub evidence_ids: Vec<EvidenceId>,
  /// Explicit evidence class; similarity can only produce `Exploratory`.
  pub evidence_state: RootEvidenceState,
  /// Deterministic request-local rank, starting at one.
  pub rank: usize,
  /// Integer score used only to order nominations under one release.
  pub ranking_basis_points: u16,
  /// Closed, sorted nomination mechanisms.
  pub matched_by: Vec<NodeMatchMechanism>,
}

/// Retrieval coverage kept separate from canonical resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootRetrievalCoverage {
  /// All nominated pointers were hydrated under the pin.
  Complete,
  /// Some pointers were absent from canonical-data and were omitted.
  PartialPublication,
  /// Qdrant-backed retrieval was unavailable and only canonical data is returned.
  CanonicalOnly,
}

/// Result after one root-first retrieval flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootRetrievalOutcome {
  /// Exact immutable release used throughout the flow.
  pub release_id: ReleaseId,
  /// Authoritative root, always present in this successful outcome.
  pub root: RankedRootItem,
  /// Evidence-grounded synthesis kept separate; this slice never creates it.
  pub inferred: Vec<RankedRootItem>,
  /// Hydrated similarity nominations that remain explicitly non-factual.
  pub exploratory: Vec<RankedRootItem>,
  /// Explicit dependency/publication coverage.
  pub coverage: RootRetrievalCoverage,
  /// Validated dense and sparse execution metadata, absent only for canonical-only degradation.
  pub execution_receipts: Option<RootQueryExecutionReceipts>,
}

/// Top-level result preserving no-root and ambiguity without touching retrieval-data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootRetrievalResult {
  /// No canonical root exists; retrieval-data was not queried.
  NotFound,
  /// Canonical ambiguity is preserved; retrieval-data was not queried.
  Ambiguous(Vec<CanonicalRoot>),
  /// One root was resolved and optionally enriched.
  Resolved(Box<RootRetrievalOutcome>),
}
