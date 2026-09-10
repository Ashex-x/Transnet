//! Scoped idempotency reservation and response-storage interface.

use std::{fmt, time::SystemTime};

use async_trait::async_trait;
use thiserror::Error;

use crate::ports::public_id::{PublicId, PublicIdGenerationError};

/// An opaque scope derived from an authorized principal and mutation route.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdempotencyScope(String);

impl IdempotencyScope {
  /// Validates an opaque bounded scope token.
  ///
  /// The caller must use a non-reversible principal representation rather than an identity display
  /// field. Route and tenant components belong in this scope so keys cannot cross mutation
  /// boundaries.
  ///
  /// # Errors
  ///
  /// Returns [`IdempotencyScopeError::InvalidFormat`] for blank, oversized, or non-printable input.
  pub fn new(value: impl Into<String>) -> Result<Self, IdempotencyScopeError> {
    let value = value.into();
    if value.is_empty() || value.len() > 256 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
      return Err(IdempotencyScopeError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the opaque scope token for a durable implementation's lookup key.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for IdempotencyScope {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IdempotencyScope([redacted])")
  }
}

/// Validation failure for an idempotency scope.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum IdempotencyScopeError {
  /// The scope was not bounded printable ASCII.
  #[error("idempotency scope must be bounded printable ASCII")]
  InvalidFormat,
}

/// A 32-byte HMAC or equivalent digest of a client-supplied idempotency key.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKeyDigest([u8; 32]);

impl IdempotencyKeyDigest {
  /// Creates an opaque key digest from an application-owned cryptographic operation.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns the digest bytes for a durable implementation's key lookup.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for IdempotencyKeyDigest {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IdempotencyKeyDigest([redacted])")
  }
}

/// A 32-byte cryptographic fingerprint of the normalized mutation request.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RequestFingerprint([u8; 32]);

impl RequestFingerprint {
  /// Creates a request fingerprint from an application-owned cryptographic operation.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns the fingerprint bytes for comparison within the same idempotency scope.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for RequestFingerprint {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("RequestFingerprint([redacted])")
  }
}

/// An idempotency reservation request for one scoped mutation.
#[derive(Clone)]
pub struct IdempotencyRequest {
  scope: IdempotencyScope,
  key_digest: IdempotencyKeyDigest,
  request_fingerprint: RequestFingerprint,
  expires_at: SystemTime,
}

impl IdempotencyRequest {
  /// Creates a reservation that expires at `expires_at`.
  pub fn new(
    scope: IdempotencyScope,
    key_digest: IdempotencyKeyDigest,
    request_fingerprint: RequestFingerprint,
    expires_at: SystemTime,
  ) -> Self {
    Self {
      scope,
      key_digest,
      request_fingerprint,
      expires_at,
    }
  }

  /// Returns the opaque route-and-principal scope.
  pub fn scope(&self) -> &IdempotencyScope {
    &self.scope
  }

  /// Returns the key digest without exposing a raw client-supplied key.
  pub fn key_digest(&self) -> &IdempotencyKeyDigest {
    &self.key_digest
  }

  /// Returns the normalized request fingerprint for conflict detection.
  pub fn request_fingerprint(&self) -> &RequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the UTC expiry of the reservation and stored response.
  pub fn expires_at(&self) -> SystemTime {
    self.expires_at
  }
}

impl fmt::Debug for IdempotencyRequest {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("IdempotencyRequest")
      .field("scope", &self.scope)
      .field("key_digest", &self.key_digest)
      .field("request_fingerprint", &self.request_fingerprint)
      .field("expires_at", &self.expires_at)
      .finish()
  }
}

/// Opaque encoded response retained for an idempotent replay.
#[derive(Clone, PartialEq, Eq)]
pub struct IdempotencyResponse(Vec<u8>);

