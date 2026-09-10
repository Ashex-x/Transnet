//! Deterministic in-memory graph-feedback catalog and private event store.
//!
//! These adapters make event, projection, replay, and ownership behavior testable. They are
//! process-local only: they provide neither database durability, encryption, authentication,
//! retention, moderation, rate limits, anomaly controls, nor community aggregation.

use std::{
  collections::BTreeMap,
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  domain::{
    feedback::{
      FeedbackIdempotencyKey, FeedbackRequestFingerprint, GraphFeedbackEvent, GraphFeedbackTarget,
      PersonalGraphFeedback,
    },
    graph::GraphEdgeId,
    learner::LearnerId,
  },
  ports::graph_feedback::{
    GraphFeedbackCatalog, GraphFeedbackCatalogError, GraphFeedbackEdge,
    GraphFeedbackIdempotencyResult, GraphFeedbackReceipt, GraphFeedbackStore,
    GraphFeedbackStoreError, GraphFeedbackWrite, GraphFeedbackWriteResult,
  },
  ports::public_id::PublicId,
};

/// Immutable process-local canonical graph-feedback eligibility catalog.
///
/// Builder methods model only canonical stored relations. A real catalog must omit derived,
/// visual-only, unpublished, or otherwise ineligible edges before returning an eligibility value.
#[derive(Debug, Clone, Default)]
pub struct InMemoryGraphFeedbackCatalog {
  edges: BTreeMap<GraphEdgeId, GraphFeedbackEdge>,
}

impl InMemoryGraphFeedbackCatalog {
  /// Creates an empty deterministic graph-feedback eligibility catalog.
  pub fn new() -> Self {
    Self::default()
  }

  /// Adds or replaces one canonical stored edge's current version and capability set.
  pub fn with_edge(mut self, edge: GraphFeedbackEdge) -> Self {
    self.edges.insert(edge.target.edge_id().clone(), edge);
    self
  }
}

#[async_trait]
impl GraphFeedbackCatalog for InMemoryGraphFeedbackCatalog {
  async fn find(
    &self,
    edge_id: &GraphEdgeId,
  ) -> Result<Option<GraphFeedbackEdge>, GraphFeedbackCatalogError> {
    Ok(self.edges.get(edge_id).cloned())
  }
}

/// Process-local private feedback ledger with atomic current-projection updates.
///
/// Clones and [`InMemoryGraphFeedbackStore::reopen`] share process-local state so restart-style
/// tests can inspect idempotent behavior. This is not a durable or encrypted implementation, and
/// its lock only models one-process atomicity rather than a database transaction boundary.
#[derive(Clone, Default)]
pub struct InMemoryGraphFeedbackStore {
  state: Arc<Mutex<FeedbackState>>,
}

impl InMemoryGraphFeedbackStore {
  /// Creates an empty deterministic private feedback ledger.
  pub fn new() -> Self {
    Self::default()
  }

  /// Reopens another handle over the same process-local state for restart-style tests.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[async_trait]
impl GraphFeedbackStore for InMemoryGraphFeedbackStore {
  async fn idempotency_result(
    &self,
    owner: &LearnerId,
    idempotency_key: &FeedbackIdempotencyKey,
    request_fingerprint: &FeedbackRequestFingerprint,
  ) -> Result<GraphFeedbackIdempotencyResult, GraphFeedbackStoreError> {
    let state = mutex_lock(&self.state);
    Ok(state.idempotency_result(owner, idempotency_key, request_fingerprint))
  }

  async fn write(
    &self,
    write: GraphFeedbackWrite,
  ) -> Result<GraphFeedbackWriteResult, GraphFeedbackStoreError> {
    let mut state = mutex_lock(&self.state);
    match state.idempotency_result(
      write.owner(),
      write.idempotency_key(),
      write.request_fingerprint(),
    ) {
      GraphFeedbackIdempotencyResult::Replayed(receipt) => {
        return Ok(GraphFeedbackWriteResult::Replayed(receipt));
      }
      GraphFeedbackIdempotencyResult::Conflict => return Ok(GraphFeedbackWriteResult::Conflict),
      GraphFeedbackIdempotencyResult::Absent => {}
    }

    if state.events.contains_key(&write.event().id) {
      return Err(GraphFeedbackStoreError::EventIdConflict);
    }

    let projection_key = ProjectionKey::new(write.owner().clone(), write.event().target.clone());
    let current = match state.current.get(&projection_key).cloned() {
      Some(mut projection) => {
        projection.apply(write.event())?;
        projection
      }
      None => PersonalGraphFeedback::from_event(write.event()),
    };
    let receipt = GraphFeedbackReceipt::new(write.event().clone(), current.clone());
    let idempotency_key =
      IdempotencyKey::new(write.owner().clone(), write.idempotency_key().clone());

    state.events.insert(
      write.event().id.clone(),
      StoredFeedbackEvent {
        owner: write.owner().clone(),
        event: write.event().clone(),
      },
    );
    state.current.insert(projection_key, current);
    state.idempotency.insert(
      idempotency_key,
      StoredIdempotency {
        request_fingerprint: write.request_fingerprint().clone(),
        receipt: receipt.clone(),
      },
    );

    Ok(GraphFeedbackWriteResult::Recorded(receipt))
  }

