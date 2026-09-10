//! Shared-cache interface for rebuildable, non-sensitive application results.

use crate::ports::clock::UtcTimestamp;

use async_trait::async_trait;
use thiserror::Error;

/// A cache value together with its absolute UTC expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry<Value> {
  value: Value,
  expires_at: UtcTimestamp,
}

impl<Value> CacheEntry<Value> {
  /// Creates an entry that is valid until `expires_at`.
  pub fn new(value: Value, expires_at: UtcTimestamp) -> Self {
    Self { value, expires_at }
  }

  /// Returns the entry value.
  pub fn value(&self) -> &Value {
    &self.value
  }

  /// Returns the absolute UTC expiry.
  pub fn expires_at(&self) -> UtcTimestamp {
    self.expires_at
  }

  /// Consumes the entry and returns its stored value.
  pub fn into_value(self) -> Value {
    self.value
  }
}

/// Failure returned by a cache adapter.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CacheError {
  /// The cache dependency cannot currently serve the operation.
  #[error("cache is unavailable")]
  Unavailable,
}

/// Stores rebuildable values addressed by an application-defined key.
///
/// Cache keys must include every content, policy, and ranking version that affects the value.
/// Implementations must not treat a cache hit as an authorization decision.
#[async_trait]
pub trait Cache<Key, Value>: Send + Sync
where
  Key: Send + Sync,
  Value: Send + Sync,
{
  /// Reads a non-expired cache entry for `key`.
  ///
  /// # Errors
  ///
  /// Returns an error when the cache dependency cannot serve the read.
  async fn get(&self, key: &Key) -> Result<Option<CacheEntry<Value>>, CacheError>;

  /// Stores a value until its absolute UTC expiry.
  ///
  /// # Errors
  ///
  /// Returns an error when the cache dependency cannot store the value.
  async fn put(&self, key: Key, value: Value, expires_at: UtcTimestamp) -> Result<(), CacheError>;

  /// Removes the entry addressed by `key`, if it exists.
  ///
  /// # Errors
  ///
  /// Returns an error when the cache dependency cannot remove the entry.
  async fn remove(&self, key: &Key) -> Result<(), CacheError>;
}
