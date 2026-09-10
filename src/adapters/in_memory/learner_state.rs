//! Deterministic process-local implementation of private learner-state ports.
//!
//! This adapter is for tests and local development only. It uses a short-held synchronous mutex
//! without awaiting while locked, shares records only between cloned handles, and does not provide
//! database durability, encrypted fields, identity hashing, cross-resource transactions, or real
//! process-restart survival.

use std::{
  collections::BTreeMap,
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  domain::{
    canonical::SenseId,
    learner::{
      HistoryEntry, LearnerDataInventory, LearnerId, LearnerPreferences, LearnerProfile,
      SavedSenseMutation, SavedVocabularyEntry,
    },
  },
  ports::{
    clock::UtcTimestamp,
    learner_state::{
      HistoryCursor, HistoryPage, HistoryPageRequest, LearnerHistoryStore,
      LearnerPrivacyInventoryStore, LearnerProfileStore, LearnerStateStoreError, PrivateUpdate,
      ProfileCreateResult, SavedVocabularyCursor, SavedVocabularyMutationResult,
      SavedVocabularyPage, SavedVocabularyPageRequest, SavedVocabularyStore, SavedVocabularyWrite,
    },
    public_id::PublicId,
  },
};

/// Shareable process-local private learner-state adapter for deterministic tests and development.
///
/// Every mutating method holds one mutex around its entire local state transition. The adapter is
/// intentionally non-production: clones share only process memory and do not encrypt, hash, or
/// durably persist any data.
#[derive(Clone, Default)]
pub struct InMemoryLearnerStateStore {
  state: Arc<Mutex<InMemoryLearnerState>>,
}

impl InMemoryLearnerStateStore {
  /// Creates an empty process-local learner-state store.
  pub fn new() -> Self {
    Self::default()
  }

  /// Reopens a handle over the same process-local records for deterministic restart-style tests.
  ///
  /// This shares memory only and does not simulate a process crash, durable database, encryption,
  /// or recovery transaction.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[derive(Default)]
struct InMemoryLearnerState {
  profiles: BTreeMap<LearnerId, LearnerProfile>,
  history: BTreeMap<LearnerId, BTreeMap<PublicId, HistoryEntry>>,
  saved_vocabulary: BTreeMap<LearnerId, BTreeMap<PublicId, SavedVocabularyEntry>>,
}

#[async_trait]
impl LearnerProfileStore for InMemoryLearnerStateStore {
  async fn create_profile(
    &self,
    profile: LearnerProfile,
  ) -> Result<ProfileCreateResult, LearnerStateStoreError> {
    let owner = profile.owner().clone();
    let mut state = mutex_lock(&self.state);
    if state.profiles.contains_key(&owner) {
      return Ok(ProfileCreateResult::AlreadyExists);
    }

    state.profiles.insert(owner, profile.clone());
    Ok(ProfileCreateResult::Created(profile))
  }

  async fn find_profile(
    &self,
    owner: &LearnerId,
  ) -> Result<Option<LearnerProfile>, LearnerStateStoreError> {
    Ok(mutex_lock(&self.state).profiles.get(owner).cloned())
  }

  async fn replace_preferences(
    &self,
    owner: &LearnerId,
    expected_revision: u64,
    preferences: LearnerPreferences,
    updated_at: UtcTimestamp,
  ) -> Result<PrivateUpdate<LearnerProfile>, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    let Some(profile) = state.profiles.get_mut(owner) else {
      return Ok(PrivateUpdate::Missing);
    };
    if profile.revision() != expected_revision {
      return Ok(PrivateUpdate::VersionConflict {
        current_revision: profile.revision(),
      });
    }

    profile.replace_preferences(preferences, updated_at)?;
    Ok(PrivateUpdate::Updated(profile.clone()))
  }
}

#[async_trait]
impl LearnerHistoryStore for InMemoryLearnerStateStore {
  async fn append_history(&self, entry: HistoryEntry) -> Result<(), LearnerStateStoreError> {
    let owner = entry.owner().clone();
    let id = entry.id().clone();
    let mut state = mutex_lock(&self.state);
    if state
      .history
      .values()
      .any(|entries| entries.contains_key(&id))
    {
      return Err(LearnerStateStoreError::Conflict);
    }

    state.history.entry(owner).or_default().insert(id, entry);
    Ok(())
  }

