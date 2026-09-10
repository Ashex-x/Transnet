//! One-claim durable-worker application loop without scheduling or runtime ownership.
//!
//! [`crate::application::durable_worker::DurableWorker::run_once`] claims at most one eligible job,
//! dispatches it to a handler for its stable [`crate::ports::durable_job::JobKind`], and records
//! completion or the handler's redacted [`crate::ports::durable_job::JobFailure`]. It deliberately
//! does not start a process, poll in a loop, apply backoff, run concurrent work, or heartbeat a
//! lease. The caller owns wakeup, backoff, concurrency, deployment, and lease policy; each handler
//! remains responsible for effective idempotency and for completing within its lease. The worker
//! itself never logs opaque job payloads.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use thiserror::Error;

use crate::ports::durable_job::{
  ClaimedJob, DurableJobQueue, JobFailure, JobFailureCode, JobFailureOutcome, JobKind,
  JobQueueError, WorkerId,
};

/// Stable redacted failure category used when no handler is registered for a claimed job kind.
pub const UNSUPPORTED_JOB_KIND_FAILURE_CODE: &str = "unsupported_job_kind";

/// Executes one family of opaque durable jobs.
///
/// Implementations receive a current lease and must treat [`ClaimedJob::payload`] as confidential.
/// They return only [`JobFailure`] values, whose categories are suitable for durable retry and
/// operations records; they must not include source text, credentials, or other raw payload data.
#[async_trait]
pub trait DurableJobHandler: Send + Sync {
  /// Returns the stable kind this handler is registered to execute.
  fn kind(&self) -> &JobKind;

  /// Processes a current claimed job exactly once effectively for its business operation.
  ///
  /// Returning `Ok(())` makes the worker complete the queue record. Returning a retryable or
  /// permanent [`JobFailure`] makes the worker pass that redacted result to the queue.
  ///
  /// # Errors
  ///
  /// Returns a redacted retryable or permanent failure that the durable queue can record.
  async fn handle(&self, job: &ClaimedJob) -> Result<(), JobFailure>;
}

/// Observable result of one [`DurableWorker::run_once`] call without any opaque payload data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableWorkerRunOutcome {
  /// No eligible job was available when the worker attempted its one claim.
  Idle,
  /// A matching handler succeeded and the worker completed the claimed queue record.
  Completed {
    /// Stable kind of the completed job.
    kind: JobKind,
  },
  /// The queue recorded a redacted handler or dispatch failure for the claimed job.
  Failed {
    /// Stable kind of the job whose failure was recorded.
    kind: JobKind,
    /// Queue-selected retry or dead-letter outcome.
    outcome: JobFailureOutcome,
  },
}

/// Construction failure for a reusable [`DurableWorker`] application loop.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DurableWorkerConfigurationError {
  /// A one-claim worker needs a positive lease duration before it can safely claim work.
  #[error("durable worker lease duration must be positive")]
  InvalidLeaseDuration,
  /// More than one handler attempted to own the same stable job kind.
  #[error("a durable job handler is already registered for `{0}`")]
  DuplicateHandler(JobKind),
  /// A built-in redacted failure category no longer meets the durable-job port's validation rules.
  #[error("durable worker built-in failure category is invalid")]
  InvalidBuiltInFailureCode,
}

/// Failure from queue interaction while executing one [`DurableWorker::run_once`] call.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DurableWorkerRunError {
  /// The queue could not atomically claim one eligible job.
  #[error("could not claim durable work: {0}")]
  Claim(JobQueueError),
  /// The handler succeeded but its lease could not be completed at the queue.
  #[error("could not complete durable work: {0}")]
  Complete(JobQueueError),
  /// The handler or dispatcher failed but the queue could not record that redacted failure.
  #[error("could not record durable work failure: {0}")]
  Fail(JobQueueError),
}

