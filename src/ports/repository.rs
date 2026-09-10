//! Generic persistence interface for canonical and learner-owned records.

use async_trait::async_trait;
use thiserror::Error;

/// Failure returned by a record repository.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RepositoryError {
  /// The backing repository cannot currently serve the operation.
  #[error("repository is unavailable")]
  Unavailable,
  /// The operation conflicted with concurrent state or an optimistic version check.
  #[error("repository operation conflicted")]
  Conflict,
}

/// Persists records addressed by an application-defined key.
///
/// Implementations must preserve application invariants atomically for each individual method.
#[async_trait]
pub trait Repository<Key, Record>: Send + Sync
where
  Key: Send + Sync,
  Record: Send + Sync,
{
  /// Loads the current record for `key` when it exists.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository cannot serve the read.
  async fn find(&self, key: &Key) -> Result<Option<Record>, RepositoryError>;

  /// Inserts or replaces the record addressed by `key`.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository cannot durably store the record.
  async fn save(&self, key: Key, record: Record) -> Result<(), RepositoryError>;

  /// Removes the record addressed by `key`, if it exists.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository cannot durably apply the removal.
  async fn remove(&self, key: &Key) -> Result<(), RepositoryError>;
}