  async fn find_history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    now: UtcTimestamp,
  ) -> Result<Option<HistoryEntry>, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    purge_expired_history(&mut state, now);
    Ok(
      state
        .history
        .get(owner)
        .and_then(|entries| entries.get(id))
        .cloned(),
    )
  }

  async fn list_history(
    &self,
    owner: &LearnerId,
    page: HistoryPageRequest,
    now: UtcTimestamp,
  ) -> Result<HistoryPage, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    purge_expired_history(&mut state, now);
    let mut entries = state
      .history
      .get(owner)
      .map(|records| records.values().cloned().collect::<Vec<_>>())
      .unwrap_or_default();
    entries.sort_by(|left, right| {
      right
        .occurred_at()
        .cmp(&left.occurred_at())
        .then_with(|| right.id().cmp(left.id()))
    });
    if let Some(before) = page.before() {
      entries.retain(|entry| history_follows_cursor(entry, before));
    }

    let limit = usize::from(page.limit().get());
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    let next_cursor = if has_more {
      entries
        .last()
        .map(|last| HistoryCursor::new(last.occurred_at(), last.id().clone()))
    } else {
      None
    };
    Ok(HistoryPage::new(entries, next_cursor))
  }

  async fn delete_history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    let removed = state
      .history
      .get_mut(owner)
      .is_some_and(|entries| entries.remove(id).is_some());
    if state.history.get(owner).is_some_and(BTreeMap::is_empty) {
      state.history.remove(owner);
    }
    Ok(removed)
  }

  async fn clear_history(&self, owner: &LearnerId) -> Result<u64, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    Ok(state.history.remove(owner).map_or(0, |entries| {
      u64::try_from(entries.len()).unwrap_or(u64::MAX)
    }))
  }
}

#[async_trait]
impl SavedVocabularyStore for InMemoryLearnerStateStore {
  async fn upsert_saved_vocabulary(
    &self,
    entry: SavedVocabularyEntry,
    now: UtcTimestamp,
  ) -> Result<SavedVocabularyWrite, LearnerStateStoreError> {
    let owner = entry.owner().clone();
    let id = entry.id().clone();
    let sense_id = entry.current_sense_id().clone();
    let mut state = mutex_lock(&self.state);
    if state
      .saved_vocabulary
      .values()
      .any(|entries| entries.contains_key(&id))
    {
      return Err(LearnerStateStoreError::Conflict);
    }

    let entries = state.saved_vocabulary.entry(owner).or_default();
    let existing_id = entries.iter().find_map(|(existing_id, existing)| {
      (existing.current_sense_id() == &sense_id).then(|| existing_id.clone())
    });
    if let Some(existing_id) = existing_id {
      let Some(existing) = entries.get_mut(&existing_id) else {
        return Err(LearnerStateStoreError::Conflict);
      };
      existing.refresh(entry.state(), now)?;
      return Ok(SavedVocabularyWrite::Updated(existing.clone()));
    }

    entries.insert(id, entry.clone());
    Ok(SavedVocabularyWrite::Created(entry))
  }

  async fn find_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateStoreError> {
    Ok(
      mutex_lock(&self.state)
        .saved_vocabulary
        .get(owner)
        .and_then(|entries| entries.get(id))
        .cloned(),
    )
  }

  async fn list_saved_vocabulary(
    &self,
    owner: &LearnerId,
    page: SavedVocabularyPageRequest,
  ) -> Result<SavedVocabularyPage, LearnerStateStoreError> {
    let state = mutex_lock(&self.state);
    let mut entries = state
      .saved_vocabulary
      .get(owner)
      .map(|records| {
        records
          .iter()
          .filter(|(id, entry)| {
            page.after().is_none_or(|after| *id > after.id())
              && page.state().is_none_or(|wanted| entry.state() == wanted)
          })
          .map(|(_, entry)| entry.clone())
          .collect::<Vec<_>>()
      })
      .unwrap_or_default();
    let limit = usize::from(page.limit().get());
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    let next_cursor = if has_more {
      entries
        .last()
        .map(|last| SavedVocabularyCursor::new(last.id().clone()))
    } else {
      None
    };
    Ok(SavedVocabularyPage::new(entries, next_cursor))
  }

