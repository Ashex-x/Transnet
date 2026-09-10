//! Leased durable-work interface shared by API and worker processes.

use std::{fmt, num::NonZeroU32, time::Duration};

use async_trait::async_trait;
use thiserror::Error;

use crate::ports::{
  clock::UtcTimestamp,
  public_id::{PublicId, PublicIdGenerationError},
};

/// A machine-readable durable-job kind.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobKind(String);

impl JobKind {
  /// Validates a job kind containing lowercase ASCII letters, digits, dots, dashes, or underscores.
  ///
  /// # Errors
  ///
  /// Returns [`JobKindError::InvalidFormat`] when `value` is blank, too long, or contains an
  /// unsupported character.
  pub fn new(value: impl Into<String>) -> Result<Self, JobKindError> {
    let value = value.into();
    if !is_machine_label(&value) {
      return Err(JobKindError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the stable machine-readable job kind.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for JobKind {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.debug_tuple("JobKind").field(&self.0).finish()
  }
}

impl fmt::Display for JobKind {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

/// Validation failure for a durable-job kind.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum JobKindError {
  /// The job kind was not a bounded lowercase machine label.
  #[error("job kind must be a bounded lowercase machine label")]
  InvalidFormat,
}

/// A redacted, machine-readable failure category for a durable job.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobFailureCode(String);

impl JobFailureCode {
  /// Validates a bounded lowercase machine-label failure category.
  ///
  /// # Errors
  ///
  /// Returns [`JobFailureCodeError::InvalidFormat`] when `value` is blank, too long, or contains an
  /// unsupported character.
  pub fn new(value: impl Into<String>) -> Result<Self, JobFailureCodeError> {
    let value = value.into();
    if !is_machine_label(&value) {
      return Err(JobFailureCodeError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the stable failure category without an unredacted error message.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for JobFailureCode {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_tuple("JobFailureCode")
      .field(&self.0)
      .finish()
  }
}

impl fmt::Display for JobFailureCode {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

/// Validation failure for a durable-job failure category.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum JobFailureCodeError {
  /// The failure category was not a bounded lowercase machine label.
  #[error("job failure code must be a bounded lowercase machine label")]
  InvalidFormat,
}

/// Identifies the worker that claimed a durable job.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(String);

impl WorkerId {
  /// Validates a worker identifier containing printable ASCII text.
  ///
  /// # Errors
  ///
  /// Returns [`WorkerIdError::InvalidFormat`] for blank, oversized, or control-character input.
  pub fn new(value: impl Into<String>) -> Result<Self, WorkerIdError> {
    let value = value.into();
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
      return Err(WorkerIdError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the worker identifier for durable-store audit fields.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for WorkerId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.debug_tuple("WorkerId").field(&self.0).finish()
  }
}

/// Validation failure for a worker identifier.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WorkerIdError {
  /// The worker identifier was not bounded printable ASCII.
  #[error("worker ID must be bounded printable ASCII")]
  InvalidFormat,
}

/// One submission to a durable queue.
#[derive(Clone, PartialEq, Eq)]
pub struct JobSubmission {
  kind: JobKind,
  payload: Vec<u8>,
  payload_version: u16,
  available_at: UtcTimestamp,
  max_attempts: NonZeroU32,
}

impl JobSubmission {
  /// Creates a versioned job that becomes eligible at `available_at`.
  ///
  /// The payload is opaque to this port. Implementations must not log it; callers are responsible
  /// for encryption and redaction when it can contain learner or identity data.
  pub fn new(
    kind: JobKind,
    payload: Vec<u8>,
    payload_version: u16,
    available_at: UtcTimestamp,
    max_attempts: NonZeroU32,
  ) -> Self {
    Self {
      kind,
      payload,
      payload_version,
      available_at,
      max_attempts,
    }
  }

  /// Returns the stable job kind.
  pub fn kind(&self) -> &JobKind {
    &self.kind
  }

  /// Returns the opaque versioned payload.
  pub fn payload(&self) -> &[u8] {
    &self.payload
  }

  /// Returns the schema version required to decode the payload.
  pub fn payload_version(&self) -> u16 {
    self.payload_version
  }

  /// Returns the first UTC instant at which the job may be claimed.
  pub fn available_at(&self) -> UtcTimestamp {
    self.available_at
  }

  /// Returns the maximum number of claims before the job becomes dead.
  pub fn max_attempts(&self) -> NonZeroU32 {
    self.max_attempts
  }
}

impl fmt::Debug for JobSubmission {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("JobSubmission")
      .field("kind", &self.kind)
      .field("payload_bytes", &self.payload.len())
      .field("payload_version", &self.payload_version)
      .field("available_at", &self.available_at)
      .field("max_attempts", &self.max_attempts)
      .finish()
  }
}

/// Persisted lifecycle state for a durable job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
  /// The job is eligible now or at its scheduled availability time.
  Queued,
  /// A worker holds a non-expired lease for the job.
  Running,
  /// A worker completed the job successfully.
  Completed,
  /// The job reached a permanent failure or its bounded attempt limit.
  Dead,
}

/// Non-sensitive status information available for polling and operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
  pub(crate) id: PublicId,
  pub(crate) kind: JobKind,
  pub(crate) payload_version: u16,
  pub(crate) status: JobStatus,
  pub(crate) attempts: u32,
  pub(crate) max_attempts: NonZeroU32,
  pub(crate) available_at: UtcTimestamp,
  pub(crate) lease_expires_at: Option<UtcTimestamp>,
  pub(crate) failure_code: Option<JobFailureCode>,
}

impl JobSnapshot {
  /// Returns the job's stable public ID.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the stable machine-readable job kind.
  pub fn kind(&self) -> &JobKind {
    &self.kind
  }

  /// Returns the schema version required to decode the opaque payload.
  pub fn payload_version(&self) -> u16 {
    self.payload_version
  }

  /// Returns the current durable-job lifecycle state.
  pub fn status(&self) -> JobStatus {
    self.status
  }

  /// Returns the number of times workers have claimed the job.
  pub fn attempts(&self) -> u32 {
    self.attempts
  }

  /// Returns the maximum number of claims permitted for the job.
  pub fn max_attempts(&self) -> NonZeroU32 {
    self.max_attempts
  }

  /// Returns the next UTC instant at which a queued job may be claimed.
  pub fn available_at(&self) -> UtcTimestamp {
    self.available_at
  }

  /// Returns the current lease expiry when the job is running.
  pub fn lease_expires_at(&self) -> Option<UtcTimestamp> {
    self.lease_expires_at
  }

  /// Returns the redacted terminal or retry failure category, when one exists.
  pub fn failure_code(&self) -> Option<&JobFailureCode> {
    self.failure_code.as_ref()
  }
}

/// A worker-owned lease and the opaque job payload it may process.
#[derive(Clone, PartialEq, Eq)]
pub struct ClaimedJob {
  pub(crate) id: PublicId,
  pub(crate) lease_id: PublicId,
  pub(crate) worker: WorkerId,
  pub(crate) kind: JobKind,
  pub(crate) payload: Vec<u8>,
  pub(crate) payload_version: u16,
  pub(crate) attempt: u32,
  pub(crate) lease_expires_at: UtcTimestamp,
}

impl ClaimedJob {
  /// Returns the stable public ID for the claimed job.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the worker identity that owns this lease.
  pub fn worker(&self) -> &WorkerId {
    &self.worker
  }

  /// Returns the stable job kind.
  pub fn kind(&self) -> &JobKind {
    &self.kind
  }

  /// Returns the opaque, versioned payload assigned to the worker.
  pub fn payload(&self) -> &[u8] {
    &self.payload
  }

  /// Returns the schema version required to decode the payload.
  pub fn payload_version(&self) -> u16 {
    self.payload_version
  }

  /// Returns this claim's one-based attempt number.
  pub fn attempt(&self) -> u32 {
    self.attempt
  }

  /// Returns the UTC instant at which the worker must renew or stop processing.
  pub fn lease_expires_at(&self) -> UtcTimestamp {
    self.lease_expires_at
  }
}

impl fmt::Debug for ClaimedJob {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("ClaimedJob")
      .field("id", &self.id)
      .field("lease_id", &"[redacted]")
      .field("kind", &self.kind)
      .field("payload_bytes", &self.payload.len())
      .field("payload_version", &self.payload_version)
      .field("attempt", &self.attempt)
      .field("lease_expires_at", &self.lease_expires_at)
      .finish()
  }
}

/// Failure reported by a worker after it holds a valid lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobFailure {
  /// Retry at the supplied UTC instant if the attempt budget remains.
  Retryable {
    /// A stable redacted category suitable for metrics and operations.
    code: JobFailureCode,
    /// The next UTC instant at which this retry may be claimed.
    available_at: UtcTimestamp,
  },
  /// Stop processing immediately and place the job in the dead state.
  Permanent {
    /// A stable redacted category suitable for metrics and operations.
    code: JobFailureCode,
  },
}

/// State selected by the queue after a worker reports a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobFailureOutcome {
  /// The job was returned to the queue for a bounded future retry.
  RetryScheduled,
  /// The job became dead and requires operator-controlled replay to run again.
  Dead,
}

