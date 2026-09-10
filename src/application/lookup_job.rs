//! Private lookup-job lifecycle, polling, and soft-deadline decisions.
//!
//! This module is intentionally a lifecycle/polling foundation, not a durable scheduling or
//! worker implementation. A future production adapter must atomically couple its lookup-job
//! record to [`crate::ports::durable_job::DurableJobQueue`] before an API handler promises `202`.

use std::{sync::Arc, time::Duration};

use thiserror::Error;

use crate::ports::{
  clock::{Clock, UtcTimestamp},
  lookup_job::{
    LookupJobAccess, LookupJobFailure, LookupJobPoll, LookupJobResult, LookupJobStore,
    LookupJobStoreError,
  },
  public_id::PublicId,
};

/// Validated soft and hard deadline budgets for one lookup request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupDeadlinePolicy {
  soft_budget: Duration,
  hard_budget: Duration,
}

impl LookupDeadlinePolicy {
  /// Creates a deadline policy with a nonzero soft budget no greater than its hard budget.
  ///
  /// # Errors
  ///
  /// Returns [`LookupDeadlinePolicyError::InvalidBudgets`] when a budget is zero or the soft
  /// budget would outlive the hard request deadline.
  pub fn new(
    soft_budget: Duration,
    hard_budget: Duration,
  ) -> Result<Self, LookupDeadlinePolicyError> {
    if soft_budget.is_zero() || hard_budget.is_zero() || soft_budget > hard_budget {
      return Err(LookupDeadlinePolicyError::InvalidBudgets);
    }

    Ok(Self {
      soft_budget,
      hard_budget,
    })
  }

  /// Returns the maximum time spent attempting a synchronous lookup before async continuation.
  pub fn soft_budget(&self) -> Duration {
    self.soft_budget
  }

  /// Returns the latest request deadline after which no continuation may be committed.
  pub fn hard_budget(&self) -> Duration {
    self.hard_budget
  }
}

/// Invalid lookup deadline configuration.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LookupDeadlinePolicyError {
  /// A deadline budget was zero or the soft budget exceeded the hard budget.
  #[error("lookup soft deadline must be nonzero and no later than its hard deadline")]
  InvalidBudgets,
}

/// The safe next step selected before issuing potentially slow lookup work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupExecutionDecision {
  /// The estimated work fits inside the remaining soft synchronous budget.
  Synchronous {
    /// UTC instant at which synchronous work must yield to a persisted continuation.
    soft_deadline_at: UtcTimestamp,
    /// UTC instant after which this request cannot safely create a continuation.
    hard_deadline_at: UtcTimestamp,
  },
  /// The work must be represented by a durable job before the soft budget is exhausted.
  Asynchronous {
    /// UTC instant at which the synchronous response budget ends.
    soft_deadline_at: UtcTimestamp,
    /// UTC instant after which this request cannot safely create a continuation.
    hard_deadline_at: UtcTimestamp,
  },
  /// The hard deadline has already elapsed, so no asynchronous continuation is safe to promise.
  HardDeadlineElapsed,
}

/// Evaluates lookup execution choices against an injected UTC clock.
#[derive(Clone)]
pub struct LookupDeadlineDecider {
  clock: Arc<dyn Clock>,
  policy: LookupDeadlinePolicy,
}

impl LookupDeadlineDecider {
  /// Creates a decider using the supplied injected UTC clock and validated policy.
  pub fn new(clock: Arc<dyn Clock>, policy: LookupDeadlinePolicy) -> Self {
    Self { clock, policy }
  }

  /// Returns the synchronous or asynchronous decision for a request that began at `started_at`.
  ///
  /// An estimate that cannot fit before the soft deadline yields [`LookupExecutionDecision::Asynchronous`].
  /// The caller must still successfully persist a durable job before responding with `202`; this
  /// pure decision does not claim that a queue, encryption, or database transaction exists.
  pub fn decide(
    &self,
    started_at: UtcTimestamp,
    estimated_remaining: Duration,
  ) -> LookupExecutionDecision {
    let Some(soft_deadline_at) = started_at.checked_add(self.policy.soft_budget) else {
      return LookupExecutionDecision::HardDeadlineElapsed;
    };
    let Some(hard_deadline_at) = started_at.checked_add(self.policy.hard_budget) else {
      return LookupExecutionDecision::HardDeadlineElapsed;
    };
    let now = self.clock.now();
    if now >= hard_deadline_at {
      return LookupExecutionDecision::HardDeadlineElapsed;
    }

    let estimated_completion = now.checked_add(estimated_remaining);
    if now < soft_deadline_at
      && estimated_completion.is_some_and(|completion| completion <= soft_deadline_at)
    {
      LookupExecutionDecision::Synchronous {
        soft_deadline_at,
        hard_deadline_at,
      }
    } else {
      LookupExecutionDecision::Asynchronous {
        soft_deadline_at,
        hard_deadline_at,
      }
    }
  }
}