  async fn mutate_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    expected_revision: u64,
    mutation: SavedSenseMutation,
    at: UtcTimestamp,
  ) -> Result<SavedVocabularyMutationResult, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    let Some(entries) = state.saved_vocabulary.get_mut(owner) else {
      return Ok(SavedVocabularyMutationResult::Missing);
    };
    let Some(entry) = entries.get_mut(id) else {
      return Ok(SavedVocabularyMutationResult::Missing);
    };
    if entry.revision() != expected_revision {
      return Ok(SavedVocabularyMutationResult::VersionConflict {
        current_revision: entry.revision(),
      });
    }

    let outcome = entry.mutate(mutation, at)?;
    Ok(SavedVocabularyMutationResult::Updated {
      entry: Box::new(entry.clone()),
      outcome,
    })
  }

  async fn delete_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    let removed = state
      .saved_vocabulary
      .get_mut(owner)
      .is_some_and(|entries| entries.remove(id).is_some());
    if state
      .saved_vocabulary
      .get(owner)
      .is_some_and(BTreeMap::is_empty)
    {
      state.saved_vocabulary.remove(owner);
    }
    Ok(removed)
  }

  async fn find_by_current_sense(
    &self,
    owner: &LearnerId,
    sense_id: &SenseId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateStoreError> {
    Ok(
      mutex_lock(&self.state)
        .saved_vocabulary
        .get(owner)
        .and_then(|entries| {
          entries
            .values()
            .find(|entry| entry.current_sense_id() == sense_id)
        })
        .cloned(),
    )
  }
}

#[async_trait]
impl LearnerPrivacyInventoryStore for InMemoryLearnerStateStore {
  async fn private_inventory(
    &self,
    owner: &LearnerId,
    now: UtcTimestamp,
  ) -> Result<LearnerDataInventory, LearnerStateStoreError> {
    let mut state = mutex_lock(&self.state);
    purge_expired_history(&mut state, now);
    let history_entries = state.history.get(owner).map_or(0, |entries| {
      u64::try_from(entries.len()).unwrap_or(u64::MAX)
    });
    let saved_vocabulary_entries = state.saved_vocabulary.get(owner).map_or(0, |entries| {
      u64::try_from(entries.len()).unwrap_or(u64::MAX)
    });
    Ok(LearnerDataInventory::new(
      state.profiles.contains_key(owner),
      history_entries,
      saved_vocabulary_entries,
    ))
  }
}

fn purge_expired_history(state: &mut InMemoryLearnerState, now: UtcTimestamp) {
  state.history.retain(|_, entries| {
    entries.retain(|_, entry| !entry.is_expired_at(now));
    !entries.is_empty()
  });
}

