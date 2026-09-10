//! Private learner-owned state values and invariants.
//!
//! This module intentionally stores only opaque ownership references and canonical identifiers.
//! It never models raw lookup queries, contexts, notes, credentials, encryption keys, or OIDC
//! claims. Production persistence and authentication must add those protections outside these
//! pure values.

use std::{collections::BTreeSet, fmt, time::Duration};

use thiserror::Error;

use crate::{
  domain::canonical::{LanguageTag, SenseId},
  ports::{clock::UtcTimestamp, public_id::PublicId},
};

/// Largest number of known languages retained in one learner preference document.
pub const MAX_KNOWN_LANGUAGES: usize = 32;
/// Smallest supported daily learning goal.
pub const MIN_DAILY_GOAL: u16 = 1;
/// Largest supported daily learning goal.
pub const MAX_DAILY_GOAL: u16 = 500;
/// Smallest supported history retention period in days.
pub const MIN_HISTORY_RETENTION_DAYS: u16 = 1;
/// Largest supported history retention period in days.
pub const MAX_HISTORY_RETENTION_DAYS: u16 = 3_650;
/// Largest number of canonical senses referenced by a history event.
pub const MAX_HISTORY_SENSE_REFERENCES: usize = 16;
/// Largest number of successor candidates retained for one retired sense.
pub const MAX_SENSE_SUCCESSORS: usize = 16;

/// Opaque equality-preserving reference to one authenticated learner.
///
/// The value must be derived from an authenticated principal by a future identity adapter. It is
/// not an OIDC subject, email address, display name, or bearer credential and its debug output is
/// intentionally redacted.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LearnerId(String);

impl LearnerId {
  /// Creates a bounded opaque owner reference suitable for equality checks and storage keys.
  ///
  /// # Errors
  ///
  /// Returns [`LearnerIdError::InvalidFormat`] when `value` is blank, oversized, or not printable
  /// ASCII.
  pub fn new(value: impl Into<String>) -> Result<Self, LearnerIdError> {
    let value = value.into();
    if value.is_empty() || value.len() > 256 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
      return Err(LearnerIdError::InvalidFormat);
    }

    Ok(Self(value))
  }

  /// Returns the opaque equality token for keyed persistence implementations.
  ///
  /// Callers must never put this value in logs, traces, URLs, or user-visible responses.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for LearnerId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("LearnerId([redacted])")
  }
}

/// Validation failure for an opaque learner owner reference.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LearnerIdError {
  /// The owner reference was not bounded printable ASCII.
  #[error("learner ID must be bounded printable ASCII")]
  InvalidFormat,
}

/// A learner's self-reported English proficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnglishLevel {
  /// Beginning proficiency.
  A1,
  /// Elementary proficiency.
  A2,
  /// Intermediate proficiency.
  B1,
  /// Upper-intermediate proficiency.
  B2,
  /// Advanced proficiency.
  C1,
  /// Proficient proficiency.
  C2,
  /// The learner has not selected a level.
  Unknown,
}

/// Learner-controlled handling of mature content in shared canonical results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatureContentMode {
  /// Exclude mature content from the learner projection.
  Hide,
  /// Include a warning before mature content is shown.
  Warn,
  /// Permit mature content without an additional warning.
  Show,
}

/// Accessibility preferences that affect a private learner projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessibilityPreferences {
  reduce_motion: bool,
  high_contrast: bool,
  disable_response_time_influence: bool,
}

impl AccessibilityPreferences {
  /// Creates accessibility preferences with explicit values for every supported control.
  pub const fn new(
    reduce_motion: bool,
    high_contrast: bool,
    disable_response_time_influence: bool,
  ) -> Self {
    Self {
      reduce_motion,
      high_contrast,
      disable_response_time_influence,
    }
  }

  /// Returns whether animated learner-facing behavior should be reduced.
  pub const fn reduce_motion(self) -> bool {
    self.reduce_motion
  }

  /// Returns whether high-contrast presentation is preferred.
  pub const fn high_contrast(self) -> bool {
    self.high_contrast
  }

  /// Returns whether response time must not influence practice scoring or scheduling.
  pub const fn disable_response_time_influence(self) -> bool {
    self.disable_response_time_influence
  }
}

impl Default for AccessibilityPreferences {
  fn default() -> Self {
    Self::new(false, false, false)
  }
}

/// Validated retention period for opt-in private lookup history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HistoryRetention {
  days: u16,
}

impl HistoryRetention {
  /// Creates a retention period between one day and ten years, inclusive.
  ///
  /// # Errors
  ///
  /// Returns [`HistoryRetentionError::OutOfRange`] when `days` is outside the supported range.
  pub fn new(days: u16) -> Result<Self, HistoryRetentionError> {
    if !(MIN_HISTORY_RETENTION_DAYS..=MAX_HISTORY_RETENTION_DAYS).contains(&days) {
      return Err(HistoryRetentionError::OutOfRange);
    }

    Ok(Self { days })
  }

  /// Returns the configured whole-day retention period.
  pub const fn days(self) -> u16 {
    self.days
  }

