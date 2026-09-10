//! Deterministic in-memory implementation of leased durable jobs.

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
    durable_job::{
      ClaimedJob, DurableJobQueue, JobFailure, JobFailureCode, JobFailureOutcome, JobKind,
      JobQueueError, JobSnapshot, JobStatus, JobSubmission, WorkerId,
    },
    public_id::{PublicId, PublicIdGenerator},
  },
};

/// Shareable durable queue with atomic in-memory lease, retry, dead-letter, and replay behavior.
///
/// This adapter is process-local. Its shared handles support deterministic restart-style tests but
/// do not persist records across a real process restart.
#[derive(Clone)]
pub struct InMemoryDurableJobQueue {
  clock: Arc<dyn Clock>,
  ids: Arc<dyn PublicIdGenerator>,
  jobs: Arc<Mutex<BTreeMap<PublicId, StoredJob>>>,
}

impl InMemoryDurableJobQueue {
  /// Creates an empty durable-queue test double using injected time and public-ID generators.
  pub fn new(clock: Arc<dyn Clock>, ids: Arc<dyn PublicIdGenerator>) -> Self {
    Self {
      clock,
      ids,
      jobs: Arc::new(Mutex::new(BTreeMap::new())),
    }
  }

  /// Reopens a handle over the same process-local queue records for deterministic tests.
  ///
  /// This does not simulate a durable database or survive an operating-system process restart.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[async_trait]
impl DurableJobQueue for InMemoryDurableJobQueue {
  async fn enqueue(&self, submission: JobSubmission) -> Result<PublicId, JobQueueError> {
    let id = self.ids.generate()?;
    let record = StoredJob::from_submission(id.clone(), submission);
    mutex_lock(&self.jobs).insert(id.clone(), record);
    Ok(id)
  }

  async fn get(&self, id: &PublicId) -> Result<Option<JobSnapshot>, JobQueueError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    Ok(jobs.get(id).map(StoredJob::snapshot))
  }

  async fn claim(
    &self,
    worker: &WorkerId,
    lease_duration: Duration,
  ) -> Result<Option<ClaimedJob>, JobQueueError> {
    let now = self.clock.now();
    let lease_expires_at = lease_expiry(now, lease_duration)?;
    if !self.has_eligible_job(now) {
      return Ok(None);
    }

    let lease_id = self.ids.generate()?;
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    let Some(id) = next_eligible_job_id(&jobs, now) else {
      return Ok(None);
    };
    let Some(record) = jobs.get_mut(&id) else {
      return Err(JobQueueError::NotFound);
    };

    record.attempts += 1;
    record.status = JobStatus::Running;
    record.lease = Some(ActiveLease {
      id: lease_id.clone(),
      worker: worker.clone(),
      expires_at: lease_expires_at,
    });

    Ok(Some(ClaimedJob {
      id: record.id.clone(),
      lease_id,
      worker: worker.clone(),
      kind: record.kind.clone(),
      payload: record.payload.clone(),
      payload_version: record.payload_version,
      attempt: record.attempts,
      lease_expires_at,
    }))
  }

  async fn heartbeat(
    &self,
    job: &ClaimedJob,
    lease_duration: Duration,
  ) -> Result<UtcTimestamp, JobQueueError> {
    let now = self.clock.now();
    let lease_expires_at = lease_expiry(now, lease_duration)?;
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    let Some(record) = jobs.get_mut(job.id()) else {
      return Err(JobQueueError::NotFound);
    };
    if !matches_current_lease(record, job, now) {
      return Err(JobQueueError::LeaseLost);
    }

    if let Some(lease) = record.lease.as_mut() {
      lease.expires_at = lease_expires_at;
    }
    Ok(lease_expires_at)
  }

  async fn complete(&self, job: &ClaimedJob) -> Result<(), JobQueueError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    let Some(record) = jobs.get_mut(job.id()) else {
      return Err(JobQueueError::NotFound);
    };
    if !matches_current_lease(record, job, now) {
      return Err(JobQueueError::LeaseLost);
    }

