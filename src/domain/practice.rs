//! Scheduler-neutral private practice values and safety invariants.
//!
//! This module freezes learner-visible exercises with canonical targets and version metadata, but
//! deliberately does not generate content, evaluate answers, select a spaced-repetition
//! algorithm, encrypt raw answers, authenticate learners, or expose an HTTP contract. The only
//! submission input represented here is an already-redacted evaluator outcome. In particular,
//! neither a frozen exercise nor a stored attempt can contain an accepted answer.

use std::{collections::BTreeSet, fmt, num::NonZeroU8, time::Duration};

use thiserror::Error;

use crate::{
  domain::{
    canonical::{LanguageTag, ReleaseId, SenseId},
    learner::{AccessibilityPreferences, EnglishLevel, LearnerId},
  },
  ports::{clock::UtcTimestamp, public_id::PublicId},
};

/// Largest number of frozen or claimed items permitted in one practice session.
pub const MAX_PRACTICE_SESSION_ITEMS: u8 = 50;
/// Largest number of secondary canonical targets retained by one frozen exercise.
pub const MAX_SECONDARY_PRACTICE_TARGETS: usize = 8;
/// Largest accepted length for one stable version label.
pub const MAX_PRACTICE_VERSION_LENGTH: usize = 128;
/// Largest accepted learner-visible prompt length in bytes.
pub const MAX_PRACTICE_PROMPT_LENGTH: usize = 12_000;
/// Largest client-observed response duration accepted before accessibility processing.
pub const MAX_RESPONSE_TIME_INPUT: Duration = Duration::from_secs(3_600);
/// Largest response duration exposed to a future evaluator or scheduler as an influence signal.
pub const MAX_RESPONSE_TIME_INFLUENCE: Duration = Duration::from_secs(120);

/// Validation or counter failure for scheduler-neutral practice state.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PracticeValidationError {
  /// A session limit was zero or exceeded the fixed bounded contract.
  #[error("practice session item limit must be between one and {MAX_PRACTICE_SESSION_ITEMS}")]
  SessionLimitOutOfRange,
  /// An exercise slot was zero or exceeded the fixed bounded contract.
  #[error("practice item slot must be between one and {MAX_PRACTICE_SESSION_ITEMS}")]
  ItemSlotOutOfRange,
  /// A frozen exercise retained more secondary targets than the fixed bound permits.
  #[error("practice exercise contains too many secondary targets")]
  TooManySecondaryTargets,
  /// A frozen exercise listed its focus target as a secondary target.
  #[error("practice exercise secondary targets must not repeat the focus target")]
  FocusTargetRepeated,
  /// A frozen exercise listed the same secondary target more than once.
  #[error("practice exercise contains duplicate secondary targets")]
  DuplicateSecondaryTarget,
  /// A version label was blank after trimming surrounding whitespace.
  #[error("practice version must not be blank")]
  BlankVersion,
  /// A version label exceeded the fixed bounded contract.
  #[error("practice version exceeds the supported length")]
  VersionTooLong,
  /// A learner-visible prompt was blank after trimming surrounding whitespace.
  #[error("practice prompt must not be blank")]
  BlankPrompt,
  /// A learner-visible prompt exceeded the fixed bounded contract.
  #[error("practice prompt exceeds the supported length")]
  PromptTooLong,
  /// The selected English dialect did not have `en` as its primary language.
  #[error("practice dialect must be an English language tag")]
  EnglishDialectRequired,
  /// A client-observed response duration exceeded the fixed bounded contract.
  #[error("practice response time exceeds the supported duration")]
  ResponseTimeTooLong,
  /// A private practice counter or revision cannot represent another accepted attempt.
  #[error("practice mastery counters are exhausted")]
  CounterOverflow,
}

/// Bounded number of items that may be frozen and claimed in one private practice session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeSessionLimit(NonZeroU8);

impl PracticeSessionLimit {
  /// Creates a nonzero session limit within the fixed private-state bound.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::SessionLimitOutOfRange`] when `value` exceeds the fixed
  /// session bound.
  pub fn new(value: NonZeroU8) -> Result<Self, PracticeValidationError> {
    if value.get() > MAX_PRACTICE_SESSION_ITEMS {
      return Err(PracticeValidationError::SessionLimitOutOfRange);
    }

    Ok(Self(value))
  }