  /// Calculates the non-null expiry instant for an event occurring at `occurred_at`.
  ///
  /// # Errors
  ///
  /// Returns [`HistoryRetentionError::ExpiryOverflow`] when the platform timestamp cannot
  /// represent the configured future instant.
  pub fn expiry_at(self, occurred_at: UtcTimestamp) -> Result<UtcTimestamp, HistoryRetentionError> {
    occurred_at
      .checked_add(Duration::from_secs(u64::from(self.days) * 86_400))
      .ok_or(HistoryRetentionError::ExpiryOverflow)
  }
}

/// Retention-policy validation or timestamp-calculation failure.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum HistoryRetentionError {
  /// The supplied period was shorter than one day or longer than ten years.
  #[error("history retention must be between one day and ten years")]
  OutOfRange,
  /// The configured retention period cannot be represented after the event timestamp.
  #[error("history retention expiry cannot be represented")]
  ExpiryOverflow,
}

/// Persistent learner policy for whether lookup history may be retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistoryPreference {
  /// Do not retain private lookup history.
  Disabled,
  /// Retain eligible private history records through the supplied non-null expiry window.
  OptedIn(HistoryRetention),
}

/// Per-request privacy mode selected for a lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistoryMode {
  /// Apply the learner's stored history preference.
  Default,
  /// Do not create a history record or private cache record for this request.
  Incognito,
}

/// Reason an otherwise eligible lookup must not create a history record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistorySuppressionReason {
  /// The request explicitly selected incognito behavior.
  Incognito,
  /// The learner has not opted in to history retention.
  HistoryDisabled,
  /// No learner profile exists, so the service must treat the request as incognito.
  MissingProfile,
}

/// Retention decision that must be made before allocating a history identifier or private cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistoryDecision {
  /// Create a history event whose lifecycle ends at `expires_at`.
  Record {
    /// Non-null expiry instant calculated from the current opt-in preference.
    expires_at: UtcTimestamp,
  },
  /// Do not create any private history or private-cache record.
  Suppress {
    /// Privacy rule that required suppression.
    reason: HistorySuppressionReason,
  },
}

/// Private learner preferences that can be projected after authentication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LearnerPreferences {
  interface_language: LanguageTag,
  explanation_language: LanguageTag,
  english_dialect: LanguageTag,
  known_languages: BTreeSet<LanguageTag>,
  english_level: EnglishLevel,
  daily_goal: u16,
  history: HistoryPreference,
  personalization_enabled: bool,
  mature_content_mode: MatureContentMode,
  accessibility: AccessibilityPreferences,
}

impl LearnerPreferences {
  /// Creates bounded preferences after validating the English dialect and daily goal.
  ///
  /// # Errors
  ///
  /// Returns [`LearnerPreferencesError`] when the dialect is not English, the daily goal is out
  /// of range, or too many known languages were supplied.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    interface_language: LanguageTag,
    explanation_language: LanguageTag,
    english_dialect: LanguageTag,
    known_languages: impl IntoIterator<Item = LanguageTag>,
    english_level: EnglishLevel,
    daily_goal: u16,
    history: HistoryPreference,
    personalization_enabled: bool,
    mature_content_mode: MatureContentMode,
    accessibility: AccessibilityPreferences,
  ) -> Result<Self, LearnerPreferencesError> {
    if english_dialect.primary_language().as_str() != "en" {
      return Err(LearnerPreferencesError::EnglishDialectRequired);
    }
    if !(MIN_DAILY_GOAL..=MAX_DAILY_GOAL).contains(&daily_goal) {
      return Err(LearnerPreferencesError::DailyGoalOutOfRange);
    }

    let known_languages = known_languages.into_iter().collect::<BTreeSet<_>>();
    if known_languages.len() > MAX_KNOWN_LANGUAGES {
      return Err(LearnerPreferencesError::TooManyKnownLanguages);
    }

    Ok(Self {
      interface_language,
      explanation_language,
      english_dialect,
      known_languages,
      english_level,
      daily_goal,
      history,
      personalization_enabled,
      mature_content_mode,
      accessibility,
    })
  }

  /// Returns the language used for interface chrome and generic learner-facing text.
  pub fn interface_language(&self) -> &LanguageTag {
    &self.interface_language
  }

  /// Returns the preferred language for explanations and localized glosses.
  pub fn explanation_language(&self) -> &LanguageTag {
    &self.explanation_language
  }

  /// Returns the selected English dialect tag.
  pub fn english_dialect(&self) -> &LanguageTag {
    &self.english_dialect
  }

  /// Returns the distinct known-language tags in deterministic order.
  pub fn known_languages(&self) -> &BTreeSet<LanguageTag> {
    &self.known_languages
  }

  /// Returns the learner's self-reported English level.
  pub const fn english_level(&self) -> EnglishLevel {
    self.english_level
  }

  /// Returns the bounded daily learning goal.
  pub const fn daily_goal(&self) -> u16 {
    self.daily_goal
  }

  /// Returns the persistent history-retention choice.
  pub const fn history(&self) -> HistoryPreference {
    self.history
  }

  /// Returns whether private personalization may be applied after shared retrieval.
  pub const fn personalization_enabled(&self) -> bool {
    self.personalization_enabled
  }

  /// Returns the mature-content projection preference.
  pub const fn mature_content_mode(&self) -> MatureContentMode {
    self.mature_content_mode
  }

  /// Returns the learner's accessibility preferences.
  pub const fn accessibility(&self) -> AccessibilityPreferences {
    self.accessibility
  }

  /// Selects the privacy-safe history action before a lookup ID or private cache is created.
  ///
  /// # Errors
  ///
  /// Returns [`HistoryRetentionError::ExpiryOverflow`] when the configured expiry cannot be
  /// represented from `occurred_at`.
  pub fn history_decision(
    &self,
    mode: HistoryMode,
    occurred_at: UtcTimestamp,
  ) -> Result<HistoryDecision, HistoryRetentionError> {
    if mode == HistoryMode::Incognito {
      return Ok(HistoryDecision::Suppress {
        reason: HistorySuppressionReason::Incognito,
      });
    }

    match self.history {
      HistoryPreference::Disabled => Ok(HistoryDecision::Suppress {
        reason: HistorySuppressionReason::HistoryDisabled,
      }),
      HistoryPreference::OptedIn(retention) => Ok(HistoryDecision::Record {
        expires_at: retention.expiry_at(occurred_at)?,
      }),
    }
  }
}

