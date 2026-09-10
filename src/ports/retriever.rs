//! Ranked-candidate retrieval interface for exact and vector search adapters.

use async_trait::async_trait;
use thiserror::Error;

/// Failure returned by a retrieval adapter.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RetrievalError {
  /// The optional or required retrieval dependency cannot currently answer.
  #[error("retrieval dependency is unavailable")]
  Unavailable,
}

/// Retrieves ordered candidates for an application-defined query.
///
/// The returned vector is ranked from most to least useful according to the adapter's documented
/// contract. Application services remain responsible for deterministic fusion across retrievers.
#[async_trait]
pub trait Retriever<Query, Candidate>: Send + Sync
where
  Query: Send + Sync,
  Candidate: Send + Sync,
{
  /// Returns ranked candidates for `query`.
  ///
  /// # Errors
  ///
  /// Returns an error when the retrieval dependency cannot serve the query.
  async fn retrieve(&self, query: &Query) -> Result<Vec<Candidate>, RetrievalError>;
}
