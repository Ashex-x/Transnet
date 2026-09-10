//! Scheduler-neutral private practice-state orchestration.
//!
//! The service freezes caller-supplied learner-visible exercise content, allocates opaque public
//! IDs and server times, and delegates atomic claims and submissions to a private storage port.
//! It does not generate exercises, retain raw answers, evaluate a response, select a scheduler,
//! authenticate an owner, persist to a database, or expose an HTTP route.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  domain::{
    learner::{AccessibilityPreferences, LearnerId},
    practice::{
      FrozenPracticeExercise, PracticeAttemptResolution, PracticeClaimOutcome,
      PracticeIdempotencyKey, PracticeItemSlot, PracticeMasteryState, PracticePrompt,
      PracticeRequestFingerprint, PracticeSession, PracticeSessionLimit, PracticeTarget,
      PracticeValidationError, PracticeVersionMetadata, ResponseTimeInfluence, ResponseTimeInput,
    },
  },
  ports::{
    clock::Clock,
    practice_state::{
      FreezePracticeExerciseResult, PracticeClaimReceipt, PracticeClaimWrite,
      PracticeClaimWriteResult, PracticeSessionCreateResult, PracticeStateStore,
      PracticeStateStoreError, PracticeSubmissionIdempotencyResult, PracticeSubmissionReceipt,
      PracticeSubmissionWrite, PracticeSubmissionWriteResult,
    },
    public_id::{PublicId, PublicIdGenerationError, PublicIdGenerator},
  },
};

/// Input for creating one bounded owner-scoped practice session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatePracticeSession {
  item_limit: PracticeSessionLimit,
}

impl CreatePracticeSession {
  /// Creates a session request with an already validated item limit.
  pub const fn new(item_limit: PracticeSessionLimit) -> Self {
    Self { item_limit }
  }

  /// Returns the maximum number of exercises that may be frozen for this session.
  pub const fn item_limit(&self) -> PracticeSessionLimit {
    self.item_limit
  }
}

/// Input for freezing one complete exercise in an owned bounded session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezePracticeExercise {
  session_id: PublicId,
  slot: PracticeItemSlot,
  kind: crate::domain::practice::PracticeExerciseKind,
  focus: PracticeTarget,
  secondary_targets: Vec<PracticeTarget>,
  prompt: PracticePrompt,
  versions: PracticeVersionMetadata,
}

impl FreezePracticeExercise {
  /// Creates a freeze request from complete immutable exercise content and provenance metadata.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    session_id: PublicId,
    slot: PracticeItemSlot,
    kind: crate::domain::practice::PracticeExerciseKind,
    focus: PracticeTarget,
    secondary_targets: Vec<PracticeTarget>,
    prompt: PracticePrompt,
    versions: PracticeVersionMetadata,
  ) -> Self {
    Self {
      session_id,
      slot,
      kind,
      focus,
      secondary_targets,
      prompt,
      versions,
    }
  }

  /// Returns the owner-scoped session that will contain the frozen exercise.
  pub fn session_id(&self) -> &PublicId {
    &self.session_id
  }

  /// Returns the one-based bounded session slot.
  pub const fn slot(&self) -> PracticeItemSlot {
    self.slot
  }

  /// Returns the frozen exercise format.
  pub const fn kind(&self) -> crate::domain::practice::PracticeExerciseKind {
    self.kind
  }

  /// Returns the canonical focus sense-and-skill target.
  pub fn focus(&self) -> &PracticeTarget {
    &self.focus
  }

  /// Returns the secondary canonical targets to validate and freeze.
  pub fn secondary_targets(&self) -> &[PracticeTarget] {
    &self.secondary_targets
  }

  /// Returns the learner-visible prompt with no accepted answer.
  pub fn prompt(&self) -> &PracticePrompt {
    &self.prompt
  }

  /// Returns the complete immutable content and component-version metadata.
  pub fn versions(&self) -> &PracticeVersionMetadata {
    &self.versions
  }
}

/// Owner-safe result of attempting to freeze a practice exercise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreezePracticeExerciseOutcome {
  /// The complete immutable exercise was frozen in the requested session.
  Frozen(Box<FrozenPracticeExercise>),
  /// The session is absent or belongs to a different learner.
  MissingSession,
  /// The requested slot is beyond the session's fixed item limit.
  SlotOutsideSessionLimit,
  /// The session already has a frozen exercise at the requested slot.
  SlotAlreadyFrozen,
}