/// Validation failure for private learner preferences.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LearnerPreferencesError {
  /// The selected dialect did not have English as its primary language.
  #[error("English dialect must use an en primary language tag")]
  EnglishDialectRequired,
  /// The daily learning goal was outside the supported inclusive range.
  #[error("daily learning goal must be between one and five hundred")]
  DailyGoalOutOfRange,
  /// The preference document included more language tags than the bounded contract permits.
  #[error("too many known languages")]
  TooManyKnownLanguages,
}

/// Versioned private profile and preferences for one opaque learner owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnerProfile {
  owner: LearnerId,
  preferences: LearnerPreferences,
  created_at: UtcTimestamp,
  updated_at: UtcTimestamp,
  revision: u64,
}

impl LearnerProfile {
  /// Creates a first private profile with revision one.
  pub fn new(owner: LearnerId, preferences: LearnerPreferences, created_at: UtcTimestamp) -> Self {
    Self {
      owner,
      preferences,
      created_at,
      updated_at: created_at,
      revision: 1,
    }
  }

  /// Returns the opaque profile owner.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the current private preference document.
  pub fn preferences(&self) -> &LearnerPreferences {
    &self.preferences
  }

  /// Returns the initial profile creation time.
  pub const fn created_at(&self) -> UtcTimestamp {
    self.created_at
  }

  /// Returns the last successful preference update time.
  pub const fn updated_at(&self) -> UtcTimestamp {
    self.updated_at
  }

  /// Returns the monotonically increasing profile revision for optimistic updates.
  pub const fn revision(&self) -> u64 {
    self.revision
  }

  /// Replaces all preferences and advances the revision at `updated_at`.
  ///
  /// # Errors
  ///
  /// Returns [`LearnerProfileError`] when time would move backwards or the revision cannot be
  /// incremented.
  pub fn replace_preferences(
    &mut self,
    preferences: LearnerPreferences,
    updated_at: UtcTimestamp,
  ) -> Result<(), LearnerProfileError> {
    self.advance_revision(updated_at)?;
    self.preferences = preferences;
    Ok(())
  }

  fn advance_revision(&mut self, updated_at: UtcTimestamp) -> Result<(), LearnerProfileError> {
    if updated_at < self.updated_at {
      return Err(LearnerProfileError::TimeMovedBackwards);
    }

    self.revision = self
      .revision
      .checked_add(1)
      .ok_or(LearnerProfileError::RevisionExhausted)?;
    self.updated_at = updated_at;
    Ok(())
  }
}

/// Failure while changing a versioned private profile.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LearnerProfileError {
  /// The update timestamp precedes the last saved profile timestamp.
  #[error("learner profile timestamp moved backwards")]
  TimeMovedBackwards,
  /// The bounded unsigned profile revision cannot be incremented further.
  #[error("learner profile revision is exhausted")]
  RevisionExhausted,
}

/// Retained private history metadata without raw query, context, note, or credential text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
  id: PublicId,
  owner: LearnerId,
  snapshot_id: Option<PublicId>,
  sense_ids: Vec<SenseId>,
  occurred_at: UtcTimestamp,
  expires_at: UtcTimestamp,
}

impl HistoryEntry {
  /// Creates a retention-bounded history entry from non-sensitive canonical references.
  ///
  /// `expires_at` must be strictly later than `occurred_at`. The entry deliberately cannot carry
  /// raw query or context text; a future encrypted storage boundary must own such fields.
  ///
  /// # Errors
  ///
  /// Returns [`HistoryEntryError`] when the expiry is invalid or the sense references are
  /// duplicated or exceed the bounded contract.
  pub fn new(
    id: PublicId,
    owner: LearnerId,
    snapshot_id: Option<PublicId>,
    sense_ids: Vec<SenseId>,
    occurred_at: UtcTimestamp,
    expires_at: UtcTimestamp,
  ) -> Result<Self, HistoryEntryError> {
    if expires_at <= occurred_at {
      return Err(HistoryEntryError::InvalidExpiry);
    }
    if sense_ids.len() > MAX_HISTORY_SENSE_REFERENCES {
      return Err(HistoryEntryError::TooManySenseReferences);
    }

    let mut distinct = BTreeSet::new();
    if !sense_ids.iter().all(|sense_id| distinct.insert(sense_id)) {
      return Err(HistoryEntryError::DuplicateSenseReference);
    }

    Ok(Self {
      id,
      owner,
      snapshot_id,
      sense_ids,
      occurred_at,
      expires_at,
    })
  }