/// Reusable application service that claims and handles at most one durable job per call.
///
/// This is not a worker binary or scheduling loop. Its caller must choose invocation cadence,
/// retry backoff, concurrency, deployment, observability, and any heartbeat policy. Handlers must
/// be idempotent because a completed side effect can still be retried if a queue completion loses
/// its lease or fails.
#[derive(Clone)]
pub struct DurableWorker {
  queue: Arc<dyn DurableJobQueue>,
  worker: WorkerId,
  lease_duration: Duration,
  handlers: BTreeMap<JobKind, Arc<dyn DurableJobHandler>>,
  unsupported_job_kind: JobFailureCode,
}

impl DurableWorker {
  /// Creates a one-claim worker with exactly one handler per stable job kind.
  ///
  /// # Errors
  ///
  /// Returns an error when the lease duration is zero, a handler kind is registered more than
  /// once, or the worker's fixed redacted failure category is invalid.
  pub fn new(
    queue: Arc<dyn DurableJobQueue>,
    worker: WorkerId,
    lease_duration: Duration,
    handlers: impl IntoIterator<Item = Arc<dyn DurableJobHandler>>,
  ) -> Result<Self, DurableWorkerConfigurationError> {
    if lease_duration.is_zero() {
      return Err(DurableWorkerConfigurationError::InvalidLeaseDuration);
    }

    let mut registered = BTreeMap::new();
    for handler in handlers {
      let kind = handler.kind().clone();
      if registered.insert(kind.clone(), handler).is_some() {
        return Err(DurableWorkerConfigurationError::DuplicateHandler(kind));
      }
    }

    let unsupported_job_kind = JobFailureCode::new(UNSUPPORTED_JOB_KIND_FAILURE_CODE)
      .map_err(|_| DurableWorkerConfigurationError::InvalidBuiltInFailureCode)?;
    Ok(Self {
      queue,
      worker,
      lease_duration,
      handlers: registered,
      unsupported_job_kind,
    })
  }

  /// Claims and processes at most one currently eligible durable job.
  ///
  /// An unregistered job kind is terminally failed with
  /// [`UNSUPPORTED_JOB_KIND_FAILURE_CODE`]. The method does not heartbeat: handlers must finish
  /// inside `lease_duration`, or callers must use a separate lease-renewal orchestration policy.
  ///
  /// # Errors
  ///
  /// Returns an error when a claim, completion, or failure-recording queue operation cannot be
  /// performed. A handler's redacted [`JobFailure`] is recorded in the returned outcome instead.
  pub async fn run_once(&self) -> Result<DurableWorkerRunOutcome, DurableWorkerRunError> {
    let Some(job) = self
      .queue
      .claim(&self.worker, self.lease_duration)
      .await
      .map_err(DurableWorkerRunError::Claim)?
    else {
      return Ok(DurableWorkerRunOutcome::Idle);
    };
    let kind = job.kind().clone();

    let result = match self.handlers.get(&kind) {
      Some(handler) => handler.handle(&job).await,
      None => Err(JobFailure::Permanent {
        code: self.unsupported_job_kind.clone(),
      }),
    };
    match result {
      Ok(()) => {
        self
          .queue
          .complete(&job)
          .await
          .map_err(DurableWorkerRunError::Complete)?;
        Ok(DurableWorkerRunOutcome::Completed { kind })
      }
      Err(failure) => {
        let outcome = self
          .queue
          .fail(&job, failure)
          .await
          .map_err(DurableWorkerRunError::Fail)?;
        Ok(DurableWorkerRunOutcome::Failed { kind, outcome })
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
  };

  use async_trait::async_trait;
  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock, in_memory::InMemoryDurableJobQueue, public_id::SequencePublicIdGenerator,
    },
    ports::{
      clock::UtcTimestamp,
      durable_job::{JobSnapshot, JobStatus, JobSubmission},
      public_id::{PublicId, PublicIdGenerator},
    },
  };

  const LEASE_DURATION: Duration = Duration::from_secs(5);

  #[derive(Clone)]
  struct RecordingHandler {
    kind: JobKind,
    result: Result<(), JobFailure>,
    calls: Arc<Mutex<Vec<HandlerCall>>>,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  struct HandlerCall {
    kind: JobKind,
    payload_version: u16,
    attempt: u32,
    debug: String,
  }