impl IdempotencyResponse {
  /// Creates a response representation owned by the application codec.
  ///
  /// Implementations must not log the returned bytes because a mutation response can contain
  /// private resource identifiers or learner-owned state.
  pub fn new(value: Vec<u8>) -> Self {
    Self(value)
  }

  /// Returns the opaque encoded response.
  pub fn as_bytes(&self) -> &[u8] {
    &self.0
  }

  /// Consumes the response and returns the application-encoded bytes.
  pub fn into_bytes(self) -> Vec<u8> {
    self.0
  }
}

impl fmt::Debug for IdempotencyResponse {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("IdempotencyResponse")
      .field("bytes", &self.0.len())
      .finish()
  }
}

/// An opaque reservation capability returned only to the matching idempotency store.
#[derive(Clone, PartialEq, Eq)]
pub struct IdempotencyLease {
  pub(crate) id: PublicId,
  pub(crate) scope: IdempotencyScope,
  pub(crate) key_digest: IdempotencyKeyDigest,
  pub(crate) request_fingerprint: RequestFingerprint,
}

impl fmt::Debug for IdempotencyLease {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("IdempotencyLease")
      .field("id", &"[redacted]")
      .field("scope", &self.scope)
      .field("key_digest", &self.key_digest)
      .field("request_fingerprint", &self.request_fingerprint)
      .finish()
  }
}

/// Result of atomically attempting to reserve an idempotent mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyBegin {
  /// This caller owns the newly created pending reservation.
  Started(IdempotencyLease),
  /// A matching mutation is still executing under another valid reservation.
  InProgress,
  /// A matching completed mutation can return the retained response without executing again.
  Replayed(IdempotencyResponse),
  /// The same scope and key were used for a different normalized request.
  Conflict,
}

/// Failure returned by an idempotency store.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum IdempotencyError {
  /// The store could not create a new opaque reservation ID.
  #[error("could not generate idempotency reservation ID: {0}")]
  IdGeneration(#[from] PublicIdGenerationError),
  /// The requested expiry is not in the future according to the store clock.
  #[error("idempotency reservation is already expired")]
  Expired,
  /// The caller no longer owns the pending reservation.
  #[error("idempotency reservation was lost")]
  LeaseLost,
  /// The backing idempotency dependency cannot currently serve the operation.
  #[error("idempotency store is unavailable")]
  Unavailable,
}

/// Stores scoped idempotency reservations and opaque completed responses.
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
  /// Atomically creates a reservation, reports in-progress work, replays a response, or conflicts.
  ///
  /// Expired reservations may be removed before this decision. A store must compare the request
  /// fingerprint before returning an in-progress or replay result.
  ///
  /// # Errors
  ///
  /// Returns [`IdempotencyError::Expired`] when the requested expiry is not in the future, or an
  /// error when the backing store cannot atomically reserve or read the record.
  async fn begin(&self, request: IdempotencyRequest) -> Result<IdempotencyBegin, IdempotencyError>;

  /// Stores the encoded response and closes a reservation held by this caller.
  ///
  /// # Errors
  ///
  /// Returns a lease-lost error when this caller no longer owns a pending reservation.
  async fn complete(
    &self,
    lease: &IdempotencyLease,
    response: IdempotencyResponse,
  ) -> Result<(), IdempotencyError>;

  /// Removes a reservation held by this caller before it creates an externally visible mutation.
  ///
  /// # Errors
  ///
  /// Returns a lease-lost error when this caller no longer owns a pending reservation.
  async fn abandon(&self, lease: &IdempotencyLease) -> Result<(), IdempotencyError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn redacts_idempotency_debug_values() {
    let scope = IdempotencyScope::new("principal-hash:POST-v1-saves").unwrap();
    let request = IdempotencyRequest::new(
      scope,
      IdempotencyKeyDigest::new([1; 32]),
      RequestFingerprint::new([2; 32]),
      SystemTime::UNIX_EPOCH,
    );

    assert!(!format!("{request:?}").contains("principal-hash"));
  }
}
