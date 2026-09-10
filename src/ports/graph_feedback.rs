//! Ports for validating and persisting private graph-feedback events.
//!
//! The ports deliberately separate immutable canonical-edge eligibility from mutable private
//! learner state. Neither port can publish, retype, rank, or otherwise mutate canonical graph
//! facts. Community aggregation, moderation, encryption, authentication, and retention policy are
//! outside this foundation.

use std::{collections::BTreeSet, fmt};

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  feedback::{
    FeedbackIdempotencyKey, FeedbackProjectionError, FeedbackRequestFingerprint,
    GraphFeedbackEvent, GraphFeedbackTarget, PersonalGraphFeedback,
  },
  graph::{GraphEdgeId, GraphFeedbackCapability},
  learner::LearnerId,
};

/// Canonical feedback eligibility for one currently published stored graph relation.
///
/// A catalog only returns canonical stored relations. Derived scale and visual-only edges must not
/// be returned, even if their public identifiers resemble stored edge identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFeedbackEdge {
  /// Exact current stored-relation target, including immutable relation version.
  pub target: GraphFeedbackTarget,
  /// Feedback dimensions explicitly enabled for this relation version.
  pub capabilities: BTreeSet<GraphFeedbackCapability>,
}

impl GraphFeedbackEdge {
  /// Creates feedback eligibility for one canonical stored relation version.
  pub fn new(target: GraphFeedbackTarget, capabilities: BTreeSet<GraphFeedbackCapability>) -> Self {
    Self {
      target,
      capabilities,
    }
  }

  /// Returns whether this edge version accepts `capability`.
  pub fn accepts(&self, capability: GraphFeedbackCapability) -> bool {
    self.capabilities.contains(&capability)
  }
}

/// Failure from the canonical graph-feedback eligibility dependency.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphFeedbackCatalogError {
  /// The canonical graph dependency could not complete the eligibility read.
  #[error("graph-feedback catalog is unavailable")]
  Unavailable,
  /// Canonical edge metadata was internally inconsistent.
  #[error("graph-feedback catalog returned inconsistent data")]
  InconsistentData,
}

/// Resolves the current canonical edge version and feedback capability set.
///
/// Implementations must return `Ok(None)` for missing, derived, visual-only, unpublished, or
/// otherwise ineligible edges. A successful result is a read-only eligibility decision; it must
/// not mutate canonical graph facts or community ranking components.
#[async_trait]
pub trait GraphFeedbackCatalog: Send + Sync {
  /// Resolves a canonical stored relation by its stable public edge ID.
  ///
  /// # Errors
  ///
  /// Returns an error when the catalog cannot safely resolve current canonical eligibility.
  async fn find(
    &self,
    edge_id: &GraphEdgeId,
  ) -> Result<Option<GraphFeedbackEdge>, GraphFeedbackCatalogError>;
}

/// One private, idempotent event write prepared by the feedback application service.
///
/// The event ID is fresh for a first attempt. On a matching retry, stores return the prior receipt
/// and ignore this new candidate ID and timestamp. The owner, idempotency digest, and request
/// fingerprint are all redacted in debug output.
#[derive(Clone)]
pub struct GraphFeedbackWrite {
  owner: LearnerId,
  idempotency_key: FeedbackIdempotencyKey,
  request_fingerprint: FeedbackRequestFingerprint,
  event: GraphFeedbackEvent,
}

impl GraphFeedbackWrite {
  /// Creates an accepted event write that a store must apply atomically.
  pub fn new(
    owner: LearnerId,
    idempotency_key: FeedbackIdempotencyKey,
    request_fingerprint: FeedbackRequestFingerprint,
    event: GraphFeedbackEvent,
  ) -> Self {
    Self {
      owner,
      idempotency_key,
      request_fingerprint,
      event,
    }
  }

  /// Returns the private owner for an equality-preserving storage key.
  ///
  /// Callers must not expose this token outside a protected persistence boundary.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the opaque idempotency-key digest for an owner-scoped lookup.
  pub fn idempotency_key(&self) -> &FeedbackIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the normalized-request fingerprint used for conflict detection.
  pub fn request_fingerprint(&self) -> &FeedbackRequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the immutable event to append if this is not a replay or conflict.
  pub fn event(&self) -> &GraphFeedbackEvent {
    &self.event
  }
}

