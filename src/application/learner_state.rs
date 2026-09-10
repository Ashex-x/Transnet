//! Private learner-profile, history, saved-vocabulary, and privacy-plan orchestration.
//!
//! The service accepts only a trusted opaque [`crate::domain::learner::LearnerId`] supplied by a
//! future authentication boundary. It does not implement OIDC, session issuance, encryption,
//! MySQL transactions, durable privacy jobs, or HTTP routes.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  domain::{
    canonical::SenseId,
    learner::{
      AccountDeletionPlan, ExportPlan, HistoryDecision, HistoryEntry, HistoryEntryError,
      HistoryMode, HistoryRetentionError, HistorySuppressionReason, LearnerId, LearnerPreferences,
      LearnerProfile, SavedSenseMutation, SavedSenseSource, SavedSenseState, SavedVocabularyEntry,
    },
  },
  ports::{
    clock::Clock,
    learner_state::{
      HistoryPage, HistoryPageRequest, LearnerStateStore, LearnerStateStoreError, PrivateUpdate,
      ProfileCreateResult, SavedVocabularyMutationResult, SavedVocabularyPage,
      SavedVocabularyPageRequest, SavedVocabularyWrite,
    },
    public_id::{PublicId, PublicIdGenerationError, PublicIdGenerator},
  },
};

/// Metadata-only input needed to decide whether one lookup may create history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordHistoryRequest {
  mode: HistoryMode,
  snapshot_id: Option<PublicId>,
  sense_ids: Vec<SenseId>,
}

impl RecordHistoryRequest {
  /// Creates a record request without raw query, context, note, or credential fields.
  ///
  /// [`HistoryEntry`] validates the bounded and distinct canonical sense references only after an
  /// opt-in decision authorizes record allocation.
  pub fn new(mode: HistoryMode, snapshot_id: Option<PublicId>, sense_ids: Vec<SenseId>) -> Self {
    Self {
      mode,
      snapshot_id,
      sense_ids,
    }
  }

  /// Returns the per-request history or incognito choice.
  pub const fn mode(&self) -> HistoryMode {
    self.mode
  }

  /// Returns the optional context-free canonical snapshot reference.
  pub fn snapshot_id(&self) -> Option<&PublicId> {
    self.snapshot_id.as_ref()
  }

  /// Returns the canonical senses referenced by the lookup result in result order.
  pub fn sense_ids(&self) -> &[SenseId] {
    &self.sense_ids
  }
}

/// Input required to save or refresh one canonical vocabulary sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveVocabularyRequest {
  sense_id: SenseId,
  state: SavedSenseState,
  source: SavedSenseSource,
}

impl SaveVocabularyRequest {
  /// Creates a save request for one current canonical sense.
  pub fn new(sense_id: SenseId, state: SavedSenseState, source: SavedSenseSource) -> Self {
    Self {
      sense_id,
      state,
      source,
    }
  }

  /// Returns the current canonical sense to save or refresh.
  pub fn sense_id(&self) -> &SenseId {
    &self.sense_id
  }

  /// Returns the requested saved-vocabulary lifecycle state.
  pub const fn state(&self) -> SavedSenseState {
    self.state
  }

  /// Returns the origin of this save action for a newly created entry.
  pub const fn source(&self) -> SavedSenseSource {
    self.source
  }
}

/// Result of attempting to record a lookup in private history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordHistoryOutcome {
  /// One bounded metadata-only history record was stored.
  Recorded(HistoryEntry),
  /// No history ID or private cache record may be created for this lookup.
  Suppressed {
    /// Privacy rule requiring the no-retention behavior.
    reason: HistorySuppressionReason,
  },
}

/// Result of attempting to save vocabulary for one opaque learner owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveVocabularyOutcome {
  /// A private entry was created for an existing profile.
  Created(SavedVocabularyEntry),
  /// An existing private entry for the current sense was refreshed.
  Updated(SavedVocabularyEntry),
  /// No local profile exists, so the service made no private-state write.
  MissingProfile,
}

/// Coordinates bounded private learner state through explicit persistence ports.
///
/// The caller must derive `LearnerId` from a verified authentication principal. This service makes
/// no authorization decision and is deliberately not connected to HTTP handlers in this branch.
#[derive(Clone)]
pub struct LearnerStateService {
  store: Arc<dyn LearnerStateStore>,
  clock: Arc<dyn Clock>,
  ids: Arc<dyn PublicIdGenerator>,
}