  /// Returns the opaque public history-event identifier.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the opaque owner used only for equality-preserving private storage.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the optional context-free canonical snapshot reference.
  pub fn snapshot_id(&self) -> Option<&PublicId> {
    self.snapshot_id.as_ref()
  }

  /// Returns the distinct referenced canonical senses in their lookup-result order.
  pub fn sense_ids(&self) -> &[SenseId] {
    &self.sense_ids
  }

  /// Returns the instant at which the lookup occurred.
  pub const fn occurred_at(&self) -> UtcTimestamp {
    self.occurred_at
  }

  /// Returns the non-null retention expiry instant.
  pub const fn expires_at(&self) -> UtcTimestamp {
    self.expires_at
  }

  /// Returns whether the entry must no longer be returned or retained at `now`.
  pub fn is_expired_at(&self, now: UtcTimestamp) -> bool {
    self.expires_at <= now
  }
}

/// Validation failure for a retention-bounded history entry.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum HistoryEntryError {
  /// The non-null expiry was not later than the occurrence instant.
  #[error("history expiry must be later than occurrence time")]
  InvalidExpiry,
  /// More canonical sense references were supplied than the bounded record permits.
  #[error("too many history sense references")]
  TooManySenseReferences,
  /// A canonical sense appeared more than once in a single history entry.
  #[error("history sense references must be distinct")]
  DuplicateSenseReference,
}

/// Lifecycle state of a private saved vocabulary entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SavedSenseState {
  /// The learner is actively studying the sense.
  Learning,
  /// The learner currently considers the sense known.
  Known,
  /// The sense is temporarily excluded from practice until the learner resumes it.
  Paused,
  /// The sense is retained privately but excluded from ordinary active lists and practice.
  Archived,
}

/// Non-sensitive origin of a saved vocabulary action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SavedSenseSource {
  /// The learner explicitly saved the sense.
  Saved,
  /// The sense was saved from a lookup result.
  Lookup,
  /// The sense was saved from a practice workflow.
  Practice,
  /// The sense was imported from a learner-controlled portable export.
  Import,
}

/// Canonical successor mapping for a retired sense in one content-release transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenseSuccessors {
  retired_sense_id: SenseId,
  successors: Vec<SenseId>,
}

impl SenseSuccessors {
  /// Creates a bounded ordered successor mapping for a retired canonical sense.
  ///
  /// An empty successor list means that no compatible replacement is known. A one-item list may
  /// migrate saved state automatically; multiple items pause an active entry for learner choice.
  ///
  /// # Errors
  ///
  /// Returns [`SavedVocabularyError`] when a successor is duplicated, points to the retired sense,
  /// or exceeds the bounded candidate count.
  pub fn new(
    retired_sense_id: SenseId,
    successors: Vec<SenseId>,
  ) -> Result<Self, SavedVocabularyError> {
    if successors.len() > MAX_SENSE_SUCCESSORS {
      return Err(SavedVocabularyError::TooManySuccessors);
    }
    if successors
      .iter()
      .any(|successor| successor == &retired_sense_id)
    {
      return Err(SavedVocabularyError::SuccessorMatchesRetiredSense);
    }

    let mut distinct = BTreeSet::new();
    if !successors
      .iter()
      .all(|successor| distinct.insert(successor))
    {
      return Err(SavedVocabularyError::DuplicateSuccessor);
    }

    Ok(Self {
      retired_sense_id,
      successors,
    })
  }

  /// Returns the retired sense to which this mapping applies.
  pub fn retired_sense_id(&self) -> &SenseId {
    &self.retired_sense_id
  }

  /// Returns the distinct ordered successor candidates.
  pub fn successors(&self) -> &[SenseId] {
    &self.successors
  }
}

/// Result of applying a canonical successor mapping to one saved entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenseEvolutionOutcome {
  /// The mapping was for a different current sense and made no change.
  NotApplicable,
  /// A one-to-one successor safely replaced the current canonical reference.
  Migrated {
    /// Retired canonical sense that was replaced.
    from: SenseId,
    /// Compatible current canonical sense.
    to: SenseId,
  },
  /// The retired sense has several plausible successors and requires learner confirmation.
  AwaitingSelection {
    /// Retired canonical sense that remains as the current reference until selection.
    retired: SenseId,
    /// Bounded deterministic successor candidates.
    candidates: Vec<SenseId>,
  },
  /// The retired sense has no compatible successor and active study was paused safely.
  RetiredWithoutSuccessor {
    /// Retired canonical sense that has no compatible replacement.
    retired: SenseId,
  },
}