  /// Returns the bounded item count as a nonzero value.
  pub const fn get(self) -> NonZeroU8 {
    self.0
  }
}

/// One stable position in a bounded private practice session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeItemSlot(NonZeroU8);

impl PracticeItemSlot {
  /// Creates a one-based slot within the global practice-session bound.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::ItemSlotOutOfRange`] when `value` exceeds the maximum
  /// supported session size.
  pub fn new(value: NonZeroU8) -> Result<Self, PracticeValidationError> {
    if value.get() > MAX_PRACTICE_SESSION_ITEMS {
      return Err(PracticeValidationError::ItemSlotOutOfRange);
    }

    Ok(Self(value))
  }

  /// Returns the one-based slot value.
  pub const fn get(self) -> NonZeroU8 {
    self.0
  }
}

/// One skill dimension associated with a canonical sense in private practice state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PracticeSkill {
  /// Recognizing a sense from a bounded learner-visible prompt.
  Recognition,
  /// Distinguishing a focus sense from related canonical senses.
  Discrimination,
  /// Recalling a target without a recognition prompt.
  Recall,
  /// Producing or recognizing an orthographic form.
  SpellingForm,
  /// Applying a collocation relation.
  Collocation,
  /// Applying a grammar relation.
  Grammar,
}

/// One exercise format whose learner-visible content is frozen before a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PracticeExerciseKind {
  /// A recognition exercise.
  Recognition,
  /// A discrimination exercise.
  Discrimination,
  /// A recall exercise.
  Recall,
  /// A spelling or form exercise.
  SpellingForm,
  /// A collocation exercise.
  Collocation,
  /// A grammar exercise.
  Grammar,
}

/// A canonical sense and skill dimension that an exercise or mastery projection targets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeTarget {
  sense_id: SenseId,
  skill: PracticeSkill,
}

impl PracticeTarget {
  /// Creates one canonical practice target.
  pub fn new(sense_id: SenseId, skill: PracticeSkill) -> Self {
    Self { sense_id, skill }
  }

  /// Returns the canonical sense this target refers to.
  pub fn sense_id(&self) -> &SenseId {
    &self.sense_id
  }

  /// Returns the private practice skill dimension for this sense.
  pub const fn skill(&self) -> PracticeSkill {
    self.skill
  }
}

/// Stable non-secret label for a frozen practice pipeline component version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeVersion(String);

impl PracticeVersion {
  /// Creates a bounded trimmed version label.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::BlankVersion`] when `value` is blank after trimming, or
  /// [`PracticeValidationError::VersionTooLong`] when it exceeds the fixed bound.
  pub fn new(value: impl AsRef<str>) -> Result<Self, PracticeValidationError> {
    let value = value.as_ref().trim();
    if value.is_empty() {
      return Err(PracticeValidationError::BlankVersion);
    }
    if value.len() > MAX_PRACTICE_VERSION_LENGTH {
      return Err(PracticeValidationError::VersionTooLong);
    }

    Ok(Self(value.to_owned()))
  }

  /// Returns the stable version label.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Learner-visible frozen prompt content without an accepted answer or correction.
///
/// Debug output is redacted because prompts can contain private learner context in a future
/// generator. The prompt text remains available to an authorized presentation boundary through
/// [`Self::text`], but this foundation never logs it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticePrompt(String);

impl PracticePrompt {
  /// Creates bounded learner-visible prompt content.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::BlankPrompt`] when `value` is blank after trimming, or
  /// [`PracticeValidationError::PromptTooLong`] when it exceeds the fixed prompt bound.
  pub fn new(value: impl Into<String>) -> Result<Self, PracticeValidationError> {
    let value = value.into();
    if value.trim().is_empty() {
      return Err(PracticeValidationError::BlankPrompt);
    }
    if value.len() > MAX_PRACTICE_PROMPT_LENGTH {
      return Err(PracticeValidationError::PromptTooLong);
    }

    Ok(Self(value))
  }

