//! Outbound-only authority boundary for offline release activation and rollback selection.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::ReleaseId,
  knowledge_publication::PublicationIdempotencyKey,
  release_control::{AuditSequence, ReleaseSelectionReceipt, RollbackSelection},
};
use crate::ports::knowledge_publication::{
  KnowledgePublicationContext, PublicationActivationCandidate,
};

/// Explicit request to submit a reconciled candidate to the external release authority.
pub struct SubmitActivationCandidate<'a> {
  /// Immutable candidate produced by successful publication reconciliation.
  pub candidate: &'a PublicationActivationCandidate,
  /// Active release the authority must still observe before switching.
  pub expected_active_release: ReleaseId,
  /// Caller key making the submission safely replayable.
  pub idempotency_key: PublicationIdempotencyKey,
  /// Expected append-only audit position for this transition.
  pub audit_sequence: AuditSequence,
}

/// Closed release-control failures that never expose authority response bodies.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum ReleaseControlError {
  /// The caller supplied an invalid or incomplete proof.
  #[error("release-control request is invalid")]
  InvalidRequest,
  /// The authority rejected a stale active-release or idempotency precondition.
  #[error("release-control transition conflicts with authority state")]
  Conflict,
  /// The target release is not immutable, verified, retained, and addressable.
  #[error("release-control target is unavailable")]
  ImmutableReleaseUnavailable,
  /// The peer does not implement the required release-control schema.
  #[error("release-control schema is incompatible")]
  SchemaIncompatible,
  /// The authority could not complete the operation.
  #[error("release-control authority is unavailable")]
  Unavailable,
  /// The shared absolute deadline expired.
  #[error("release-control operation timed out")]
  Timeout,
  /// The authority returned malformed or contradictory data.
  #[error("release-control authority returned inconsistent data")]
  InconsistentData,
}

/// External mutation authority used only by authenticated offline publication tooling.
#[async_trait]
pub trait ReleaseControlPort: Send + Sync {
  /// Submits one complete immutable activation proof for atomic authority-side selection.
  async fn submit_activation_candidate(
    &self,
    context: &KnowledgePublicationContext,
    request: &SubmitActivationCandidate<'_>,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError>;

  /// Selects one previously verified and retained immutable release.
  async fn select_rollback(
    &self,
    context: &KnowledgePublicationContext,
    request: &RollbackSelection,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError>;
}
