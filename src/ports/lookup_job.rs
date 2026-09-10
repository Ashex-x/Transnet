//! Private lookup-job lifecycle and polling interface.
//!
//! This is a lifecycle/polling contract only. It intentionally does not schedule a durable queue
//! entry or claim transactionality with [`crate::ports::durable_job::DurableJobQueue`].

use std::{fmt, time::Duration};

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

use crate::ports::{clock::UtcTimestamp, durable_job::JobFailureCode, public_id::PublicId};

/// An opaque reference to the authenticated owner of a lookup job.
///
/// Callers must derive this value from an authenticated principal using a non-reversible internal
/// representation. It is deliberately not a display name, email address, or other identity field.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LookupJobOwner(String);

impl LookupJobOwner {
  /// Creates an opaque owner reference suitable for equality checks and durable storage keys.
  ///
  /// # Errors
  ///
  /// Returns [`LookupJobOwnerError::InvalidFormat`] when `value` is blank, oversized, or contains
  /// non-printable ASCII text.
  pub fn new(value: impl Into<String>) -> Result<Self, LookupJobOwnerError> {
    let value = value.into();
    if value.is_empty() || value.len() > 256 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
      return Err(LookupJobOwnerError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the opaque owner value for an equality-preserving persistence implementation.
  ///
  /// This is not a display field and must never be added to logs or telemetry.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for LookupJobOwner {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("LookupJobOwner([redacted])")
  }
}

/// Validation failure for an opaque lookup-job owner reference.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LookupJobOwnerError {
  /// The owner reference was not bounded printable ASCII.
  #[error("lookup-job owner must be bounded printable ASCII")]
  InvalidFormat,
}

/// A high-entropy anonymous lookup-job capability.
///
/// This value is a bearer secret. It is carried only in the `Lookup-Capability` HTTP header and
/// never in a URL, log, trace, or error response. The type validates a conservative URL-safe
/// token shape but does not generate, hash, encrypt, or persist the secret.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LookupJobCapability(String);

impl LookupJobCapability {
  /// Parses a header-safe anonymous lookup-job capability.
  ///
  /// # Errors
  ///
  /// Returns [`LookupJobCapabilityError::InvalidFormat`] when `value` is too short, oversized, or
  /// not an unpadded URL-safe token.
  pub fn parse(value: &str) -> Result<Self, LookupJobCapabilityError> {
    if !(32..=512).contains(&value.len())
      || !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
      return Err(LookupJobCapabilityError::InvalidFormat);
    }

    Ok(Self(value.to_owned()))
  }

  /// Returns the bearer secret only for comparison or writing its dedicated HTTP header.
  ///
  /// Callers must never use this value in a URL, log message, trace field, or error response.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for LookupJobCapability {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("LookupJobCapability([redacted])")
  }
}

/// Validation failure for an anonymous lookup-job capability.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LookupJobCapabilityError {
  /// The capability was not a sufficiently long URL-safe bearer token.
  #[error("lookup-job capability must be a high-entropy URL-safe token")]
  InvalidFormat,
}

/// A caller credential accepted for lookup-job access.
#[derive(Clone, PartialEq, Eq)]
pub enum LookupJobAccess {
  /// Access derived from an authenticated owner principal.
  Owner(LookupJobOwner),
  /// Access derived from an anonymous bearer capability.
  Capability(LookupJobCapability),
}

impl LookupJobAccess {
  /// Creates owner-based access from a previously authenticated opaque owner reference.
  pub fn owner(owner: LookupJobOwner) -> Self {
    Self::Owner(owner)
  }

  /// Creates anonymous capability-based access from a bearer capability.
  pub fn capability(capability: LookupJobCapability) -> Self {
    Self::Capability(capability)
  }
}

impl fmt::Debug for LookupJobAccess {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Owner(_) => formatter.write_str("LookupJobAccess::Owner([redacted])"),
      Self::Capability(_) => formatter.write_str("LookupJobAccess::Capability([redacted])"),
    }
  }
}