  /// Returns the frozen learner-visible prompt text for an authorized presentation boundary.
  pub fn text(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for PracticePrompt {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("PracticePrompt([redacted])")
  }
}

/// Immutable pipeline and content versions attached to every frozen exercise.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PracticeVersionMetadata {
  prompt_language: LanguageTag,
  english_dialect: LanguageTag,
  level: EnglishLevel,
  content_release: ReleaseId,
  model_version: PracticeVersion,
  prompt_version: PracticeVersion,
  normalization_version: PracticeVersion,
  rubric_version: PracticeVersion,
  evaluator_version: PracticeVersion,
  scheduler_version: PracticeVersion,
}

impl PracticeVersionMetadata {
  /// Creates complete immutable metadata for one frozen exercise.
  ///
  /// `content_release` identifies canonical lexical content. The remaining versions identify the
  /// generation, normalization, grading, and scheduler components without choosing or executing
  /// any of them in this module.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::EnglishDialectRequired`] when `english_dialect` is not
  /// an English BCP-47 language tag.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    prompt_language: LanguageTag,
    english_dialect: LanguageTag,
    level: EnglishLevel,
    content_release: ReleaseId,
    model_version: PracticeVersion,
    prompt_version: PracticeVersion,
    normalization_version: PracticeVersion,
    rubric_version: PracticeVersion,
    evaluator_version: PracticeVersion,
    scheduler_version: PracticeVersion,
  ) -> Result<Self, PracticeValidationError> {
    if english_dialect.primary_language().as_str() != "en" {
      return Err(PracticeValidationError::EnglishDialectRequired);
    }

    Ok(Self {
      prompt_language,
      english_dialect,
      level,
      content_release,
      model_version,
      prompt_version,
      normalization_version,
      rubric_version,
      evaluator_version,
      scheduler_version,
    })
  }

  /// Returns the language in which the frozen learner-visible prompt is written.
  pub fn prompt_language(&self) -> &LanguageTag {
    &self.prompt_language
  }

  /// Returns the English dialect applicable to the exercise content and evaluation contract.
  pub fn english_dialect(&self) -> &LanguageTag {
    &self.english_dialect
  }

  /// Returns the self-reported learner level used to select this frozen exercise.
  pub const fn level(&self) -> EnglishLevel {
    self.level
  }

  /// Returns the immutable canonical release backing the frozen exercise.
  pub fn content_release(&self) -> &ReleaseId {
    &self.content_release
  }

  /// Returns the content-generation model version recorded for auditability.
  pub fn model_version(&self) -> &PracticeVersion {
    &self.model_version
  }

  /// Returns the prompt-template version recorded for auditability.
  pub fn prompt_version(&self) -> &PracticeVersion {
    &self.prompt_version
  }

  /// Returns the normalization implementation version recorded for auditability.
  pub fn normalization_version(&self) -> &PracticeVersion {
    &self.normalization_version
  }

  /// Returns the rubric version recorded for auditability.
  pub fn rubric_version(&self) -> &PracticeVersion {
    &self.rubric_version
  }

  /// Returns the evaluator version recorded for auditability.
  pub fn evaluator_version(&self) -> &PracticeVersion {
    &self.evaluator_version
  }

  /// Returns the scheduler version recorded for auditability without selecting its algorithm.
  pub fn scheduler_version(&self) -> &PracticeVersion {
    &self.scheduler_version
  }
}

/// Immutable learner-visible exercise that is frozen before it can be claimed.
///
/// This value has focus and secondary canonical targets plus all version metadata needed to
/// reproduce the generation and grading context. It intentionally has no accepted answer,
/// correction, evaluator input, or mutable scheduling decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenPracticeExercise {
  id: PublicId,
  session_id: PublicId,
  slot: PracticeItemSlot,
  kind: PracticeExerciseKind,
  focus: PracticeTarget,
  secondary_targets: Vec<PracticeTarget>,
  prompt: PracticePrompt,
  versions: PracticeVersionMetadata,
  frozen_at: UtcTimestamp,
}