    record.status = JobStatus::Completed;
    record.lease = None;
    record.failure_code = None;
    Ok(())
  }

  async fn fail(
    &self,
    job: &ClaimedJob,
    failure: JobFailure,
  ) -> Result<JobFailureOutcome, JobQueueError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    let Some(record) = jobs.get_mut(job.id()) else {
      return Err(JobQueueError::NotFound);
    };
    if !matches_current_lease(record, job, now) {
      return Err(JobQueueError::LeaseLost);
    }

    record.lease = None;
    match failure {
      JobFailure::Retryable { code, available_at }
        if record.attempts < record.max_attempts.get() =>
      {
        record.status = JobStatus::Queued;
        record.available_at = available_at;
        record.failure_code = Some(code);
        Ok(JobFailureOutcome::RetryScheduled)
      }
      JobFailure::Retryable { code, .. } | JobFailure::Permanent { code } => {
        record.status = JobStatus::Dead;
        record.failure_code = Some(code);
        Ok(JobFailureOutcome::Dead)
      }
    }
  }

  async fn replay(&self, id: &PublicId) -> Result<(), JobQueueError> {
    let now = self.clock.now();
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    let Some(record) = jobs.get_mut(id) else {
      return Err(JobQueueError::NotFound);
    };
    if record.status != JobStatus::Dead {
      return Err(JobQueueError::NotReplayable);
    }

    record.status = JobStatus::Queued;
    record.attempts = 0;
    record.available_at = now;
    record.failure_code = None;
    record.lease = None;
    Ok(())
  }
}

impl InMemoryDurableJobQueue {
  fn has_eligible_job(&self, now: UtcTimestamp) -> bool {
    let mut jobs = mutex_lock(&self.jobs);
    expire_leases(&mut jobs, now);
    next_eligible_job_id(&jobs, now).is_some()
  }
}

struct StoredJob {
  id: PublicId,
  kind: JobKind,
  payload: Vec<u8>,
  payload_version: u16,
  available_at: UtcTimestamp,
  max_attempts: std::num::NonZeroU32,
  attempts: u32,
  status: JobStatus,
  lease: Option<ActiveLease>,
  failure_code: Option<JobFailureCode>,
}

impl StoredJob {
  fn from_submission(id: PublicId, submission: JobSubmission) -> Self {
    Self {
      id,
      kind: submission.kind().clone(),
      payload: submission.payload().to_vec(),
      payload_version: submission.payload_version(),
      available_at: submission.available_at(),
      max_attempts: submission.max_attempts(),
      attempts: 0,
      status: JobStatus::Queued,
      lease: None,
      failure_code: None,
    }
  }

  fn snapshot(&self) -> JobSnapshot {
    JobSnapshot {
      id: self.id.clone(),
      kind: self.kind.clone(),
      payload_version: self.payload_version,
      status: self.status,
      attempts: self.attempts,
      max_attempts: self.max_attempts,
      available_at: self.available_at,
      lease_expires_at: self.lease.as_ref().map(|lease| lease.expires_at),
      failure_code: self.failure_code.clone(),
    }
  }
}

struct ActiveLease {
  id: PublicId,
  worker: WorkerId,
  expires_at: UtcTimestamp,
}

fn lease_expiry(now: UtcTimestamp, duration: Duration) -> Result<UtcTimestamp, JobQueueError> {
  if duration.is_zero() {
    return Err(JobQueueError::InvalidLeaseDuration);
  }

  now
    .checked_add(duration)
    .ok_or(JobQueueError::InvalidLeaseDuration)
}

fn expire_leases(jobs: &mut BTreeMap<PublicId, StoredJob>, now: UtcTimestamp) {
  for record in jobs.values_mut() {
    let lease_expired = match record.lease.as_ref() {
      Some(lease) => lease.expires_at <= now,
      None => false,
    };
    if record.status != JobStatus::Running || !lease_expired {
      continue;
    }

    record.lease = None;
    if record.attempts >= record.max_attempts.get() {
      record.status = JobStatus::Dead;
    } else {
      record.status = JobStatus::Queued;
      record.available_at = now;
    }
  }
}

fn next_eligible_job_id(
  jobs: &BTreeMap<PublicId, StoredJob>,
  now: UtcTimestamp,
) -> Option<PublicId> {
  jobs
    .iter()
    .filter(|(_, record)| {
      record.status == JobStatus::Queued
        && record.available_at <= now
        && record.attempts < record.max_attempts.get()
    })
    .min_by(|(left_id, left), (right_id, right)| {
      left
        .available_at
        .cmp(&right.available_at)
        .then_with(|| left_id.cmp(right_id))
    })
    .map(|(id, _)| id.clone())
}

