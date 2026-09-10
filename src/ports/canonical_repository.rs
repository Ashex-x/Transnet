//! Repository port for active canonical content and lexical candidates.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::ActiveContentVersion,
  retrieval::{CandidateLoadRequest, CanonicalCandidate, LexicalSearchRequest, RepositoryMatch},
};

/// Typed failure from a canonical lexical repository.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalRepositoryError {
  /// The canonical store could not complete a bounded read.
  #[error("canonical repository unavailable")]
  Unavailable,
  /// The canonical store returned records that failed its own integrity checks.
  #[error("canonical repository returned inconsistent data")]
  InconsistentData,
}

/// Reads immutable canonical lexical content without exposing storage details.
///
/// Implementations must apply the supplied release, language, status, and source-permission
/// filters before returning data. The application service repeats those checks before fusion so a
/// stale index or adapter bug cannot make restricted material visible.
#[async_trait]
pub trait CanonicalRepository: Send + Sync {
  /// Resolves the one compatible lexical and vector version currently active for new lookups.
  async fn active_content_version(&self) -> Result<ActiveContentVersion, CanonicalRepositoryError>;

  /// Finds exact-form, phrase, lemma, morphology, and full-text canonical candidates.
  ///
  /// Each returned candidate must belong to `request.content.release_id` and expose the signal
  /// that selected it. Repository order is intentionally not semantically meaningful.
  async fn search_lexical(
    &self,
    request: &LexicalSearchRequest,
  ) -> Result<Vec<RepositoryMatch>, CanonicalRepositoryError>;

  /// Hydrates canonical candidates identified by the vector retriever.
  ///
  /// Implementations must not return a candidate outside the requested release or one whose
  /// definition evidence lacks the requested permission.
  async fn load_candidates(
    &self,
    request: &CandidateLoadRequest,
  ) -> Result<Vec<CanonicalCandidate>, CanonicalRepositoryError>;
}
