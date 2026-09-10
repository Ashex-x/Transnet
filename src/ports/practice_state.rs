//! Private scheduler-neutral practice-state persistence contract.
//!
//! The contract requires atomic owner checks, idempotency, one-outstanding-item claims, one-time
//! submissions, attempt recording, and counter updates. It persists no raw learner answer or
//! accepted answer. It does not authenticate an owner, encrypt raw input, select a scheduler,
//! evaluate a response, provide durable storage, or implement an HTTP interface.

use std::fmt;

use async_trait::async_trait;
use thiserror::Error;

use crate::{
  domain::{
    learner::LearnerId,
    practice::{
      FrozenPracticeExercise, PracticeAttempt, PracticeAttemptResolution, PracticeClaimOutcome,
      PracticeIdempotencyKey, PracticeMasteryState, PracticeRequestFingerprint, PracticeSession,
      PracticeValidationError, ResponseTimeInfluence,
    },
  },
  ports::{clock::UtcTimestamp, public_id::PublicId},
};

/// Failure returned by a private practice-state storage dependency.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PracticeStateStoreError {
  /// A frozen exercise's generated public ID already exists.
  #[error("practice exercise identifier already exists")]
  ExerciseIdConflict,
  /// An accepted attempt's generated public ID already exists.
  #[error("practice attempt identifier already exists")]
  AttemptIdConflict,
  /// A mastery counter transition violated a pure practice invariant.
  #[error(transparent)]
  Validation(#[from] PracticeValidationError),
  /// The private practice-state dependency cannot currently serve the operation.
  #[error("practice state store is unavailable")]
  Unavailable,
}

/// Result of atomically creating a private bounded practice session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeSessionCreateResult {
  /// The session was stored for the first time.
  Created(PracticeSession),
  /// The generated public session ID already exists, so no session was written.
  IdConflict,
}

/// Result of atomically freezing an exercise for one owned bounded session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreezePracticeExerciseResult {
  /// The fully validated frozen exercise was stored.
  Frozen(Box<FrozenPracticeExercise>),
  /// The session is absent or belongs to another learner.
  MissingSession,
  /// The exercise slot is beyond this session's configured item limit.
  SlotOutsideSessionLimit,
  /// Another frozen exercise already occupies this session slot.
  SlotAlreadyFrozen,
}

/// One private claim mutation prepared by practice-state application logic.
///
/// The raw client idempotency key and request input must not reach this value. Its owner and
/// digest fields are redacted in debug output.
#[derive(Clone)]
pub struct PracticeClaimWrite {
  owner: LearnerId,
  session_id: PublicId,
  idempotency_key: PracticeIdempotencyKey,
  request_fingerprint: PracticeRequestFingerprint,
  claimed_at: UtcTimestamp,
}

impl PracticeClaimWrite {
  /// Creates a claim-or-return request with application-derived idempotency material.
  pub fn new(
    owner: LearnerId,
    session_id: PublicId,
    idempotency_key: PracticeIdempotencyKey,
    request_fingerprint: PracticeRequestFingerprint,
    claimed_at: UtcTimestamp,
  ) -> Self {
    Self {
      owner,
      session_id,
      idempotency_key,
      request_fingerprint,
      claimed_at,
    }
  }

  /// Returns the protected owner reference for an equality-preserving storage key.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the requested bounded practice session identifier.
  pub fn session_id(&self) -> &PublicId {
    &self.session_id
  }

  /// Returns the redacted idempotency-key digest.
  pub fn idempotency_key(&self) -> &PracticeIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the redacted normalized-request fingerprint.
  pub fn request_fingerprint(&self) -> &PracticeRequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the server-side instant at which a new item would be claimed.
  pub const fn claimed_at(&self) -> UtcTimestamp {
    self.claimed_at
  }
}

impl fmt::Debug for PracticeClaimWrite {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("PracticeClaimWrite")
      .field("owner", &self.owner)
      .field("session_id", &self.session_id)
      .field("idempotency_key", &self.idempotency_key)
      .field("request_fingerprint", &self.request_fingerprint)
      .field("claimed_at", &self.claimed_at)
      .finish()
  }
}

/// Receipt retained for a recorded or replayed private claim request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeClaimReceipt {
  outcome: PracticeClaimOutcome,
}

impl PracticeClaimReceipt {
  /// Creates an idempotent claim receipt from an exact claim-or-return outcome.
  pub fn new(outcome: PracticeClaimOutcome) -> Self {
    Self { outcome }
  }

  /// Returns the exact claimed, outstanding, or exhausted outcome.
  pub fn outcome(&self) -> &PracticeClaimOutcome {
    &self.outcome
  }
}

