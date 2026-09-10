//! Deterministic process-local scheduler-neutral private practice-state storage.
//!
//! This adapter models atomic owner-scoped claims and submit-once transitions for tests and local
//! development. It is not durable, encrypted, authenticated, replicated, distributed-locking, or
//! a database transaction implementation. It intentionally stores no raw learner answer or
//! accepted answer because the practice-state port never accepts either value.

use std::{
  collections::BTreeMap,
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  domain::{
    learner::LearnerId,
    practice::{
      FrozenPracticeExercise, OutstandingPracticeItem, PracticeAttempt, PracticeClaimOutcome,
      PracticeIdempotencyKey, PracticeMasteryState, PracticeRequestFingerprint, PracticeSession,
      PracticeTarget,
    },
  },
  ports::{
    practice_state::{
      FreezePracticeExerciseResult, PracticeClaimReceipt, PracticeClaimWrite,
      PracticeClaimWriteResult, PracticeSessionCreateResult, PracticeStateStore,
      PracticeStateStoreError, PracticeSubmissionIdempotencyResult, PracticeSubmissionReceipt,
      PracticeSubmissionWrite, PracticeSubmissionWriteResult,
    },
    public_id::PublicId,
  },
};

/// Process-local private practice-state store with atomic owner-scoped mutations.
///
/// Clones and [`InMemoryPracticeStateStore::reopen`] share one deterministic in-memory state. The
/// mutex models atomicity only inside one process; production adapters must supply durable,
/// transactionally isolated storage before any real practice workflow is enabled.
#[derive(Clone, Default)]
pub struct InMemoryPracticeStateStore {
  state: Arc<Mutex<PracticeState>>,
}

impl InMemoryPracticeStateStore {
  /// Creates an empty deterministic private practice-state store.
  pub fn new() -> Self {
    Self::default()
  }

  /// Opens another handle over the same process-local practice-state data.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[async_trait]
impl PracticeStateStore for InMemoryPracticeStateStore {
  async fn create_session(
    &self,
    session: PracticeSession,
  ) -> Result<PracticeSessionCreateResult, PracticeStateStoreError> {
    let mut state = mutex_lock(&self.state);
    if state.sessions.contains_key(session.id()) {
      return Ok(PracticeSessionCreateResult::IdConflict);
    }

    state.sessions.insert(session.id().clone(), session.clone());
    Ok(PracticeSessionCreateResult::Created(session))
  }

  async fn find_session(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<PracticeSession>, PracticeStateStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      state
        .sessions
        .get(id)
        .filter(|session| session.owner() == owner)
        .cloned(),
    )
  }

  async fn freeze_exercise(
    &self,
    owner: &LearnerId,
    exercise: FrozenPracticeExercise,
  ) -> Result<FreezePracticeExerciseResult, PracticeStateStoreError> {
    let mut state = mutex_lock(&self.state);
    let Some(session) = state.sessions.get(exercise.session_id()) else {
      return Ok(FreezePracticeExerciseResult::MissingSession);
    };
    if session.owner() != owner {
      return Ok(FreezePracticeExerciseResult::MissingSession);
    }
    if exercise.slot().get() > session.item_limit().get() {
      return Ok(FreezePracticeExerciseResult::SlotOutsideSessionLimit);
    }
    if state.exercises.values().any(|stored| {
      stored.exercise.session_id() == exercise.session_id()
        && stored.exercise.slot() == exercise.slot()
    }) {
      return Ok(FreezePracticeExerciseResult::SlotAlreadyFrozen);
    }
    if state.exercises.contains_key(exercise.id()) {
      return Err(PracticeStateStoreError::ExerciseIdConflict);
    }

    state.exercises.insert(
      exercise.id().clone(),
      StoredExercise {
        exercise: exercise.clone(),
        state: ExerciseState::Frozen,
      },
    );
    Ok(FreezePracticeExerciseResult::Frozen(Box::new(exercise)))
  }

