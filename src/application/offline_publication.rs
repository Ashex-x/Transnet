//! Explicit offline composition for publication, activation submission, and rollback selection.

use std::sync::Arc;

use crate::{
  application::knowledge_publication::{KnowledgePublicationPlan, KnowledgePublicationService},
  domain::{
    knowledge_release::KnowledgeReleaseFailure,
    release_control::{ReleaseSelectionReceipt, RollbackSelection},
  },
  ports::{
    knowledge_publication::{KnowledgePublicationContext, PublicationActivationCandidate},
    release_control::{ReleaseControlError, ReleaseControlPort, SubmitActivationCandidate},
  },
};

/// Offline-only publication composition; it is intentionally absent from online runtime state.
#[derive(Clone)]
pub struct OfflinePublicationService {
  publication: KnowledgePublicationService,
  release_control: Arc<dyn ReleaseControlPort>,
}

impl OfflinePublicationService {
  /// Creates offline orchestration from separate projection and release-control authorities.
  pub fn new(
    publication: KnowledgePublicationService,
    release_control: Arc<dyn ReleaseControlPort>,
  ) -> Self {
    Self {
      publication,
      release_control,
    }
  }

  /// Builds and reconciles one immutable trio without submitting or activating it.
  ///
  /// # Errors
  ///
  /// Returns the closed publication failure from build, freeze, or reconciliation.
  pub async fn build_activation_candidate(
    &self,
    context: &KnowledgePublicationContext,
    plan: &KnowledgePublicationPlan,
  ) -> Result<PublicationActivationCandidate, KnowledgeReleaseFailure> {
    self.publication.publish(context, plan).await
  }

  /// Explicitly submits a previously reconciled candidate to the external authority.
  ///
  /// This method never runs as a side effect of candidate construction. A successful receipt says
  /// the external authority selected the release atomically; Transnet does not mutate the pointer.
  ///
  /// # Errors
  ///
  /// Returns a closed release-control failure for invalid proofs, conflicts, or dependency failure.
  pub async fn submit_activation_candidate(
    &self,
    context: &KnowledgePublicationContext,
    request: &SubmitActivationCandidate<'_>,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    self
      .release_control
      .submit_activation_candidate(context, request)
      .await
  }

  /// Explicitly selects one retained rollback target through the external authority.
  ///
  /// # Errors
  ///
  /// Returns a closed release-control failure when the target is not retained and verified, the
  /// active-release or audit precondition conflicts, or the dependency cannot complete the call.
  pub async fn select_rollback(
    &self,
    context: &KnowledgePublicationContext,
    request: &RollbackSelection,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    self.release_control.select_rollback(context, request).await
  }
}