/// Mutation requested for a private saved vocabulary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedSenseMutation {
  /// Set a visible lifecycle state after any required successor choice is resolved.
  SetState(SavedSenseState),
  /// Refresh the last-seen time without changing lifecycle state.
  MarkSeen,
  /// Reconcile the entry with a canonical retired-sense successor mapping.
  ApplySuccessors(SenseSuccessors),
  /// Confirm one previously offered successor candidate.
  SelectSuccessor(SenseId),
}

/// Observable effect of one saved vocabulary mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedSenseMutationOutcome {
  /// The lifecycle state changed or was explicitly reaffirmed.
  StateSet,
  /// The entry's last-seen timestamp was refreshed.
  Seen,
  /// A canonical successor mapping was evaluated.
  Evolution(SenseEvolutionOutcome),
  /// The learner confirmed a pending successor candidate.
  SuccessorSelected {
    /// Retired sense replaced after explicit learner confirmation.
    from: SenseId,
    /// Selected compatible successor.
    to: SenseId,
  },
}

/// Versioned private saved vocabulary state independent of a mutable canonical sense key.
///
/// `id` remains stable across successor migration while `current_sense_id` may change. The entry
/// retains its original sense reference for export, audit, and migration explanations. Encrypted
/// learner notes are intentionally absent until an authenticated encryption boundary exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedVocabularyEntry {
  id: PublicId,
  owner: LearnerId,
  original_sense_id: SenseId,
  current_sense_id: SenseId,
  state: SavedSenseState,
  source: SavedSenseSource,
  first_seen_at: UtcTimestamp,
  last_seen_at: UtcTimestamp,
  updated_at: UtcTimestamp,
  revision: u64,
  pending_successors: Vec<SenseId>,
}

impl SavedVocabularyEntry {
  /// Creates a first saved-sense record with a stable entry ID and revision one.
  pub fn new(
    id: PublicId,
    owner: LearnerId,
    sense_id: SenseId,
    state: SavedSenseState,
    source: SavedSenseSource,
    created_at: UtcTimestamp,
  ) -> Self {
    Self {
      id,
      owner,
      original_sense_id: sense_id.clone(),
      current_sense_id: sense_id,
      state,
      source,
      first_seen_at: created_at,
      last_seen_at: created_at,
      updated_at: created_at,
      revision: 1,
      pending_successors: Vec::new(),
    }
  }

  /// Returns the stable private-entry public identifier.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the opaque owner used only for private equality checks.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the canonical sense that was initially saved.
  pub fn original_sense_id(&self) -> &SenseId {
    &self.original_sense_id
  }

  /// Returns the canonical sense currently associated with the entry.
  pub fn current_sense_id(&self) -> &SenseId {
    &self.current_sense_id
  }

  /// Returns the current learner-selected lifecycle state.
  pub const fn state(&self) -> SavedSenseState {
    self.state
  }

  /// Returns the immutable origin of the initial save action.
  pub const fn source(&self) -> SavedSenseSource {
    self.source
  }

  /// Returns the first time the learner saved or encountered this entry.
  pub const fn first_seen_at(&self) -> UtcTimestamp {
    self.first_seen_at
  }

  /// Returns the most recent save or lookup time for this entry.
  pub const fn last_seen_at(&self) -> UtcTimestamp {
    self.last_seen_at
  }

  /// Returns the last state mutation time.
  pub const fn updated_at(&self) -> UtcTimestamp {
    self.updated_at
  }

  /// Returns the monotonically increasing revision for optimistic updates.
  pub const fn revision(&self) -> u64 {
    self.revision
  }

  /// Returns pending successor candidates that must be selected before active study resumes.
  pub fn pending_successors(&self) -> &[SenseId] {
    &self.pending_successors
  }

  /// Refreshes a save action atomically by updating state and last-seen time once.
  ///
  /// # Errors
  ///
  /// Returns [`SavedVocabularyError`] when a pending split requires selection, time would move
  /// backwards, or the entry revision is exhausted.
  pub fn refresh(
    &mut self,
    state: SavedSenseState,
    at: UtcTimestamp,
  ) -> Result<(), SavedVocabularyError> {
    self.ensure_state_is_safe(state)?;
    self.advance_revision(at)?;
    self.state = state;
    self.last_seen_at = at;
    Ok(())
  }

  /// Applies one lifecycle or successor mutation at a monotonic UTC instant.
  ///
  /// # Errors
  ///
  /// Returns [`SavedVocabularyError`] when an unsafe state transition is requested, the successor
  /// choice is invalid, time moves backwards, or the revision is exhausted.
  pub fn mutate(
    &mut self,
    mutation: SavedSenseMutation,
    at: UtcTimestamp,
  ) -> Result<SavedSenseMutationOutcome, SavedVocabularyError> {
    match mutation {
      SavedSenseMutation::SetState(state) => {
        self.ensure_state_is_safe(state)?;
        self.advance_revision(at)?;
        self.state = state;
        Ok(SavedSenseMutationOutcome::StateSet)
      }
      SavedSenseMutation::MarkSeen => {
        self.advance_revision(at)?;
        self.last_seen_at = at;
        Ok(SavedSenseMutationOutcome::Seen)
      }
      SavedSenseMutation::ApplySuccessors(mapping) => Ok(SavedSenseMutationOutcome::Evolution(
        self.apply_successors(&mapping, at)?,
      )),
      SavedSenseMutation::SelectSuccessor(successor) => {
        if !self
          .pending_successors
          .iter()
          .any(|candidate| candidate == &successor)
        {
          return Err(SavedVocabularyError::NotPendingSuccessor);
        }

        let from = self.current_sense_id.clone();
        self.advance_revision(at)?;
        self.current_sense_id = successor.clone();
        self.pending_successors.clear();
        Ok(SavedSenseMutationOutcome::SuccessorSelected {
          from,
          to: successor,
        })
      }
    }
  }

