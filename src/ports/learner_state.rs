//! Private learner-owned state persistence contracts.
//!
//! These interfaces preserve ownership-safe missing semantics: a caller querying a record through
//! the wrong [`crate::domain::learner::LearnerId`] receives the same missing result as for an
//! absent record. They do not authenticate callers, hash owner references, encrypt source text or
//! notes, provide cross-resource transactions, or implement durable privacy workflows.

use std::num::NonZeroU8;

use async_trait::async_trait;
use thiserror::Error;

use crate::{
  domain::{
    canonical::SenseId,
    learner::{
      HistoryEntry, LearnerDataInventory, LearnerId, LearnerPreferences, LearnerProfile,
      LearnerProfileError, SavedSenseMutation, SavedSenseMutationOutcome, SavedSenseState,
      SavedVocabularyEntry, SavedVocabularyError,
    },
  },
  ports::{clock::UtcTimestamp, public_id::PublicId},
};

/// Largest number of private history records one page may request.
pub const MAX_HISTORY_PAGE_SIZE: u8 = 100;
/// Largest number of saved vocabulary entries one page may request.
pub const MAX_SAVED_VOCABULARY_PAGE_SIZE: u8 = 100;

/// Failure returned by a learner-state storage dependency.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LearnerStateStoreError {
  /// The private-state dependency cannot currently serve the operation.
  #[error("learner state store is unavailable")]
  Unavailable,
  /// An atomic storage operation conflicted with existing private state.
  #[error("learner state store operation conflicted")]
  Conflict,
  /// A profile transition violated a pure profile invariant before it could be stored.
  #[error(transparent)]
  Profile(#[from] LearnerProfileError),
  /// A saved-vocabulary transition violated a pure lifecycle or successor invariant.
  #[error(transparent)]
  SavedVocabulary(#[from] SavedVocabularyError),
}

/// Opaque history pagination position ordered by occurrence time descending and ID descending.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HistoryCursor {
  occurred_at: UtcTimestamp,
  id: PublicId,
}

impl HistoryCursor {
  /// Creates a cursor positioned immediately after the supplied history ordering key.
  pub fn new(occurred_at: UtcTimestamp, id: PublicId) -> Self {
    Self { occurred_at, id }
  }

  /// Returns the occurrence instant used in the deterministic history ordering.
  pub const fn occurred_at(&self) -> UtcTimestamp {
    self.occurred_at
  }

  /// Returns the stable history ID used to break same-instant ordering ties.
  pub fn id(&self) -> &PublicId {
    &self.id
  }
}

/// Validated bounded page request for private history metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPageRequest {
  before: Option<HistoryCursor>,
  limit: NonZeroU8,
}

impl HistoryPageRequest {
  /// Creates a bounded history page request.
  ///
  /// # Errors
  ///
  /// Returns [`HistoryPageRequestError::LimitTooLarge`] when `limit` exceeds the public bound.
  pub fn new(
    before: Option<HistoryCursor>,
    limit: NonZeroU8,
  ) -> Result<Self, HistoryPageRequestError> {
    if limit.get() > MAX_HISTORY_PAGE_SIZE {
      return Err(HistoryPageRequestError::LimitTooLarge);
    }

    Ok(Self { before, limit })
  }

  /// Returns the exclusive previous-page cursor, when one was supplied.
  pub fn before(&self) -> Option<&HistoryCursor> {
    self.before.as_ref()
  }

  /// Returns the bounded number of records requested.
  pub const fn limit(&self) -> NonZeroU8 {
    self.limit
  }
}

/// Invalid history-page request.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum HistoryPageRequestError {
  /// The requested history page would exceed the bounded contract.
  #[error("history page limit is too large")]
  LimitTooLarge,
}

/// One bounded page of owned, unexpired history metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPage {
  entries: Vec<HistoryEntry>,
  next_cursor: Option<HistoryCursor>,
}