/// Failure returned by durable queue operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum JobQueueError {
  /// The queue could not create a new opaque job or lease ID.
  #[error("could not generate durable-job ID: {0}")]
  IdGeneration(#[from] PublicIdGenerationError),
  /// The requested lease duration was zero or could not produce a future UTC expiry.
  #[error("durable-job lease duration is invalid")]
  InvalidLeaseDuration,
  /// The requested job does not exist.
  #[error("durable job does not exist")]
  NotFound,
  /// The worker no longer holds a current lease for the job.
  #[error("durable-job lease was lost")]
  LeaseLost,
  /// Replay is valid only for a job already in the dead state.
  #[error("durable job is not replayable")]
  NotReplayable,
  /// The queue dependency cannot currently serve the operation.
  #[error("durable queue is unavailable")]
  Unavailable,
}

/// Coordinates persistence, leased execution, retry, dead-lettering, and replay of durable jobs.
#[async_trait]
pub trait DurableJobQueue: Send + Sync {
  /// Persists a job before any dependent background work begins.
  ///
  /// # Errors
  ///
  /// Returns an error when the submission cannot be made durable or no public ID can be generated.
  async fn enqueue(&self, submission: JobSubmission) -> Result<PublicId, JobQueueError>;

  /// Returns non-sensitive lifecycle information for one job.
  ///
  /// # Errors
  ///
  /// Returns an error when the queue dependency cannot serve the read.
  async fn get(&self, id: &PublicId) -> Result<Option<JobSnapshot>, JobQueueError>;