  fn ensure_state_is_safe(&self, state: SavedSenseState) -> Result<(), SavedVocabularyError> {
    if !self.pending_successors.is_empty()
      && !matches!(state, SavedSenseState::Paused | SavedSenseState::Archived)
    {
      return Err(SavedVocabularyError::SuccessorChoiceRequired);
    }

    Ok(())
  }

  fn apply_successors(
    &mut self,
    mapping: &SenseSuccessors,
    at: UtcTimestamp,
  ) -> Result<SenseEvolutionOutcome, SavedVocabularyError> {
    if mapping.retired_sense_id != self.current_sense_id {
      return Ok(SenseEvolutionOutcome::NotApplicable);
    }

    match mapping.successors.as_slice() {
      [] => {
        let already_paused_without_candidates =
          self.state == SavedSenseState::Paused && self.pending_successors.is_empty();
        if !already_paused_without_candidates {
          self.advance_revision(at)?;
          if self.state != SavedSenseState::Archived {
            self.state = SavedSenseState::Paused;
          }
          self.pending_successors.clear();
        }
        Ok(SenseEvolutionOutcome::RetiredWithoutSuccessor {
          retired: mapping.retired_sense_id.clone(),
        })
      }
      [successor] => {
        self.advance_revision(at)?;
        let from = self.current_sense_id.clone();
        self.current_sense_id = successor.clone();
        self.pending_successors.clear();
        Ok(SenseEvolutionOutcome::Migrated {
          from,
          to: successor.clone(),
        })
      }
      successors => {
        let already_pending = self.pending_successors == successors;
        if !already_pending {
          self.advance_revision(at)?;
          if self.state != SavedSenseState::Archived {
            self.state = SavedSenseState::Paused;
          }
          self.pending_successors = successors.to_vec();
        }
        Ok(SenseEvolutionOutcome::AwaitingSelection {
          retired: mapping.retired_sense_id.clone(),
          candidates: successors.to_vec(),
        })
      }
    }
  }

  fn advance_revision(&mut self, at: UtcTimestamp) -> Result<(), SavedVocabularyError> {
    if at < self.updated_at {
      return Err(SavedVocabularyError::TimeMovedBackwards);
    }

    self.revision = self
      .revision
      .checked_add(1)
      .ok_or(SavedVocabularyError::RevisionExhausted)?;
    self.updated_at = at;
    Ok(())
  }
}

/// Failure while validating or mutating private saved vocabulary state.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SavedVocabularyError {
  /// More successor candidates were supplied than the bounded contract permits.
  #[error("too many sense successor candidates")]
  TooManySuccessors,
  /// A successor candidate appeared more than once.
  #[error("sense successor candidates must be distinct")]
  DuplicateSuccessor,
  /// A successor mapping pointed a retired sense back to itself.
  #[error("a retired sense cannot be its own successor")]
  SuccessorMatchesRetiredSense,
  /// An active state was requested before an ambiguous successor choice was resolved.
  #[error("a successor choice is required before active study can resume")]
  SuccessorChoiceRequired,
  /// The selected successor was not one of the pending canonical candidates.
  #[error("selected sense is not a pending successor")]
  NotPendingSuccessor,
  /// The mutation timestamp precedes the last saved entry timestamp.
  #[error("saved vocabulary timestamp moved backwards")]
  TimeMovedBackwards,
  /// The bounded unsigned entry revision cannot be incremented further.
  #[error("saved vocabulary revision is exhausted")]
  RevisionExhausted,
}

/// Count-only inventory used to plan a private export or deletion workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LearnerDataInventory {
  profile_present: bool,
  history_entries: u64,
  saved_vocabulary_entries: u64,
}

impl LearnerDataInventory {
  /// Creates a count-only inventory that intentionally contains no private content.
  pub const fn new(
    profile_present: bool,
    history_entries: u64,
    saved_vocabulary_entries: u64,
  ) -> Self {
    Self {
      profile_present,
      history_entries,
      saved_vocabulary_entries,
    }
  }

  /// Returns whether a local profile and preference record exists.
  pub const fn profile_present(self) -> bool {
    self.profile_present
  }

  /// Returns the count of unexpired private history metadata records.
  pub const fn history_entries(self) -> u64 {
    self.history_entries
  }

  /// Returns the count of private saved vocabulary records.
  pub const fn saved_vocabulary_entries(self) -> u64 {
    self.saved_vocabulary_entries
  }
}