/// Result of atomically recording or replaying one owner-scoped claim mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeClaimWriteResult {
  /// The claim request was applied exactly once and its receipt was retained.
  Recorded(PracticeClaimReceipt),
  /// A matching owner-scoped retry returned its original retained receipt.
  Replayed(PracticeClaimReceipt),
  /// The same owner reused this claim key for a different normalized request.
  IdempotencyConflict,
  /// The requested session is absent or belongs to another learner.
  MissingSession,
}

/// One private submit-once mutation prepared by practice-state application logic.
///
/// `resolution` is already redacted and contains no raw answer, accepted answer, correction, or
/// evaluator input. The storage layer must use one atomic transaction for ownership, outstanding
/// state, idempotency, attempt persistence, and mastery counters.
#[derive(Clone)]
pub struct PracticeSubmissionWrite {
  owner: LearnerId,
  exercise_id: PublicId,
  idempotency_key: PracticeIdempotencyKey,
  request_fingerprint: PracticeRequestFingerprint,
  attempt_id: PublicId,
  resolution: PracticeAttemptResolution,
  response_time: ResponseTimeInfluence,
  submitted_at: UtcTimestamp,
}

impl PracticeSubmissionWrite {
  /// Creates a one-time submission request from redacted evaluator and accessibility-safe inputs.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    owner: LearnerId,
    exercise_id: PublicId,
    idempotency_key: PracticeIdempotencyKey,
    request_fingerprint: PracticeRequestFingerprint,
    attempt_id: PublicId,
    resolution: PracticeAttemptResolution,
    response_time: ResponseTimeInfluence,
    submitted_at: UtcTimestamp,
  ) -> Self {
    Self {
      owner,
      exercise_id,
      idempotency_key,
      request_fingerprint,
      attempt_id,
      resolution,
      response_time,
      submitted_at,
    }
  }

  /// Returns the protected owner reference for an equality-preserving storage key.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the frozen exercise that may be submitted exactly once.
  pub fn exercise_id(&self) -> &PublicId {
    &self.exercise_id
  }

  /// Returns the redacted idempotency-key digest.
  pub fn idempotency_key(&self) -> &PracticeIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the redacted normalized-request fingerprint.
  pub fn request_fingerprint(&self) -> &PracticeRequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the generated stable identifier for a first accepted immutable attempt.
  pub fn attempt_id(&self) -> &PublicId {
    &self.attempt_id
  }

  /// Returns the redacted evaluator outcome that must be recorded exactly once.
  pub const fn resolution(&self) -> PracticeAttemptResolution {
    self.resolution
  }

  /// Returns the capped or disabled response-time treatment.
  pub const fn response_time(&self) -> ResponseTimeInfluence {
    self.response_time
  }

  /// Returns the server-side instant at which the attempt would be accepted.
  pub const fn submitted_at(&self) -> UtcTimestamp {
    self.submitted_at
  }
}

impl fmt::Debug for PracticeSubmissionWrite {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("PracticeSubmissionWrite")
      .field("owner", &self.owner)
      .field("exercise_id", &self.exercise_id)
      .field("idempotency_key", &self.idempotency_key)
      .field("request_fingerprint", &self.request_fingerprint)
      .field("attempt_id", &self.attempt_id)
      .field("resolution", &self.resolution)
      .field("response_time", &self.response_time)
      .field("submitted_at", &self.submitted_at)
      .finish()
  }
}

/// Receipt returned for a first or replayed private one-time practice submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeSubmissionReceipt {
  attempt: PracticeAttempt,
  mastery: PracticeMasteryState,
}

impl PracticeSubmissionReceipt {
  /// Creates the immutable attempt and resulting private mastery-counter receipt.
  pub fn new(attempt: PracticeAttempt, mastery: PracticeMasteryState) -> Self {
    Self { attempt, mastery }
  }

  /// Returns the immutable one-time attempt with no answer material.
  pub fn attempt(&self) -> &PracticeAttempt {
    &self.attempt
  }

  /// Returns the owner-scoped focus-target mastery counters after the atomic transition.
  pub fn mastery(&self) -> &PracticeMasteryState {
    &self.mastery
  }
}

/// Result of checking an owner-scoped private submission idempotency key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeSubmissionIdempotencyResult {
  /// No prior accepted submission owns this key for this learner.
  Absent,
  /// A matching accepted request can return its original receipt without allocating another ID.
  Replayed(Box<PracticeSubmissionReceipt>),
  /// This key belongs to another normalized submission request for the same learner.
  Conflict,
}

