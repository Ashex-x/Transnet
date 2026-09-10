//! Read-only port for release-pinned canonical sense details.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::{EvidenceUse, ReleaseId, SenseId},
  canonical_content::CanonicalSenseDetails,
};

/// Typed failure from the canonical sense-details store.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalSenseDetailsRepositoryError {
  /// The canonical detail store could not complete the bounded read.
  #[error("canonical sense-details repository unavailable")]
  Unavailable,
  /// Stored detail records contradicted the release, sense, or evidence contract.
  #[error("canonical sense-details repository returned inconsistent data")]
  InconsistentData,
}

/// Exact, permission-scoped request for one canonical sense's bounded details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSenseDetailsReadRequest {
  release_id: ReleaseId,
  sense_id: SenseId,
  evidence_use: EvidenceUse,
}

impl CanonicalSenseDetailsReadRequest {
  /// Creates a read request pinned to one immutable canonical release and sense.
  ///
  /// This request intentionally does not select an active release. A caller composing details
  /// with canonical retrieval must pass the same release selected for that retrieval.
  pub fn new(release_id: ReleaseId, sense_id: SenseId, evidence_use: EvidenceUse) -> Self {
    Self {
      release_id,
      sense_id,
      evidence_use,
    }
  }

  /// Returns the immutable canonical release that may supply the details.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release_id
  }

  /// Returns the one canonical sense whose details may be read.
  pub fn sense_id(&self) -> &SenseId {
    &self.sense_id
  }

  /// Returns the source-permission operation required by the caller.
  pub const fn evidence_use(&self) -> EvidenceUse {
    self.evidence_use
  }
}

/// Reads bounded canonical details without selecting a release or exposing storage details.
///
/// Implementations must use the request's exact release and sense identifiers, then expose an
/// aggregate only when its target lexeme and sense are lookup-eligible and every factual
/// assertion's source and asset lineage permits `CanonicalSenseDetailsReadRequest::evidence_use`.
/// `Ok(None)` means no matching detail aggregate is safely available; implementations must not
/// substitute another release, sense, or a partially filtered aggregate. The application service
/// defensively repeats those checks before returning a result.
#[async_trait]
pub trait CanonicalSenseDetailsRepository: Send + Sync {
  /// Loads the one complete bounded detail aggregate that matches `request`, when safely usable.
  ///
  /// # Errors
  ///
  /// Returns an error when the canonical detail store is unavailable or returns records that
  /// contradict the pinned read contract.
  async fn load(
    &self,
    request: &CanonicalSenseDetailsReadRequest,
  ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError>;
}
