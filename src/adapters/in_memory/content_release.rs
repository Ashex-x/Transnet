//! Deterministic in-memory content-release repository for tests and local development.

use std::{
  collections::{BTreeMap, BTreeSet},
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  domain::{
    canonical::{ActiveContentVersion, ReleaseId, ReleaseStatus, SourceId},
    content_release::{
      ContentRelease, ContentReleaseDraft, ContentReleaseGate, ContentReleaseGateResult,
      SourceRemovalImpact, VectorCollectionBuild,
    },
  },
  ports::content_release_repository::{ContentReleaseRepository, ContentReleaseRepositoryError},
};

/// Process-local content-release repository with atomic in-memory pointer transitions.
///
/// Clones and [`InMemoryContentReleaseRepository::reopen`] share release metadata so tests can
/// exercise restart-style handoffs. This adapter is not durable, does not perform source-artifact
/// deletion, and cannot substitute for a transactional production release coordinator.
#[derive(Clone)]
pub struct InMemoryContentReleaseRepository {
  state: Arc<Mutex<ContentReleaseState>>,
}

impl InMemoryContentReleaseRepository {
  /// Creates an empty content-release repository without an active content pointer.
  pub fn new() -> Self {
    Self {
      state: Arc::new(Mutex::new(ContentReleaseState::default())),
    }
  }

  /// Reopens another handle over the same process-local release records for deterministic tests.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

impl Default for InMemoryContentReleaseRepository {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl ContentReleaseRepository for InMemoryContentReleaseRepository {
  async fn stage(
    &self,
    draft: ContentReleaseDraft,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    let release_id = draft.release_id().clone();
    let mut state = mutex_lock(&self.state);
    if state.releases.contains_key(&release_id) {
      return Err(ContentReleaseRepositoryError::AlreadyExists);
    }
    if draft
      .source_ids()
      .iter()
      .any(|source_id| state.quarantined_sources.contains(source_id))
    {
      return Err(ContentReleaseRepositoryError::SourceQuarantined);
    }

    let release = draft.into_release();
    state.releases.insert(release_id, release.clone());
    Ok(release)
  }

  async fn release(
    &self,
    release_id: &ReleaseId,
  ) -> Result<Option<ContentRelease>, ContentReleaseRepositoryError> {
    Ok(mutex_lock(&self.state).releases.get(release_id).cloned())
  }

  async fn record_gate_result(
    &self,
    release_id: &ReleaseId,
    gate: ContentReleaseGate,
    result: ContentReleaseGateResult,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let release = state
      .releases
      .get_mut(release_id)
      .ok_or(ContentReleaseRepositoryError::NotFound)?;
    release.record_gate_result(gate, result)?;
    Ok(release.clone())
  }

  async fn begin_vector_build(
    &self,
    release_id: &ReleaseId,
    build: VectorCollectionBuild,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let release = state
      .releases
      .get_mut(release_id)
      .ok_or(ContentReleaseRepositoryError::NotFound)?;
    release.begin_vector_build(build)?;
    Ok(release.clone())
  }

  async fn reconcile_vector_build(
    &self,
    release_id: &ReleaseId,
    indexed_records: u64,
  ) -> Result<ContentRelease, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let release = state
      .releases
      .get_mut(release_id)
      .ok_or(ContentReleaseRepositoryError::NotFound)?;
    release.reconcile_vector_build(indexed_records)?;
    Ok(release.clone())
  }

  async fn publish(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let candidate = state
      .releases
      .get(release_id)
      .ok_or(ContentReleaseRepositoryError::NotFound)?;
    ensure_sources_are_safe(candidate, &state.quarantined_sources)?;
    candidate.ensure_publishable()?;

    let previous_active_id = state.active_release_id.clone();
    if let Some(previous_active_id) = previous_active_id.as_ref() {
      let previous_release = state
        .releases
        .get_mut(previous_active_id)
        .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
      previous_release.retire()?;
    }

    let candidate = state
      .releases
      .get_mut(release_id)
      .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
    let active_content = candidate.mark_published(previous_active_id)?;
    state.active_release_id = Some(release_id.clone());
    Ok(active_content)
  }

  async fn rollback(
    &self,
    release_id: &ReleaseId,
  ) -> Result<ActiveContentVersion, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let active_release_id = state
      .active_release_id
      .clone()
      .ok_or(ContentReleaseRepositoryError::InvalidRollbackTarget)?;
    let active_release = state
      .releases
      .get(&active_release_id)
      .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
    if active_release.release().status != ReleaseStatus::Published {
      return Err(ContentReleaseRepositoryError::InconsistentState);
    }
    if active_release.release().rollback_predecessor.as_ref() != Some(release_id) {
      return Err(ContentReleaseRepositoryError::InvalidRollbackTarget);
    }

    let rollback_target = state
      .releases
      .get(release_id)
      .ok_or(ContentReleaseRepositoryError::NotFound)?;
    ensure_sources_are_safe(rollback_target, &state.quarantined_sources)?;
    if rollback_target.release().status != ReleaseStatus::Retired {
      return Err(ContentReleaseRepositoryError::InconsistentState);
    }