impl HistoryPage {
  /// Creates one page from deterministic ordered entries and an optional next cursor.
  pub fn new(entries: Vec<HistoryEntry>, next_cursor: Option<HistoryCursor>) -> Self {
    Self {
      entries,
      next_cursor,
    }
  }

  /// Returns history entries in occurrence-descending, ID-descending order.
  pub fn entries(&self) -> &[HistoryEntry] {
    &self.entries
  }

  /// Returns the exclusive cursor for the next older page, when more records exist.
  pub fn next_cursor(&self) -> Option<&HistoryCursor> {
    self.next_cursor.as_ref()
  }
}

/// Opaque saved-vocabulary pagination position ordered by stable entry ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SavedVocabularyCursor {
  id: PublicId,
}

impl SavedVocabularyCursor {
  /// Creates a cursor positioned immediately after the supplied saved vocabulary entry ID.
  pub fn new(id: PublicId) -> Self {
    Self { id }
  }

  /// Returns the stable entry ID used for deterministic pagination.
  pub fn id(&self) -> &PublicId {
    &self.id
  }
}

/// Validated bounded page request for owned saved vocabulary entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedVocabularyPageRequest {
  after: Option<SavedVocabularyCursor>,
  limit: NonZeroU8,
  state: Option<SavedSenseState>,
}

impl SavedVocabularyPageRequest {
  /// Creates a bounded saved-vocabulary page request with an optional lifecycle-state filter.
  ///
  /// # Errors
  ///
  /// Returns [`SavedVocabularyPageRequestError::LimitTooLarge`] when `limit` exceeds the public
  /// bound.
  pub fn new(
    after: Option<SavedVocabularyCursor>,
    limit: NonZeroU8,
    state: Option<SavedSenseState>,
  ) -> Result<Self, SavedVocabularyPageRequestError> {
    if limit.get() > MAX_SAVED_VOCABULARY_PAGE_SIZE {
      return Err(SavedVocabularyPageRequestError::LimitTooLarge);
    }

    Ok(Self {
      after,
      limit,
      state,
    })
  }

  /// Returns the exclusive prior-page cursor, when one was supplied.
  pub fn after(&self) -> Option<&SavedVocabularyCursor> {
    self.after.as_ref()
  }

  /// Returns the bounded number of entries requested.
  pub const fn limit(&self) -> NonZeroU8 {
    self.limit
  }

  /// Returns the optional lifecycle-state filter.
  pub const fn state(&self) -> Option<SavedSenseState> {
    self.state
  }
}

/// Invalid saved-vocabulary page request.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SavedVocabularyPageRequestError {
  /// The requested saved-vocabulary page would exceed the bounded contract.
  #[error("saved vocabulary page limit is too large")]
  LimitTooLarge,
}

/// One bounded page of owned saved vocabulary entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedVocabularyPage {
  entries: Vec<SavedVocabularyEntry>,
  next_cursor: Option<SavedVocabularyCursor>,
}

impl SavedVocabularyPage {
  /// Creates one page from deterministic entry-ID order and an optional next cursor.
  pub fn new(
    entries: Vec<SavedVocabularyEntry>,
    next_cursor: Option<SavedVocabularyCursor>,
  ) -> Self {
    Self {
      entries,
      next_cursor,
    }
  }

  /// Returns saved vocabulary entries in stable entry-ID order.
  pub fn entries(&self) -> &[SavedVocabularyEntry] {
    &self.entries
  }

  /// Returns the exclusive cursor for the next page, when more entries exist.
  pub fn next_cursor(&self) -> Option<&SavedVocabularyCursor> {
    self.next_cursor.as_ref()
  }
}

/// Result of trying to create a private learner profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileCreateResult {
  /// The private profile was stored for the first time.
  Created(LearnerProfile),
  /// A profile for the same opaque owner already exists.
  AlreadyExists,
}

