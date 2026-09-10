//! Version-pinned personal graph-feedback events and projections.
//!
//! This module models only private, per-owner feedback. It deliberately has no community score,
//! trust policy, moderation decision, canonical-content mutation, transport DTO, encryption, or
//! authentication implementation. A caller must derive
//! [`LearnerId`](crate::domain::learner::LearnerId) from an authenticated principal before using
//! a feedback port.

use std::{fmt, time::SystemTime};

use thiserror::Error;

use crate::{
  domain::graph::{GraphEdgeId, GraphFeedbackCapability, RelationVersion},
  ports::public_id::PublicId,
};

/// A 32-byte digest of a client idempotency key scoped to one feedback mutation route and owner.
///
/// The API boundary must derive this digest with an application-held HMAC or equivalent. Raw
/// client keys must never be stored or logged by feedback code.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeedbackIdempotencyKey([u8; 32]);

impl FeedbackIdempotencyKey {
  /// Wraps an application-derived opaque idempotency-key digest.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns the opaque digest bytes for an equality-preserving storage lookup.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for FeedbackIdempotencyKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("FeedbackIdempotencyKey([redacted])")
  }
}

/// A 32-byte fingerprint of the normalized semantic feedback request.
///
/// The application must include the target edge ID, relation version, feedback dimension, and
/// value when computing this value. A different fingerprint under the same idempotency key is a
/// conflict rather than a second mutation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FeedbackRequestFingerprint([u8; 32]);

impl FeedbackRequestFingerprint {
  /// Wraps an application-derived fingerprint of the normalized feedback request.
  pub fn new(value: [u8; 32]) -> Self {
    Self(value)
  }

  /// Returns the opaque fingerprint bytes for a same-request comparison.
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl fmt::Debug for FeedbackRequestFingerprint {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("FeedbackRequestFingerprint([redacted])")
  }
}

/// One relation edge and immutable version to which personal feedback is pinned.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphFeedbackTarget {
  edge_id: GraphEdgeId,
  relation_version: RelationVersion,
}

impl GraphFeedbackTarget {
  /// Creates a target for exactly one canonical stored relation version.
  pub fn new(edge_id: GraphEdgeId, relation_version: RelationVersion) -> Self {
    Self {
      edge_id,
      relation_version,
    }
  }

  /// Returns the stable public identifier of the canonical stored relation.
  pub fn edge_id(&self) -> &GraphEdgeId {
    &self.edge_id
  }

  /// Returns the immutable relation version that owns this feedback.
  pub const fn relation_version(&self) -> RelationVersion {
    self.relation_version
  }
}

/// A learner's current usefulness judgment for one graph-relation version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PersonalUsefulness {
  /// The relation is more useful to this learner.
  More,
  /// The relation is less useful to this learner.
  Less,
}

/// A usefulness event action, including removal of the current usefulness judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UsefulnessFeedback {
  /// Sets this learner's current usefulness judgment to more useful.
  More,
  /// Sets this learner's current usefulness judgment to less useful.
  Less,
  /// Removes this learner's current usefulness judgment without deleting the audit event.
  Reset,
}

impl UsefulnessFeedback {
  /// Returns the current usefulness value represented by this action, if any.
  pub const fn current(self) -> Option<PersonalUsefulness> {
    match self {
      Self::More => Some(PersonalUsefulness::More),
      Self::Less => Some(PersonalUsefulness::Less),
      Self::Reset => None,
    }
  }
}

/// A learner's current accuracy report for one graph-relation version.
///
/// [`Self::Unsure`] is an abstention. It is not a negative or positive vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PersonalAccuracy {
  /// This learner believes the relation is accurate.
  Accurate,
  /// This learner believes the relation connects the wrong sense.
  WrongSense,
  /// This learner believes the relation type is wrong.
  WrongType,
  /// This learner believes the relation scope is too broad.
  TooBroad,
  /// This learner believes the relation lacks a necessary restriction.
  MissingRestriction,
  /// This learner believes the relation is not supported by adequate evidence.
  Unsupported,
  /// This learner abstains from an accuracy judgment.
  Unsure,
}

