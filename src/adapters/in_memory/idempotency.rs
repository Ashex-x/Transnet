//! Deterministic in-memory scoped idempotency store.

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  ports::{
    clock::Clock,
    idempotency::{
      IdempotencyBegin, IdempotencyError, IdempotencyKeyDigest, IdempotencyLease,
      IdempotencyRequest, IdempotencyResponse, IdempotencyScope, IdempotencyStore,
      RequestFingerprint,
    },
    public_id::{PublicId, PublicIdGenerator},
  },
};

/// Shareable idempotency store with atomic reservation, replay, conflict, expiry, and abandonment.
#[derive(Clone)]
pub struct InMemoryIdempotencyStore {
  clock: Arc<dyn Clock>,
  ids: Arc<dyn PublicIdGenerator>,
  entries: Arc<Mutex<HashMap<EntryKey, StoredEntry>>>,
}

impl InMemoryIdempotencyStore {
  /// Creates an empty idempotency store using injected UTC time and public-ID generation.
  pub fn new(clock: Arc<dyn Clock>, ids: Arc<dyn PublicIdGenerator>) -> Self {
    Self {
      clock,
      ids,
      entries: Arc::new(Mutex::new(HashMap::new())),
    }
  }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
  async fn begin(&self, request: IdempotencyRequest) -> Result<IdempotencyBegin, IdempotencyError> {
    let now = self.clock.now();
    if request.expires_at() <= now {
      return Err(IdempotencyError::Expired);
    }
    let key = EntryKey::from_request(&request);
    if let Some(result) = self.existing_result(&key, request.request_fingerprint(), now) {
      return Ok(result);
    }

    let lease_id = self.ids.generate()?;
    let mut entries = mutex_lock(&self.entries);
    expire_entries(&mut entries, now);
    if let Some(entry) = entries.get(&key) {
      return Ok(entry.begin_result(request.request_fingerprint()));
    }

    entries.insert(
      key,
      StoredEntry {
        request_fingerprint: request.request_fingerprint().clone(),
        expires_at: request.expires_at(),
        reservation_id: lease_id.clone(),
        state: EntryState::Pending,
      },
    );
    Ok(IdempotencyBegin::Started(IdempotencyLease {
      id: lease_id,
      scope: request.scope().clone(),
      key_digest: request.key_digest().clone(),
      request_fingerprint: request.request_fingerprint().clone(),
    }))
  }

  async fn complete(
    &self,
    lease: &IdempotencyLease,
    response: IdempotencyResponse,
  ) -> Result<(), IdempotencyError> {
    let now = self.clock.now();
    let key = EntryKey::from_lease(lease);
    let mut entries = mutex_lock(&self.entries);
    expire_entries(&mut entries, now);
    let Some(entry) = entries.get_mut(&key) else {
      return Err(IdempotencyError::LeaseLost);
    };
    if entry.reservation_id != lease.id
      || entry.request_fingerprint != lease.request_fingerprint
      || !matches!(entry.state, EntryState::Pending)
    {
      return Err(IdempotencyError::LeaseLost);
    }

    entry.state = EntryState::Completed(response);
    Ok(())
  }

  async fn abandon(&self, lease: &IdempotencyLease) -> Result<(), IdempotencyError> {
    let now = self.clock.now();
    let key = EntryKey::from_lease(lease);
    let mut entries = mutex_lock(&self.entries);
    expire_entries(&mut entries, now);
    let Some(entry) = entries.get(&key) else {
      return Err(IdempotencyError::LeaseLost);
    };
    if entry.reservation_id != lease.id
      || entry.request_fingerprint != lease.request_fingerprint
      || !matches!(entry.state, EntryState::Pending)
    {
      return Err(IdempotencyError::LeaseLost);
    }

    entries.remove(&key);
    Ok(())
  }
}

impl InMemoryIdempotencyStore {
  fn existing_result(
    &self,
    key: &EntryKey,
    fingerprint: &RequestFingerprint,
    now: std::time::SystemTime,
  ) -> Option<IdempotencyBegin> {
    let mut entries = mutex_lock(&self.entries);
    expire_entries(&mut entries, now);
    entries
      .get(key)
      .map(|entry| entry.begin_result(fingerprint))
  }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct EntryKey {
  scope: IdempotencyScope,
  key_digest: IdempotencyKeyDigest,
}

impl EntryKey {
  fn from_request(request: &IdempotencyRequest) -> Self {
    Self {
      scope: request.scope().clone(),
      key_digest: request.key_digest().clone(),
    }
  }