    let active_release = state
      .releases
      .get_mut(&active_release_id)
      .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
    active_release.retire()?;
    let rollback_target = state
      .releases
      .get_mut(release_id)
      .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
    let active_content = rollback_target.restore_for_rollback()?;
    state.active_release_id = Some(release_id.clone());
    Ok(active_content)
  }

  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ContentReleaseRepositoryError> {
    let state = mutex_lock(&self.state);
    let Some(active_release_id) = state.active_release_id.as_ref() else {
      return Ok(None);
    };
    let release = state
      .releases
      .get(active_release_id)
      .ok_or(ContentReleaseRepositoryError::InconsistentState)?;
    if release.release().status != ReleaseStatus::Published {
      return Err(ContentReleaseRepositoryError::InconsistentState);
    }
    if release
      .source_ids()
      .iter()
      .any(|source_id| state.quarantined_sources.contains(source_id))
    {
      return Err(ContentReleaseRepositoryError::ActiveContentUnsafe);
    }

    Ok(Some(release.active_content_version().clone()))
  }

  async fn quarantine_source(
    &self,
    source_id: &SourceId,
  ) -> Result<SourceRemovalImpact, ContentReleaseRepositoryError> {
    let mut state = mutex_lock(&self.state);
    let affected_release_ids = state
      .releases
      .values()
      .filter(|release| release.uses_source(source_id))
      .map(|release| release.id().clone())
      .collect::<BTreeSet<_>>();
    if affected_release_ids.is_empty() {
      return Err(ContentReleaseRepositoryError::SourceNotTracked);
    }

    let active_release_id = state
      .active_release_id
      .as_ref()
      .filter(|release_id| affected_release_ids.contains(*release_id))
      .cloned();
    let staging_release_ids = affected_release_ids
      .iter()
      .filter(|release_id| {
        state
          .releases
          .get(*release_id)
          .is_some_and(|release| release.release().status == ReleaseStatus::Staging)
      })
      .cloned()
      .collect();
    let retained_release_ids = affected_release_ids
      .iter()
      .filter(|release_id| {
        state
          .releases
          .get(*release_id)
          .is_some_and(|release| release.release().status == ReleaseStatus::Retired)
      })
      .cloned()
      .collect();

    state.quarantined_sources.insert(source_id.clone());
    Ok(SourceRemovalImpact::new(
      source_id.clone(),
      active_release_id,
      staging_release_ids,
      retained_release_ids,
    ))
  }
}

fn ensure_sources_are_safe(
  release: &ContentRelease,
  quarantined_sources: &BTreeSet<SourceId>,
) -> Result<(), ContentReleaseRepositoryError> {
  if release
    .source_ids()
    .iter()
    .any(|source_id| quarantined_sources.contains(source_id))
  {
    return Err(ContentReleaseRepositoryError::SourceQuarantined);
  }
  Ok(())
}