/// Input for idempotently claiming or returning one outstanding practice item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimPracticeItem {
  session_id: PublicId,
  idempotency_key: PracticeIdempotencyKey,
  request_fingerprint: PracticeRequestFingerprint,
}

impl ClaimPracticeItem {
  /// Creates a claim request from application-derived redacted idempotency material.
  pub fn new(
    session_id: PublicId,
    idempotency_key: PracticeIdempotencyKey,
    request_fingerprint: PracticeRequestFingerprint,
  ) -> Self {
    Self {
      session_id,
      idempotency_key,
      request_fingerprint,
    }
  }

  /// Returns the requested owner-scoped practice session.
  pub fn session_id(&self) -> &PublicId {
    &self.session_id
  }

  /// Returns the redacted claim idempotency-key digest.
  pub fn idempotency_key(&self) -> &PracticeIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the redacted normalized-claim request fingerprint.
  pub fn request_fingerprint(&self) -> &PracticeRequestFingerprint {
    &self.request_fingerprint
  }
}

/// Result of a first or replayed owner-scoped claim mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeClaimSubmission {
  /// The claim request was stored exactly once.
  Recorded(PracticeClaimReceipt),
  /// A matching retry returned the exact original claim receipt.
  Replayed(PracticeClaimReceipt),
  /// The requested session is absent or belongs to another learner.
  MissingSession,
}

impl PracticeClaimSubmission {
  /// Returns the retained claim outcome for a recorded or replayed submission, when present.
  pub fn outcome(&self) -> Option<&PracticeClaimOutcome> {
    match self {
      Self::Recorded(receipt) | Self::Replayed(receipt) => Some(receipt.outcome()),
      Self::MissingSession => None,
    }
  }

  /// Returns whether this result came from a retained matching retry.
  pub const fn is_replayed(&self) -> bool {
    matches!(self, Self::Replayed(_))
  }
}

/// Input for exactly one redacted submission of a claimed frozen exercise.
///
/// A trusted application boundary must produce `resolution` from a future evaluator and load
/// `accessibility` from the current private learner profile. This service receives no raw answer
/// and no accepted answer, so it cannot grade or leak either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitPracticeAttempt {
  exercise_id: PublicId,
  idempotency_key: PracticeIdempotencyKey,
  request_fingerprint: PracticeRequestFingerprint,
  resolution: PracticeAttemptResolution,
  response_time: Option<ResponseTimeInput>,
  accessibility: AccessibilityPreferences,
}

impl SubmitPracticeAttempt {
  /// Creates a submit-once request from redacted evaluator and response-time inputs.
  pub fn new(
    exercise_id: PublicId,
    idempotency_key: PracticeIdempotencyKey,
    request_fingerprint: PracticeRequestFingerprint,
    resolution: PracticeAttemptResolution,
    response_time: Option<ResponseTimeInput>,
    accessibility: AccessibilityPreferences,
  ) -> Self {
    Self {
      exercise_id,
      idempotency_key,
      request_fingerprint,
      resolution,
      response_time,
      accessibility,
    }
  }

  /// Returns the frozen exercise identifier that may be submitted exactly once.
  pub fn exercise_id(&self) -> &PublicId {
    &self.exercise_id
  }

  /// Returns the redacted submission idempotency-key digest.
  pub fn idempotency_key(&self) -> &PracticeIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the redacted normalized-submission request fingerprint.
  pub fn request_fingerprint(&self) -> &PracticeRequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the redacted evaluator outcome.
  pub const fn resolution(&self) -> PracticeAttemptResolution {
    self.resolution
  }

  /// Returns optional bounded client-observed response time before accessibility handling.
  pub const fn response_time(&self) -> Option<ResponseTimeInput> {
    self.response_time
  }

  /// Returns the private accessibility snapshot that must suppress timing influence when opted out.
  pub const fn accessibility(&self) -> AccessibilityPreferences {
    self.accessibility
  }
}

/// Owner-safe result of a first or replayed private submit-once mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeSubmission {
  /// The outstanding exercise was submitted once, and counters were updated atomically.
  Recorded(PracticeSubmissionReceipt),
  /// A matching retry returned the exact original submission receipt.
  Replayed(PracticeSubmissionReceipt),
  /// The exercise is absent or belongs to another learner.
  MissingExercise,
  /// The owned exercise was never claimed or is no longer outstanding.
  NotOutstanding,
  /// The owned exercise already has a different accepted one-time submission.
  AlreadySubmitted,
}