  async fn find_exercise(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<FrozenPracticeExercise>, PracticeStateStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      state
        .exercises
        .get(id)
        .filter(|stored| state.owns_session(owner, stored.exercise.session_id()))
        .map(|stored| stored.exercise.clone()),
    )
  }

  async fn claim_or_return(
    &self,
    write: PracticeClaimWrite,
  ) -> Result<PracticeClaimWriteResult, PracticeStateStoreError> {
    let mut state = mutex_lock(&self.state);
    match state.claim_idempotency(&write) {
      ClaimIdempotency::Replayed(receipt) => {
        return Ok(PracticeClaimWriteResult::Replayed(*receipt));
      }
      ClaimIdempotency::Conflict => return Ok(PracticeClaimWriteResult::IdempotencyConflict),
      ClaimIdempotency::Absent => {}
    }

    if !state.owns_session(write.owner(), write.session_id()) {
      return Ok(PracticeClaimWriteResult::MissingSession);
    }

    let outcome = if let Some(item) = state.outstanding_for(write.owner()) {
      PracticeClaimOutcome::Outstanding(item)
    } else if let Some(exercise_id) = state.next_frozen_exercise(write.session_id()) {
      let Some(stored) = state.exercises.get_mut(&exercise_id) else {
        return Err(PracticeStateStoreError::Unavailable);
      };
      let exercise = stored.exercise.clone();
      stored.state = ExerciseState::Outstanding {
        claimed_at: write.claimed_at(),
      };
      PracticeClaimOutcome::Claimed(OutstandingPracticeItem::new(exercise, write.claimed_at()))
    } else {
      PracticeClaimOutcome::NoItemAvailable
    };
    let receipt = PracticeClaimReceipt::new(outcome);
    state.store_claim_idempotency(&write, receipt.clone());
    Ok(PracticeClaimWriteResult::Recorded(receipt))
  }

  async fn submission_idempotency_result(
    &self,
    owner: &LearnerId,
    idempotency_key: &PracticeIdempotencyKey,
    request_fingerprint: &PracticeRequestFingerprint,
  ) -> Result<PracticeSubmissionIdempotencyResult, PracticeStateStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      match state.submission_idempotency(owner, idempotency_key, request_fingerprint) {
        SubmissionIdempotency::Absent => PracticeSubmissionIdempotencyResult::Absent,
        SubmissionIdempotency::Replayed(receipt) => {
          PracticeSubmissionIdempotencyResult::Replayed(receipt)
        }
        SubmissionIdempotency::Conflict => PracticeSubmissionIdempotencyResult::Conflict,
      },
    )
  }

  async fn submit_once(
    &self,
    write: PracticeSubmissionWrite,
  ) -> Result<PracticeSubmissionWriteResult, PracticeStateStoreError> {
    let mut state = mutex_lock(&self.state);
    match state.submission_idempotency(
      write.owner(),
      write.idempotency_key(),
      write.request_fingerprint(),
    ) {
      SubmissionIdempotency::Replayed(receipt) => {
        return Ok(PracticeSubmissionWriteResult::Replayed(*receipt));
      }
      SubmissionIdempotency::Conflict => {
        return Ok(PracticeSubmissionWriteResult::IdempotencyConflict);
      }
      SubmissionIdempotency::Absent => {}
    }

    let Some(stored) = state.exercises.get(write.exercise_id()).cloned() else {
      return Ok(PracticeSubmissionWriteResult::MissingExercise);
    };
    if !state.owns_session(write.owner(), stored.exercise.session_id()) {
      return Ok(PracticeSubmissionWriteResult::MissingExercise);
    }
    if matches!(stored.state, ExerciseState::Frozen) {
      return Ok(PracticeSubmissionWriteResult::NotOutstanding);
    }
    if matches!(stored.state, ExerciseState::Submitted) {
      return Ok(PracticeSubmissionWriteResult::AlreadySubmitted);
    }
    if state.attempts.contains_key(write.attempt_id()) {
      return Err(PracticeStateStoreError::AttemptIdConflict);
    }

    let attempt = PracticeAttempt::new(
      write.attempt_id().clone(),
      write.exercise_id().clone(),
      write.resolution(),
      write.response_time(),
      write.submitted_at(),
    );
    let mastery_key = MasteryKey::new(write.owner().clone(), stored.exercise.focus().clone());
    let mut mastery = state.mastery.get(&mastery_key).cloned().unwrap_or_else(|| {
      PracticeMasteryState::initial(
        stored.exercise.focus().clone(),
        stored.exercise.versions().scheduler_version().clone(),
        write.submitted_at(),
      )
    });
    mastery.record(
      write.resolution(),
      stored.exercise.versions().scheduler_version().clone(),
      write.submitted_at(),
    )?;
    let receipt = PracticeSubmissionReceipt::new(attempt.clone(), mastery.clone());

    let Some(stored) = state.exercises.get_mut(write.exercise_id()) else {
      return Err(PracticeStateStoreError::Unavailable);
    };
    stored.state = ExerciseState::Submitted;
    state.attempts.insert(
      write.attempt_id().clone(),
      StoredAttempt {
        owner: write.owner().clone(),
        attempt,
      },
    );
    state.mastery.insert(mastery_key, mastery);
    state.store_submission_idempotency(&write, receipt.clone());
    Ok(PracticeSubmissionWriteResult::Recorded(receipt))
  }

  async fn mastery(
    &self,
    owner: &LearnerId,
    target: &PracticeTarget,
  ) -> Result<Option<PracticeMasteryState>, PracticeStateStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      state
        .mastery
        .get(&MasteryKey::new(owner.clone(), target.clone()))
        .cloned(),
    )
  }
}

