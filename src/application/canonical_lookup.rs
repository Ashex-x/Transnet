//! Evidence-backed canonical lookup composition for transport adapters.
//!
//! The service obtains a public-only snapshot through the cache foundation, then maps the pinned
//! retrieval outcome into the canonical-card contract. It never invokes the learning model and it
//! never applies identity, context, history, mature-content, or learner-specific policy.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  application::{
    canonical_lookup_cache::{
      CanonicalLookupSnapshotCacheError, CanonicalLookupSnapshotCacheService,
    },
    canonical_lookup_card::CanonicalLookupCardMapper,
    retrieval::RetrievalOutcome,
  },
  domain::{
    canonical_lookup_cache::{CanonicalLookupCacheEligibility, PublicCanonicalLookupRequest},
    lookup_card::CanonicalLookupCard,
  },
};

/// Failure while obtaining a deterministic evidence-backed canonical lookup card.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupError {
  /// The public snapshot cache or authoritative canonical retrieval could not produce a card.
  #[error(transparent)]
  Snapshot(#[from] CanonicalLookupSnapshotCacheError),
}

impl CanonicalLookupError {
  /// Returns whether retrying the lookup can recover a transient canonical dependency failure.
  ///
  /// Integrity, cache-contract, and configuration failures remain safely typed but are not
  /// retryable because a caller cannot repair them by repeating the same request.
  pub fn is_retryable(&self) -> bool {
    match self {
      Self::Snapshot(error) => error.is_retryable(),
    }
  }
}

/// Coordinates public snapshot retrieval and canonical-card assembly.
///
/// The service accepts an explicit cache eligibility declaration so callers must classify private
/// influences before lookup. A bypass still returns an uncached public base card; private
/// projection remains outside this service.
#[derive(Clone)]
pub struct CanonicalLookupService {
  snapshots: Arc<CanonicalLookupSnapshotCacheService>,
}

impl CanonicalLookupService {
  /// Creates a canonical lookup service around an explicit public snapshot-cache service.
  pub fn new(snapshots: Arc<CanonicalLookupSnapshotCacheService>) -> Self {
    Self { snapshots }
  }

  /// Obtains a pinned, evidence-backed canonical lookup card without model generation.
  ///
  /// # Errors
  ///
  /// Returns an error when authoritative canonical retrieval cannot produce a safe public
  /// snapshot. Cache adapter misses and unavailability retain their best-effort fallback behavior.
  pub async fn lookup(
    &self,
    request: PublicCanonicalLookupRequest,
    eligibility: CanonicalLookupCacheEligibility,
  ) -> Result<CanonicalLookupCard, CanonicalLookupError> {
    let snapshot = self
      .snapshots
      .lookup(&request, eligibility)
      .await?
      .into_snapshot();
    let outcome = RetrievalOutcome {
      content: snapshot.content().clone(),
      path: snapshot.retrieval_path(),
      candidates: snapshot.candidates().to_vec(),
    };
    Ok(CanonicalLookupCardMapper::assemble(
      request.retrieval(),
      outcome,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    application::retrieval::CanonicalRetrievalError,
    domain::canonical_lookup_cache::CanonicalLookupCacheValidationError,
    ports::canonical_repository::CanonicalRepositoryError,
  };

  #[test]
  fn only_transient_canonical_repository_unavailability_is_retryable() {
    let unavailable = CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Retrieval(
      CanonicalRetrievalError::Repository(CanonicalRepositoryError::Unavailable),
    ));
    let inconsistent =
      CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Retrieval(
        CanonicalRetrievalError::Repository(CanonicalRepositoryError::InconsistentData),
      ));
    let contract = CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Contract(
      CanonicalLookupCacheValidationError::CandidateLimitExceeded,
    ));
    let invalid_configuration =
      CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::ZeroTtl);

    assert!(unavailable.is_retryable());
    assert!(!inconsistent.is_retryable());
    assert!(!contract.is_retryable());
    assert!(!invalid_configuration.is_retryable());
  }
}