  impl RecordingHandler {
    fn new(
      kind: &str,
      result: Result<(), JobFailure>,
      calls: Arc<Mutex<Vec<HandlerCall>>>,
    ) -> Self {
      Self {
        kind: JobKind::new(kind).unwrap(),
        result,
        calls,
      }
    }
  }

  #[async_trait]
  impl DurableJobHandler for RecordingHandler {
    fn kind(&self) -> &JobKind {
      &self.kind
    }

    async fn handle(&self, job: &ClaimedJob) -> Result<(), JobFailure> {
      self.calls.lock().unwrap().push(HandlerCall {
        kind: job.kind().clone(),
        payload_version: job.payload_version(),
        attempt: job.attempt(),
        debug: format!("{job:?}"),
      });
      self.result.clone()
    }
  }

  #[derive(Clone)]
  struct LeaseExpiringHandler {
    kind: JobKind,
    clock: Arc<FixedClock>,
    result: Result<(), JobFailure>,
  }

  #[async_trait]
  impl DurableJobHandler for LeaseExpiringHandler {
    fn kind(&self) -> &JobKind {
      &self.kind
    }

    async fn handle(&self, _job: &ClaimedJob) -> Result<(), JobFailure> {
      let _ = self.clock.advance(LEASE_DURATION);
      self.result.clone()
    }
  }

  struct UnavailableQueue;

  #[async_trait]
  impl DurableJobQueue for UnavailableQueue {
    async fn enqueue(&self, _submission: JobSubmission) -> Result<PublicId, JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn get(&self, _id: &PublicId) -> Result<Option<JobSnapshot>, JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn claim(
      &self,
      _worker: &WorkerId,
      _lease_duration: Duration,
    ) -> Result<Option<ClaimedJob>, JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn heartbeat(
      &self,
      _job: &ClaimedJob,
      _lease_duration: Duration,
    ) -> Result<UtcTimestamp, JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn complete(&self, _job: &ClaimedJob) -> Result<(), JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn fail(
      &self,
      _job: &ClaimedJob,
      _failure: JobFailure,
    ) -> Result<JobFailureOutcome, JobQueueError> {
      Err(JobQueueError::Unavailable)
    }

    async fn replay(&self, _id: &PublicId) -> Result<(), JobQueueError> {
      Err(JobQueueError::Unavailable)
    }
  }

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn queue(
    clock: Arc<FixedClock>,
    ids: impl IntoIterator<Item = PublicId>,
  ) -> InMemoryDurableJobQueue {
    let ids: Arc<dyn PublicIdGenerator> = Arc::new(SequencePublicIdGenerator::new(ids));
    InMemoryDurableJobQueue::new(clock, ids)
  }

  fn submission(
    kind: &str,
    payload: &[u8],
    payload_version: u16,
    available_at: UtcTimestamp,
    max_attempts: u32,
  ) -> JobSubmission {
    JobSubmission::new(
      JobKind::new(kind).unwrap(),
      payload.to_vec(),
      payload_version,
      available_at,
      NonZeroU32::new(max_attempts).unwrap(),
    )
  }

  fn worker(
    queue: Arc<dyn DurableJobQueue>,
    handlers: impl IntoIterator<Item = Arc<dyn DurableJobHandler>>,
  ) -> DurableWorker {
    DurableWorker::new(
      queue,
      WorkerId::new("test-worker").unwrap(),
      LEASE_DURATION,
      handlers,
    )
    .unwrap()
  }

  #[tokio::test]
  async fn returns_idle_when_no_eligible_job_exists() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = Arc::new(queue(clock, []));

    assert_eq!(
      worker(queue, []).run_once().await,
      Ok(DurableWorkerRunOutcome::Idle)
    );
  }