#[derive(Default)]
struct PracticeState {
  sessions: BTreeMap<PublicId, PracticeSession>,
  exercises: BTreeMap<PublicId, StoredExercise>,
  attempts: BTreeMap<PublicId, StoredAttempt>,
  mastery: BTreeMap<MasteryKey, PracticeMasteryState>,
  idempotency: BTreeMap<IdempotencyScope, StoredIdempotency>,
}

impl PracticeState {
  fn owns_session(&self, owner: &LearnerId, session_id: &PublicId) -> bool {
    self
      .sessions
      .get(session_id)
      .is_some_and(|session| session.owner() == owner)
  }

  fn outstanding_for(&self, owner: &LearnerId) -> Option<OutstandingPracticeItem> {
    self
      .exercises
      .values()
      .filter_map(|stored| match stored.state {
        ExerciseState::Outstanding { claimed_at }
          if self.owns_session(owner, stored.exercise.session_id()) =>
        {
          Some(OutstandingPracticeItem::new(
            stored.exercise.clone(),
            claimed_at,
          ))
        }
        ExerciseState::Frozen | ExerciseState::Submitted | ExerciseState::Outstanding { .. } => {
          None
        }
      })
      .min_by(|left, right| {
        left
          .claimed_at()
          .cmp(&right.claimed_at())
          .then_with(|| left.exercise().id().cmp(right.exercise().id()))
      })
  }

  fn next_frozen_exercise(&self, session_id: &PublicId) -> Option<PublicId> {
    self
      .exercises
      .iter()
      .filter(|(_, stored)| {
        stored.exercise.session_id() == session_id && matches!(stored.state, ExerciseState::Frozen)
      })
      .min_by(|(left_id, left), (right_id, right)| {
        left
          .exercise
          .slot()
          .cmp(&right.exercise.slot())
          .then_with(|| left_id.cmp(right_id))
      })
      .map(|(id, _)| id.clone())
  }