  async fn current(
    &self,
    owner: &LearnerId,
    target: &GraphFeedbackTarget,
  ) -> Result<Option<PersonalGraphFeedback>, GraphFeedbackStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      state
        .current
        .get(&ProjectionKey::new(owner.clone(), target.clone()))
        .cloned(),
    )
  }
}

#[derive(Default)]
struct FeedbackState {
  events: BTreeMap<PublicId, StoredFeedbackEvent>,
  current: BTreeMap<ProjectionKey, PersonalGraphFeedback>,
  idempotency: BTreeMap<IdempotencyKey, StoredIdempotency>,
}

impl FeedbackState {
  fn idempotency_result(
    &self,
    owner: &LearnerId,
    idempotency_key: &FeedbackIdempotencyKey,
    request_fingerprint: &FeedbackRequestFingerprint,
  ) -> GraphFeedbackIdempotencyResult {
    let key = IdempotencyKey::new(owner.clone(), idempotency_key.clone());
    let Some(stored) = self.idempotency.get(&key) else {
      return GraphFeedbackIdempotencyResult::Absent;
    };
    if &stored.request_fingerprint == request_fingerprint {
      return GraphFeedbackIdempotencyResult::Replayed(stored.receipt.clone());
    }
    GraphFeedbackIdempotencyResult::Conflict
  }
}

struct StoredFeedbackEvent {
  #[allow(dead_code)]
  owner: LearnerId,
  #[allow(dead_code)]
  event: GraphFeedbackEvent,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProjectionKey {
  owner: LearnerId,
  target: GraphFeedbackTarget,
}

impl ProjectionKey {
  fn new(owner: LearnerId, target: GraphFeedbackTarget) -> Self {
    Self { owner, target }
  }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IdempotencyKey {
  owner: LearnerId,
  key: FeedbackIdempotencyKey,
}

impl IdempotencyKey {
  fn new(owner: LearnerId, key: FeedbackIdempotencyKey) -> Self {
    Self { owner, key }
  }
}

struct StoredIdempotency {
  request_fingerprint: FeedbackRequestFingerprint,
  receipt: GraphFeedbackReceipt,
}

#[cfg(test)]
mod tests {
  use std::{collections::BTreeSet, time::SystemTime};

  use ulid::Ulid;

