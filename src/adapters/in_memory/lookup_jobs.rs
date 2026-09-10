//! Deterministic in-memory lookup-job lifecycle adapter.

use std::{
  collections::BTreeMap,
  sync::{Arc, Mutex},
  time::Duration,
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  ports::{
    clock::{Clock, UtcTimestamp},
    lookup_job::{
      LookupJobAccess, LookupJobCreation, LookupJobFailure, LookupJobPendingState, LookupJobPoll,
      LookupJobResult, LookupJobStore, LookupJobStoreError,
    },
    public_id::PublicId,
  },
};

/// Deterministic delay supplied to pending lookup-job polls by default.
pub const DEFAULT_LOOKUP_JOB_RETRY_AFTER: Duration = Duration::from_secs(1);

/// Process-local lookup-job state adapter for tests and local development.
///
/// Clones and [`InMemoryLookupJobStore::reopen`] share the same in-memory records, which permits
/// restart-style tests without waiting. This is not durable storage and does not encrypt data.
#[derive(Clone)]
pub struct InMemoryLookupJobStore {
  clock: Arc<dyn Clock>,
  retry_after: Duration,
  jobs: Arc<Mutex<BTreeMap<PublicId, StoredLookupJob>>>,
}

impl InMemoryLookupJobStore {
  /// Creates an empty process-local lookup-job store with a deterministic polling delay.
  pub fn new(clock: Arc<dyn Clock>) -> Self {
    Self {
      clock,
      retry_after: DEFAULT_LOOKUP_JOB_RETRY_AFTER,
      jobs: Arc::new(Mutex::new(BTreeMap::new())),
    }
  }

  /// Sets the nonzero minimum delay returned while a job is queued or running.
  ///
  /// A zero duration is normalized to the one-second default so test callers cannot accidentally
  /// model a busy-polling public contract.
  pub fn with_retry_after(mut self, retry_after: Duration) -> Self {
    self.retry_after = if retry_after.is_zero() {
      DEFAULT_LOOKUP_JOB_RETRY_AFTER
    } else {
      retry_after
    };
    self
  }

  /// Reopens another handle over the same process-local records for restart-style tests.
  ///
  /// The shared state is intentional for deterministic tests only; it does not simulate a process
  /// crash, persisted database record, token hashing, or encryption.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[async_trait]
impl LookupJobStore for InMemoryLookupJobStore {
  async fn create(
    &self,
    id: PublicId,
    creation: LookupJobCreation,
  ) -> Result<(), LookupJobStoreError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_jobs(&mut jobs, now);
    if jobs.contains_key(&id) {
      return Err(LookupJobStoreError::AlreadyExists);
    }

    jobs.insert(id, StoredLookupJob::new(creation));
    Ok(())
  }

  async fn start(&self, id: &PublicId) -> Result<(), LookupJobStoreError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_jobs(&mut jobs, now);
    let Some(job) = jobs.get_mut(id) else {
      return Err(LookupJobStoreError::NotFound);
    };
    if job.status != StoredLookupJobStatus::Queued {
      return Err(LookupJobStoreError::InvalidTransition);
    }

    job.status = StoredLookupJobStatus::Running;
    Ok(())
  }

  async fn complete(
    &self,
    id: &PublicId,
    result: LookupJobResult,
  ) -> Result<(), LookupJobStoreError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_jobs(&mut jobs, now);
    let Some(job) = jobs.get_mut(id) else {
      return Err(LookupJobStoreError::NotFound);
    };
    if job.status != StoredLookupJobStatus::Running {
      return Err(LookupJobStoreError::InvalidTransition);
    }

    job.status = StoredLookupJobStatus::Completed(result);
    Ok(())
  }

  async fn fail(
    &self,
    id: &PublicId,
    failure: LookupJobFailure,
  ) -> Result<(), LookupJobStoreError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_jobs(&mut jobs, now);
    let Some(job) = jobs.get_mut(id) else {
      return Err(LookupJobStoreError::NotFound);
    };
    if job.status != StoredLookupJobStatus::Running {
      return Err(LookupJobStoreError::InvalidTransition);
    }

    job.status = StoredLookupJobStatus::Failed(failure);
    Ok(())
  }

  async fn poll(
    &self,
    id: &PublicId,
    access: &LookupJobAccess,
  ) -> Result<Option<LookupJobPoll>, LookupJobStoreError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    let Some(job) = jobs.get_mut(id) else {
      return Ok(None);
    };
    if &job.access != access {
      return Ok(None);
    }

    job.expire_if_needed(now);
    Ok(Some(job.poll(self.retry_after)))
  }
}

struct StoredLookupJob {
  access: LookupJobAccess,
  expires_at: UtcTimestamp,
  status: StoredLookupJobStatus,
}

impl StoredLookupJob {
  fn new(creation: LookupJobCreation) -> Self {
    Self {
      access: creation.access().clone(),
      expires_at: creation.expires_at(),
      status: StoredLookupJobStatus::Queued,
    }
  }

  fn expire_if_needed(&mut self, now: UtcTimestamp) {
    if self.expires_at <= now {
      self.status = StoredLookupJobStatus::Expired;
    }
  }