  #[tokio::test]
  async fn completes_successful_handler_and_never_exposes_payload_in_diagnostics() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = Arc::new(queue(clock.clone(), [public_id(1), public_id(2)]));
    let id = queue
      .enqueue(submission(
        "lookup.generate",
        b"raw-secret-payload",
        7,
        now,
        1,
      ))
      .await
      .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let handler: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "lookup.generate",
      Ok(()),
      calls.clone(),
    ));

    let outcome = worker(queue.clone(), [handler]).run_once().await.unwrap();

    assert_eq!(
      outcome,
      DurableWorkerRunOutcome::Completed {
        kind: JobKind::new("lookup.generate").unwrap(),
      }
    );
    assert_eq!(
      queue.get(&id).await.unwrap().unwrap().status(),
      JobStatus::Completed
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].payload_version, 7);
    assert!(calls[0].debug.contains("payload_bytes"));
    assert!(!calls[0].debug.contains("raw-secret-payload"));
    assert!(!format!("{outcome:?}").contains("raw-secret-payload"));
  }

  #[tokio::test]
  async fn dispatches_matching_kind_and_preserves_each_payload_version_for_handlers() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = Arc::new(queue(
      clock.clone(),
      [public_id(1), public_id(2), public_id(3), public_id(4)],
    ));
    queue
      .enqueue(submission("lookup.generate", b"one", 1, now, 1))
      .await
      .unwrap();
    queue
      .enqueue(submission("practice.grade", b"two", 9, now, 1))
      .await
      .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let lookup: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "lookup.generate",
      Ok(()),
      calls.clone(),
    ));
    let practice: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "practice.grade",
      Ok(()),
      calls.clone(),
    ));
    let worker = worker(queue, [lookup, practice]);

    assert!(matches!(
      worker.run_once().await,
      Ok(DurableWorkerRunOutcome::Completed { .. })
    ));
    assert!(matches!(
      worker.run_once().await,
      Ok(DurableWorkerRunOutcome::Completed { .. })
    ));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].kind.as_str(), "lookup.generate");
    assert_eq!(calls[0].payload_version, 1);
    assert_eq!(calls[1].kind.as_str(), "practice.grade");
    assert_eq!(calls[1].payload_version, 9);
  }

  #[tokio::test]
  async fn records_retryable_and_permanent_handler_failures() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = Arc::new(queue(
      clock.clone(),
      [public_id(1), public_id(2), public_id(3), public_id(4)],
    ));
    let retry_id = queue
      .enqueue(submission("lookup.generate", b"opaque", 1, now, 2))
      .await
      .unwrap();
    let dead_id = queue
      .enqueue(submission("practice.grade", b"opaque", 1, now, 1))
      .await
      .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let retry: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "lookup.generate",
      Err(JobFailure::Retryable {
        code: JobFailureCode::new("provider_unavailable").unwrap(),
        available_at: now + Duration::from_secs(30),
      }),
      calls.clone(),
    ));
    let permanent: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "practice.grade",
      Err(JobFailure::Permanent {
        code: JobFailureCode::new("invalid_payload_version").unwrap(),
      }),
      calls,
    ));
    let worker = worker(queue.clone(), [retry, permanent]);

    assert_eq!(
      worker.run_once().await,
      Ok(DurableWorkerRunOutcome::Failed {
        kind: JobKind::new("lookup.generate").unwrap(),
        outcome: JobFailureOutcome::RetryScheduled,
      })
    );
    assert_eq!(
      queue.get(&retry_id).await.unwrap().unwrap().status(),
      JobStatus::Queued
    );
    assert_eq!(
      worker.run_once().await,
      Ok(DurableWorkerRunOutcome::Failed {
        kind: JobKind::new("practice.grade").unwrap(),
        outcome: JobFailureOutcome::Dead,
      })
    );
    let dead = queue.get(&dead_id).await.unwrap().unwrap();
    assert_eq!(dead.status(), JobStatus::Dead);
    assert_eq!(
      dead.failure_code().map(JobFailureCode::as_str),
      Some("invalid_payload_version")
    );
  }

  #[tokio::test]
  async fn dead_letters_unregistered_job_kinds_with_a_stable_redacted_category() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000);
    let clock = Arc::new(FixedClock::new(now));
    let queue = Arc::new(queue(clock.clone(), [public_id(1), public_id(2)]));
    let id = queue
      .enqueue(submission("unknown.family", b"opaque", 3, now, 1))
      .await
      .unwrap();

    assert_eq!(
      worker(queue.clone(), []).run_once().await,
      Ok(DurableWorkerRunOutcome::Failed {
        kind: JobKind::new("unknown.family").unwrap(),
        outcome: JobFailureOutcome::Dead,
      })
    );
    let snapshot = queue.get(&id).await.unwrap().unwrap();
    assert_eq!(snapshot.status(), JobStatus::Dead);
    assert_eq!(
      snapshot.failure_code().map(JobFailureCode::as_str),
      Some(UNSUPPORTED_JOB_KIND_FAILURE_CODE)
    );
  }

  #[tokio::test]
  async fn returns_queue_failures_without_invoking_handlers() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let handler: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "lookup.generate",
      Ok(()),
      calls.clone(),
    ));

    assert_eq!(
      worker(Arc::new(UnavailableQueue), [handler])
        .run_once()
        .await,
      Err(DurableWorkerRunError::Claim(JobQueueError::Unavailable))
    );
    assert!(calls.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn reports_lease_loss_when_completion_or_failure_cannot_be_recorded() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(5_000);
    let clock = Arc::new(FixedClock::new(now));
    let completion_queue = Arc::new(queue(clock.clone(), [public_id(1), public_id(2)]));
    completion_queue
      .enqueue(submission("lookup.generate", b"opaque", 1, now, 2))
      .await
      .unwrap();
    let completion_handler: Arc<dyn DurableJobHandler> = Arc::new(LeaseExpiringHandler {
      kind: JobKind::new("lookup.generate").unwrap(),
      clock: clock.clone(),
      result: Ok(()),
    });

    assert_eq!(
      worker(completion_queue, [completion_handler])
        .run_once()
        .await,
      Err(DurableWorkerRunError::Complete(JobQueueError::LeaseLost))
    );

    let retry_clock = Arc::new(FixedClock::new(now));
    let failure_queue = Arc::new(queue(retry_clock.clone(), [public_id(3), public_id(4)]));
    failure_queue
      .enqueue(submission("practice.grade", b"opaque", 1, now, 2))
      .await
      .unwrap();
    let failure_handler: Arc<dyn DurableJobHandler> = Arc::new(LeaseExpiringHandler {
      kind: JobKind::new("practice.grade").unwrap(),
      clock: retry_clock,
      result: Err(JobFailure::Permanent {
        code: JobFailureCode::new("invalid_input").unwrap(),
      }),
    });

    assert_eq!(
      worker(failure_queue, [failure_handler]).run_once().await,
      Err(DurableWorkerRunError::Fail(JobQueueError::LeaseLost))
    );
  }

  #[test]
  fn rejects_zero_lease_and_duplicate_handler_kinds() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let first: Arc<dyn DurableJobHandler> = Arc::new(RecordingHandler::new(
      "lookup.generate",
      Ok(()),
      calls.clone(),
    ));
    let second: Arc<dyn DurableJobHandler> =
      Arc::new(RecordingHandler::new("lookup.generate", Ok(()), calls));
    let queue: Arc<dyn DurableJobQueue> = Arc::new(UnavailableQueue);
    let worker_id = WorkerId::new("test-worker").unwrap();

    assert!(matches!(
      DurableWorker::new(queue.clone(), worker_id.clone(), Duration::ZERO, []),
      Err(DurableWorkerConfigurationError::InvalidLeaseDuration)
    ));
    assert!(matches!(
      DurableWorker::new(queue, worker_id, LEASE_DURATION, [first, second]),
      Err(DurableWorkerConfigurationError::DuplicateHandler(kind))
        if kind == JobKind::new("lookup.generate").unwrap()
    ));
  }
}