impl LearnerStateService {
  /// Creates learner-state orchestration with injected storage, UTC time, and public IDs.
  pub fn new(
    store: Arc<dyn LearnerStateStore>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn PublicIdGenerator>,
  ) -> Self {
    Self { store, clock, ids }
  }

  /// Creates the initial profile for an authenticated learner.
  ///
  /// # Errors
  ///
  /// Returns an error when the private profile cannot be stored.
  pub async fn create_profile(
    &self,
    owner: LearnerId,
    preferences: LearnerPreferences,
  ) -> Result<ProfileCreateResult, LearnerStateServiceError> {
    let profile = LearnerProfile::new(owner, preferences, self.clock.now());
    Ok(self.store.create_profile(profile).await?)
  }

  /// Finds the profile owned by `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn profile(
    &self,
    owner: &LearnerId,
  ) -> Result<Option<LearnerProfile>, LearnerStateServiceError> {
    Ok(self.store.find_profile(owner).await?)
  }

  /// Replaces owned preferences when `expected_revision` matches.
  ///
  /// [`PrivateUpdate::Missing`] intentionally covers absent and not-owned profiles.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot apply the update.
  pub async fn replace_preferences(
    &self,
    owner: &LearnerId,
    expected_revision: u64,
    preferences: LearnerPreferences,
  ) -> Result<PrivateUpdate<LearnerProfile>, LearnerStateServiceError> {
    Ok(
      self
        .store
        .replace_preferences(owner, expected_revision, preferences, self.clock.now())
        .await?,
    )
  }

  /// Selects the history decision for an owner and per-request privacy mode before ID allocation.
  ///
  /// A missing profile is deliberately treated as incognito and returns no history identifier.
  ///
  /// # Errors
  ///
  /// Returns an error when the profile cannot be read or its configured retention expiry cannot be
  /// represented.
  pub async fn decide_history(
    &self,
    owner: &LearnerId,
    mode: HistoryMode,
  ) -> Result<HistoryDecision, LearnerStateServiceError> {
    self.decide_history_at(owner, mode, self.clock.now()).await
  }

  /// Applies history preference and records non-sensitive metadata only when retention is allowed.
  ///
  /// The service allocates a public history ID only after the opt-in decision is `Record`; an
  /// incognito or disabled request therefore consumes no ID and creates no private state.
  ///
  /// # Errors
  ///
  /// Returns an error when an allowed entry cannot be assigned an ID, fails invariant validation,
  /// or cannot be persisted.
  pub async fn record_history(
    &self,
    owner: &LearnerId,
    request: RecordHistoryRequest,
  ) -> Result<RecordHistoryOutcome, LearnerStateServiceError> {
    let now = self.clock.now();
    let decision = self.decide_history_at(owner, request.mode(), now).await?;
    let expires_at = match decision {
      HistoryDecision::Record { expires_at } => expires_at,
      HistoryDecision::Suppress { reason } => {
        return Ok(RecordHistoryOutcome::Suppressed { reason });
      }
    };

    let entry = HistoryEntry::new(
      self.ids.generate()?,
      owner.clone(),
      request.snapshot_id().cloned(),
      request.sense_ids().to_vec(),
      now,
      expires_at,
    )?;
    self.store.append_history(entry.clone()).await?;
    Ok(RecordHistoryOutcome::Recorded(entry))
  }

  /// Finds an unexpired history entry only for its opaque owner.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<HistoryEntry>, LearnerStateServiceError> {
    Ok(self.store.find_history(owner, id, self.clock.now()).await?)
  }

  /// Lists a bounded page of unexpired owned history metadata.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn history_page(
    &self,
    owner: &LearnerId,
    page: HistoryPageRequest,
  ) -> Result<HistoryPage, LearnerStateServiceError> {
    Ok(
      self
        .store
        .list_history(owner, page, self.clock.now())
        .await?,
    )
  }

  /// Deletes one owned history record without distinguishing absent and foreign records.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot complete the deletion.
  pub async fn delete_history(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateServiceError> {
    Ok(self.store.delete_history(owner, id).await?)
  }

  /// Deletes all locally retained history metadata for the supplied owner.
  ///
  /// This is an immediate local operation only. Public API behavior should instead arrange a
  /// durable, authorized privacy workflow before promising completion.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot complete the deletion.
  pub async fn clear_history(&self, owner: &LearnerId) -> Result<u64, LearnerStateServiceError> {
    Ok(self.store.clear_history(owner).await?)
  }

  /// Creates or refreshes an owned saved vocabulary entry after confirming a local profile exists.
  ///
  /// # Errors
  ///
  /// Returns an error when an entry cannot be assigned a public ID or the store cannot apply the
  /// atomic save-or-refresh operation.
  pub async fn save_vocabulary(
    &self,
    owner: &LearnerId,
    request: SaveVocabularyRequest,
  ) -> Result<SaveVocabularyOutcome, LearnerStateServiceError> {
    if self.store.find_profile(owner).await?.is_none() {
      return Ok(SaveVocabularyOutcome::MissingProfile);
    }

    let now = self.clock.now();
    let entry = SavedVocabularyEntry::new(
      self.ids.generate()?,
      owner.clone(),
      request.sense_id().clone(),
      request.state(),
      request.source(),
      now,
    );
    match self.store.upsert_saved_vocabulary(entry, now).await? {
      SavedVocabularyWrite::Created(entry) => Ok(SaveVocabularyOutcome::Created(entry)),
      SavedVocabularyWrite::Updated(entry) => Ok(SaveVocabularyOutcome::Updated(entry)),
    }
  }

  /// Finds one saved vocabulary entry only when it is owned by `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateServiceError> {
    Ok(self.store.find_saved_vocabulary(owner, id).await?)
  }

  /// Finds the one owned saved entry currently tracking `sense_id`.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn saved_vocabulary_for_sense(
    &self,
    owner: &LearnerId,
    sense_id: &SenseId,
  ) -> Result<Option<SavedVocabularyEntry>, LearnerStateServiceError> {
    Ok(self.store.find_by_current_sense(owner, sense_id).await?)
  }

  /// Lists a bounded page of owned saved vocabulary state.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot serve the read.
  pub async fn saved_vocabulary_page(
    &self,
    owner: &LearnerId,
    page: SavedVocabularyPageRequest,
  ) -> Result<SavedVocabularyPage, LearnerStateServiceError> {
    Ok(self.store.list_saved_vocabulary(owner, page).await?)
  }

  /// Applies a lifecycle or canonical-successor update to an owned saved entry.
  ///
  /// The returned missing result deliberately hides foreign entry IDs. A caller must pass the
  /// revision returned by its most recent owned snapshot.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot apply the mutation.
  pub async fn mutate_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    expected_revision: u64,
    mutation: SavedSenseMutation,
  ) -> Result<SavedVocabularyMutationResult, LearnerStateServiceError> {
    Ok(
      self
        .store
        .mutate_saved_vocabulary(owner, id, expected_revision, mutation, self.clock.now())
        .await?,
    )
  }

  /// Deletes one owned saved vocabulary entry without distinguishing absent and foreign records.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot complete the deletion.
  pub async fn delete_saved_vocabulary(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<bool, LearnerStateServiceError> {
    Ok(self.store.delete_saved_vocabulary(owner, id).await?)
  }

  /// Builds a count-only portable-export manifest for local private state.
  ///
  /// The result is not an export job, archive, capability, or authorization decision.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot inspect the current inventory.
  pub async fn plan_export(
    &self,
    owner: &LearnerId,
  ) -> Result<ExportPlan, LearnerStateServiceError> {
    let inventory = self
      .store
      .private_inventory(owner, self.clock.now())
      .await?;
    Ok(ExportPlan::for_inventory(owner.clone(), inventory))
  }

  /// Builds the required ordered local-and-external account-deletion manifest.
  ///
  /// The result makes no destructive change. A production workflow must authenticate the request,
  /// persist it, revoke sessions, delete all private boundaries, and audit completion separately.
  ///
  /// # Errors
  ///
  /// Returns an error when the private-state store cannot inspect the current inventory.
  pub async fn plan_account_deletion(
    &self,
    owner: &LearnerId,
  ) -> Result<AccountDeletionPlan, LearnerStateServiceError> {
    let inventory = self
      .store
      .private_inventory(owner, self.clock.now())
      .await?;
    Ok(AccountDeletionPlan::for_inventory(owner.clone(), inventory))
  }

  async fn decide_history_at(
    &self,
    owner: &LearnerId,
    mode: HistoryMode,
    now: std::time::SystemTime,
  ) -> Result<HistoryDecision, LearnerStateServiceError> {
    let Some(profile) = self.store.find_profile(owner).await? else {
      return Ok(HistoryDecision::Suppress {
        reason: HistorySuppressionReason::MissingProfile,
      });
    };

    Ok(profile.preferences().history_decision(mode, now)?)
  }
}