impl PracticeSubmission {
  /// Returns the retained submission receipt for a first or replayed accepted mutation.
  pub fn receipt(&self) -> Option<&PracticeSubmissionReceipt> {
    match self {
      Self::Recorded(receipt) | Self::Replayed(receipt) => Some(receipt),
      Self::MissingExercise | Self::NotOutstanding | Self::AlreadySubmitted => None,
    }
  }

  /// Returns whether this result came from a retained matching retry.
  pub const fn is_replayed(&self) -> bool {
    matches!(self, Self::Replayed(_))
  }
}

/// Coordinates private practice state through explicit clock, ID, and storage ports.
///
/// The caller must derive `LearnerId` from a verified authentication principal. The service does
/// not authenticate the caller, choose a scheduler, invoke an evaluator, expose corrections, or
/// perform database transactions itself. Those atomic semantics are required of
/// [`PracticeStateStore`].
#[derive(Clone)]
pub struct PracticeStateService {
  store: Arc<dyn PracticeStateStore>,
  clock: Arc<dyn Clock>,
  ids: Arc<dyn PublicIdGenerator>,
}

impl PracticeStateService {
  /// Creates scheduler-neutral practice-state orchestration from explicit dependencies.
  pub fn new(
    store: Arc<dyn PracticeStateStore>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn PublicIdGenerator>,
  ) -> Self {
    Self { store, clock, ids }
  }

  /// Creates one bounded private practice session for `owner`.
  ///
  /// # Errors
  ///
  /// Returns an error when a public ID cannot be generated, the improbable generated-ID collision
  /// occurs, or the private store cannot create the session.
  pub async fn create_session(
    &self,
    owner: LearnerId,
    request: CreatePracticeSession,
  ) -> Result<PracticeSession, PracticeStateServiceError> {
    let session = PracticeSession::new(
      self.ids.generate()?,
      owner,
      request.item_limit(),
      self.clock.now(),
    );
    match self.store.create_session(session).await? {
      PracticeSessionCreateResult::Created(session) => Ok(session),
      PracticeSessionCreateResult::IdConflict => Err(PracticeStateServiceError::SessionIdConflict),
    }
  }

  /// Freezes one fully specified learner-visible exercise in an owned session.
  ///
  /// The generated exercise ID is discarded if the storage result is missing or rejected; no
  /// read-then-write race can change an already frozen session slot.
  ///
  /// # Errors
  ///
  /// Returns an error when a public ID cannot be generated, the immutable exercise violates a
  /// domain invariant, or the private store cannot apply the atomic freeze operation.
  pub async fn freeze_exercise(
    &self,
    owner: &LearnerId,
    request: FreezePracticeExercise,
  ) -> Result<FreezePracticeExerciseOutcome, PracticeStateServiceError> {
    let exercise = FrozenPracticeExercise::new(
      self.ids.generate()?,
      request.session_id().clone(),
      request.slot(),
      request.kind(),
      request.focus().clone(),
      request.secondary_targets().to_vec(),
      request.prompt().clone(),
      request.versions().clone(),
      self.clock.now(),
    )?;
    match self.store.freeze_exercise(owner, exercise).await? {
      FreezePracticeExerciseResult::Frozen(exercise) => {
        Ok(FreezePracticeExerciseOutcome::Frozen(exercise))
      }
      FreezePracticeExerciseResult::MissingSession => {
        Ok(FreezePracticeExerciseOutcome::MissingSession)
      }
      FreezePracticeExerciseResult::SlotOutsideSessionLimit => {
        Ok(FreezePracticeExerciseOutcome::SlotOutsideSessionLimit)
      }
      FreezePracticeExerciseResult::SlotAlreadyFrozen => {
        Ok(FreezePracticeExerciseOutcome::SlotAlreadyFrozen)
      }
    }
  }

