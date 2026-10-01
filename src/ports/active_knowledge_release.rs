//! Read-only authority for the atomically active canonical and retrieval projection tuple.

use async_trait::async_trait;
use thiserror::Error;

use crate::{
  domain::retrieval_data::NeighborProjectionExecutionExpectation,
  ports::canonical_read::CanonicalReadContext,
};

/// Closed failures from active knowledge-release selection.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ActiveKnowledgeReleaseError {
  /// The request context was malformed or already expired.
  #[error("active knowledge release request is invalid")]
  InvalidRequest,
  /// The peer does not implement the required canonical-data schema.
  #[error("active knowledge release schema is incompatible")]
  SchemaIncompatible,
  /// Island-port could not serve the active-pointer read.
  #[error("active knowledge release authority is unavailable")]
  Unavailable,
  /// The shared request deadline elapsed.
  #[error("active knowledge release request timed out")]
  Timeout,
  /// The authority returned contradictory, partial, or malformed release data.
  #[error("active knowledge release data is inconsistent")]
  InconsistentData,
}

/// Selects one complete, verified active canonical/node/edge tuple without mutating it.
#[async_trait]
pub trait ActiveKnowledgeReleasePort: Send + Sync {
  /// Returns `None` only when no complete knowledge release is safely active.
  async fn active_knowledge_release(
    &self,
    context: &CanonicalReadContext,
  ) -> Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError>;
}
