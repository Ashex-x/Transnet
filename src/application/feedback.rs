//! Private graph-feedback orchestration with canonical eligibility validation.
//!
//! This service accepts a typed event only after the current canonical stored-edge version and its
//! explicit feedback capability agree. It does not authenticate callers, persist to MySQL,
//! encrypt private state, rate-limit abuse, moderate reports, aggregate community values, expose
//! HTTP routes, or mutate canonical graph facts.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  domain::feedback::{
    FeedbackIdempotencyKey, FeedbackRequestFingerprint, GraphFeedback, GraphFeedbackEvent,
    GraphFeedbackTarget, PersonalGraphFeedback,
  },
  domain::learner::LearnerId,
  ports::{
    clock::Clock,
    graph_feedback::{
      GraphFeedbackCatalog, GraphFeedbackCatalogError, GraphFeedbackIdempotencyResult,
      GraphFeedbackReceipt, GraphFeedbackStore, GraphFeedbackStoreError, GraphFeedbackWrite,
      GraphFeedbackWriteResult,
    },
    public_id::{PublicIdGenerationError, PublicIdGenerator},
  },
};

/// One authenticated caller's requested private graph-feedback mutation.
///
/// The API boundary must derive `owner`, `idempotency_key`, and `request_fingerprint` from
/// protected authentication and request data. The service does not implement OIDC or accept raw
/// owner identities, client idempotency keys, or unnormalized payloads.
#[derive(Clone)]
pub struct SubmitGraphFeedback {
  owner: LearnerId,
  idempotency_key: FeedbackIdempotencyKey,
  request_fingerprint: FeedbackRequestFingerprint,
  target: GraphFeedbackTarget,
  feedback: GraphFeedback,
}

impl SubmitGraphFeedback {
  /// Creates one typed, owner-scoped feedback submission.
  pub fn new(
    owner: LearnerId,
    idempotency_key: FeedbackIdempotencyKey,
    request_fingerprint: FeedbackRequestFingerprint,
    target: GraphFeedbackTarget,
    feedback: GraphFeedback,
  ) -> Self {
    Self {
      owner,
      idempotency_key,
      request_fingerprint,
      target,
      feedback,
    }
  }

  /// Returns the authenticated opaque owner reference used only for private storage access.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the opaque owner-scoped idempotency-key digest.
  pub fn idempotency_key(&self) -> &FeedbackIdempotencyKey {
    &self.idempotency_key
  }

  /// Returns the normalized semantic-request fingerprint used for conflict detection.
  pub fn request_fingerprint(&self) -> &FeedbackRequestFingerprint {
    &self.request_fingerprint
  }

  /// Returns the exact stored relation version evaluated by the learner.
  pub fn target(&self) -> &GraphFeedbackTarget {
    &self.target
  }

  /// Returns the requested usefulness or accuracy action.
  pub const fn feedback(&self) -> GraphFeedback {
    self.feedback
  }
}

impl std::fmt::Debug for SubmitGraphFeedback {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("SubmitGraphFeedback")
      .field("owner", &self.owner)
      .field("idempotency_key", &self.idempotency_key)
      .field("request_fingerprint", &self.request_fingerprint)
      .field("target", &self.target)
      .field("feedback", &self.feedback)
      .finish()
  }
}

/// Result of submitting personal graph feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphFeedbackSubmission {
  /// A new immutable event was accepted and applied to the owner projection.
  Recorded(GraphFeedbackReceipt),
  /// A matching prior submission returned the original immutable receipt.
  Replayed(GraphFeedbackReceipt),
}

impl GraphFeedbackSubmission {
  /// Returns the accepted immutable event and current private projection.
  pub fn receipt(&self) -> &GraphFeedbackReceipt {
    match self {
      Self::Recorded(receipt) | Self::Replayed(receipt) => receipt,
    }
  }