impl FrozenPracticeExercise {
  /// Creates one complete frozen practice exercise.
  ///
  /// Secondary targets are de-duplicated by canonical sense-and-skill identity and sorted in a
  /// deterministic order. The focus target cannot appear as a secondary target.
  ///
  /// # Errors
  ///
  /// Returns a validation error when the secondary target list is oversized, duplicated, or
  /// includes `focus`.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    id: PublicId,
    session_id: PublicId,
    slot: PracticeItemSlot,
    kind: PracticeExerciseKind,
    focus: PracticeTarget,
    mut secondary_targets: Vec<PracticeTarget>,
    prompt: PracticePrompt,
    versions: PracticeVersionMetadata,
    frozen_at: UtcTimestamp,
  ) -> Result<Self, PracticeValidationError> {
    if secondary_targets.len() > MAX_SECONDARY_PRACTICE_TARGETS {
      return Err(PracticeValidationError::TooManySecondaryTargets);
    }
    if secondary_targets.iter().any(|target| target == &focus) {
      return Err(PracticeValidationError::FocusTargetRepeated);
    }

    let mut distinct = BTreeSet::new();
    if !secondary_targets
      .iter()
      .all(|target| distinct.insert(target.clone()))
    {
      return Err(PracticeValidationError::DuplicateSecondaryTarget);
    }
    secondary_targets.sort();

    Ok(Self {
      id,
      session_id,
      slot,
      kind,
      focus,
      secondary_targets,
      prompt,
      versions,
      frozen_at,
    })
  }

  /// Returns the stable public exercise identifier.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the private session identifier that owns this frozen exercise.
  pub fn session_id(&self) -> &PublicId {
    &self.session_id
  }

  /// Returns the stable one-based order within the bounded session.
  pub const fn slot(&self) -> PracticeItemSlot {
    self.slot
  }

  /// Returns the frozen exercise format.
  pub const fn kind(&self) -> PracticeExerciseKind {
    self.kind
  }

  /// Returns the canonical focus sense and skill.
  pub fn focus(&self) -> &PracticeTarget {
    &self.focus
  }

  /// Returns the deterministic, duplicate-free secondary canonical targets.
  pub fn secondary_targets(&self) -> &[PracticeTarget] {
    &self.secondary_targets
  }

  /// Returns the frozen learner-visible prompt without an accepted answer.
  pub fn prompt(&self) -> &PracticePrompt {
    &self.prompt
  }

  /// Returns the complete immutable provenance and component-version metadata.
  pub fn versions(&self) -> &PracticeVersionMetadata {
    &self.versions
  }

  /// Returns the server-side instant at which the exercise was frozen.
  pub const fn frozen_at(&self) -> UtcTimestamp {
    self.frozen_at
  }
}

/// Bounded private practice session for one opaque learner owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeSession {
  id: PublicId,
  owner: LearnerId,
  item_limit: PracticeSessionLimit,
  created_at: UtcTimestamp,
}

impl PracticeSession {
  /// Creates a bounded private session owned by an authenticated learner reference.
  pub fn new(
    id: PublicId,
    owner: LearnerId,
    item_limit: PracticeSessionLimit,
    created_at: UtcTimestamp,
  ) -> Self {
    Self {
      id,
      owner,
      item_limit,
      created_at,
    }
  }

  /// Returns the stable public identifier for this session.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the opaque private owner reference for protected persistence only.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the maximum number of items that may be frozen for this session.
  pub const fn item_limit(&self) -> PracticeSessionLimit {
    self.item_limit
  }

  /// Returns the server-side instant at which the session was created.
  pub const fn created_at(&self) -> UtcTimestamp {
    self.created_at
  }
}

/// A claimed but not-yet-submitted learner-visible frozen exercise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutstandingPracticeItem {
  exercise: FrozenPracticeExercise,
  claimed_at: UtcTimestamp,
}

impl OutstandingPracticeItem {
  /// Creates an outstanding item from an existing frozen exercise and a server-side claim time.
  pub fn new(exercise: FrozenPracticeExercise, claimed_at: UtcTimestamp) -> Self {
    Self {
      exercise,
      claimed_at,
    }
  }

  /// Returns the frozen learner-visible exercise without an accepted answer.
  pub fn exercise(&self) -> &FrozenPracticeExercise {
    &self.exercise
  }

  /// Returns the server-side instant at which this item became outstanding.
  pub const fn claimed_at(&self) -> UtcTimestamp {
    self.claimed_at
  }
}

