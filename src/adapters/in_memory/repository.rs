//! Deterministic in-memory record repository.

use std::{
  collections::HashMap,
  hash::Hash,
  sync::{Arc, RwLock},
};

use async_trait::async_trait;

use crate::{
  adapters::clock::{read_lock, write_lock},
  ports::repository::{Repository, RepositoryError},
};

/// Shareable in-memory repository that atomically replaces records by key.
#[derive(Clone)]
pub struct InMemoryRepository<Key, Record> {
  records: Arc<RwLock<HashMap<Key, Record>>>,
}

impl<Key, Record> InMemoryRepository<Key, Record> {
  /// Creates an empty in-memory repository.
  pub fn new() -> Self {
    Self {
      records: Arc::new(RwLock::new(HashMap::new())),
    }
  }
}

impl<Key, Record> Default for InMemoryRepository<Key, Record> {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<Key, Record> Repository<Key, Record> for InMemoryRepository<Key, Record>
where
  Key: Clone + Eq + Hash + Send + Sync + 'static,
  Record: Clone + Send + Sync + 'static,
{
  async fn find(&self, key: &Key) -> Result<Option<Record>, RepositoryError> {
    Ok(read_lock(&self.records).get(key).cloned())
  }

  async fn save(&self, key: Key, record: Record) -> Result<(), RepositoryError> {
    write_lock(&self.records).insert(key, record);
    Ok(())
  }

  async fn remove(&self, key: &Key) -> Result<(), RepositoryError> {
    write_lock(&self.records).remove(key);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn saves_replaces_and_removes_records() {
    let repository = InMemoryRepository::new();

    repository.save("sense-1", "first").await.unwrap();
    repository.save("sense-1", "second").await.unwrap();

    assert_eq!(repository.find(&"sense-1").await.unwrap(), Some("second"));

    repository.remove(&"sense-1").await.unwrap();

    assert_eq!(repository.find(&"sense-1").await.unwrap(), None);
  }
}