/// Locally represented personal-data category in an export plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExportDataCategory {
  /// Profile settings and preferences without identity-provider claims.
  ProfileAndPreferences,
  /// Retained non-sensitive history metadata and canonical references.
  HistoryMetadata,
  /// Saved vocabulary lifecycle state and canonical sense references.
  SavedVocabulary,
}

/// Plan for an authenticated, externally executed portable data export.
///
/// This is a manifest only. It does not generate an archive, issue a download capability, encrypt
/// an object, or determine authorization. Those steps require durable storage, KMS, and identity
/// integrations that are intentionally outside this foundation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPlan {
  owner: LearnerId,
  inventory: LearnerDataInventory,
  categories: Vec<ExportDataCategory>,
}

impl ExportPlan {
  /// Creates a minimal deterministic export manifest for the supplied count-only inventory.
  pub fn for_inventory(owner: LearnerId, inventory: LearnerDataInventory) -> Self {
    let mut categories = Vec::with_capacity(3);
    if inventory.profile_present() {
      categories.push(ExportDataCategory::ProfileAndPreferences);
    }
    if inventory.history_entries() > 0 {
      categories.push(ExportDataCategory::HistoryMetadata);
    }
    if inventory.saved_vocabulary_entries() > 0 {
      categories.push(ExportDataCategory::SavedVocabulary);
    }

    Self {
      owner,
      inventory,
      categories,
    }
  }

  /// Returns the opaque learner owner for a trusted private-workflow executor.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the count-only inventory captured while planning.
  pub const fn inventory(&self) -> LearnerDataInventory {
    self.inventory
  }

  /// Returns the bounded local categories that an export worker must include.
  pub fn categories(&self) -> &[ExportDataCategory] {
    &self.categories
  }
}

/// Ordered deletion step required for an account-deletion workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountDeletionStep {
  /// Prevent new personalization writes before destructive work starts.
  BlockNewPersonalizationWrites,
  /// Revoke identity-backed service sessions in the identity/session boundary.
  RevokeSessions,
  /// Delete or anonymize local profile and preference records.
  DeleteProfileAndPreferences,
  /// Delete all retained private history records and context-free snapshots owned by the learner.
  DeleteHistory,
  /// Delete saved vocabulary state and private notes once encrypted storage is available.
  DeleteSavedVocabulary,
  /// Remove private caches that are not represented by this in-memory store.
  ErasePrivateCaches,
  /// Remove queued lookup payloads and private jobs at the durable-job boundary.
  EraseLookupJobs,
  /// Revoke pending capabilities and remove export artifacts.
  RevokeExports,
}

/// Plan for an externally authorized account-deletion workflow.
///
/// This value is deliberately not an execution API. It neither deletes records nor revokes
/// sessions; production code must complete the ordered steps with durable, auditable operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDeletionPlan {
  owner: LearnerId,
  inventory: LearnerDataInventory,
  steps: Vec<AccountDeletionStep>,
}

impl AccountDeletionPlan {
  /// Creates the bounded ordered plan required before an external deletion executor runs.
  pub fn for_inventory(owner: LearnerId, inventory: LearnerDataInventory) -> Self {
    Self {
      owner,
      inventory,
      steps: vec![
        AccountDeletionStep::BlockNewPersonalizationWrites,
        AccountDeletionStep::RevokeSessions,
        AccountDeletionStep::DeleteProfileAndPreferences,
        AccountDeletionStep::DeleteHistory,
        AccountDeletionStep::DeleteSavedVocabulary,
        AccountDeletionStep::ErasePrivateCaches,
        AccountDeletionStep::EraseLookupJobs,
        AccountDeletionStep::RevokeExports,
      ],
    }
  }

  /// Returns the opaque learner owner for a trusted private-workflow executor.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the count-only inventory captured while planning.
  pub const fn inventory(&self) -> LearnerDataInventory {
    self.inventory
  }

