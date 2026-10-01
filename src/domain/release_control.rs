//! Offline release-control values for activation submission and safe rollback selection.

use std::fmt;

use thiserror::Error;

use super::{
  canonical::ReleaseId,
  knowledge_publication::{PublicationIdempotencyKey, PublicationManifestHash},
};

/// Strictly positive append-only audit sequence assigned by the offline publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditSequence(u64);

impl AuditSequence {
  /// Creates a nonzero audit sequence.
  ///
  /// # Errors
  ///
  /// Returns an error for zero, which cannot follow an append-only event.
  pub fn new(value: u64) -> Result<Self, ReleaseControlValidationError> {
    if value == 0 {
      Err(ReleaseControlValidationError::InvalidAuditSequence)
    } else {
      Ok(Self(value))
    }
  }

  /// Returns the wire sequence value.
  pub const fn get(self) -> u64 {
    self.0
  }

  /// Validates the exact next receipt in an offline control sequence.
  ///
  /// # Errors
  ///
  /// Returns an error when `next` is not this value plus one.
  pub fn require_next(self, next: Self) -> Result<(), ReleaseControlValidationError> {
    self
      .0
      .checked_add(1)
      .filter(|expected| *expected == next.0)
      .map(|_| ())
      .ok_or(ReleaseControlValidationError::InvalidAuditSequence)
  }
}

/// Closed machine-readable reason for selecting a retained rollback target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackReasonCode {
  /// The active release failed post-activation verification.
  VerificationFailure,
  /// The active release has a confirmed canonical-content defect.
  CanonicalDefect,
  /// The active release has a confirmed projection defect.
  ProjectionDefect,
  /// A security quarantine requires returning to a retained release.
  SecurityQuarantine,
}

impl RollbackReasonCode {
  /// Returns the stable wire spelling.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::VerificationFailure => "verification_failure",
      Self::CanonicalDefect => "canonical_defect",
      Self::ProjectionDefect => "projection_defect",
      Self::SecurityQuarantine => "security_quarantine",
    }
  }
}

/// Immutable proof naming one previously verified rollback target.
pub struct RollbackSelection {
  /// Active release the authority must still observe before selection.
  pub expected_active_release: ReleaseId,
  /// Previously verified and retained release to select.
  pub target_release: ReleaseId,
  /// Reconciled publication manifest of the retained target.
  pub target_manifest_hash: PublicationManifestHash,
  /// Storage-neutral canonical-content proof of the retained target.
  pub target_canonical_content_hash: String,
  /// Closed operational reason without reviewer prose.
  pub reason_code: RollbackReasonCode,
  /// Caller key making the selection safely replayable.
  pub idempotency_key: PublicationIdempotencyKey,
  /// Last audit position authoritatively observed before this transition.
  pub prior_audit_sequence: AuditSequence,
  /// Expected append-only audit position for this transition.
  pub audit_sequence: AuditSequence,
}

impl fmt::Debug for RollbackSelection {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("RollbackSelection")
      .field("reason_code", &self.reason_code)
      .field("audit_sequence", &self.audit_sequence)
      .field("proofs", &"[redacted]")
      .finish()
  }
}

/// Validated authority receipt for activation or rollback selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSelectionReceipt {
  /// Release selected by the authority after the atomic transition.
  pub active_release: ReleaseId,
  /// Release active immediately before the transition.
  pub previous_release: ReleaseId,
  /// RFC 3339 authority timestamp for the completed transition.
  pub selected_at: String,
  /// Append-only audit position committed with the transition.
  pub audit_sequence: AuditSequence,
  /// Reconciled manifest selected by the transition.
  pub manifest_hash: PublicationManifestHash,
}

/// Closed validation failures for offline release-control values.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum ReleaseControlValidationError {
  /// An audit position was zero, nonconsecutive, or did not match the receipt.
  #[error("release-control audit sequence is invalid")]
  InvalidAuditSequence,
  /// A canonical-content proof was not a canonical SHA-256 value.
  #[error("canonical content hash is invalid")]
  InvalidCanonicalContentHash,
  /// The selected and previous releases contradict the requested transition.
  #[error("release-control receipt contradicts the request")]
  InconsistentReceipt,
}

/// Validates the storage-neutral canonical-content proof syntax.
///
/// # Errors
///
/// Returns an error unless the value is a canonical SHA-256 proof.
pub fn validate_canonical_content_hash(value: &str) -> Result<(), ReleaseControlValidationError> {
  PublicationManifestHash::parse(value.to_string())
    .map(|_| ())
    .map_err(|_| ReleaseControlValidationError::InvalidCanonicalContentHash)
}
