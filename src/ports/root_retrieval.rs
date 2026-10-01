//! Request-local canonical root and lexical query-encoding ports.
//!
//! The root resolver remains an application-facing seam. Authoritative node hydration uses the
//! shared [`crate::ports::canonical_read::CanonicalReadPort`] contract.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::{LanguageTag, ReleaseId},
  request_context::RequestContext,
  retrieval_data::SparseQueryVector,
  root_retrieval::{CanonicalRootResolution, RootQuery, SparseQueryExecutionReceipt},
};

/// Sparse vector paired with release-pinned encoder execution metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryLexicalEncoding {
  /// Ephemeral sparse vector sent to retrieval-data.
  pub vector: SparseQueryVector,
  /// Exact release, encoder, revision, and input contract used.
  pub receipt: SparseQueryExecutionReceipt,
}

/// Content-free failures from canonical root resolution and hydration.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum CanonicalRootError {
  /// The canonical request or release pin was invalid.
  #[error("canonical root request is invalid")]
  InvalidRequest,
  /// The selected immutable release is unavailable.
  #[error("canonical root release is unavailable")]
  ContentReleaseUnavailable,
  /// The canonical-data dependency is unavailable.
  #[error("canonical root dependency is unavailable")]
  Unavailable,
  /// The shared deadline elapsed.
  #[error("canonical root request timed out")]
  Timeout,
  /// Canonical-data returned contradictory identities or releases.
  #[error("canonical root data is inconsistent")]
  InconsistentData,
}

/// Canonical-data operations needed by bounded root retrieval.
#[async_trait]
pub trait CanonicalRootPort: Send + Sync {
  /// Resolves zero, one, or multiple same-precedence roots before vector retrieval.
  async fn resolve_root(
    &self,
    context: &RequestContext,
    query: &RootQuery,
    source_language: &LanguageTag,
    explanation_language: &LanguageTag,
  ) -> Result<CanonicalRootResolution, CanonicalRootError>;
}

/// Closed lexical encoder failures without query content.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum QueryLexicalEncoderError {
  /// The release-local dictionary or encoder is unavailable.
  #[error("query lexical encoder is unavailable")]
  Unavailable,
  /// The query could not be represented by the frozen release-local dictionary.
  #[error("query lexical encoding is invalid")]
  InvalidEncoding,
  /// The shared request deadline elapsed.
  #[error("query lexical encoding timed out")]
  Timeout,
}

/// Trusted request-local encoder for the frozen release-local lexical dictionary.
#[async_trait]
pub trait QueryLexicalEncoderPort: Send + Sync {
  /// Encodes private query text without forwarding it to retrieval-data.
  async fn encode(
    &self,
    context: &RequestContext,
    release_id: &ReleaseId,
    query: &RootQuery,
  ) -> Result<QueryLexicalEncoding, QueryLexicalEncoderError>;
}