  /// Returns the ordered deletion steps that must be completed and audited.
  pub fn steps(&self) -> &[AccountDeletionStep] {
    &self.steps
  }
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, SystemTime};

  use ulid::Ulid;

  use super::*;

  fn sense(value: &str) -> SenseId {
    SenseId::new(value).unwrap()
  }

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn preferences(history: HistoryPreference) -> LearnerPreferences {
    LearnerPreferences::new(
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("en-US").unwrap(),
      [LanguageTag::parse("zh-CN").unwrap()],
      EnglishLevel::B1,
      12,
      history,
      true,
      MatureContentMode::Warn,
      AccessibilityPreferences::default(),
    )
    .unwrap()
  }

  #[test]
  fn learner_owner_debug_is_redacted_and_format_is_bounded() {
    let owner = LearnerId::new("private-owner-equality-token").unwrap();

    assert!(!format!("{owner:?}").contains("private-owner-equality-token"));
    assert!(LearnerId::new("contains whitespace").is_err());
    assert!(LearnerId::new("".to_owned()).is_err());
  }

  #[test]
  fn history_decision_respects_opt_in_and_incognito_before_id_allocation() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let opted_in = preferences(HistoryPreference::OptedIn(
      HistoryRetention::new(90).unwrap(),
    ));

    assert!(matches!(
      opted_in.history_decision(HistoryMode::Default, now).unwrap(),
      HistoryDecision::Record { expires_at } if expires_at == now + Duration::from_secs(90 * 86_400)
    ));
    assert_eq!(
      opted_in
        .history_decision(HistoryMode::Incognito, now)
        .unwrap(),
      HistoryDecision::Suppress {
        reason: HistorySuppressionReason::Incognito,
      }
    );
    assert_eq!(
      preferences(HistoryPreference::Disabled)
        .history_decision(HistoryMode::Default, now)
        .unwrap(),
      HistoryDecision::Suppress {
        reason: HistorySuppressionReason::HistoryDisabled,
      }
    );
  }

  #[test]
  fn history_entries_reject_duplicate_references_and_expire_at_the_boundary() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000);
    assert_eq!(
      HistoryEntry::new(
        public_id(1),
        LearnerId::new("owner-token").unwrap(),
        None,
        vec![sense("sense-a"), sense("sense-a")],
        now,
        now + Duration::from_secs(1),
      ),
      Err(HistoryEntryError::DuplicateSenseReference)
    );

    let entry = HistoryEntry::new(
      public_id(2),
      LearnerId::new("owner-token").unwrap(),
      None,
      vec![sense("sense-a")],
      now,
      now + Duration::from_secs(1),
    )
    .unwrap();
    assert!(entry.is_expired_at(now + Duration::from_secs(1)));
  }

  #[test]
  fn successor_migration_is_safe_for_one_to_one_and_pauses_ambiguous_splits() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3_000);
    let owner = LearnerId::new("owner-token").unwrap();
    let mut entry = SavedVocabularyEntry::new(
      public_id(3),
      owner,
      sense("retired-sense"),
      SavedSenseState::Learning,
      SavedSenseSource::Lookup,
      now,
    );

    let split = SenseSuccessors::new(
      sense("retired-sense"),
      vec![sense("successor-a"), sense("successor-b")],
    )
    .unwrap();
    assert!(matches!(
      entry
        .mutate(
          SavedSenseMutation::ApplySuccessors(split),
          now + Duration::from_secs(1),
        )
        .unwrap(),
      SavedSenseMutationOutcome::Evolution(SenseEvolutionOutcome::AwaitingSelection { .. })
    ));
    assert_eq!(entry.state(), SavedSenseState::Paused);
    assert_eq!(
      entry.mutate(
        SavedSenseMutation::SetState(SavedSenseState::Learning),
        now + Duration::from_secs(2),
      ),
      Err(SavedVocabularyError::SuccessorChoiceRequired)
    );
    assert!(matches!(
      entry
        .mutate(
          SavedSenseMutation::SelectSuccessor(sense("successor-b")),
          now + Duration::from_secs(2),
        )
        .unwrap(),
      SavedSenseMutationOutcome::SuccessorSelected { .. }
    ));
    assert_eq!(entry.current_sense_id(), &sense("successor-b"));
    assert_eq!(entry.state(), SavedSenseState::Paused);

    let mut one_to_one = SavedVocabularyEntry::new(
      public_id(4),
      LearnerId::new("owner-other").unwrap(),
      sense("old-sense"),
      SavedSenseState::Known,
      SavedSenseSource::Saved,
      now,
    );
    assert!(matches!(
      one_to_one
        .mutate(
          SavedSenseMutation::ApplySuccessors(
            SenseSuccessors::new(sense("old-sense"), vec![sense("new-sense")]).unwrap(),
          ),
          now + Duration::from_secs(1),
        )
        .unwrap(),
      SavedSenseMutationOutcome::Evolution(SenseEvolutionOutcome::Migrated { .. })
    ));
    assert_eq!(one_to_one.current_sense_id(), &sense("new-sense"));

    let mut no_successor = SavedVocabularyEntry::new(
      public_id(5),
      LearnerId::new("owner-third").unwrap(),
      sense("removed-sense"),
      SavedSenseState::Learning,
      SavedSenseSource::Practice,
      now,
    );
    assert!(matches!(
      no_successor
        .mutate(
          SavedSenseMutation::ApplySuccessors(
            SenseSuccessors::new(sense("removed-sense"), vec![]).unwrap(),
          ),
          now + Duration::from_secs(1),
        )
        .unwrap(),
      SavedSenseMutationOutcome::Evolution(SenseEvolutionOutcome::RetiredWithoutSuccessor { .. })
    ));
    assert_eq!(no_successor.state(), SavedSenseState::Paused);
  }

  #[test]
  fn privacy_plans_are_manifest_only_and_include_external_deletion_steps() {
    let owner = LearnerId::new("owner-token").unwrap();
    let inventory = LearnerDataInventory::new(true, 2, 3);
    let export = ExportPlan::for_inventory(owner.clone(), inventory);
    let deletion = AccountDeletionPlan::for_inventory(owner, inventory);

    assert_eq!(export.categories().len(), 3);
    assert_eq!(
      deletion.steps().first(),
      Some(&AccountDeletionStep::BlockNewPersonalizationWrites)
    );
    assert!(deletion
      .steps()
      .contains(&AccountDeletionStep::RevokeSessions));
    assert!(deletion
      .steps()
      .contains(&AccountDeletionStep::EraseLookupJobs));
  }
}