  /// Atomically claims a next frozen item or returns one outstanding item for `owner`.
  ///
  /// At most one item can remain outstanding across an owner's sessions. A matching idempotent
  /// retry receives its original receipt; the same key with another fingerprint is rejected.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot complete the atomic claim or detects an
  /// owner-scoped idempotency conflict.
  pub async fn claim_or_return(
    &self,
    owner: LearnerId,
    request: ClaimPracticeItem,
  ) -> Result<PracticeClaimSubmission, PracticeStateServiceError> {
    let write = PracticeClaimWrite::new(
      owner,
      request.session_id().clone(),
      request.idempotency_key().clone(),
      request.request_fingerprint().clone(),
      self.clock.now(),
    );
    match self.store.claim_or_return(write).await? {
      PracticeClaimWriteResult::Recorded(receipt) => Ok(PracticeClaimSubmission::Recorded(receipt)),
      PracticeClaimWriteResult::Replayed(receipt) => Ok(PracticeClaimSubmission::Replayed(receipt)),
      PracticeClaimWriteResult::IdempotencyConflict => {
        Err(PracticeStateServiceError::IdempotencyConflict)
      }
      PracticeClaimWriteResult::MissingSession => Ok(PracticeClaimSubmission::MissingSession),
    }
  }

  /// Atomically records one redacted submission and its focus-target mastery counters.
  ///
  /// The timing input is capped or disabled before the store sees it. `NeedsReview` cannot carry
  /// a mastery transition in its type, and the port must preserve that non-advancing rule in the
  /// same transaction that marks the exercise submitted.
  ///
  /// # Errors
  ///
  /// Returns an error when an attempt ID cannot be generated, the private store cannot apply the
  /// atomic submit-once transition, or the owner reuses an idempotency key for another request.
  pub async fn submit_once(
    &self,
    owner: LearnerId,
    request: SubmitPracticeAttempt,
  ) -> Result<PracticeSubmission, PracticeStateServiceError> {
    match self
      .store
      .submission_idempotency_result(
        &owner,
        request.idempotency_key(),
        request.request_fingerprint(),
      )
      .await?
    {
      PracticeSubmissionIdempotencyResult::Replayed(receipt) => {
        return Ok(PracticeSubmission::Replayed(*receipt));
      }
      PracticeSubmissionIdempotencyResult::Conflict => {
        return Err(PracticeStateServiceError::IdempotencyConflict);
      }
      PracticeSubmissionIdempotencyResult::Absent => {}
    }
    let response_time =
      ResponseTimeInfluence::from_input(request.response_time(), request.accessibility());
    let write = PracticeSubmissionWrite::new(
      owner,
      request.exercise_id().clone(),
      request.idempotency_key().clone(),
      request.request_fingerprint().clone(),
      self.ids.generate()?,
      request.resolution(),
      response_time,
      self.clock.now(),
    );
    match self.store.submit_once(write).await? {
      PracticeSubmissionWriteResult::Recorded(receipt) => Ok(PracticeSubmission::Recorded(receipt)),
      PracticeSubmissionWriteResult::Replayed(receipt) => Ok(PracticeSubmission::Replayed(receipt)),
      PracticeSubmissionWriteResult::IdempotencyConflict => {
        Err(PracticeStateServiceError::IdempotencyConflict)
      }
      PracticeSubmissionWriteResult::MissingExercise => Ok(PracticeSubmission::MissingExercise),
      PracticeSubmissionWriteResult::NotOutstanding => Ok(PracticeSubmission::NotOutstanding),
      PracticeSubmissionWriteResult::AlreadySubmitted => Ok(PracticeSubmission::AlreadySubmitted),
    }
  }

  /// Returns current owner-scoped counters for one canonical sense-and-skill target.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot serve the owner-scoped read.
  pub async fn mastery(
    &self,
    owner: &LearnerId,
    target: &PracticeTarget,
  ) -> Result<Option<PracticeMasteryState>, PracticeStateServiceError> {
    Ok(self.store.mastery(owner, target).await?)
  }
}