/// Ownership-safe result of an optimistic private state update.
///
/// `Missing` deliberately covers both absent resources and resources owned by a different learner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateUpdate<Value> {
  /// The owned record was updated and its resulting snapshot is returned.
  Updated(Value),
  /// The record is absent or not owned by the supplied opaque learner.
  Missing,
  /// The owned record exists but its current revision differs from the caller's expected revision.
  VersionConflict {
    /// Current revision visible only after a successful ownership match.
    current_revision: u64,
  },
}

/// Result of atomically saving one vocabulary sense for an owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedVocabularyWrite {
  /// A new stable entry was created.
  Created(SavedVocabularyEntry),
  /// An existing current-sense entry was refreshed without allocating a duplicate entry.
  Updated(SavedVocabularyEntry),
}

/// Result of atomically mutating one owned saved vocabulary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedVocabularyMutationResult {
  /// The owned entry was mutated and its resulting snapshot is returned.
  Updated {
    /// Stored entry after the mutation.
    entry: Box<SavedVocabularyEntry>,
    /// Domain-level effect of the mutation.
    outcome: SavedSenseMutationOutcome,
  },
  /// The entry is absent or owned by a different learner.
  Missing,
  /// The owned entry exists but was modified after the caller's expected revision.
  VersionConflict {
    /// Current revision visible only after a successful ownership match.
    current_revision: u64,
  },
}

/// Stores and updates private learner profiles and preference documents.
#[async_trait]
pub trait LearnerProfileStore: Send + Sync {
  /// Creates a profile unless the opaque owner already has one.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot atomically persist the private profile.
  async fn create_profile(
    &self,
    profile: LearnerProfile,
  ) -> Result<ProfileCreateResult, LearnerStateStoreError>;

  /// Finds the private profile only when it is owned by `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the read.
  async fn find_profile(
    &self,
    owner: &LearnerId,
  ) -> Result<Option<LearnerProfile>, LearnerStateStoreError>;

  /// Atomically replaces preferences when `expected_revision` matches the owned profile.
  ///
  /// The returned [`PrivateUpdate::Missing`] must not distinguish an absent profile from another
  /// learner's profile. A production implementation must execute the owner and revision check in
  /// the same persistence operation.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot complete the private update.
  async fn replace_preferences(
    &self,
    owner: &LearnerId,
    expected_revision: u64,
    preferences: LearnerPreferences,
    updated_at: UtcTimestamp,
  ) -> Result<PrivateUpdate<LearnerProfile>, LearnerStateStoreError>;
}

/// Stores retention-bounded private history metadata.
#[async_trait]
pub trait LearnerHistoryStore: Send + Sync {
  /// Persists one private history entry after its opt-in retention decision has been made.
  ///
  /// Implementations must reject duplicate event IDs and must not retain the entry after its
  /// non-null expiry. The entry cannot contain raw lookup query or context data.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot atomically persist the entry.
  async fn append_history(&self, entry: HistoryEntry) -> Result<(), LearnerStateStoreError>;

  /// Finds an unexpired history entry only when it is owned by `owner`.
  ///
  /// `None` covers absent, expired, and not-owned records.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the read or purge expired metadata.
  async fn find_history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    now: UtcTimestamp,
  ) -> Result<Option<HistoryEntry>, LearnerStateStoreError>;

  /// Returns one bounded page of unexpired history owned by `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the read or purge expired metadata.
  async fn list_history(
    &self,
    owner: &LearnerId,
    page: HistoryPageRequest,
    now: UtcTimestamp,
  ) -> Result<HistoryPage, LearnerStateStoreError>;

  /// Deletes one private history record only when it is owned by `owner`.
  ///
  /// `false` covers absent and not-owned records.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot complete the deletion.
  async fn delete_history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateStoreError>;

  /// Deletes all history metadata owned by `owner` and returns the number removed.
  ///
  /// The result does not include raw text because this port never accepts raw query or context
  /// fields. A production clear-history workflow still requires durable privacy-job orchestration.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot complete the deletion.
  async fn clear_history(&self, owner: &LearnerId) -> Result<u64, LearnerStateStoreError>;
}