/// Metadata required to create a private lookup job.
#[derive(Clone)]
pub struct LookupJobCreation {
  access: LookupJobAccess,
  expires_at: UtcTimestamp,
}

impl LookupJobCreation {
  /// Creates metadata for an owner-bound or anonymous-capability-bound lookup job.
  pub fn new(access: LookupJobAccess, expires_at: UtcTimestamp) -> Self {
    Self { access, expires_at }
  }

  /// Returns the sole credential class permitted to poll this job.
  pub fn access(&self) -> &LookupJobAccess {
    &self.access
  }

  /// Returns the UTC instant after which the job is no longer pollable.
  pub fn expires_at(&self) -> UtcTimestamp {
    self.expires_at
  }
}

impl fmt::Debug for LookupJobCreation {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("LookupJobCreation")
      .field("access", &self.access)
      .field("expires_at", &self.expires_at)
      .finish()
  }
}

/// A non-terminal lookup-job state visible to an authorized polling client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupJobPendingState {
  /// The lookup work has been recorded but no worker currently owns it.
  Queued,
  /// A worker currently owns the lookup work.
  Running,
}

impl LookupJobPendingState {
  /// Returns the stable lower-case transport status name.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Queued => "queued",
      Self::Running => "running",
    }
  }
}

/// A completed lookup response retained for an authorized polling client.
///
/// The value is the already-validated public lookup envelope. It can contain a learner query, so
/// debug rendering intentionally redacts it and implementations must not log it.
#[derive(Clone, PartialEq)]
pub struct LookupJobResult(Value);

impl LookupJobResult {
  /// Wraps an already-validated lookup response envelope.
  pub fn new(value: Value) -> Self {
    Self(value)
  }

  /// Returns the completed lookup response envelope for its authorized HTTP response.
  pub fn as_value(&self) -> &Value {
    &self.0
  }

  /// Consumes the result and returns the completed lookup response envelope.
  pub fn into_value(self) -> Value {
    self.0
  }
}

impl fmt::Debug for LookupJobResult {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("LookupJobResult([redacted])")
  }
}

/// A redacted terminal lookup-job failure visible to an authorized client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupJobFailure {
  code: JobFailureCode,
}

impl LookupJobFailure {
  /// Creates a terminal failure from a stable, redacted category.
  pub fn new(code: JobFailureCode) -> Self {
    Self { code }
  }

  /// Returns the stable failure category without an underlying provider message.
  pub fn code(&self) -> &JobFailureCode {
    &self.code
  }
}

/// Authorized polling state for one lookup job.
#[derive(Clone, PartialEq)]
pub enum LookupJobPoll {
  /// Work is still pending; clients should wait at least `retry_after` before polling again.
  Pending {
    /// The non-terminal lifecycle state.
    state: LookupJobPendingState,
    /// The minimum server-selected delay before the next poll.
    retry_after: Duration,
  },
  /// Work completed and can return the stored public lookup envelope.
  Completed(LookupJobResult),
  /// Work reached a terminal redacted failure.
  Failed(LookupJobFailure),
  /// The authorized job is known but has passed its retention window.
  Expired,
}

impl fmt::Debug for LookupJobPoll {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Pending { state, retry_after } => formatter
        .debug_struct("LookupJobPoll::Pending")
        .field("state", state)
        .field("retry_after", retry_after)
        .finish(),
      Self::Completed(_) => formatter.write_str("LookupJobPoll::Completed([redacted])"),
      Self::Failed(failure) => formatter
        .debug_tuple("LookupJobPoll::Failed")
        .field(failure)
        .finish(),
      Self::Expired => formatter.write_str("LookupJobPoll::Expired"),
    }
  }
}