/// Result of atomically claiming a session item or returning a prior outstanding item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeClaimOutcome {
  /// A new frozen item was claimed for this owner.
  Claimed(OutstandingPracticeItem),
  /// An already claimed unsubmitted item was returned without claiming another item.
  Outstanding(OutstandingPracticeItem),
  /// No frozen unclaimed item remains for the requested bounded session.
  NoItemAvailable,
}

/// Redacted digest of one client idempotency key used for a private practice mutation.
///
/// An API boundary must derive this digest using an application-held HMAC or equivalent. Raw
/// client keys must never enter logs, traces, or persistence through this foundation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeIdempotencyKey([u8; 32]);

impl PracticeIdempotencyKey {
  /// Wraps an application-derived opaque idempotency-key digest.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns opaque bytes for equality-preserving persistence lookup only.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for PracticeIdempotencyKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("PracticeIdempotencyKey([redacted])")
  }
}

/// Redacted digest of one normalized private practice mutation request.
///
/// The digest must include the mutation kind and all normalized mutation inputs. In particular, a
/// claim includes its session ID, while a submission includes its exercise ID, redacted evaluator
/// resolution, and accessibility-safe response-time treatment. Reuse of an idempotency key with a
/// different fingerprint is a conflict rather than another mutation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PracticeRequestFingerprint([u8; 32]);

impl PracticeRequestFingerprint {
  /// Wraps an application-derived normalized-request fingerprint.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns opaque bytes for same-request comparison only.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for PracticeRequestFingerprint {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("PracticeRequestFingerprint([redacted])")
  }
}

/// A fully resolved evaluator outcome that a future scheduler may act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvedPracticeOutcome {
  /// The future evaluator accepted the learner's response.
  Correct,
  /// The future evaluator rejected the learner's response.
  Incorrect,
  /// The learner explicitly skipped the frozen item.
  Skipped,
}

/// An externally selected mastery effect recorded atomically with a resolved attempt.
///
/// This is a persistence-safe boundary, not a scheduler implementation. A future versioned
/// scheduler must choose the effect before invoking practice-state storage; this module never
/// computes due times, intervals, grades, or proficiency levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MasteryTransition {
  /// Record that the external scheduler advanced mastery for this resolved attempt.
  Advance,
  /// Record the resolved attempt without advancing mastery.
  Hold,
}

impl MasteryTransition {
  /// Returns whether this external transition advances mastery.
  pub const fn advances(self) -> bool {
    matches!(self, Self::Advance)
  }
}

/// A resolved attempt and the already-selected non-algorithmic mastery effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedPracticeAttempt {
  outcome: ResolvedPracticeOutcome,
  mastery_transition: MasteryTransition,
}

impl ResolvedPracticeAttempt {
  /// Creates a resolved evaluator outcome with an external scheduler transition.
  pub const fn new(
    outcome: ResolvedPracticeOutcome,
    mastery_transition: MasteryTransition,
  ) -> Self {
    Self {
      outcome,
      mastery_transition,
    }
  }

  /// Returns the resolved evaluator outcome.
  pub const fn outcome(self) -> ResolvedPracticeOutcome {
    self.outcome
  }

  /// Returns the externally selected scheduler transition.
  pub const fn mastery_transition(self) -> MasteryTransition {
    self.mastery_transition
  }
}

/// Categorical reason a future evaluator cannot safely resolve a learner attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NeedsReviewReason {
  /// The evaluator could not determine a reliable grade.
  EvaluatorUncertain,
  /// The frozen prompt or its evidence was found to be ambiguous.
  AmbiguousPrompt,
  /// A safety or policy check requires human review.
  PolicyHold,
}

/// Redacted grading result accepted by the practice-state boundary.
///
/// `NeedsReview` deliberately has no [`MasteryTransition`] field. Its type therefore makes it
/// impossible for a caller to request a mastery advance alongside an uncertain grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PracticeAttemptResolution {
  /// A future evaluator resolved the attempt and a future scheduler selected its effect.
  Resolved(ResolvedPracticeAttempt),
  /// The evaluator is uncertain, so the attempt must not advance mastery.
  NeedsReview(NeedsReviewReason),
}

impl PracticeAttemptResolution {
  /// Returns whether this outcome is an uncertain, non-advancing review result.
  pub const fn is_needs_review(self) -> bool {
    matches!(self, Self::NeedsReview(_))
  }
}