fn history_follows_cursor(entry: &HistoryEntry, cursor: &HistoryCursor) -> bool {
  entry.occurred_at() < cursor.occurred_at()
    || (entry.occurred_at() == cursor.occurred_at() && entry.id() < cursor.id())
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU8,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;
  use crate::{
    domain::{
      canonical::{LanguageTag, SenseId},
      learner::{
        AccessibilityPreferences, EnglishLevel, HistoryPreference, HistoryRetention,
        LearnerPreferences, SavedSenseSource, SavedSenseState, SenseSuccessors,
      },
    },
    ports::learner_state::{
      HistoryPageRequest, SavedVocabularyMutationResult, SavedVocabularyPageRequest,
    },
  };

  fn id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn sense(value: &str) -> SenseId {
    SenseId::new(value).unwrap()
  }

  fn preferences() -> LearnerPreferences {
    LearnerPreferences::new(
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("en-US").unwrap(),
      [],
      EnglishLevel::Unknown,
      10,
      HistoryPreference::OptedIn(HistoryRetention::new(2).unwrap()),
      true,
      crate::domain::learner::MatureContentMode::Warn,
      AccessibilityPreferences::default(),
    )
    .unwrap()
  }

  fn history(
    id_value: PublicId,
    owner_value: LearnerId,
    occurred_at: UtcTimestamp,
  ) -> HistoryEntry {
    HistoryEntry::new(
      id_value,
      owner_value,
      None,
      vec![sense("sense-a")],
      occurred_at,
      occurred_at + Duration::from_secs(10),
    )
    .unwrap()
  }

  #[tokio::test]
  async fn profiles_are_optimistic_and_private_history_hides_foreign_and_expired_records() {
    let store = InMemoryLearnerStateStore::new();
    let first = owner("first-owner");
    let second = owner("second-owner");
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let profile = LearnerProfile::new(first.clone(), preferences(), now);
    assert!(matches!(
      store.create_profile(profile.clone()).await.unwrap(),
      ProfileCreateResult::Created(_)
    ));
    assert_eq!(
      store.create_profile(profile).await.unwrap(),
      ProfileCreateResult::AlreadyExists
    );
    assert!(matches!(
      store
        .replace_preferences(&first, 2, preferences(), now)
        .await
        .unwrap(),
      PrivateUpdate::VersionConflict {
        current_revision: 1
      }
    ));

    let entry = history(id(1), first.clone(), now);
    store.append_history(entry.clone()).await.unwrap();
    assert_eq!(
      store.find_history(&second, entry.id(), now).await.unwrap(),
      None
    );
    assert!(!store.delete_history(&second, entry.id()).await.unwrap());
    assert_eq!(
      store
        .find_history(&first, entry.id(), now + Duration::from_secs(10))
        .await
        .unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn history_pages_are_bounded_ordered_and_cursor_stable() {
    let store = InMemoryLearnerStateStore::new();
    let learner = owner("owner-token");
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
    store
      .append_history(history(id(2), learner.clone(), now))
      .await
      .unwrap();
    store
      .append_history(history(
        id(3),
        learner.clone(),
        now + Duration::from_secs(1),
      ))
      .await
      .unwrap();
    store
      .append_history(history(id(4), learner.clone(), now))
      .await
      .unwrap();

    let first = store
      .list_history(
        &learner,
        HistoryPageRequest::new(None, NonZeroU8::new(2).unwrap()).unwrap(),
        now,
      )
      .await
      .unwrap();
    assert_eq!(first.entries().len(), 2);
    assert_eq!(first.entries()[0].id(), &id(3));
    assert_eq!(first.entries()[1].id(), &id(4));
    let second = store
      .list_history(
        &learner,
        HistoryPageRequest::new(first.next_cursor().cloned(), NonZeroU8::new(2).unwrap()).unwrap(),
        now,
      )
      .await
      .unwrap();
    assert_eq!(second.entries().len(), 1);
    assert_eq!(second.entries()[0].id(), &id(2));
  }

  #[tokio::test]
  async fn saved_entries_upsert_successors_and_reopen_only_same_process_memory() {
    let store = InMemoryLearnerStateStore::new();
    let reopened = store.reopen();
    let learner = owner("owner-token");
    let other = owner("other-token");
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(300);
    let entry = SavedVocabularyEntry::new(
      id(10),
      learner.clone(),
      sense("old-sense"),
      SavedSenseState::Learning,
      SavedSenseSource::Lookup,
      now,
    );
    let created = match store.upsert_saved_vocabulary(entry, now).await.unwrap() {
      SavedVocabularyWrite::Created(entry) => entry,
      _ => panic!("first upsert must create"),
    };
    let refreshing = SavedVocabularyEntry::new(
      id(11),
      learner.clone(),
      sense("old-sense"),
      SavedSenseState::Known,
      SavedSenseSource::Saved,
      now + Duration::from_secs(1),
    );
    assert!(matches!(
      store
        .upsert_saved_vocabulary(refreshing, now + Duration::from_secs(1))
        .await
        .unwrap(),
      SavedVocabularyWrite::Updated(ref entry) if entry.id() == created.id() && entry.state() == SavedSenseState::Known
    ));
    let stored = reopened
      .find_saved_vocabulary(&learner, created.id())
      .await
      .unwrap()
      .unwrap();
    assert!(matches!(
      reopened
        .mutate_saved_vocabulary(
          &learner,
          stored.id(),
          stored.revision(),
          SavedSenseMutation::ApplySuccessors(
            SenseSuccessors::new(sense("old-sense"), vec![sense("new-a"), sense("new-b")],)
              .unwrap(),
          ),
          now + Duration::from_secs(2),
        )
        .await
        .unwrap(),
      SavedVocabularyMutationResult::Updated { .. }
    ));
    assert!(matches!(
      reopened
        .mutate_saved_vocabulary(
          &other,
          created.id(),
          2,
          SavedSenseMutation::MarkSeen,
          now + Duration::from_secs(3),
        )
        .await
        .unwrap(),
      SavedVocabularyMutationResult::Missing
    ));
    let page = reopened
      .list_saved_vocabulary(
        &learner,
        SavedVocabularyPageRequest::new(None, NonZeroU8::new(10).unwrap(), None).unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(page.entries().len(), 1);
  }
}