fn matches_current_lease(record: &StoredJob, job: &ClaimedJob, now: UtcTimestamp) -> bool {
  record.status == JobStatus::Running
    && record.lease.as_ref().is_some_and(|lease| {
      lease.id == job.lease_id && lease.worker == job.worker && lease.expires_at > now
    })
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{clock::FixedClock, public_id::SequencePublicIdGenerator},
    ports::public_id::PublicIdGenerator,
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn submission(now: UtcTimestamp) -> JobSubmission {
    JobSubmission::new(
      JobKind::new("lookup.generate").unwrap(),
      b"opaque payload".to_vec(),
      1,
      now,
      NonZeroU32::new(2).unwrap(),
    )
  }

  fn queue(clock: Arc<FixedClock>, ids: Vec<PublicId>) -> InMemoryDurableJobQueue {
    let ids: Arc<dyn PublicIdGenerator> = Arc::new(SequencePublicIdGenerator::new(ids));
    InMemoryDurableJobQueue::new(clock, ids)
  }

  #[tokio::test]
  async fn retries_dead_letters_and_replays_jobs() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = queue(
      clock.clone(),
      vec![public_id(1), public_id(2), public_id(3), public_id(4)],
    );
    let worker = WorkerId::new("worker-a").unwrap();
    let job_id = queue.enqueue(submission(now)).await.unwrap();
    let first = queue
      .claim(&worker, Duration::from_secs(30))
      .await
      .unwrap()
      .unwrap();

    assert_eq!(
      queue
        .fail(
          &first,
          JobFailure::Retryable {
            code: JobFailureCode::new("provider_unavailable").unwrap(),
            available_at: now + Duration::from_secs(5),
          },
        )
        .await
        .unwrap(),
      JobFailureOutcome::RetryScheduled
    );
    assert!(queue
      .claim(&worker, Duration::from_secs(30))
      .await
      .unwrap()
      .is_none());

    clock.advance(Duration::from_secs(5));
    let second = queue
      .claim(&worker, Duration::from_secs(30))
      .await
      .unwrap()
      .unwrap();
    assert_eq!(second.attempt(), 2);
    assert_eq!(
      queue
        .fail(
          &second,
          JobFailure::Retryable {
            code: JobFailureCode::new("provider_unavailable").unwrap(),
            available_at: now + Duration::from_secs(10),
          },
        )
        .await
        .unwrap(),
      JobFailureOutcome::Dead
    );
    assert_eq!(
      queue.get(&job_id).await.unwrap().unwrap().status(),
      JobStatus::Dead
    );

    queue.replay(&job_id).await.unwrap();
    let replayed = queue.get(&job_id).await.unwrap().unwrap();
    assert_eq!(replayed.status(), JobStatus::Queued);
    assert_eq!(replayed.attempts(), 0);
    assert!(replayed.failure_code().is_none());
  }

  #[tokio::test]
  async fn expired_leases_are_reclaimed_and_stale_workers_cannot_complete() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = queue(
      clock.clone(),
      vec![public_id(11), public_id(12), public_id(13)],
    );
    let worker = WorkerId::new("worker-a").unwrap();
    let job_id = queue.enqueue(submission(now)).await.unwrap();
    let first = queue
      .claim(&worker, Duration::from_secs(5))
      .await
      .unwrap()
      .unwrap();

    clock.advance(Duration::from_secs(5));
    let second = queue
      .claim(&worker, Duration::from_secs(5))
      .await
      .unwrap()
      .unwrap();

    assert_eq!(second.attempt(), 2);
    assert_eq!(queue.complete(&first).await, Err(JobQueueError::LeaseLost));
    queue.complete(&second).await.unwrap();
    assert_eq!(
      queue.get(&job_id).await.unwrap().unwrap().status(),
      JobStatus::Completed
    );
  }

  #[tokio::test]
  async fn heartbeat_extends_a_current_lease() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = queue(clock.clone(), vec![public_id(21), public_id(22)]);
    let worker = WorkerId::new("worker-a").unwrap();
    queue.enqueue(submission(now)).await.unwrap();
    let claimed = queue
      .claim(&worker, Duration::from_secs(5))
      .await
      .unwrap()
      .unwrap();

    clock.advance(Duration::from_secs(4));
    assert_eq!(
      queue
        .heartbeat(&claimed, Duration::from_secs(5))
        .await
        .unwrap(),
      now + Duration::from_secs(9)
    );
    clock.advance(Duration::from_secs(1));

    assert!(queue
      .claim(&worker, Duration::from_secs(5))
      .await
      .unwrap()
      .is_none());
  }
}
