//! Vector-retrieval port for immutable, metadata-filtered collections.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::retrieval::{VectorMatch, VectorSearchRequest};

/// Typed failure from the derived vector index.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VectorRetrieverError {
  /// The vector engine or required embedding path is temporarily unavailable.
  #[error("vector retriever unavailable")]
  Unavailable,
  /// The vector engine returned a response that violates the requested collection contract.
  #[error("vector retriever returned invalid metadata")]
  InvalidResponse,
}

/// Searches one immutable vector collection through explicit release and metadata filters.
///
/// The vector database is derived state, not factual authority. Implementations must apply every
/// field in `VectorSearchRequest` as a server-side filter and return the same metadata in each
/// result so the application can reject stale or mismatched records defensively.
#[async_trait]
pub trait VectorRetriever: Send + Sync {
  /// Returns bounded vector matches for one purpose and content language.
  async fn search(
    &self,
    request: &VectorSearchRequest,
  ) -> Result<Vec<VectorMatch>, VectorRetrieverError>;
}