/// Failure returned by scheduler-neutral private practice-state orchestration.
#[derive(Debug, Error)]
pub enum PracticeStateServiceError {
  /// The private practice-state dependency could not complete the operation.
  #[error(transparent)]
  Store(#[from] PracticeStateStoreError),
  /// A stable public session, exercise, or attempt identifier could not be generated.
  #[error("could not generate practice identifier: {0}")]
  IdGeneration(#[from] PublicIdGenerationError),
  /// A frozen practice exercise violated a pure domain invariant.
  #[error(transparent)]
  Validation(#[from] PracticeValidationError),
  /// A generated session identifier collided with existing private state.
  #[error("practice session identifier already exists")]
  SessionIdConflict,
  /// One owner reused a mutation idempotency key for another normalized request.
  #[error("practice idempotency key conflicts with a different request")]
  IdempotencyConflict,
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU8,
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use tokio::join;
  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock, in_memory::InMemoryPracticeStateStore,
      public_id::SequencePublicIdGenerator,
    },
    domain::{
      canonical::{CanonicalId, LanguageTag, SenseId},
      learner::EnglishLevel,
      practice::{
        MasteryTransition, NeedsReviewReason, PracticeExerciseKind, PracticeSkill, PracticeVersion,
        ResolvedPracticeAttempt, ResolvedPracticeOutcome,
      },
    },
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn target(value: &str) -> PracticeTarget {
    PracticeTarget::new(SenseId::new(value).unwrap(), PracticeSkill::Recognition)
  }

  fn metadata() -> PracticeVersionMetadata {
    PracticeVersionMetadata::new(
      LanguageTag::parse("zh-CN").unwrap(),
      LanguageTag::parse("en-GB").unwrap(),
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

  fn freeze(session_id: PublicId, slot: u8, focus: &str) -> FreezePracticeExercise {
    FreezePracticeExercise::new(
      session_id,
      PracticeItemSlot::new(NonZeroU8::new(slot).unwrap()).unwrap(),
      PracticeExerciseKind::Recognition,
      target(focus),
      vec![],
      PracticePrompt::new("Choose the matching meaning.").unwrap(),
      metadata(),
    )
  }

  fn session_limit(value: u8) -> PracticeSessionLimit {
    PracticeSessionLimit::new(NonZeroU8::new(value).unwrap()).unwrap()
  }

  fn service(ids: impl IntoIterator<Item = PublicId>) -> PracticeStateService {
    PracticeStateService::new(
      Arc::new(InMemoryPracticeStateStore::new()),
      Arc::new(FixedClock::new(
        SystemTime::UNIX_EPOCH + Duration::from_secs(100),
      )),
      Arc::new(SequencePublicIdGenerator::new(ids)),
    )
  }

  #[tokio::test]
  async fn claim_returns_one_outstanding_item_then_submit_records_needs_review_without_advancement()
  {
    let service = service([
      public_id(1),
      public_id(2),
      public_id(3),
      public_id(4),
      public_id(5),
    ]);
    let learner = owner("owner-a");
    let session = service
      .create_session(
        learner.clone(),
        CreatePracticeSession::new(session_limit(2)),
      )
      .await
      .unwrap();
    let FreezePracticeExerciseOutcome::Frozen(first) = service
      .freeze_exercise(&learner, freeze(session.id().clone(), 1, "sense-a"))
      .await
      .unwrap()
    else {
      panic!("the owned session should accept its first frozen slot");
    };
    let FreezePracticeExerciseOutcome::Frozen(_) = service
      .freeze_exercise(&learner, freeze(session.id().clone(), 2, "sense-b"))
      .await
      .unwrap()
    else {
      panic!("the owned session should accept its second frozen slot");
    };

    let first_claim = service
      .claim_or_return(
        learner.clone(),
        ClaimPracticeItem::new(
          session.id().clone(),
          PracticeIdempotencyKey::new([1; 32]),
          PracticeRequestFingerprint::new([2; 32]),
        ),
      )
      .await
      .unwrap();
    assert!(matches!(
      first_claim.outcome(),
      Some(PracticeClaimOutcome::Claimed(item)) if item.exercise().id() == first.id()
    ));
    let replayed_claim = service
      .claim_or_return(
        learner.clone(),
        ClaimPracticeItem::new(
          session.id().clone(),
          PracticeIdempotencyKey::new([1; 32]),
          PracticeRequestFingerprint::new([2; 32]),
        ),
      )
      .await
      .unwrap();
    assert!(replayed_claim.is_replayed());
    assert_eq!(replayed_claim.outcome(), first_claim.outcome());
    let next_claim = service
      .claim_or_return(
        learner.clone(),
        ClaimPracticeItem::new(
          session.id().clone(),
          PracticeIdempotencyKey::new([3; 32]),
          PracticeRequestFingerprint::new([4; 32]),
        ),
      )
      .await
      .unwrap();
    assert!(matches!(
      next_claim.outcome(),
      Some(PracticeClaimOutcome::Outstanding(item)) if item.exercise().id() == first.id()
    ));

    let submitted = service
      .submit_once(
        learner.clone(),
        SubmitPracticeAttempt::new(
          first.id().clone(),
          PracticeIdempotencyKey::new([5; 32]),
          PracticeRequestFingerprint::new([6; 32]),
          PracticeAttemptResolution::NeedsReview(NeedsReviewReason::EvaluatorUncertain),
          Some(ResponseTimeInput::new(Duration::from_secs(900)).unwrap()),
          AccessibilityPreferences::new(false, false, true),
        ),
      )
      .await
      .unwrap();
    let receipt = submitted.receipt().unwrap().clone();
    assert_eq!(receipt.mastery().needs_review_attempts(), 1);
    assert_eq!(receipt.mastery().mastery_advancements(), 0);
    assert!(receipt
      .attempt()
      .response_time()
      .is_disabled_by_accessibility());
    let replay = service
      .submit_once(
        learner.clone(),
        SubmitPracticeAttempt::new(
          first.id().clone(),
          PracticeIdempotencyKey::new([5; 32]),
          PracticeRequestFingerprint::new([6; 32]),
          PracticeAttemptResolution::NeedsReview(NeedsReviewReason::EvaluatorUncertain),
          Some(ResponseTimeInput::new(Duration::from_secs(900)).unwrap()),
          AccessibilityPreferences::new(false, false, true),
        ),
      )
      .await
      .unwrap();
    assert!(replay.is_replayed());
    assert_eq!(replay.receipt(), Some(&receipt));
    assert_eq!(
      service
        .submit_once(
          learner.clone(),
          SubmitPracticeAttempt::new(
            first.id().clone(),
            PracticeIdempotencyKey::new([7; 32]),
            PracticeRequestFingerprint::new([8; 32]),
            PracticeAttemptResolution::Resolved(ResolvedPracticeAttempt::new(
              ResolvedPracticeOutcome::Correct,
              MasteryTransition::Advance,
            )),
            None,
            AccessibilityPreferences::default(),
          ),
        )
        .await
        .unwrap(),
      PracticeSubmission::AlreadySubmitted
    );
  }

  #[tokio::test]
  async fn concurrent_matching_submit_retries_advance_mastery_once() {
    let service = service([public_id(10), public_id(11), public_id(12), public_id(13)]);
    let learner = owner("owner-a");
    let session = service
      .create_session(
        learner.clone(),
        CreatePracticeSession::new(session_limit(1)),
      )
      .await
      .unwrap();
    let FreezePracticeExerciseOutcome::Frozen(exercise) = service
      .freeze_exercise(&learner, freeze(session.id().clone(), 1, "sense-a"))
      .await
      .unwrap()
    else {
      panic!("the exercise should freeze");
    };
    service
      .claim_or_return(
        learner.clone(),
        ClaimPracticeItem::new(
          session.id().clone(),
          PracticeIdempotencyKey::new([1; 32]),
          PracticeRequestFingerprint::new([2; 32]),
        ),
      )
      .await
      .unwrap();
    let request = SubmitPracticeAttempt::new(
      exercise.id().clone(),
      PracticeIdempotencyKey::new([3; 32]),
      PracticeRequestFingerprint::new([4; 32]),
      PracticeAttemptResolution::Resolved(ResolvedPracticeAttempt::new(
        ResolvedPracticeOutcome::Correct,
        MasteryTransition::Advance,
      )),
      Some(ResponseTimeInput::new(Duration::from_secs(5)).unwrap()),
      AccessibilityPreferences::default(),
    );

    let (first, second) = join!(
      service.submit_once(learner.clone(), request.clone()),
      service.submit_once(learner.clone(), request),
    );
    let outcomes = [first.unwrap(), second.unwrap()];
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, PracticeSubmission::Recorded(_)))
        .count(),
      1
    );
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, PracticeSubmission::Replayed(_)))
        .count(),
      1
    );
    let mastery = service
      .mastery(&learner, &target("sense-a"))
      .await
      .unwrap()
      .unwrap();
    assert_eq!(mastery.submitted_attempts(), 1);
    assert_eq!(mastery.mastery_advancements(), 1);
  }
}