/// Bounded client-observed duration captured while one frozen item was outstanding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResponseTimeInput(Duration);

impl ResponseTimeInput {
  /// Creates a bounded response-time input before accessibility processing.
  ///
  /// # Errors
  ///
  /// Returns [`PracticeValidationError::ResponseTimeTooLong`] when `value` exceeds one hour.
  pub fn new(value: Duration) -> Result<Self, PracticeValidationError> {
    if value > MAX_RESPONSE_TIME_INPUT {
      return Err(PracticeValidationError::ResponseTimeTooLong);
    }

    Ok(Self(value))
  }

  /// Returns the client-observed duration before a privacy-safe influence cap is applied.
  pub const fn observed(self) -> Duration {
    self.0
  }
}

/// Response-time value made available to a future evaluator or scheduler.
///
/// The value is absent when no input was supplied, suppressed when the learner opts out through
/// accessibility preferences, and otherwise capped before storage. It is descriptive only: this
/// foundation never converts it into a grade, interval, or mastery transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResponseTimeInfluence(ResponseTimeInfluenceKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ResponseTimeInfluenceKind {
  NotProvided,
  DisabledByAccessibility,
  Capped(Duration),
}

impl ResponseTimeInfluence {
  /// Applies the accessibility opt-out and fixed influence cap to an optional input.
  pub fn from_input(
    input: Option<ResponseTimeInput>,
    accessibility: AccessibilityPreferences,
  ) -> Self {
    if accessibility.disable_response_time_influence() {
      return Self(ResponseTimeInfluenceKind::DisabledByAccessibility);
    }

    match input {
      Some(input) => Self(ResponseTimeInfluenceKind::Capped(
        input.observed().min(MAX_RESPONSE_TIME_INFLUENCE),
      )),
      None => Self(ResponseTimeInfluenceKind::NotProvided),
    }
  }

  /// Returns whether no bounded response-time input was supplied.
  pub const fn is_not_provided(self) -> bool {
    matches!(self.0, ResponseTimeInfluenceKind::NotProvided)
  }

  /// Returns whether accessibility preferences disabled timing influence entirely.
  pub const fn is_disabled_by_accessibility(self) -> bool {
    matches!(self.0, ResponseTimeInfluenceKind::DisabledByAccessibility)
  }

  /// Returns the capped duration when timing influence remains enabled.
  pub const fn capped_duration(self) -> Option<Duration> {
    match self.0 {
      ResponseTimeInfluenceKind::Capped(value) => Some(value),
      ResponseTimeInfluenceKind::NotProvided
      | ResponseTimeInfluenceKind::DisabledByAccessibility => None,
    }
  }
}

/// Immutable record of one accepted submission without raw or accepted answer material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeAttempt {
  id: PublicId,
  exercise_id: PublicId,
  resolution: PracticeAttemptResolution,
  response_time: ResponseTimeInfluence,
  submitted_at: UtcTimestamp,
}

impl PracticeAttempt {
  /// Creates an immutable accepted attempt from redacted evaluator and response-time outcomes.
  pub fn new(
    id: PublicId,
    exercise_id: PublicId,
    resolution: PracticeAttemptResolution,
    response_time: ResponseTimeInfluence,
    submitted_at: UtcTimestamp,
  ) -> Self {
    Self {
      id,
      exercise_id,
      resolution,
      response_time,
      submitted_at,
    }
  }

  /// Returns the stable public attempt identifier.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the frozen exercise identifier submitted exactly once.
  pub fn exercise_id(&self) -> &PublicId {
    &self.exercise_id
  }

  /// Returns the redacted resolved or needs-review grading result.
  pub const fn resolution(&self) -> PracticeAttemptResolution {
    self.resolution
  }

  /// Returns the accessibility-safe capped response-time treatment.
  pub const fn response_time(&self) -> ResponseTimeInfluence {
    self.response_time
  }

  /// Returns the server-side instant at which the attempt was accepted.
  pub const fn submitted_at(&self) -> UtcTimestamp {
    self.submitted_at
  }
}