  fn from_lease(lease: &IdempotencyLease) -> Self {
    Self {
      scope: lease.scope.clone(),
      key_digest: lease.key_digest.clone(),
    }
  }
}

struct StoredEntry {
  request_fingerprint: RequestFingerprint,
  expires_at: std::time::SystemTime,
  reservation_id: PublicId,
  state: EntryState,
}

impl StoredEntry {
  fn begin_result(&self, fingerprint: &RequestFingerprint) -> IdempotencyBegin {
    if &self.request_fingerprint != fingerprint {
      return IdempotencyBegin::Conflict;
    }

    match &self.state {
      EntryState::Pending => IdempotencyBegin::InProgress,
      EntryState::Completed(response) => IdempotencyBegin::Replayed(response.clone()),
    }
  }
}

enum EntryState {
  Pending,
  Completed(IdempotencyResponse),
}

fn expire_entries(entries: &mut HashMap<EntryKey, StoredEntry>, now: std::time::SystemTime) {
  entries.retain(|_, entry| entry.expires_at > now);
}

#[cfg(test)]
mod tests {
  use std::{
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

  fn request(fingerprint: u8, expires_at: SystemTime) -> IdempotencyRequest {
    IdempotencyRequest::new(
      IdempotencyScope::new("principal-hash:POST-v1-saves").unwrap(),
      IdempotencyKeyDigest::new([7; 32]),
      RequestFingerprint::new([fingerprint; 32]),
      expires_at,
    )
  }

  #[tokio::test]
  async fn reserves_replays_conflicts_and_expires_records() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
    let clock = Arc::new(FixedClock::new(now));
    let ids: Arc<dyn PublicIdGenerator> =
      Arc::new(SequencePublicIdGenerator::new([public_id(1), public_id(2)]));
    let store = InMemoryIdempotencyStore::new(clock.clone(), ids);
    let expires_at = now + Duration::from_secs(10);
    assert_eq!(
      store.begin(request(1, now)).await,
      Err(IdempotencyError::Expired)
    );
    let started = store.begin(request(1, expires_at)).await.unwrap();
    let IdempotencyBegin::Started(lease) = started else {
      panic!("first reservation should start");
    };

    assert_eq!(
      store.begin(request(1, expires_at)).await.unwrap(),
      IdempotencyBegin::InProgress
    );
    assert_eq!(
      store.begin(request(2, expires_at)).await.unwrap(),
      IdempotencyBegin::Conflict
    );

    store
      .complete(&lease, IdempotencyResponse::new(b"saved".to_vec()))
      .await
      .unwrap();
    assert_eq!(
      store.begin(request(1, expires_at)).await.unwrap(),
      IdempotencyBegin::Replayed(IdempotencyResponse::new(b"saved".to_vec()))
    );

    clock.advance(Duration::from_secs(10));
    assert!(matches!(
      store
        .begin(request(1, now + Duration::from_secs(20)))
        .await
        .unwrap(),
      IdempotencyBegin::Started(_)
    ));
  }

  #[tokio::test]
  async fn abandons_only_the_matching_pending_reservation() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
    let clock = Arc::new(FixedClock::new(now));
    let ids: Arc<dyn PublicIdGenerator> = Arc::new(SequencePublicIdGenerator::new([
      public_id(11),
      public_id(12),
    ]));
    let store = InMemoryIdempotencyStore::new(clock, ids);
    let expires_at = now + Duration::from_secs(10);
    let IdempotencyBegin::Started(lease) = store.begin(request(1, expires_at)).await.unwrap()
    else {
      panic!("reservation should start");
    };

    store.abandon(&lease).await.unwrap();

    assert!(matches!(
      store.begin(request(1, expires_at)).await.unwrap(),
      IdempotencyBegin::Started(_)
    ));
    assert_eq!(
      store.abandon(&lease).await,
      Err(IdempotencyError::LeaseLost)
    );
  }
}