/// Failure returned by lookup-job lifecycle storage.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LookupJobStoreError {
  /// A caller attempted to create a job with a public ID already in use.
  #[error("lookup job already exists")]
  AlreadyExists,
  /// A worker attempted to update a job that does not exist.
  #[error("lookup job does not exist")]
  NotFound,
  /// A worker attempted a transition that is no longer valid for the stored state.
  #[error("lookup job lifecycle transition is invalid")]
  InvalidTransition,
  /// The backing lookup-job dependency cannot currently serve the operation.
  #[error("lookup-job store is unavailable")]
  Unavailable,
}

/// Stores private lookup-job lifecycle metadata and authorized polling results.
///
/// Implementations must return `Ok(None)` from [`LookupJobStore::poll`] for both a missing job
/// and a credential that does not authorize it, preventing private-resource enumeration. This
/// port deliberately does not promise encryption, token hashing, database durability, job
/// scheduling, or atomic coupling to [`crate::ports::durable_job::DurableJobQueue`]; production
/// adapters must provide those properties explicitly at their deployment boundary.
#[async_trait]
pub trait LookupJobStore: Send + Sync {
  /// Creates a queued lookup job under the supplied externally generated public ID.
  ///
  /// This lifecycle-only operation does not enqueue a matching durable queue entry. A production
  /// scheduler must make both records visible atomically before it returns an asynchronous job to
  /// a client.
  ///
  /// # Errors
  ///
  /// Returns an error when the job metadata cannot be stored.
  async fn create(
    &self,
    id: PublicId,
    creation: LookupJobCreation,
  ) -> Result<(), LookupJobStoreError>;

  /// Marks a queued lookup job as running once a worker has claimed its durable queue entry.
  ///
  /// # Errors
  ///
  /// Returns an error when the job is absent, expired, or not queued.
  async fn start(&self, id: &PublicId) -> Result<(), LookupJobStoreError>;

  /// Stores a completed public lookup envelope for a running job.
  ///
  /// This port never accepts raw query or context data, so it cannot retain either after a job
  /// completes. A queue or request-payload adapter must erase its own opaque input separately.
  ///
  /// # Errors
  ///
  /// Returns an error when the job is absent, expired, or not running.
  async fn complete(
    &self,
    id: &PublicId,
    result: LookupJobResult,
  ) -> Result<(), LookupJobStoreError>;

  /// Stores a redacted terminal failure for a running lookup job.
  ///
  /// # Errors
  ///
  /// Returns an error when the job is absent, expired, or not running.
  async fn fail(&self, id: &PublicId, failure: LookupJobFailure)
    -> Result<(), LookupJobStoreError>;

  /// Returns the lifecycle state for an authorized poll, or hides an absent or unauthorized job.
  ///
  /// # Errors
  ///
  /// Returns an error when the backing lookup-job dependency cannot serve the read.
  async fn poll(
    &self,
    id: &PublicId,
    access: &LookupJobAccess,
  ) -> Result<Option<LookupJobPoll>, LookupJobStoreError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn redacts_owner_capability_and_result_debug_output() {
    let owner = LookupJobOwner::new("account-equality-token").unwrap();
    let capability = LookupJobCapability::parse("aB_1-23456789012345678901234567890").unwrap();
    let result = LookupJobResult::new(serde_json::json!({"query": "private learner query"}));

    assert!(!format!("{owner:?}").contains("account-equality-token"));
    assert!(!format!("{capability:?}").contains(capability.as_str()));
    assert!(!format!("{result:?}").contains("private learner query"));
  }

  #[test]
  fn rejects_short_or_non_url_safe_capabilities() {
    assert_eq!(
      LookupJobCapability::parse("short"),
      Err(LookupJobCapabilityError::InvalidFormat)
    );
    assert_eq!(
      LookupJobCapability::parse("aB_1-2345678901234567890123456789="),
      Err(LookupJobCapabilityError::InvalidFormat)
    );
  }
}