  /// Returns whether this request returned a prior result rather than appending another event.
  pub const fn is_replayed(&self) -> bool {
    matches!(self, Self::Replayed(_))
  }
}

/// Failure from graph-feedback orchestration.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphFeedbackError {
  /// The current canonical catalog did not expose a feedback-eligible stored edge.
  #[error("graph-feedback target was not found")]
  TargetNotFound,
  /// The request referenced a version other than the current stored relation version.
  #[error("graph-feedback relation version is unavailable")]
  RelationVersionUnavailable,
  /// The stored relation version did not enable the requested feedback dimension.
  #[error("graph-feedback capability is unavailable")]
  CapabilityUnavailable,
  /// The same owner reused an idempotency key for a different normalized request.
  #[error("graph-feedback idempotency key conflicts with another request")]
  IdempotencyConflict,
  /// The canonical graph-feedback eligibility dependency failed.
  #[error(transparent)]
  Catalog(#[from] GraphFeedbackCatalogError),
  /// The private feedback persistence dependency failed.
  #[error(transparent)]
  Store(#[from] GraphFeedbackStoreError),
  /// The public event-ID generator could not produce a new immutable event ID.
  #[error(transparent)]
  EventIdGeneration(#[from] PublicIdGenerationError),
}

/// Coordinates validated private graph-feedback events and current per-owner projections.
#[derive(Clone)]
pub struct GraphFeedbackService {
  catalog: Arc<dyn GraphFeedbackCatalog>,
  store: Arc<dyn GraphFeedbackStore>,
  clock: Arc<dyn Clock>,
  ids: Arc<dyn PublicIdGenerator>,
}

impl GraphFeedbackService {
  /// Creates a feedback service from explicit graph eligibility, private-storage, clock, and ID
  /// dependencies.
  pub fn new(
    catalog: Arc<dyn GraphFeedbackCatalog>,
    store: Arc<dyn GraphFeedbackStore>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn PublicIdGenerator>,
  ) -> Self {
    Self {
      catalog,
      store,
      clock,
      ids,
    }
  }

  /// Validates and records one private usefulness or accuracy action.
  ///
  /// A matching accepted retry is returned before a current graph lookup so a later release does
  /// not make the old event unreplayable. For a new write, the service requires the exact current
  /// relation version and explicit dimension capability, then relies on the store for an atomic
  /// append-plus-projection update and concurrent idempotency result.
  ///
  /// # Errors
  ///
  /// Returns an error for an unknown or superseded relation, an unsupported capability, a
  /// conflicting idempotency key, or an unavailable dependency.
  pub async fn submit(
    &self,
    command: SubmitGraphFeedback,
  ) -> Result<GraphFeedbackSubmission, GraphFeedbackError> {
    match self
      .store
      .idempotency_result(
        command.owner(),
        command.idempotency_key(),
        command.request_fingerprint(),
      )
      .await?
    {
      GraphFeedbackIdempotencyResult::Replayed(receipt) => {
        return Ok(GraphFeedbackSubmission::Replayed(receipt));
      }
      GraphFeedbackIdempotencyResult::Conflict => {
        return Err(GraphFeedbackError::IdempotencyConflict)
      }
      GraphFeedbackIdempotencyResult::Absent => {}
    }

    let edge = self
      .catalog
      .find(command.target().edge_id())
      .await?
      .ok_or(GraphFeedbackError::TargetNotFound)?;
    if edge.target.edge_id() != command.target().edge_id() {
      return Err(GraphFeedbackCatalogError::InconsistentData.into());
    }
    if edge.target.relation_version() != command.target().relation_version() {
      return Err(GraphFeedbackError::RelationVersionUnavailable);
    }
    if !edge.accepts(command.feedback().capability()) {
      return Err(GraphFeedbackError::CapabilityUnavailable);
    }

    let event = GraphFeedbackEvent::new(
      self.ids.generate()?,
      command.target().clone(),
      command.feedback(),
      self.clock.now(),
    );
    let write = GraphFeedbackWrite::new(
      command.owner().clone(),
      command.idempotency_key().clone(),
      command.request_fingerprint().clone(),
      event,
    );
    match self.store.write(write).await? {
      GraphFeedbackWriteResult::Recorded(receipt) => Ok(GraphFeedbackSubmission::Recorded(receipt)),
      GraphFeedbackWriteResult::Replayed(receipt) => Ok(GraphFeedbackSubmission::Replayed(receipt)),
      GraphFeedbackWriteResult::Conflict => Err(GraphFeedbackError::IdempotencyConflict),
    }
  }

  /// Returns the current private projection for one authenticated owner and exact relation version.
  ///
  /// This lookup intentionally does not require current catalog eligibility: a learner may inspect
  /// a historical projection that remains pinned to a superseded relation version.
  ///
  /// # Errors
  ///
  /// Returns an error when the private feedback store cannot serve the owner-scoped read.
  pub async fn current(
    &self,
    owner: &LearnerId,
    target: &GraphFeedbackTarget,
  ) -> Result<Option<PersonalGraphFeedback>, GraphFeedbackError> {
    Ok(self.store.current(owner, target).await?)
  }
}

#[cfg(test)]
mod tests {
  use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock,
      in_memory::{InMemoryGraphFeedbackCatalog, InMemoryGraphFeedbackStore},
      public_id::SequencePublicIdGenerator,
    },
    domain::{
      canonical::CanonicalId,
      feedback::{AccuracyFeedback, PersonalAccuracy, PersonalUsefulness, UsefulnessFeedback},
      graph::{GraphEdgeId, GraphFeedbackCapability, GraphNodeKey, GraphNodeKind, RelationVersion},
    },
    ports::graph_feedback::GraphFeedbackEdge,
  };

  fn target(version: u32) -> GraphFeedbackTarget {
    GraphFeedbackTarget::new(
      GraphEdgeId::stored(CanonicalId::new("edge-1").unwrap()),
      RelationVersion::new(version).unwrap(),
    )
  }

  fn derived_target() -> GraphFeedbackTarget {
    let release = CanonicalId::new("release-1").unwrap();
    let scale = CanonicalId::new("scale-1").unwrap();
    let lower = GraphNodeKey::new(GraphNodeKind::Sense, CanonicalId::new("sense-low").unwrap());
    let higher = GraphNodeKey::new(
      GraphNodeKind::Sense,
      CanonicalId::new("sense-high").unwrap(),
    );
    GraphFeedbackTarget::new(
      GraphEdgeId::derived_scale(&release, &scale, &lower, &higher),
      RelationVersion::new(1).unwrap(),
    )
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn command(
    owner: LearnerId,
    key: u8,
    fingerprint: u8,
    target: GraphFeedbackTarget,
    feedback: GraphFeedback,
  ) -> SubmitGraphFeedback {
    SubmitGraphFeedback::new(
      owner,
      FeedbackIdempotencyKey::new([key; 32]),
      FeedbackRequestFingerprint::new([fingerprint; 32]),
      target,
      feedback,
    )
  }

  fn id(random: u128) -> crate::ports::public_id::PublicId {
    Ulid::from_parts(1_700_000_000_000, random).into()
  }

  fn service(
    catalog: InMemoryGraphFeedbackCatalog,
    store: InMemoryGraphFeedbackStore,
    ids: impl IntoIterator<Item = crate::ports::public_id::PublicId>,
  ) -> GraphFeedbackService {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
    GraphFeedbackService::new(
      Arc::new(catalog),
      Arc::new(store),
      Arc::new(FixedClock::new(now)),
      Arc::new(SequencePublicIdGenerator::new(ids)),
    )
  }

  #[tokio::test]
  async fn validates_edge_version_and_capability_then_updates_only_one_owner_projection() {
    let allowed = InMemoryGraphFeedbackCatalog::new().with_edge(GraphFeedbackEdge::new(
      target(2),
      BTreeSet::from([
        GraphFeedbackCapability::Usefulness,
        GraphFeedbackCapability::Accuracy,
      ]),
    ));
    let store = InMemoryGraphFeedbackStore::new();
    let feedback = service(allowed, store.clone(), [id(1), id(2)]);

    let first = feedback
      .submit(command(
        owner("owner-a"),
        1,
        10,
        target(2),
        GraphFeedback::Usefulness(UsefulnessFeedback::More),
      ))
      .await
      .unwrap();
    assert!(!first.is_replayed());
    let second = feedback
      .submit(command(
        owner("owner-a"),
        2,
        20,
        target(2),
        GraphFeedback::Accuracy(AccuracyFeedback::Unsure),
      ))
      .await
      .unwrap();
    assert_eq!(second.receipt().current.version, 2);

    let projection = feedback
      .current(&owner("owner-a"), &target(2))
      .await
      .unwrap()
      .unwrap();
    assert_eq!(projection.usefulness, Some(PersonalUsefulness::More));
    assert_eq!(projection.accuracy, Some(PersonalAccuracy::Unsure));
    assert_eq!(
      feedback
        .current(&owner("owner-b"), &target(2))
        .await
        .unwrap(),
      None
    );

    let stale = feedback
      .submit(command(
        owner("owner-a"),
        3,
        30,
        target(1),
        GraphFeedback::Usefulness(UsefulnessFeedback::Less),
      ))
      .await;
    assert_eq!(stale, Err(GraphFeedbackError::RelationVersionUnavailable));

    let usefulness_only = service(
      InMemoryGraphFeedbackCatalog::new().with_edge(GraphFeedbackEdge::new(
        target(2),
        BTreeSet::from([GraphFeedbackCapability::Usefulness]),
      )),
      store,
      [id(3)],
    );
    let unsupported = usefulness_only
      .submit(command(
        owner("owner-a"),
        4,
        40,
        target(2),
        GraphFeedback::Accuracy(AccuracyFeedback::Unsupported),
      ))
      .await;
    assert_eq!(unsupported, Err(GraphFeedbackError::CapabilityUnavailable));

    let derived = feedback
      .submit(command(
        owner("owner-a"),
        5,
        50,
        derived_target(),
        GraphFeedback::Usefulness(UsefulnessFeedback::More),
      ))
      .await;
    assert_eq!(derived, Err(GraphFeedbackError::TargetNotFound));
  }

  #[tokio::test]
  async fn replay_precedes_catalog_validation_and_conflicts_do_not_append_another_event() {
    let catalog = InMemoryGraphFeedbackCatalog::new().with_edge(GraphFeedbackEdge::new(
      target(1),
      BTreeSet::from([GraphFeedbackCapability::Accuracy]),
    ));
    let store = InMemoryGraphFeedbackStore::new();
    let first_service = service(catalog, store.clone(), [id(10)]);
    let first_command = command(
      owner("owner-a"),
      1,
      2,
      target(1),
      GraphFeedback::Accuracy(AccuracyFeedback::Accurate),
    );
    let first = first_service.submit(first_command.clone()).await.unwrap();

    let replay_service = service(InMemoryGraphFeedbackCatalog::new(), store.clone(), [id(11)]);
    let replay = replay_service.submit(first_command).await.unwrap();
    assert!(replay.is_replayed());
    assert_eq!(replay.receipt(), first.receipt());

    let conflict = replay_service
      .submit(command(
        owner("owner-a"),
        1,
        3,
        target(1),
        GraphFeedback::Accuracy(AccuracyFeedback::Unsupported),
      ))
      .await;
    assert_eq!(conflict, Err(GraphFeedbackError::IdempotencyConflict));
    assert_eq!(
      store
        .current(&owner("owner-a"), &target(1))
        .await
        .unwrap()
        .unwrap()
        .version,
      1
    );
  }
}
