//! Content-release persistence port for atomic publication and safe rollback.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::{ActiveContentVersion, ReleaseId, SourceId},
  content_release::{
    ContentRelease, ContentReleaseDraft, ContentReleaseGate, ContentReleaseGateResult,
    ContentReleaseValidationError, SourceRemovalImpact, VectorCollectionBuild,
  },
};

/// Typed failure from the authoritative content-release store.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ContentReleaseRepositoryError {
  /// A caller attempted to stage a release identifier that already exists.
  #[error("content release already exists")]
  AlreadyExists,
  /// A requested staged, published, or retained release does not exist.
  #[error("content release does not exist")]
  NotFound,
  /// Stored lifecycle records contradicted the singleton active-content pointer.
  #[error("content release store contains inconsistent lifecycle state")]
  InconsistentState,
  /// A source is not declared by any content-release manifest.
  #[error("source is not tracked by a content release")]
  SourceNotTracked,
  /// A permanently quarantined source would be introduced or served by a release.
  #[error("content release contains a quarantined source")]
  SourceQuarantined,
  /// The active pointer references a release made unsafe by source quarantine.
  #[error("active content is unsafe because a source was quarantined")]
  ActiveContentUnsafe,
  /// A rollback target is not the direct validated predecessor of the active release.
  #[error("content release is not the active release's rollback predecessor")]
  InvalidRollbackTarget,
  /// The repository could not complete the bounded state transition or read.
  #[error("content release repository unavailable")]
  Unavailable,
  /// A domain validation or lifecycle invariant rejected the requested transition.
  #[error(transparent)]
  Validation(#[from] ContentReleaseValidationError),
}

/// Stores staged release state and atomically selects one compatible active content tuple.
///
/// Implementations must make [`ContentReleaseRepository::publish`] and
/// [`ContentReleaseRepository::rollback`] atomic with the active-content pointer. They must also
/// refuse publication or rollback for releases that contain a source previously quarantined by
/// [`ContentReleaseRepository::quarantine_source`]. This port stores release metadata only; a
/// production implementation must separately coordinate canonical-record, vector, cache, and
/// source-artifact cleanup according to its deployment transaction and outbox design.
#[async_trait]
pub trait ContentReleaseRepository: Send + Sync {
  /// Persists a new immutable staging release.
  ///
  /// # Errors
  ///
  /// Returns an error when the release exists, contains a quarantined source, or cannot be stored.
  async fn stage(
    &self,
    draft: ContentReleaseDraft,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError>;

  /// Returns one release aggregate without changing its lifecycle state.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository cannot serve the bounded read.
  async fn release(
    &self,
    release_id: &ReleaseId,
  ) -> Result<Option<ContentRelease>, ContentReleaseRepositoryError>;

  /// Records the latest result for a gate required by a staging release.
  ///
  /// # Errors
  ///
  /// Returns an error when the release is absent, no longer staging, or the gate is not required.
  async fn record_gate_result(
    &self,
    release_id: &ReleaseId,
    gate: ContentReleaseGate,
    result: ContentReleaseGateResult,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError>;

  /// Records the one immutable vector build pinned to the staging release's active-content tuple.
  ///
  /// # Errors
  ///
  /// Returns an error when the release is absent, no longer staging, or the vector tuple differs.
  async fn begin_vector_build(
    &self,
    release_id: &ReleaseId,
    build: VectorCollectionBuild,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError>;

  /// Records the expected-count portion of vector reconciliation for the immutable collection.
  ///
  /// Implementations must require record-identity and content-hash reconciliation before accepting
  /// this count; the count alone is not a proof that the collection has no stale payloads.
  ///
  /// # Errors
  ///
  /// Returns an error when no build exists, the count differs, or the release cannot transition.
  async fn reconcile_vector_build(
    &self,
    release_id: &ReleaseId,
    indexed_records: u64,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError>;

  /// Atomically publishes a fully validated lexical and vector pair as the active content tuple.
  ///
  /// The prior active release is retained as the direct rollback predecessor. This method never
  /// selects a partial lexical/vector pair.
  ///
  /// # Errors
  ///
  /// Returns an error when validation is incomplete, source safety blocks serving, or persistence
  /// cannot atomically update the release lifecycle and active pointer.
  async fn publish(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError>;

  /// Atomically restores the active release's direct retained rollback predecessor.
  ///
  /// # Errors
  ///
  /// Returns an error when no active release exists, the target is not its direct predecessor,
  /// the target is unsafe after source quarantine, or the atomic transition fails.
  async fn rollback(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError>;

  /// Returns the currently selected compatible content pair only when it is safe to serve.
  ///
  /// `Ok(None)` means no release has been published. A quarantined source in the active release
  /// returns [`ContentReleaseRepositoryError::ActiveContentUnsafe`] rather than silently serving
  /// stale or forbidden material.
  ///
  /// # Errors
  ///
  /// Returns an error when source safety or repository integrity prevents a safe response.
  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ContentReleaseRepositoryError>;

  /// Permanently quarantines a source and reports every release metadata record it affects.
  ///
  /// Quarantine blocks all future publication and rollback of affected releases immediately. It
  /// does not claim to erase canonical rows, vectors, caches, or generated artifacts; those
  /// physical cleanup actions remain explicit production workflow steps before retention expires.
  ///
  /// # Errors
  ///
  /// Returns an error when no release tracks the source or the repository cannot persist the
  /// safety state.
  async fn quarantine_source(
    &self,
    source_id: &SourceId,
  ) -> Result<SourceRemovalImpact, ContentReleaseRepositoryError>;
}