/// Owner-scoped counters and version marker for one canonical sense-and-skill mastery target.
///
/// The owner is intentionally carried by the storage key rather than this outward-facing value.
/// Counters record externally supplied evaluator and scheduler decisions; they do not implement a
/// spaced-repetition algorithm or expose a proficiency grade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeMasteryState {
  target: PracticeTarget,
  scheduler_version: PracticeVersion,
  revision: u64,
  submitted_attempts: u64,
  correct_attempts: u64,
  incorrect_attempts: u64,
  skipped_attempts: u64,
  needs_review_attempts: u64,
  mastery_advancements: u64,
  last_attempt_at: UtcTimestamp,
}

impl PracticeMasteryState {
  pub(crate) fn initial(
    target: PracticeTarget,
    scheduler_version: PracticeVersion,
    at: UtcTimestamp,
  ) -> Self {
    Self {
      target,
      scheduler_version,
      revision: 0,
      submitted_attempts: 0,
      correct_attempts: 0,
      incorrect_attempts: 0,
      skipped_attempts: 0,
      needs_review_attempts: 0,
      mastery_advancements: 0,
      last_attempt_at: at,
    }
  }

  pub(crate) fn record(
    &mut self,
    resolution: PracticeAttemptResolution,
    scheduler_version: PracticeVersion,
    at: UtcTimestamp,
  ) -> Result<(), PracticeValidationError> {
    let revision = increment(self.revision)?;
    let submitted_attempts = increment(self.submitted_attempts)?;
    let mut correct_attempts = self.correct_attempts;
    let mut incorrect_attempts = self.incorrect_attempts;
    let mut skipped_attempts = self.skipped_attempts;
    let mut needs_review_attempts = self.needs_review_attempts;
    let mut mastery_advancements = self.mastery_advancements;

    match resolution {
      PracticeAttemptResolution::Resolved(resolved) => {
        match resolved.outcome() {
          ResolvedPracticeOutcome::Correct => correct_attempts = increment(correct_attempts)?,
          ResolvedPracticeOutcome::Incorrect => incorrect_attempts = increment(incorrect_attempts)?,
          ResolvedPracticeOutcome::Skipped => skipped_attempts = increment(skipped_attempts)?,
        }
        if resolved.mastery_transition().advances() {
          mastery_advancements = increment(mastery_advancements)?;
        }
      }
      PracticeAttemptResolution::NeedsReview(_) => {
        needs_review_attempts = increment(needs_review_attempts)?;
      }
    }

    self.scheduler_version = scheduler_version;
    self.revision = revision;
    self.submitted_attempts = submitted_attempts;
    self.correct_attempts = correct_attempts;
    self.incorrect_attempts = incorrect_attempts;
    self.skipped_attempts = skipped_attempts;
    self.needs_review_attempts = needs_review_attempts;
    self.mastery_advancements = mastery_advancements;
    self.last_attempt_at = at;
    Ok(())
  }

  /// Returns the canonical sense-and-skill target whose counters this projection represents.
  pub fn target(&self) -> &PracticeTarget {
    &self.target
  }

  /// Returns the scheduler version that supplied the most recent recorded transition.
  pub fn scheduler_version(&self) -> &PracticeVersion {
    &self.scheduler_version
  }

  /// Returns the monotonic revision of accepted attempts for this private target.
  pub const fn revision(&self) -> u64 {
    self.revision
  }

  /// Returns the number of accepted one-time submissions, including needs-review outcomes.
  pub const fn submitted_attempts(&self) -> u64 {
    self.submitted_attempts
  }

  /// Returns the number of externally resolved correct outcomes.
  pub const fn correct_attempts(&self) -> u64 {
    self.correct_attempts
  }

  /// Returns the number of externally resolved incorrect outcomes.
  pub const fn incorrect_attempts(&self) -> u64 {
    self.incorrect_attempts
  }

  /// Returns the number of explicitly skipped outcomes.
  pub const fn skipped_attempts(&self) -> u64 {
    self.skipped_attempts
  }

  /// Returns the number of uncertain outcomes retained for review without advancement.
  pub const fn needs_review_attempts(&self) -> u64 {
    self.needs_review_attempts
  }

  /// Returns the number of externally directed mastery advancements.
  pub const fn mastery_advancements(&self) -> u64 {
    self.mastery_advancements
  }