/// Coordinates lookup-job lifecycle transitions and authorized polling through one store port.
///
/// This service does not enqueue, claim, or execute durable work. A worker must apply `start`,
/// `complete`, and `fail` only after a separately durable queue implementation establishes the
/// corresponding lease and atomicity guarantees.
#[derive(Clone)]
pub struct LookupJobService {
  store: Arc<dyn LookupJobStore>,
}

impl LookupJobService {
  /// Creates a lookup-job application service from an explicit lifecycle store.
  pub fn new(store: Arc<dyn LookupJobStore>) -> Self {
    Self { store }
  }

  /// Marks an already-claimed lookup job as running.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot apply the lifecycle transition.
  pub async fn start(&self, id: &PublicId) -> Result<(), LookupJobServiceError> {
    self.store.start(id).await?;
    Ok(())
  }

  /// Completes an already-running lookup job with a validated public lookup envelope.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot apply the lifecycle transition.
  pub async fn complete(
    &self,
    id: &PublicId,
    result: LookupJobResult,
  ) -> Result<(), LookupJobServiceError> {
    self.store.complete(id, result).await?;
    Ok(())
  }

  /// Stores a redacted terminal failure for an already-running lookup job.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot apply the lifecycle transition.
  pub async fn fail(
    &self,
    id: &PublicId,
    failure: LookupJobFailure,
  ) -> Result<(), LookupJobServiceError> {
    self.store.fail(id, failure).await?;
    Ok(())
  }

  /// Polls a job without distinguishing absent and unauthorized resources.
  ///
  /// # Errors
  ///
  /// Returns an error only when the lookup-job store cannot serve the read.
  pub async fn poll(
    &self,
    id: &PublicId,
    access: &LookupJobAccess,
  ) -> Result<Option<LookupJobPoll>, LookupJobServiceError> {
    Ok(self.store.poll(id, access).await?)
  }
}

/// Failure returned by lookup-job application orchestration.
#[derive(Debug, Error)]
pub enum LookupJobServiceError {
  /// The lookup-job lifecycle store could not perform the requested operation.
  #[error(transparent)]
  Store(#[from] LookupJobStoreError),
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
    adapters::{clock::FixedClock, in_memory::InMemoryLookupJobStore},
    ports::{
      lookup_job::{LookupJobCreation, LookupJobOwner},
      public_id::PublicId,
    },
  };

  #[test]
  fn soft_deadline_selects_sync_async_or_rejects_after_hard_deadline() {
    let started = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let clock = Arc::new(FixedClock::new(started));
    let decider = LookupDeadlineDecider::new(
      clock.clone(),
      LookupDeadlinePolicy::new(Duration::from_secs(2), Duration::from_secs(5)).unwrap(),
    );

    assert!(matches!(
      decider.decide(started, Duration::from_secs(1)),
      LookupExecutionDecision::Synchronous { .. }
    ));
    assert!(matches!(
      decider.decide(started, Duration::from_secs(3)),
      LookupExecutionDecision::Asynchronous { .. }
    ));

    clock.advance(Duration::from_secs(5));
    assert_eq!(
      decider.decide(started, Duration::ZERO),
      LookupExecutionDecision::HardDeadlineElapsed
    );
  }

  #[tokio::test]
  async fn service_preserves_hidden_authorization_semantics() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
    let clock = Arc::new(FixedClock::new(now));
    let store = Arc::new(InMemoryLookupJobStore::new(clock));
    let service = LookupJobService::new(store.clone());
    let id = PublicId::from(Ulid::from_parts(1_700_000_000_000, 10));
    let owner = LookupJobAccess::owner(LookupJobOwner::new("owner-equality-token").unwrap());
    let other_owner = LookupJobAccess::owner(LookupJobOwner::new("other-owner-token").unwrap());
    store
      .create(
        id.clone(),
        LookupJobCreation::new(owner.clone(), now + Duration::from_secs(20)),
      )
      .await
      .unwrap();

    assert!(service.poll(&id, &owner).await.unwrap().is_some());
    assert_eq!(service.poll(&id, &other_owner).await.unwrap(), None);
  }
}