  /// Atomically claims one eligible job for `worker` with a bounded lease.
  ///
  /// An expired lease is eligible for reclamation. The returned payload must be treated as
  /// confidential by worker logging and telemetry.
  ///
  /// # Errors
  ///
  /// Returns an error when the queue cannot atomically select and lease a job.
  async fn claim(
    &self,
    worker: &WorkerId,
    lease_duration: Duration,
  ) -> Result<Option<ClaimedJob>, JobQueueError>;

  /// Extends a current lease and returns its new UTC expiry.
  ///
  /// # Errors
  ///
  /// Returns [`JobQueueError::LeaseLost`] for a stale, expired, or foreign claim.
  async fn heartbeat(
    &self,
    job: &ClaimedJob,
    lease_duration: Duration,
  ) -> Result<UtcTimestamp, JobQueueError>;

  /// Marks a current leased job complete exactly once.
  ///
  /// # Errors
  ///
  /// Returns [`JobQueueError::LeaseLost`] for a stale, expired, or foreign claim.
  async fn complete(&self, job: &ClaimedJob) -> Result<(), JobQueueError>;

  /// Records a redacted failure and applies the bounded retry or dead-state policy.
  ///
  /// # Errors
  ///
  /// Returns [`JobQueueError::LeaseLost`] for a stale, expired, or foreign claim.
  async fn fail(
    &self,
    job: &ClaimedJob,
    failure: JobFailure,
  ) -> Result<JobFailureOutcome, JobQueueError>;

  /// Resets a dead job to queued state for explicit operator-controlled replay.
  ///
  /// Replay clears its prior failure category and claim count; handlers must remain idempotent by
  /// job kind, entity, and operation version.
  ///
  /// # Errors
  ///
  /// Returns [`JobQueueError::NotReplayable`] when the job is not dead.
  async fn replay(&self, id: &PublicId) -> Result<(), JobQueueError>;
}

fn is_machine_label(value: &str) -> bool {
  !value.is_empty()
    && value.len() <= 64
    && value.bytes().all(|byte| {
      byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_non_machine_job_labels() {
    assert_eq!(JobKind::new("Lookup"), Err(JobKindError::InvalidFormat));
    assert_eq!(
      JobFailureCode::new("provider unavailable"),
      Err(JobFailureCodeError::InvalidFormat)
    );
  }

  #[test]
  fn accepts_versioned_job_submission() {
    let kind = JobKind::new("lookup.generate").unwrap();
    let job = JobSubmission::new(
      kind,
      vec![1, 2],
      3,
      UtcTimestamp::UNIX_EPOCH,
      NonZeroU32::new(2).unwrap(),
    );

    assert_eq!(job.payload(), [1, 2]);
    assert_eq!(job.max_attempts().get(), 2);
  }
}