/// Failure returned by private learner-state orchestration.
#[derive(Debug, Error)]
pub enum LearnerStateServiceError {
  /// A private storage port could not complete the requested operation.
  #[error(transparent)]
  Store(#[from] LearnerStateStoreError),
  /// A public history or saved-entry ID could not be generated.
  #[error("could not generate learner-state public ID: {0}")]
  IdGeneration(#[from] PublicIdGenerationError),
  /// An opt-in history retention period could not produce a future expiry instant.
  #[error(transparent)]
  Retention(#[from] HistoryRetentionError),
  /// Proposed private history metadata violated a bounded domain invariant.
  #[error(transparent)]
  HistoryEntry(#[from] HistoryEntryError),
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU8,
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock, in_memory::InMemoryLearnerStateStore, public_id::SequencePublicIdGenerator,
    },
    domain::{
      canonical::LanguageTag,
      learner::{
        AccessibilityPreferences, EnglishLevel, HistoryPreference, HistoryRetention,
        MatureContentMode, SavedSenseMutationOutcome, SenseEvolutionOutcome, SenseSuccessors,
      },
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

  fn preferences(history: HistoryPreference) -> LearnerPreferences {
    LearnerPreferences::new(
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("en-US").unwrap(),
      [LanguageTag::parse("zh-CN").unwrap()],
      EnglishLevel::B1,
      10,
      history,
      true,
      MatureContentMode::Warn,
      AccessibilityPreferences::default(),
    )
    .unwrap()
  }

  fn service(
    now: SystemTime,
    ids: Vec<PublicId>,
  ) -> (
    LearnerStateService,
    Arc<FixedClock>,
    Arc<InMemoryLearnerStateStore>,
  ) {
    let clock = Arc::new(FixedClock::new(now));
    let store = Arc::new(InMemoryLearnerStateStore::new());
    let ids: Arc<dyn PublicIdGenerator> = Arc::new(SequencePublicIdGenerator::new(ids));
    let service = LearnerStateService::new(store.clone(), clock.clone(), ids);
    (service, clock, store)
  }

  #[tokio::test]
  async fn incognito_and_missing_profiles_create_no_history_id_or_private_record() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let (service, _, _) = service(now, vec![id(1), id(2)]);
    let learner = owner("owner-token");
    service
      .create_profile(
        learner.clone(),
        preferences(HistoryPreference::OptedIn(
          HistoryRetention::new(90).unwrap(),
        )),
      )
      .await
      .unwrap();

    assert_eq!(
      service
        .record_history(
          &learner,
          RecordHistoryRequest::new(HistoryMode::Incognito, None, vec![sense("sense-a")]),
        )
        .await
        .unwrap(),
      RecordHistoryOutcome::Suppressed {
        reason: HistorySuppressionReason::Incognito,
      }
    );
    assert_eq!(
      service
        .record_history(
          &owner("missing-owner"),
          RecordHistoryRequest::new(HistoryMode::Default, None, vec![sense("sense-a")]),
        )
        .await
        .unwrap(),
      RecordHistoryOutcome::Suppressed {
        reason: HistorySuppressionReason::MissingProfile,
      }
    );

    let recorded = service
      .record_history(
        &learner,
        RecordHistoryRequest::new(HistoryMode::Default, None, vec![sense("sense-a")]),
      )
      .await
      .unwrap();
    assert!(matches!(
      recorded,
      RecordHistoryOutcome::Recorded(ref entry) if entry.id() == &id(1)
    ));
  }

  #[tokio::test]
  async fn history_expiry_and_cross_owner_reads_are_indistinguishable() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
    let (service, clock, _) = service(now, vec![id(10)]);
    let first = owner("first-owner");
    let second = owner("second-owner");
    service
      .create_profile(
        first.clone(),
        preferences(HistoryPreference::OptedIn(
          HistoryRetention::new(1).unwrap(),
        )),
      )
      .await
      .unwrap();
    let event = match service
      .record_history(
        &first,
        RecordHistoryRequest::new(HistoryMode::Default, None, vec![sense("sense-a")]),
      )
      .await
      .unwrap()
    {
      RecordHistoryOutcome::Recorded(entry) => entry,
      RecordHistoryOutcome::Suppressed { .. } => panic!("opted-in profile must record history"),
    };

    assert_eq!(service.history(&second, event.id()).await.unwrap(), None);
    assert!(!service.delete_history(&second, event.id()).await.unwrap());
    clock.advance(Duration::from_secs(86_400));
    assert_eq!(service.history(&first, event.id()).await.unwrap(), None);
    let page = service
      .history_page(
        &first,
        HistoryPageRequest::new(None, NonZeroU8::new(10).unwrap()).unwrap(),
      )
      .await
      .unwrap();
    assert!(page.entries().is_empty());
  }

  #[tokio::test]
  async fn saved_vocabulary_upserts_then_handles_successor_choices_without_cross_owner_leaks() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(300);
    let (service, _, _) = service(now, vec![id(20), id(21), id(22)]);
    let learner = owner("learner-owner");
    service
      .create_profile(learner.clone(), preferences(HistoryPreference::Disabled))
      .await
      .unwrap();

    let created = match service
      .save_vocabulary(
        &learner,
        SaveVocabularyRequest::new(
          sense("retired-sense"),
          SavedSenseState::Learning,
          SavedSenseSource::Lookup,
        ),
      )
      .await
      .unwrap()
    {
      SaveVocabularyOutcome::Created(entry) => entry,
      _ => panic!("first save must create an entry"),
    };
    assert!(matches!(
      service
        .save_vocabulary(
          &learner,
          SaveVocabularyRequest::new(
            sense("retired-sense"),
            SavedSenseState::Known,
            SavedSenseSource::Saved,
          ),
        )
        .await
        .unwrap(),
      SaveVocabularyOutcome::Updated(entry) if entry.id() == created.id() && entry.state() == SavedSenseState::Known
    ));

    let current = service
      .saved_vocabulary(&learner, created.id())
      .await
      .unwrap()
      .unwrap();
    let result = service
      .mutate_saved_vocabulary(
        &learner,
        created.id(),
        current.revision(),
        SavedSenseMutation::ApplySuccessors(
          SenseSuccessors::new(
            sense("retired-sense"),
            vec![sense("successor-a"), sense("successor-b")],
          )
          .unwrap(),
        ),
      )
      .await
      .unwrap();
    let paused = match result {
      SavedVocabularyMutationResult::Updated { entry, outcome } => {
        assert!(matches!(
          outcome,
          SavedSenseMutationOutcome::Evolution(SenseEvolutionOutcome::AwaitingSelection { .. })
        ));
        entry
      }
      _ => panic!("owned entry must mutate"),
    };
    assert_eq!(paused.state(), SavedSenseState::Paused);
    assert!(matches!(
      service
        .mutate_saved_vocabulary(
          &owner("other-owner"),
          created.id(),
          paused.revision(),
          SavedSenseMutation::MarkSeen,
        )
        .await
        .unwrap(),
      SavedVocabularyMutationResult::Missing
    ));
  }

  #[tokio::test]
  async fn planning_inventory_is_count_only_and_does_not_execute_deletion() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(400);
    let (service, _, _) = service(now, vec![id(30), id(31)]);
    let learner = owner("learner-owner");
    service
      .create_profile(
        learner.clone(),
        preferences(HistoryPreference::OptedIn(
          HistoryRetention::new(2).unwrap(),
        )),
      )
      .await
      .unwrap();
    service
      .record_history(
        &learner,
        RecordHistoryRequest::new(HistoryMode::Default, None, vec![sense("sense-a")]),
      )
      .await
      .unwrap();
    service
      .save_vocabulary(
        &learner,
        SaveVocabularyRequest::new(
          sense("sense-a"),
          SavedSenseState::Learning,
          SavedSenseSource::Saved,
        ),
      )
      .await
      .unwrap();

    let export = service.plan_export(&learner).await.unwrap();
    let deletion = service.plan_account_deletion(&learner).await.unwrap();
    assert_eq!(export.inventory().history_entries(), 1);
    assert_eq!(deletion.inventory().saved_vocabulary_entries(), 1);
    assert!(service.profile(&learner).await.unwrap().is_some());
    assert!(service
      .saved_vocabulary_for_sense(&learner, &sense("sense-a"))
      .await
      .unwrap()
      .is_some());
  }
}