  /// Returns the server-side instant of the most recently accepted submission.
  pub const fn last_attempt_at(&self) -> UtcTimestamp {
    self.last_attempt_at
  }
}

fn increment(value: u64) -> Result<u64, PracticeValidationError> {
  value
    .checked_add(1)
    .ok_or(PracticeValidationError::CounterOverflow)
}

#[cfg(test)]
mod tests {
  use std::{num::NonZeroU8, time::SystemTime};

  use ulid::Ulid;

  use super::*;
  use crate::domain::canonical::CanonicalId;

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn target(value: &str) -> PracticeTarget {
    PracticeTarget::new(SenseId::new(value).unwrap(), PracticeSkill::Recognition)
  }

  fn versions() -> PracticeVersionMetadata {
    PracticeVersionMetadata::new(
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("en-US").unwrap(),
      EnglishLevel::B1,
      CanonicalId::new("release-1").unwrap(),
      PracticeVersion::new("model-v1").unwrap(),
      PracticeVersion::new("prompt-v1").unwrap(),
      PracticeVersion::new("normalization-v1").unwrap(),
      PracticeVersion::new("rubric-v1").unwrap(),
      PracticeVersion::new("evaluator-v1").unwrap(),
      PracticeVersion::new("scheduler-v1").unwrap(),
    )
    .unwrap()
  }

  #[test]
  fn frozen_exercises_reject_duplicate_or_focus_secondary_targets() {
    let focus = target("sense-focus");
    let duplicate = FrozenPracticeExercise::new(
      public_id(1),
      public_id(2),
      PracticeItemSlot::new(NonZeroU8::new(1).unwrap()).unwrap(),
      PracticeExerciseKind::Recognition,
      focus.clone(),
      vec![target("sense-secondary"), target("sense-secondary")],
      PracticePrompt::new("Choose the matching meaning.").unwrap(),
      versions(),
      SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
      duplicate,
      Err(PracticeValidationError::DuplicateSecondaryTarget)
    );

    let repeats_focus = FrozenPracticeExercise::new(
      public_id(1),
      public_id(2),
      PracticeItemSlot::new(NonZeroU8::new(1).unwrap()).unwrap(),
      PracticeExerciseKind::Recognition,
      focus.clone(),
      vec![focus],
      PracticePrompt::new("Choose the matching meaning.").unwrap(),
      versions(),
      SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
      repeats_focus,
      Err(PracticeValidationError::FocusTargetRepeated)
    );
  }

  #[test]
  fn needs_review_never_advances_mastery_and_response_time_respects_accessibility() {
    let mut mastery = PracticeMasteryState::initial(
      target("sense-a"),
      PracticeVersion::new("scheduler-v1").unwrap(),
      SystemTime::UNIX_EPOCH,
    );
    mastery
      .record(
        PracticeAttemptResolution::NeedsReview(NeedsReviewReason::EvaluatorUncertain),
        PracticeVersion::new("scheduler-v1").unwrap(),
        SystemTime::UNIX_EPOCH,
      )
      .unwrap();

    assert_eq!(mastery.needs_review_attempts(), 1);
    assert_eq!(mastery.mastery_advancements(), 0);
    let response = ResponseTimeInput::new(Duration::from_secs(600)).unwrap();
    assert!(ResponseTimeInfluence::from_input(
      Some(response),
      AccessibilityPreferences::new(false, false, true),
    )
    .is_disabled_by_accessibility());
    assert_eq!(
      ResponseTimeInfluence::from_input(Some(response), AccessibilityPreferences::default())
        .capped_duration(),
      Some(MAX_RESPONSE_TIME_INFLUENCE)
    );
  }

  #[test]
  fn sensitive_practice_values_are_redacted_in_debug_output() {
    let prompt = PracticePrompt::new("private learner context").unwrap();
    let idempotency = PracticeIdempotencyKey::new([9; 32]);
    let fingerprint = PracticeRequestFingerprint::new([7; 32]);

    assert!(!format!("{prompt:?}").contains("private learner context"));
    assert!(!format!("{idempotency:?}").contains('9'));
    assert!(!format!("{fingerprint:?}").contains('7'));
  }
}
