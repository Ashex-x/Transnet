//! Deterministic in-memory cache with injected-clock expiry.

use std::{
  collections::HashMap,
  hash::Hash,
  sync::{Arc, RwLock},
};

use async_trait::async_trait;

use crate::{
  adapters::clock::write_lock,
  ports::{
    cache::{Cache, CacheEntry, CacheError},
    clock::Clock,
  },
};

/// Shareable in-memory cache that enforces absolute UTC expiry through an injected clock.
#[derive(Clone)]
pub struct InMemoryCache<Key, Value> {
  clock: Arc<dyn Clock>,
  entries: Arc<RwLock<HashMap<Key, CacheEntry<Value>>>>,
}

impl<Key, Value> InMemoryCache<Key, Value> {
  /// Creates an empty cache whose expiry decisions use `clock`.
  pub fn new(clock: Arc<dyn Clock>) -> Self {
    Self {
      clock,
      entries: Arc::new(RwLock::new(HashMap::new())),
    }
  }
}

#[async_trait]
impl<Key, Value> Cache<Key, Value> for InMemoryCache<Key, Value>
where
  Key: Clone + Eq + Hash + Send + Sync + 'static,
  Value: Clone + Send + Sync + 'static,
{
  async fn get(&self, key: &Key) -> Result<Option<CacheEntry<Value>>, CacheError> {
    let now = self.clock.now();
    let mut entries = write_lock(&self.entries);
    match entries.get(key) {
      Some(entry) if entry.expires_at() > now => Ok(Some(entry.clone())),
      Some(_) => {
        entries.remove(key);
        Ok(None)
      }
      None => Ok(None),
    }
  }

  async fn put(
    &self,
    key: Key,
    value: Value,
    expires_at: std::time::SystemTime,
  ) -> Result<(), CacheError> {
    write_lock(&self.entries).insert(key, CacheEntry::new(value, expires_at));
    Ok(())
  }

  async fn remove(&self, key: &Key) -> Result<(), CacheError> {
    write_lock(&self.entries).remove(key);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::{
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use super::*;
  use crate::adapters::clock::FixedClock;

  #[tokio::test]
  async fn expires_entries_using_the_injected_clock() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let clock = Arc::new(FixedClock::new(now));
    let cache = InMemoryCache::new(clock.clone());

    cache
      .put("card", "cached", now + Duration::from_secs(5))
      .await
      .unwrap();
    assert_eq!(
      cache.get(&"card").await.unwrap().unwrap().value(),
      &"cached"
    );

    clock.advance(Duration::from_secs(5));

    assert_eq!(cache.get(&"card").await.unwrap(), None);
  }
}