  fn claim_idempotency(&self, write: &PracticeClaimWrite) -> ClaimIdempotency {
    let key = IdempotencyScope::claim(write.owner().clone(), write.idempotency_key().clone());
    let Some(StoredIdempotency::Claim(stored)) = self.idempotency.get(&key) else {
      return ClaimIdempotency::Absent;
    };
    if stored.request_fingerprint == *write.request_fingerprint() {
      ClaimIdempotency::Replayed(Box::new(stored.receipt.clone()))
    } else {
      ClaimIdempotency::Conflict
    }
  }

  fn store_claim_idempotency(&mut self, write: &PracticeClaimWrite, receipt: PracticeClaimReceipt) {
    self.idempotency.insert(
      IdempotencyScope::claim(write.owner().clone(), write.idempotency_key().clone()),
      StoredIdempotency::Claim(StoredClaimIdempotency {
        request_fingerprint: write.request_fingerprint().clone(),
        receipt,
      }),
    );
  }

  fn submission_idempotency(
    &self,
    owner: &LearnerId,
    idempotency_key: &PracticeIdempotencyKey,
    request_fingerprint: &PracticeRequestFingerprint,
  ) -> SubmissionIdempotency {
    let key = IdempotencyScope::submission(owner.clone(), idempotency_key.clone());
    let Some(StoredIdempotency::Submission(stored)) = self.idempotency.get(&key) else {
      return SubmissionIdempotency::Absent;
    };
    if stored.request_fingerprint == *request_fingerprint {
      SubmissionIdempotency::Replayed(Box::new(stored.receipt.clone()))
    } else {
      SubmissionIdempotency::Conflict
    }
  }

  fn store_submission_idempotency(
    &mut self,
    write: &PracticeSubmissionWrite,
    receipt: PracticeSubmissionReceipt,
  ) {
    self.idempotency.insert(
      IdempotencyScope::submission(write.owner().clone(), write.idempotency_key().clone()),
      StoredIdempotency::Submission(StoredSubmissionIdempotency {
        request_fingerprint: write.request_fingerprint().clone(),
        receipt,
      }),
    );
  }
}

#[derive(Clone)]
struct StoredExercise {
  exercise: FrozenPracticeExercise,
  state: ExerciseState,
}

#[derive(Clone)]
enum ExerciseState {
  Frozen,
  Outstanding {
    claimed_at: crate::ports::clock::UtcTimestamp,
  },
  Submitted,
}