/// An accuracy event action, including removal of the current accuracy report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccuracyFeedback {
  /// Sets this learner's current report to accurate.
  Accurate,
  /// Sets this learner's current report to wrong sense.
  WrongSense,
  /// Sets this learner's current report to wrong relation type.
  WrongType,
  /// Sets this learner's current report to too broad.
  TooBroad,
  /// Sets this learner's current report to missing a restriction.
  MissingRestriction,
  /// Sets this learner's current report to unsupported.
  Unsupported,
  /// Records an abstention rather than a positive or negative report.
  Unsure,
  /// Removes this learner's current accuracy report without deleting the audit event.
  Reset,
}

impl AccuracyFeedback {
  /// Returns the current accuracy value represented by this action, if any.
  pub const fn current(self) -> Option<PersonalAccuracy> {
    match self {
      Self::Accurate => Some(PersonalAccuracy::Accurate),
      Self::WrongSense => Some(PersonalAccuracy::WrongSense),
      Self::WrongType => Some(PersonalAccuracy::WrongType),
      Self::TooBroad => Some(PersonalAccuracy::TooBroad),
      Self::MissingRestriction => Some(PersonalAccuracy::MissingRestriction),
      Self::Unsupported => Some(PersonalAccuracy::Unsupported),
      Self::Unsure => Some(PersonalAccuracy::Unsure),
      Self::Reset => None,
    }
  }
}

/// One typed personal feedback action for a graph-relation version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphFeedback {
  /// A usefulness action that affects only the submitting learner's projection.
  Usefulness(UsefulnessFeedback),
  /// An accuracy report that remains private in this foundation.
  Accuracy(AccuracyFeedback),
}

impl GraphFeedback {
  /// Returns the graph capability required to accept this action.
  pub const fn capability(self) -> GraphFeedbackCapability {
    match self {
      Self::Usefulness(_) => GraphFeedbackCapability::Usefulness,
      Self::Accuracy(_) => GraphFeedbackCapability::Accuracy,
    }
  }
}

/// Immutable audit event for one accepted personal graph-feedback action.
///
/// Ownership is deliberately carried by the storage command rather than this outward-facing event
/// value, preventing callers from accidentally expose an owner reference in a receipt or log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFeedbackEvent {
  /// Stable public identifier assigned to this immutable event.
  pub id: PublicId,
  /// Exact canonical relation version evaluated by the learner.
  pub target: GraphFeedbackTarget,
  /// Typed usefulness or accuracy action accepted for this target.
  pub feedback: GraphFeedback,
  /// Server-side UTC instant at which the event was accepted.
  pub recorded_at: SystemTime,
}

impl GraphFeedbackEvent {
  /// Creates an immutable event after graph capability validation has completed.
  pub fn new(
    id: PublicId,
    target: GraphFeedbackTarget,
    feedback: GraphFeedback,
    recorded_at: SystemTime,
  ) -> Self {
    Self {
      id,
      target,
      feedback,
      recorded_at,
    }
  }
}

/// Current private feedback projection for one owner and relation version.
///
/// `version` increments in accepted event order, including reset events. It supports future
/// private ETags or optimistic writes but is not a canonical graph-version field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalGraphFeedback {
  /// Exact canonical relation version evaluated by this owner.
  pub target: GraphFeedbackTarget,
  /// Current usefulness judgment, or `None` after no action or a reset.
  pub usefulness: Option<PersonalUsefulness>,
  /// Current accuracy report, or `None` after no action or a reset.
  pub accuracy: Option<PersonalAccuracy>,
  /// Latest immutable event applied to this projection.
  pub latest_event_id: PublicId,
  /// Server-side UTC instant of the latest accepted event.
  pub updated_at: SystemTime,
  /// Monotonic per-owner-and-target projection version.
  pub version: u64,
}