  use super::*;
  use crate::{
    domain::{
      canonical::CanonicalId,
      feedback::{
        AccuracyFeedback, GraphFeedback, PersonalAccuracy, PersonalUsefulness, UsefulnessFeedback,
      },
      graph::{GraphFeedbackCapability, GraphNodeKey, GraphNodeKind, RelationVersion},
    },
    ports::graph_feedback::GraphFeedbackCatalog,
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn target() -> GraphFeedbackTarget {
    GraphFeedbackTarget::new(
      GraphEdgeId::stored(CanonicalId::new("edge-1").unwrap()),
      RelationVersion::new(1).unwrap(),
    )
  }

  fn derived_edge_id() -> GraphEdgeId {
    let release = CanonicalId::new("release-1").unwrap();
    let scale = CanonicalId::new("scale-1").unwrap();
    let lower = GraphNodeKey::new(GraphNodeKind::Sense, CanonicalId::new("sense-low").unwrap());
    let higher = GraphNodeKey::new(
      GraphNodeKind::Sense,
      CanonicalId::new("sense-high").unwrap(),
    );
    GraphEdgeId::derived_scale(&release, &scale, &lower, &higher)
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn write(
    owner: LearnerId,
    idempotency_byte: u8,
    fingerprint_byte: u8,
    event_random: u128,
    feedback: GraphFeedback,
  ) -> GraphFeedbackWrite {
    GraphFeedbackWrite::new(
      owner,
      FeedbackIdempotencyKey::new([idempotency_byte; 32]),
      FeedbackRequestFingerprint::new([fingerprint_byte; 32]),
      GraphFeedbackEvent::new(
        public_id(event_random),
        target(),
        feedback,
        SystemTime::UNIX_EPOCH,
      ),
    )
  }

  #[tokio::test]
  async fn catalog_returns_only_configured_canonical_edges() {
    let catalog = InMemoryGraphFeedbackCatalog::new().with_edge(GraphFeedbackEdge::new(
      target(),
      BTreeSet::from([GraphFeedbackCapability::Accuracy]),
    ));

    assert!(catalog.find(target().edge_id()).await.unwrap().is_some());
    assert!(catalog.find(&derived_edge_id()).await.unwrap().is_none());
  }

  #[tokio::test]
  async fn atomically_projects_events_and_replays_only_the_same_owner_request() {
    let store = InMemoryGraphFeedbackStore::new();
    let first = write(
      owner("owner-a"),
      1,
      2,
      10,
      GraphFeedback::Usefulness(UsefulnessFeedback::More),
    );
    let GraphFeedbackWriteResult::Recorded(first_receipt) = store.write(first).await.unwrap()
    else {
      panic!("first write should be recorded");
    };
    assert_eq!(
      first_receipt.current.usefulness,
      Some(PersonalUsefulness::More)
    );
    assert_eq!(first_receipt.current.version, 1);

    let replay = write(
      owner("owner-a"),
      1,
      2,
      11,
      GraphFeedback::Usefulness(UsefulnessFeedback::More),
    );
    let GraphFeedbackWriteResult::Replayed(replayed_receipt) = store.write(replay).await.unwrap()
    else {
      panic!("matching write should replay");
    };
    assert_eq!(replayed_receipt, first_receipt);

    assert_eq!(
      store
        .write(write(
          owner("owner-a"),
          1,
          3,
          12,
          GraphFeedback::Accuracy(AccuracyFeedback::Unsupported),
        ))
        .await
        .unwrap(),
      GraphFeedbackWriteResult::Conflict
    );

    let GraphFeedbackWriteResult::Recorded(second_owner) = store
      .write(write(
        owner("owner-b"),
        1,
        2,
        13,
        GraphFeedback::Accuracy(AccuracyFeedback::Unsure),
      ))
      .await
      .unwrap()
    else {
      panic!("idempotency keys must be scoped to the owner");
    };
    assert_eq!(
      second_owner.current.accuracy,
      Some(PersonalAccuracy::Unsure)
    );
    assert_eq!(
      store.current(&owner("owner-a"), &target()).await.unwrap(),
      Some(first_receipt.current)
    );
  }

  #[tokio::test]
  async fn reopens_shared_process_local_state_for_restart_style_tests() {
    let store = InMemoryGraphFeedbackStore::new();
    store
      .write(write(
        owner("owner-a"),
        1,
        2,
        20,
        GraphFeedback::Accuracy(AccuracyFeedback::Accurate),
      ))
      .await
      .unwrap();

    let reopened = store.reopen();
    assert!(matches!(
      reopened
        .idempotency_result(
          &owner("owner-a"),
          &FeedbackIdempotencyKey::new([1; 32]),
          &FeedbackRequestFingerprint::new([2; 32]),
        )
        .await
        .unwrap(),
      GraphFeedbackIdempotencyResult::Replayed(_)
    ));
  }

  #[tokio::test]
  async fn concurrent_matching_writes_append_once_and_replay_once() {
    let store = InMemoryGraphFeedbackStore::new();
    let first = write(
      owner("owner-a"),
      7,
      8,
      30,
      GraphFeedback::Usefulness(UsefulnessFeedback::Less),
    );
    let duplicate = write(
      owner("owner-a"),
      7,
      8,
      31,
      GraphFeedback::Usefulness(UsefulnessFeedback::Less),
    );

    let (left, right) = tokio::join!(store.write(first), store.write(duplicate));
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, GraphFeedbackWriteResult::Recorded(_)))
        .count(),
      1
    );
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, GraphFeedbackWriteResult::Replayed(_)))
        .count(),
      1
    );
    assert_eq!(
      store
        .current(&owner("owner-a"), &target())
        .await
        .unwrap()
        .unwrap()
        .version,
      1
    );
  }
}