struct StoredAttempt {
  #[allow(dead_code)]
  owner: LearnerId,
  #[allow(dead_code)]
  attempt: PracticeAttempt,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MasteryKey {
  owner: LearnerId,
  target: PracticeTarget,
}

impl MasteryKey {
  fn new(owner: LearnerId, target: PracticeTarget) -> Self {
    Self { owner, target }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum IdempotencyOperation {
  Claim,
  Submission,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IdempotencyScope {
  owner: LearnerId,
  operation: IdempotencyOperation,
  key: PracticeIdempotencyKey,
}

impl IdempotencyScope {
  fn claim(owner: LearnerId, key: PracticeIdempotencyKey) -> Self {
    Self {
      owner,
      operation: IdempotencyOperation::Claim,
      key,
    }
  }

  fn submission(owner: LearnerId, key: PracticeIdempotencyKey) -> Self {
    Self {
      owner,
      operation: IdempotencyOperation::Submission,
      key,
    }
  }
}

enum StoredIdempotency {
  Claim(StoredClaimIdempotency),
  Submission(StoredSubmissionIdempotency),
}

struct StoredClaimIdempotency {
  request_fingerprint: PracticeRequestFingerprint,
  receipt: PracticeClaimReceipt,
}

struct StoredSubmissionIdempotency {
  request_fingerprint: PracticeRequestFingerprint,
  receipt: PracticeSubmissionReceipt,
}

enum ClaimIdempotency {
  Absent,
  Replayed(Box<PracticeClaimReceipt>),
  Conflict,
}

enum SubmissionIdempotency {
  Absent,
  Replayed(Box<PracticeSubmissionReceipt>),
  Conflict,
}

#[cfg(test)]
mod tests {
  use std::{
    num::NonZeroU8,
    time::{Duration, SystemTime},
  };

  use tokio::join;
  use ulid::Ulid;

  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalId, LanguageTag, SenseId},
      learner::{AccessibilityPreferences, EnglishLevel},
      practice::{
        MasteryTransition, PracticeAttemptResolution, PracticeExerciseKind, PracticeItemSlot,
        PracticePrompt, PracticeSessionLimit, PracticeSkill, PracticeValidationError,
        PracticeVersion, PracticeVersionMetadata, ResolvedPracticeAttempt, ResolvedPracticeOutcome,
        ResponseTimeInput,
      },
    },
    ports::practice_state::PracticeStateStore,
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

  fn session(id: u128, owner: LearnerId) -> PracticeSession {
    PracticeSession::new(
      public_id(id),
      owner,
      PracticeSessionLimit::new(NonZeroU8::new(2).unwrap()).unwrap(),
      SystemTime::UNIX_EPOCH,
    )
  }

  fn exercise(id: u128, session_id: PublicId, slot: u8, focus: &str) -> FrozenPracticeExercise {
    FrozenPracticeExercise::new(
      public_id(id),
      session_id,
      PracticeItemSlot::new(NonZeroU8::new(slot).unwrap()).unwrap(),
      PracticeExerciseKind::Recognition,
      target(focus),
      vec![],
      PracticePrompt::new("Choose the matching meaning.").unwrap(),
      metadata(),
      SystemTime::UNIX_EPOCH,
    )
    .unwrap()
  }

  fn claim(owner: LearnerId, session_id: PublicId, key: u8, fingerprint: u8) -> PracticeClaimWrite {
    PracticeClaimWrite::new(
      owner,
      session_id,
      PracticeIdempotencyKey::new([key; 32]),
      PracticeRequestFingerprint::new([fingerprint; 32]),
      SystemTime::UNIX_EPOCH + Duration::from_secs(10),
    )
  }

  fn submission(
    owner: LearnerId,
    exercise_id: PublicId,
    key: u8,
    attempt: u128,
  ) -> PracticeSubmissionWrite {
    PracticeSubmissionWrite::new(
      owner,
      exercise_id,
      PracticeIdempotencyKey::new([key; 32]),
      PracticeRequestFingerprint::new([key.saturating_add(1); 32]),
      public_id(attempt),
      PracticeAttemptResolution::Resolved(ResolvedPracticeAttempt::new(
        ResolvedPracticeOutcome::Correct,
        MasteryTransition::Advance,
      )),
      crate::domain::practice::ResponseTimeInfluence::from_input(
        Some(ResponseTimeInput::new(Duration::from_secs(5)).unwrap()),
        AccessibilityPreferences::default(),
      ),
      SystemTime::UNIX_EPOCH + Duration::from_secs(20),
    )
  }

  #[tokio::test]
  async fn concurrent_distinct_claims_return_one_claimed_and_one_outstanding_item() {
    let store = InMemoryPracticeStateStore::new();
    let learner = owner("owner-a");
    let saved = session(1, learner.clone());
    store.create_session(saved.clone()).await.unwrap();
    let frozen = exercise(2, saved.id().clone(), 1, "sense-a");
    store
      .freeze_exercise(&learner, frozen.clone())
      .await
      .unwrap();

    let (left, right) = join!(
      store.claim_or_return(claim(learner.clone(), saved.id().clone(), 1, 2)),
      store.claim_or_return(claim(learner.clone(), saved.id().clone(), 3, 4)),
    );
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| {
          matches!(
            outcome,
            PracticeClaimWriteResult::Recorded(receipt)
              if matches!(receipt.outcome(), PracticeClaimOutcome::Claimed(item)
                if item.exercise().id() == frozen.id())
          )
        })
        .count(),
      1
    );
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| {
          matches!(
            outcome,
            PracticeClaimWriteResult::Recorded(receipt)
              if matches!(receipt.outcome(), PracticeClaimOutcome::Outstanding(item)
                if item.exercise().id() == frozen.id())
          )
        })
        .count(),
      1
    );
  }

  #[tokio::test]
  async fn concurrent_matching_submissions_record_one_attempt_and_one_replay() {
    let store = InMemoryPracticeStateStore::new();
    let learner = owner("owner-a");
    let saved = session(10, learner.clone());
    store.create_session(saved.clone()).await.unwrap();
    let frozen = exercise(11, saved.id().clone(), 1, "sense-a");
    store
      .freeze_exercise(&learner, frozen.clone())
      .await
      .unwrap();
    store
      .claim_or_return(claim(learner.clone(), saved.id().clone(), 1, 2))
      .await
      .unwrap();
    let first = submission(learner.clone(), frozen.id().clone(), 3, 12);
    let duplicate = submission(learner.clone(), frozen.id().clone(), 3, 13);

    let (left, right) = join!(store.submit_once(first), store.submit_once(duplicate));
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, PracticeSubmissionWriteResult::Recorded(_)))
        .count(),
      1
    );
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, PracticeSubmissionWriteResult::Replayed(_)))
        .count(),
      1
    );
    let mastery = store
      .mastery(&learner, &target("sense-a"))
      .await
      .unwrap()
      .unwrap();
    assert_eq!(mastery.submitted_attempts(), 1);
    assert_eq!(mastery.mastery_advancements(), 1);
  }

  #[tokio::test]
  async fn foreign_reads_and_freezes_are_missing_and_session_slot_is_bounded() {
    let store = InMemoryPracticeStateStore::new();
    let first = owner("owner-a");
    let second = owner("owner-b");
    let saved = session(20, first.clone());
    store.create_session(saved.clone()).await.unwrap();
    let frozen = exercise(21, saved.id().clone(), 1, "sense-a");

    assert_eq!(
      store
        .freeze_exercise(&second, frozen.clone())
        .await
        .unwrap(),
      FreezePracticeExerciseResult::MissingSession
    );
    assert_eq!(store.find_session(&second, saved.id()).await.unwrap(), None);
    assert_eq!(
      store.find_exercise(&second, frozen.id()).await.unwrap(),
      None
    );
    let outside = FrozenPracticeExercise::new(
      public_id(22),
      saved.id().clone(),
      PracticeItemSlot::new(NonZeroU8::new(2).unwrap()).unwrap(),
      PracticeExerciseKind::Recognition,
      target("sense-b"),
      vec![],
      PracticePrompt::new("Choose the matching meaning.").unwrap(),
      metadata(),
      SystemTime::UNIX_EPOCH,
    )
    .unwrap();
    let one_item = PracticeSession::new(
      public_id(23),
      first.clone(),
      PracticeSessionLimit::new(NonZeroU8::new(1).unwrap()).unwrap(),
      SystemTime::UNIX_EPOCH,
    );
    store.create_session(one_item.clone()).await.unwrap();
    let outside = FrozenPracticeExercise::new(
      public_id(24),
      one_item.id().clone(),
      outside.slot(),
      outside.kind(),
      outside.focus().clone(),
      outside.secondary_targets().to_vec(),
      outside.prompt().clone(),
      outside.versions().clone(),
      outside.frozen_at(),
    )
    .unwrap();
    assert_eq!(
      store.freeze_exercise(&first, outside).await.unwrap(),
      FreezePracticeExerciseResult::SlotOutsideSessionLimit
    );
    assert_eq!(
      PracticeSessionLimit::new(NonZeroU8::new(51).unwrap()),
      Err(PracticeValidationError::SessionLimitOutOfRange)
    );
  }
}