impl PersonalGraphFeedback {
  /// Creates the initial current projection from one accepted event.
  pub fn from_event(event: &GraphFeedbackEvent) -> Self {
    let (usefulness, accuracy) = match event.feedback {
      GraphFeedback::Usefulness(action) => (action.current(), None),
      GraphFeedback::Accuracy(action) => (None, action.current()),
    };
    Self {
      target: event.target.clone(),
      usefulness,
      accuracy,
      latest_event_id: event.id.clone(),
      updated_at: event.recorded_at,
      version: 1,
    }
  }

  /// Applies a later event for the same target to this private current projection.
  ///
  /// # Errors
  ///
  /// Returns [`FeedbackProjectionError::TargetMismatch`] when the event belongs to another edge
  /// or relation version, and [`FeedbackProjectionError::VersionExhausted`] when the projection
  /// version cannot be incremented.
  pub fn apply(&mut self, event: &GraphFeedbackEvent) -> Result<(), FeedbackProjectionError> {
    if self.target != event.target {
      return Err(FeedbackProjectionError::TargetMismatch);
    }
    self.version = self
      .version
      .checked_add(1)
      .ok_or(FeedbackProjectionError::VersionExhausted)?;
    match event.feedback {
      GraphFeedback::Usefulness(action) => self.usefulness = action.current(),
      GraphFeedback::Accuracy(action) => self.accuracy = action.current(),
    }
    self.latest_event_id = event.id.clone();
    self.updated_at = event.recorded_at;
    Ok(())
  }
}

/// Failure while applying an immutable feedback event to a current projection.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum FeedbackProjectionError {
  /// The event was pinned to another relation edge or version.
  #[error("feedback event target does not match the current projection")]
  TargetMismatch,
  /// The projection version cannot represent another accepted event.
  #[error("feedback projection version is exhausted")]
  VersionExhausted,
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, SystemTime};

  use ulid::Ulid;

  use super::*;
  use crate::domain::{canonical::CanonicalId, learner::LearnerId};

  fn event(id_random: u128, feedback: GraphFeedback) -> GraphFeedbackEvent {
    GraphFeedbackEvent::new(
      PublicId::from(Ulid::from_parts(1_700_000_000_000, id_random)),
      GraphFeedbackTarget::new(
        GraphEdgeId::stored(CanonicalId::new("edge-1").unwrap()),
        RelationVersion::new(1).unwrap(),
      ),
      feedback,
      SystemTime::UNIX_EPOCH + Duration::from_secs(id_random as u64),
    )
  }

  #[test]
  fn current_projection_retains_an_unsure_abstention_and_reset_history() {
    let first = event(1, GraphFeedback::Accuracy(AccuracyFeedback::Unsure));
    let mut projection = PersonalGraphFeedback::from_event(&first);
    assert_eq!(projection.accuracy, Some(PersonalAccuracy::Unsure));
    assert_eq!(projection.version, 1);

    let second = event(2, GraphFeedback::Usefulness(UsefulnessFeedback::More));
    projection.apply(&second).unwrap();
    assert_eq!(projection.usefulness, Some(PersonalUsefulness::More));
    assert_eq!(projection.accuracy, Some(PersonalAccuracy::Unsure));
    assert_eq!(projection.version, 2);

    let reset = event(3, GraphFeedback::Accuracy(AccuracyFeedback::Reset));
    projection.apply(&reset).unwrap();
    assert_eq!(projection.accuracy, None);
    assert_eq!(projection.usefulness, Some(PersonalUsefulness::More));
    assert_eq!(projection.version, 3);
  }

  #[test]
  fn learner_owner_and_idempotency_debug_values_are_redacted() {
    let owner = LearnerId::new("principal-hash:learner-1").unwrap();
    let key = FeedbackIdempotencyKey::new([1; 32]);
    let fingerprint = FeedbackRequestFingerprint::new([2; 32]);

    assert!(!format!("{owner:?}").contains("learner-1"));
    assert!(!format!("{key:?}").contains("1"));
    assert!(!format!("{fingerprint:?}").contains("2"));
  }
}
