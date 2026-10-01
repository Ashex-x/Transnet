//! Request-safe correlation, deadline, schema, and release context.

use std::{fmt, time::Duration};

use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use ulid::Ulid;

use super::canonical::ReleaseId;

/// Maximum accepted request-ID length in bytes.
pub const MAX_REQUEST_ID_BYTES: usize = 128;
/// Maximum accepted schema-version length in bytes.
pub const MAX_SCHEMA_VERSION_BYTES: usize = 64;

/// A bounded, header-safe request correlation identifier.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct RequestId(String);

impl RequestId {
  /// Validates a caller-provided request identifier.
  ///
  /// # Errors
  ///
  /// Returns an error when the value is empty, oversized, or unsafe in an HTTP header.
  pub fn new(value: impl Into<String>) -> Result<Self, RequestContextError> {
    let value = value.into();
    if !(1..=MAX_REQUEST_ID_BYTES).contains(&value.len())
      || !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
      return Err(RequestContextError::InvalidRequestId);
    }
    Ok(Self(value))
  }

  /// Generates a new opaque request identifier.
  pub fn generate() -> Self {
    Self(Ulid::new().to_string())
  }

  /// Returns the validated identifier.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for RequestId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("RequestId(REDACTED)")
  }
}

impl fmt::Display for RequestId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.as_str())
  }
}

/// Immutable context shared by every operation performed for one admitted request.
#[derive(Clone, Debug)]
pub struct RequestContext {
  request_id: RequestId,
  deadline_at: OffsetDateTime,
  schema_version: String,
  content_release: Option<ReleaseId>,
}

impl RequestContext {
  /// Creates a request context from already bounded boundary values.
  ///
  /// # Errors
  ///
  /// Returns an error for an invalid deadline, schema version, or release identifier.
  pub fn new(
    request_id: RequestId,
    deadline_at: OffsetDateTime,
    schema_version: impl Into<String>,
    content_release: Option<ReleaseId>,
  ) -> Result<Self, RequestContextError> {
    if !deadline_at.nanosecond().is_multiple_of(1_000) {
      return Err(RequestContextError::InvalidDeadline);
    }
    let schema_version = schema_version.into();
    if !(1..=MAX_SCHEMA_VERSION_BYTES).contains(&schema_version.len())
      || !schema_version
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
      return Err(RequestContextError::InvalidSchemaVersion);
    }
    validate_release(content_release.as_ref())?;
    Ok(Self {
      request_id,
      deadline_at,
      schema_version,
      content_release,
    })
  }

  /// Returns the correlation identifier shared with the response.
  pub fn request_id(&self) -> &RequestId {
    &self.request_id
  }

  /// Returns the absolute UTC deadline.
  pub fn deadline_at(&self) -> OffsetDateTime {
    self.deadline_at
  }

  /// Formats the absolute deadline as RFC 3339 for downstream operation envelopes.
  pub fn deadline_rfc3339(&self) -> String {
    self.deadline_at.format(&Rfc3339).unwrap_or_default()
  }

  /// Returns the budget remaining at `now`, saturating at zero after expiry.
  pub fn remaining_budget_at(&self, now: OffsetDateTime) -> Duration {
    if self.deadline_at <= now {
      return Duration::ZERO;
    }
    (self.deadline_at - now)
      .try_into()
      .unwrap_or(Duration::ZERO)
  }

  /// Returns the budget remaining at the current UTC time.
  pub fn remaining_budget(&self) -> Duration {
    self.remaining_budget_at(OffsetDateTime::now_utc())
  }

  /// Reports whether the deadline has been exhausted at `now`.
  pub fn is_expired_at(&self, now: OffsetDateTime) -> bool {
    self.deadline_at <= now
  }

  /// Returns the transport schema expected by this request.
  pub fn schema_version(&self) -> &str {
    &self.schema_version
  }

  /// Returns the immutable content release pinned for this request, when one was selected.
  pub fn content_release(&self) -> Option<&ReleaseId> {
    self.content_release.as_ref()
  }

  /// Returns a copy pinned to an immutable release without changing the request deadline.
  ///
  /// # Errors
  ///
  /// Returns an error when the release identifier is not request-safe.
  pub fn with_content_release(mut self, release: ReleaseId) -> Result<Self, RequestContextError> {
    validate_release(Some(&release))?;
    self.content_release = Some(release);
    Ok(self)
  }
}

fn validate_release(release: Option<&ReleaseId>) -> Result<(), RequestContextError> {
  if release.is_some_and(|release| {
    !(1..=MAX_REQUEST_ID_BYTES).contains(&release.as_str().len())
      || !release
        .as_str()
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
  }) {
    return Err(RequestContextError::InvalidContentRelease);
  }
  Ok(())
}

/// Closed validation failures for request-safe context values.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RequestContextError {
  /// The request identifier is not bounded header-safe ASCII.
  #[error("request ID must be bounded header-safe ASCII")]
  InvalidRequestId,
  /// The absolute deadline is not representable at the contract's microsecond precision.
  #[error("request deadline must use UTC RFC 3339 microsecond precision")]
  InvalidDeadline,
  /// The schema version is not bounded identifier-safe ASCII.
  #[error("schema version must be bounded identifier-safe ASCII")]
  InvalidSchemaVersion,
  /// The release identifier is not bounded request-safe ASCII.
  #[error("content release must be bounded request-safe ASCII")]
  InvalidContentRelease,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::CanonicalId;

  fn deadline() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-01T12:00:00.123456Z", &Rfc3339).unwrap()
  }

  #[test]
  fn computes_a_saturating_remaining_budget() {
    let context =
      RequestContext::new(RequestId::generate(), deadline(), "transnet-v1", None).unwrap();
    let before = OffsetDateTime::parse("2026-10-01T11:59:58.123456Z", &Rfc3339).unwrap();
    let after = OffsetDateTime::parse("2026-10-01T12:00:01Z", &Rfc3339).unwrap();
    assert_eq!(context.remaining_budget_at(before), Duration::from_secs(2));
    assert_eq!(context.remaining_budget_at(after), Duration::ZERO);
    assert!(context.is_expired_at(after));
  }

  #[test]
  fn preserves_one_deadline_when_pinning_a_release() {
    let context = RequestContext::new(RequestId::generate(), deadline(), "transnet-v1", None)
      .unwrap()
      .with_content_release(CanonicalId::new("release-2026-10").unwrap())
      .unwrap();
    assert_eq!(context.deadline_rfc3339(), "2026-10-01T12:00:00.123456Z");
    assert_eq!(
      context.content_release().unwrap().as_str(),
      "release-2026-10"
    );
  }

  #[test]
  fn debug_output_does_not_disclose_correlation_values() {
    let request_id = RequestId::new("secret-correlation-value").unwrap();
    assert_eq!(format!("{request_id:?}"), "RequestId(REDACTED)");
  }
}