/// Result of atomically recording or replaying one owner-scoped submit-once mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeSubmissionWriteResult {
  /// The outstanding item was submitted once and the receipt was retained.
  Recorded(PracticeSubmissionReceipt),
  /// A matching owner-scoped retry returned its original retained receipt.
  Replayed(PracticeSubmissionReceipt),
  /// The same owner reused this submission key for a different normalized request.
  IdempotencyConflict,
  /// The exercise is absent or belongs to another learner.
  MissingExercise,
  /// The owned exercise has not been claimed, so it cannot be submitted.
  NotOutstanding,
  /// The owned exercise already has a distinct accepted attempt.
  AlreadySubmitted,
}

/// Stores private bounded sessions, frozen exercises, claims, and one-time attempt receipts.
///
/// Implementations must keep the following mutations atomic at the owner scope:
///
/// - session ownership and frozen-slot validation;
/// - idempotent claim-or-return of at most one outstanding item per learner across sessions;
/// - idempotent submit-once state transition, immutable attempt persistence, and focus-target
///   mastery-counter update.
///
/// Reads return `None` for both missing and foreign resources. Implementations must never expose
/// accepted answers because this contract never accepts or returns them.
#[async_trait]
pub trait PracticeStateStore: Send + Sync {
  /// Creates one bounded owner-scoped practice session unless its public ID already exists.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot atomically create the session.
  async fn create_session(
    &self,
    session: PracticeSession,
  ) -> Result<PracticeSessionCreateResult, PracticeStateStoreError>;

  /// Finds one private session only when it belongs to `owner`.
  ///
  /// `None` covers both missing and foreign session IDs.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot serve the owner-scoped read.
  async fn find_session(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<PracticeSession>, PracticeStateStoreError>;

  /// Freezes one complete exercise in an owned bounded session.
  ///
  /// Implementations must reject foreign sessions exactly as absent sessions, preserve the
  /// exercise's immutable metadata and prompt, and prevent more than one frozen item per slot.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot atomically reserve the exercise ID and slot.
  async fn freeze_exercise(
    &self,
    owner: &LearnerId,
    exercise: FrozenPracticeExercise,
  ) -> Result<FreezePracticeExerciseResult, PracticeStateStoreError>;

  /// Finds one frozen exercise only when its session belongs to `owner`.
  ///
  /// `None` covers both missing and foreign exercise IDs. A returned value has no accepted
  /// answer, correction, or raw learner submission.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot serve the owner-scoped read.
  async fn find_exercise(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<FrozenPracticeExercise>, PracticeStateStoreError>;

  /// Atomically claims the next frozen item or returns the owner's existing outstanding item.
  ///
  /// The store must retain an exact receipt under the owner-scoped claim idempotency key. A
  /// matching retry returns that receipt; a different fingerprint conflicts. A first request
  /// records `NoItemAvailable` as well, preventing later frozen items from changing its replay.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot enforce the atomic private claim transition.
  async fn claim_or_return(
    &self,
    write: PracticeClaimWrite,
  ) -> Result<PracticeClaimWriteResult, PracticeStateStoreError>;

  /// Checks whether an owner-scoped submission key has an accepted retained receipt.
  ///
  /// Application services use this read before allocating a candidate attempt ID so a normal
  /// matching retry cannot fail merely because an ID generator is temporarily unavailable. The
  /// subsequent [`Self::submit_once`] call remains the required atomic authority for racing first
  /// requests.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot serve the owner-scoped idempotency read.
  async fn submission_idempotency_result(
    &self,
    owner: &LearnerId,
    idempotency_key: &PracticeIdempotencyKey,
    request_fingerprint: &PracticeRequestFingerprint,
  ) -> Result<PracticeSubmissionIdempotencyResult, PracticeStateStoreError>;

  /// Atomically records one outstanding exercise submission and its focus-target counters.
  ///
  /// A matching retry returns the original attempt and mastery receipt without applying another
  /// transition. The store must reject a different idempotency fingerprint or second distinct
  /// submission, and a `NeedsReview` resolution must never increment mastery advancements.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot apply the atomic private submission transition.
  async fn submit_once(
    &self,
    write: PracticeSubmissionWrite,
  ) -> Result<PracticeSubmissionWriteResult, PracticeStateStoreError>;

  /// Returns current private mastery counters for one canonical sense-and-skill target.
  ///
  /// `None` covers both no recorded attempt and a query through another learner owner.
  ///
  /// # Errors
  ///
  /// Returns an error when the private store cannot serve the owner-scoped read.
  async fn mastery(
    &self,
    owner: &LearnerId,
    target: &crate::domain::practice::PracticeTarget,
  ) -> Result<Option<PracticeMasteryState>, PracticeStateStoreError>;
}