impl fmt::Debug for GraphFeedbackWrite {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("GraphFeedbackWrite")
      .field("owner", &self.owner)
      .field("idempotency_key", &self.idempotency_key)
      .field("request_fingerprint", &self.request_fingerprint)
      .field("event", &self.event)
      .finish()
  }
}

/// Receipt returned for a recorded or replayed private feedback write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFeedbackReceipt {
  /// Immutable event that was appended exactly once.
  pub event: GraphFeedbackEvent,
  /// Current private projection after this event was applied.
  pub current: PersonalGraphFeedback,
}

impl GraphFeedbackReceipt {
  /// Creates the event-and-projection receipt retained for idempotent replay.
  pub fn new(event: GraphFeedbackEvent, current: PersonalGraphFeedback) -> Self {
    Self { event, current }
  }
}

/// Result of checking an owner-scoped graph-feedback idempotency key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphFeedbackIdempotencyResult {
  /// No prior accepted request owns this key for this owner.
  Absent,
  /// A matching accepted request can return its original receipt without revalidating the graph.
  Replayed(GraphFeedbackReceipt),
  /// This key belongs to a different normalized request for the same owner.
  Conflict,
}

/// Result of atomically appending one private feedback event and updating its projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphFeedbackWriteResult {
  /// The store appended the new immutable event and advanced the current projection.
  Recorded(GraphFeedbackReceipt),
  /// A concurrent or later matching request already completed and returned its original receipt.
  Replayed(GraphFeedbackReceipt),
  /// The owner reused the same idempotency key for a different normalized request.
  Conflict,
}

/// Failure from private graph-feedback persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphFeedbackStoreError {
  /// A fresh event identifier collided with an existing immutable event.
  #[error("graph-feedback event identifier already exists")]
  EventIdConflict,
  /// A current projection could not represent another accepted event.
  #[error(transparent)]
  Projection(#[from] FeedbackProjectionError),
  /// The private feedback dependency could not complete the operation.
  #[error("graph-feedback store is unavailable")]
  Unavailable,
}

/// Appends private feedback events and maintains per-owner current projections.
///
/// Production implementations must atomically compare the owner-scoped idempotency key and
/// fingerprint, append the event, and update the current projection. The event ledger is append
/// only; reset actions clear a current dimension but do not erase the corresponding event. A
/// store never authorizes an owner or mutates a canonical relation, source, edge type, embedding,
/// canonical status, or community aggregate.
#[async_trait]
pub trait GraphFeedbackStore: Send + Sync {
  /// Finds an already accepted result for one owner-scoped idempotency key.
  ///
  /// The application service calls this before canonical eligibility validation so an accepted
  /// retry remains replayable even if a later release supersedes the relation version. The write
  /// method remains authoritative because another caller can race after this preflight.
  ///
  /// # Errors
  ///
  /// Returns an error when the private feedback dependency cannot serve the read.
  async fn idempotency_result(
    &self,
    owner: &LearnerId,
    idempotency_key: &FeedbackIdempotencyKey,
    request_fingerprint: &FeedbackRequestFingerprint,
  ) -> Result<GraphFeedbackIdempotencyResult, GraphFeedbackStoreError>;

  /// Atomically appends an event and applies it to exactly one owner-and-target projection.
  ///
  /// A matching existing owner/key/fingerprint must return `Replayed`; a different fingerprint
  /// must return `Conflict`; otherwise the store must append the event and current projection as
  /// one durable operation. The result must remain correct under concurrent duplicate requests.
  ///
  /// # Errors
  ///
  /// Returns an error when the event ID conflicts or the store cannot atomically persist state.
  async fn write(
    &self,
    write: GraphFeedbackWrite,
  ) -> Result<GraphFeedbackWriteResult, GraphFeedbackStoreError>;

  /// Loads one current private projection for exactly the supplied owner and target.
  ///
  /// The caller must derive `owner` from authorization. The method must never return another
  /// owner's projection for the requested target.
  ///
  /// # Errors
  ///
  /// Returns an error when the private feedback dependency cannot serve the read.
  async fn current(
    &self,
    owner: &LearnerId,
    target: &GraphFeedbackTarget,
  ) -> Result<Option<PersonalGraphFeedback>, GraphFeedbackStoreError>;
}