#[derive(Default)]
struct ContentReleaseState {
  releases: BTreeMap<ReleaseId, ContentRelease>,
  active_release_id: Option<ReleaseId>,
  quarantined_sources: BTreeSet<SourceId>,
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use crate::{
    application::content_release::ContentReleaseService,
    domain::{
      canonical::CanonicalId,
      content_release::{
        ContentReleaseGatePolicy, ContentReleaseValidationError, VectorCollectionBuildState,
      },
    },
    ports::content_release_repository::ContentReleaseRepository,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn content(release_id: &str, vector_collection_id: &str) -> ActiveContentVersion {
    ActiveContentVersion {
      release_id: id(release_id),
      vector_collection_id: id(vector_collection_id),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "lookup-rank-v1".to_string(),
    }
  }

  fn draft(release_id: &str, vector_collection_id: &str, source_id: &str) -> ContentReleaseDraft {
    ContentReleaseDraft::new(
      id(release_id),
      format!("manifest-{release_id}"),
      content(release_id, vector_collection_id),
      [id(source_id)],
      ContentReleaseGatePolicy::strict(),
    )
    .unwrap()
  }

  async fn prepare_publishable(
    repository: &dyn ContentReleaseRepository,
    release_id: &str,
    vector_collection_id: &str,
    source_id: &str,
  ) {
    let release_id = id(release_id);
    repository
      .stage(draft(release_id.as_str(), vector_collection_id, source_id))
      .await
      .unwrap();
    for gate in ContentReleaseGatePolicy::strict().required_gates().clone() {
      repository
        .record_gate_result(&release_id, gate, ContentReleaseGateResult::Passed)
        .await
        .unwrap();
    }
    repository
      .begin_vector_build(
        &release_id,
        VectorCollectionBuild::new(content(release_id.as_str(), vector_collection_id), 8).unwrap(),
      )
      .await
      .unwrap();
    let release = repository
      .reconcile_vector_build(&release_id, 8)
      .await
      .unwrap();
    assert_eq!(
      release.vector_build().unwrap().state(),
      VectorCollectionBuildState::Ready
    );
  }

  #[tokio::test]
  async fn publishes_only_after_every_gate_and_expected_vector_record_count() {
    let repository = InMemoryContentReleaseRepository::new();
    let release_id = id("release-1");
    repository
      .stage(draft("release-1", "vectors-1", "source-1"))
      .await
      .unwrap();

    assert_eq!(
      repository.publish(&release_id).await,
      Err(ContentReleaseRepositoryError::Validation(
        ContentReleaseValidationError::RequiredGatesPending
      ))
    );
    for gate in ContentReleaseGatePolicy::strict().required_gates().clone() {
      repository
        .record_gate_result(&release_id, gate, ContentReleaseGateResult::Passed)
        .await
        .unwrap();
    }
    repository
      .begin_vector_build(
        &release_id,
        VectorCollectionBuild::new(content("release-1", "vectors-1"), 8).unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(
      repository.reconcile_vector_build(&release_id, 7).await,
      Err(ContentReleaseRepositoryError::Validation(
        ContentReleaseValidationError::VectorReconciliationMismatch {
          expected: 8,
          indexed: 7,
        }
      ))
    );
    assert_eq!(
      repository.publish(&release_id).await,
      Err(ContentReleaseRepositoryError::Validation(
        ContentReleaseValidationError::VectorBuildNotReady
      ))
    );

    repository
      .reconcile_vector_build(&release_id, 8)
      .await
      .unwrap();
    let active = repository.publish(&release_id).await.unwrap();
    assert_eq!(active, content("release-1", "vectors-1"));
    assert_eq!(
      repository.active_content_version().await.unwrap(),
      Some(active)
    );
  }

  #[tokio::test]
  async fn rollback_requires_the_direct_retained_predecessor() {
    let repository = InMemoryContentReleaseRepository::new();
    prepare_publishable(&repository, "release-1", "vectors-1", "source-1").await;
    repository.publish(&id("release-1")).await.unwrap();
    prepare_publishable(&repository, "release-2", "vectors-2", "source-2").await;
    repository.publish(&id("release-2")).await.unwrap();
    prepare_publishable(&repository, "release-3", "vectors-3", "source-3").await;
    repository.publish(&id("release-3")).await.unwrap();

    assert_eq!(
      repository.rollback(&id("release-1")).await,
      Err(ContentReleaseRepositoryError::InvalidRollbackTarget)
    );
    let restored = repository.rollback(&id("release-2")).await.unwrap();
    assert_eq!(restored, content("release-2", "vectors-2"));
    assert_eq!(
      repository
        .release(&id("release-3"))
        .await
        .unwrap()
        .unwrap()
        .release()
        .status,
      ReleaseStatus::Retired
    );
    assert_eq!(
      repository
        .release(&id("release-2"))
        .await
        .unwrap()
        .unwrap()
        .release()
        .status,
      ReleaseStatus::Published
    );
  }

  #[tokio::test]
  async fn source_quarantine_blocks_serving_staging_and_unsafe_rollbacks() {
    let repository = InMemoryContentReleaseRepository::new();
    prepare_publishable(&repository, "release-1", "vectors-1", "source-1").await;
    repository.publish(&id("release-1")).await.unwrap();

    let impact = repository.quarantine_source(&id("source-1")).await.unwrap();
    assert_eq!(impact.active_release_id(), Some(&id("release-1")));
    assert_eq!(
      repository.active_content_version().await,
      Err(ContentReleaseRepositoryError::ActiveContentUnsafe)
    );
    assert_eq!(
      repository
        .stage(draft("release-unsafe", "vectors-unsafe", "source-1"))
        .await,
      Err(ContentReleaseRepositoryError::SourceQuarantined)
    );

    prepare_publishable(&repository, "release-2", "vectors-2", "source-2").await;
    repository.publish(&id("release-2")).await.unwrap();
    assert_eq!(
      repository.rollback(&id("release-1")).await,
      Err(ContentReleaseRepositoryError::SourceQuarantined)
    );
    assert_eq!(
      repository.active_content_version().await.unwrap(),
      Some(content("release-2", "vectors-2"))
    );
  }

  #[tokio::test]
  async fn service_and_reopened_handle_share_atomic_release_state() {
    let repository = InMemoryContentReleaseRepository::new();
    let shared: Arc<dyn ContentReleaseRepository> = Arc::new(repository.clone());
    let service = ContentReleaseService::new(shared);
    let release_id = id("release-1");

    service
      .stage(draft("release-1", "vectors-1", "source-1"))
      .await
      .unwrap();
    for gate in ContentReleaseGatePolicy::strict().required_gates().clone() {
      service
        .record_gate_result(&release_id, gate, ContentReleaseGateResult::Passed)
        .await
        .unwrap();
    }
    service
      .begin_vector_build(
        &release_id,
        VectorCollectionBuild::new(content("release-1", "vectors-1"), 1).unwrap(),
      )
      .await
      .unwrap();
    service
      .reconcile_vector_build(&release_id, 1)
      .await
      .unwrap();
    service.publish(&release_id).await.unwrap();

    assert_eq!(
      repository.reopen().active_content_version().await.unwrap(),
      Some(content("release-1", "vectors-1"))
    );
  }
}
