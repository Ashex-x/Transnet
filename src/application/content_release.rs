//! Application orchestration for staged content-release publication and safe rollback.

use std::sync::Arc;

use crate::{
  domain::{
    canonical::{ActiveContentVersion, ReleaseId, SourceId},
    content_release::{
      ContentRelease, ContentReleaseDraft, ContentReleaseGate, ContentReleaseGateResult,
      SourceRemovalImpact, VectorCollectionBuild,
    },
  },
  ports::content_release_repository::{ContentReleaseRepository, ContentReleaseRepositoryError},
};

/// Coordinates content-release commands through an authoritative atomic repository port.
#[derive(Clone)]
pub struct ContentReleaseService {
  repository: Arc<dyn ContentReleaseRepository>,
}

impl ContentReleaseService {
  /// Creates a content-release service from an explicit authoritative repository.
  pub fn new(repository: Arc<dyn ContentReleaseRepository>) -> Self {
    Self { repository }
  }

  /// Stages a new immutable lexical release with its frozen publication policy.
  ///
  /// # Errors
  ///
  /// Returns an error when the release cannot be staged safely.
  pub async fn stage(
    &self,
    draft: ContentReleaseDraft,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    self.repository.stage(draft).await
  }

  /// Returns one release aggregate for bounded administrative inspection.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository cannot serve the read.
  pub async fn release(
    &self,
    release_id: &ReleaseId,
  ) -> Result<Option<ContentRelease>, ContentReleaseRepositoryError> {
    self.repository.release(release_id).await
  }

  /// Records the result of a required validation gate while the release remains staging.
  ///
  /// # Errors
  ///
  /// Returns an error when the requested lifecycle transition is not valid.
  pub async fn record_gate_result(
    &self,
    release_id: &ReleaseId,
    gate: ContentReleaseGate,
    result: ContentReleaseGateResult,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    self
      .repository
      .record_gate_result(release_id, gate, result)
      .await
  }

  /// Records the one vector build pinned to the staged release's exact active-content tuple.
  ///
  /// # Errors
  ///
  /// Returns an error when the vector build does not match the staging release.
  pub async fn begin_vector_build(
    &self,
    release_id: &ReleaseId,
    build: VectorCollectionBuild,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    self.repository.begin_vector_build(release_id, build).await
  }

  /// Records the expected count observed after completed vector reconciliation.
  ///
  /// # Errors
  ///
  /// Returns an error when reconciliation cannot make the pinned vector collection ready.
  pub async fn reconcile_vector_build(
    &self,
    release_id: &ReleaseId,
    indexed_records: u64,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    self
      .repository
      .reconcile_vector_build(release_id, indexed_records)
      .await
  }

  /// Atomically publishes a validated compatible lexical and vector release pair.
  ///
  /// # Errors
  ///
  /// Returns an error when validation, source safety, or the atomic repository transition fails.
  pub async fn publish(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError> {
    self.repository.publish(release_id).await
  }

  /// Atomically restores the active release's direct safe rollback predecessor.
  ///
  /// # Errors
  ///
  /// Returns an error when the target is not an eligible, safe rollback predecessor.
  pub async fn rollback(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError> {
    self.repository.rollback(release_id).await
  }

  /// Returns the active compatible content pair only when source safety permits serving it.
  ///
  /// # Errors
  ///
  /// Returns an error when the pointer or a source quarantine makes serving unsafe.
  pub async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ContentReleaseRepositoryError> {
    self.repository.active_content_version().await
  }

  /// Permanently quarantines a source and reports the release metadata it affects.
  ///
  /// # Errors
  ///
  /// Returns an error when the source is unknown or its safety state cannot be recorded.
  pub async fn quarantine_source(
    &self,
    source_id: &SourceId,
  ) -> Result<SourceRemovalImpact, ContentReleaseRepositoryError> {
    self.repository.quarantine_source(source_id).await
  }
}