  fn poll(&self, retry_after: Duration) -> LookupJobPoll {
    match &self.status {
      StoredLookupJobStatus::Queued => LookupJobPoll::Pending {
        state: LookupJobPendingState::Queued,
        retry_after,
      },
      StoredLookupJobStatus::Running => LookupJobPoll::Pending {
        state: LookupJobPendingState::Running,
        retry_after,
      },
      StoredLookupJobStatus::Completed(result) => LookupJobPoll::Completed(result.clone()),
      StoredLookupJobStatus::Failed(failure) => LookupJobPoll::Failed(failure.clone()),
      StoredLookupJobStatus::Expired => LookupJobPoll::Expired,
    }
  }
}

fn expire_jobs(jobs: &mut BTreeMap<PublicId, StoredLookupJob>, now: UtcTimestamp) {
  for job in jobs.values_mut() {
    job.expire_if_needed(now);
  }
}

#[derive(PartialEq)]
enum StoredLookupJobStatus {
  Queued,
  Running,
  Completed(LookupJobResult),
  Failed(LookupJobFailure),
  Expired,
}

#[cfg(test)]
mod tests {
  use std::{
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::clock::FixedClock,
    ports::{
      durable_job::JobFailureCode,
      lookup_job::{LookupJobCapability, LookupJobOwner},
    },
  };

  fn id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn owner() -> LookupJobAccess {
    LookupJobAccess::owner(LookupJobOwner::new("owner-equality-token").unwrap())
  }

  fn capability() -> LookupJobAccess {
    LookupJobAccess::capability(
      LookupJobCapability::parse("aB_1-23456789012345678901234567890").unwrap(),
    )
  }

  #[tokio::test]
  async fn owner_and_capability_polling_do_not_disclose_other_jobs() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let clock = Arc::new(FixedClock::new(now));
    let store = InMemoryLookupJobStore::new(clock);
    let job_id = id(1);
    store
      .create(
        job_id.clone(),
        LookupJobCreation::new(owner(), now + Duration::from_secs(60)),
      )
      .await
      .unwrap();

    assert!(matches!(
      store.poll(&job_id, &owner()).await.unwrap(),
      Some(LookupJobPoll::Pending {
        state: LookupJobPendingState::Queued,
        retry_after: DEFAULT_LOOKUP_JOB_RETRY_AFTER,
      })
    ));
    assert_eq!(store.poll(&job_id, &capability()).await.unwrap(), None);
    assert_eq!(store.poll(&id(999), &owner()).await.unwrap(), None);
  }

  #[tokio::test]
  async fn poll_returns_result_failure_and_expiry_without_leaking_result_debug_data() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(20);
    let clock = Arc::new(FixedClock::new(now));
    let store = InMemoryLookupJobStore::new(clock.clone());
    let completed_id = id(2);
    let failed_id = id(3);
    store
      .create(
        completed_id.clone(),
        LookupJobCreation::new(capability(), now + Duration::from_secs(10)),
      )
      .await
      .unwrap();
    store.start(&completed_id).await.unwrap();
    store
      .complete(
        &completed_id,
        LookupJobResult::new(serde_json::json!({"query": "private learner query"})),
      )
      .await
      .unwrap();

    store
      .create(
        failed_id.clone(),
        LookupJobCreation::new(owner(), now + Duration::from_secs(5)),
      )
      .await
      .unwrap();
    store.start(&failed_id).await.unwrap();
    store
      .fail(
        &failed_id,
        LookupJobFailure::new(JobFailureCode::new("provider_unavailable").unwrap()),
      )
      .await
      .unwrap();

    let completed = store
      .poll(&completed_id, &capability())
      .await
      .unwrap()
      .unwrap();
    assert!(matches!(completed, LookupJobPoll::Completed(_)));
    assert!(!format!("{completed:?}").contains("private learner query"));
    assert!(matches!(
      store.poll(&failed_id, &owner()).await.unwrap(),
      Some(LookupJobPoll::Failed(failure)) if failure.code().as_str() == "provider_unavailable"
    ));

    clock.advance(Duration::from_secs(10));
    assert!(matches!(
      store.poll(&completed_id, &capability()).await.unwrap(),
      Some(LookupJobPoll::Expired)
    ));
    assert!(matches!(
      store.poll(&failed_id, &owner()).await.unwrap(),
      Some(LookupJobPoll::Expired)
    ));
  }

  #[tokio::test]
  async fn reopened_handle_preserves_queued_state_for_restart_style_tests() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(30);
    let clock = Arc::new(FixedClock::new(now));
    let store = InMemoryLookupJobStore::new(clock);
    let job_id = id(4);
    store
      .create(
        job_id.clone(),
        LookupJobCreation::new(owner(), now + Duration::from_secs(60)),
      )
      .await
      .unwrap();

    let reopened = store.reopen();
    reopened.start(&job_id).await.unwrap();
    assert!(matches!(
      store.poll(&job_id, &owner()).await.unwrap(),
      Some(LookupJobPoll::Pending {
        state: LookupJobPendingState::Running,
        ..
      })
    ));
  }
}