/// Stores private saved vocabulary entries and successor-aware lifecycle transitions.
#[async_trait]
pub trait SavedVocabularyStore: Send + Sync {
  /// Creates or refreshes one entry for the owner's current canonical sense atomically.
  ///
  /// When an entry already tracks the same current sense, implementations must refresh it rather
  /// than create a duplicate. The supplied entry ID may be unused in that update case.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot atomically write the entry.
  async fn upsert_saved_vocabulary(
    &self,
    entry: SavedVocabularyEntry,
    now: UtcTimestamp,
  ) -> Result<SavedVocabularyWrite, LearnerStateStoreError>;

  /// Finds a saved entry only when it is owned by `owner`.
  ///
  /// `None` covers absent and not-owned records.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the private read.
  async fn find_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateStoreError>;

  /// Returns one bounded page of saved vocabulary owned by `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the private read.
  async fn list_saved_vocabulary(
    &self,
    owner: &LearnerId,
    page: SavedVocabularyPageRequest,
  ) -> Result<SavedVocabularyPage, LearnerStateStoreError>;

  /// Applies one optimistic lifecycle or successor mutation to an owned entry.
  ///
  /// `Missing` deliberately covers absent and not-owned records. Implementations must verify
  /// ownership and expected revision in the same storage operation.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot complete the private mutation.
  async fn mutate_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    expected_revision: u64,
    mutation: SavedSenseMutation,
    at: UtcTimestamp,
  ) -> Result<SavedVocabularyMutationResult, LearnerStateStoreError>;

  /// Deletes a saved entry only when it is owned by `owner`.
  ///
  /// `false` covers absent and not-owned records.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot complete the deletion.
  async fn delete_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateStoreError>;

  /// Finds the one entry currently tracking `sense_id`, only when it is owned by `owner`.
  ///
  /// This helper is intended for private current-sense reopening; `None` covers absent and
  /// not-owned records. Implementations must choose deterministic behavior if legacy data
  /// contains duplicates until a production migration resolves them.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the private read.
  async fn find_by_current_sense(
    &self,
    owner: &LearnerId,
    sense_id: &SenseId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateStoreError>;
}

/// Provides count-only locally owned data inventory for export and deletion planning.
#[async_trait]
pub trait LearnerPrivacyInventoryStore: Send + Sync {
  /// Returns a count-only inventory after excluding history records expired at `now`.
  ///
  /// This is planning input only and must not be treated as proof that a durable export or account
  /// deletion transaction has completed.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot inspect and purge the local private inventory.
  async fn private_inventory(
    &self,
    owner: &LearnerId,
    now: UtcTimestamp,
  ) -> Result<LearnerDataInventory, LearnerStateStoreError>;
}

/// Complete private learner-state dependency used by learner application services.
pub trait LearnerStateStore:
  LearnerProfileStore + LearnerHistoryStore + SavedVocabularyStore + LearnerPrivacyInventoryStore
{
}

impl<Value> LearnerStateStore for Value where
  Value:
    LearnerProfileStore + LearnerHistoryStore + SavedVocabularyStore + LearnerPrivacyInventoryStore
{
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU8,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;

  #[test]
  fn private_pages_enforce_explicit_bounds() {
    let id = PublicId::from(Ulid::from_parts(1_700_000_000_000, 1));
    let cursor = HistoryCursor::new(SystemTime::UNIX_EPOCH + Duration::from_secs(1), id);

    assert!(HistoryPageRequest::new(Some(cursor), NonZeroU8::new(100).unwrap()).is_ok());
    assert_eq!(
      HistoryPageRequest::new(None, NonZeroU8::new(101).unwrap()),
      Err(HistoryPageRequestError::LimitTooLarge)
    );
    assert_eq!(
      SavedVocabularyPageRequest::new(None, NonZeroU8::new(101).unwrap(), None),
      Err(SavedVocabularyPageRequestError::LimitTooLarge)
    );
  }
}
