//! Deterministic process-local private saved graph-view storage.
//!
//! The adapter models owner-safe reads and one-process atomic `If-Match` replacements for tests
//! and local development. It is not durable, encrypted, authenticated, replicated, or a database
//! transaction implementation.

use std::{
  collections::BTreeMap,
  sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
  adapters::public_id::mutex_lock,
  domain::{
    graph_view::{GraphViewEtag, GraphViewIfMatch, GraphViewPositions, SavedGraphView},
    learner::LearnerId,
  },
  ports::{
    graph_view::{
      GraphViewCreateResult, GraphViewReplaceResult, GraphViewStore, GraphViewStoreError,
    },
    public_id::PublicId,
  },
};

/// Process-local saved graph-view store with owner-safe optimistic replacements.
///
/// Clones and [`InMemoryGraphViewStore::reopen`] share one deterministic in-memory state. The
/// mutex makes each replacement atomic only within this process; it provides no production
/// durability, distributed concurrency, authorization, encryption, retention, or deletion flow.
#[derive(Clone, Default)]
pub struct InMemoryGraphViewStore {
  state: Arc<Mutex<GraphViewState>>,
}

impl InMemoryGraphViewStore {
  /// Creates an empty process-local private saved-view store.
  pub fn new() -> Self {
    Self::default()
  }

  /// Opens another handle over the same process-local saved-view state.
  pub fn reopen(&self) -> Self {
    self.clone()
  }
}

#[async_trait]
impl GraphViewStore for InMemoryGraphViewStore {
  async fn create(
    &self,
    view: SavedGraphView,
  ) -> Result<GraphViewCreateResult, GraphViewStoreError> {
    let mut state = mutex_lock(&self.state);
    if state.views.contains_key(view.id()) {
      return Ok(GraphViewCreateResult::IdConflict);
    }

    state.views.insert(view.id().clone(), view.clone());
    Ok(GraphViewCreateResult::Created(view))
  }

  async fn find(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedGraphView>, GraphViewStoreError> {
    let state = mutex_lock(&self.state);
    Ok(
      state
        .views
        .get(id)
        .filter(|view| view.owner() == owner)
        .cloned(),
    )
  }

  async fn replace(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    if_match: &GraphViewIfMatch,
    next_etag: GraphViewEtag,
    positions: GraphViewPositions,
  ) -> Result<GraphViewReplaceResult, GraphViewStoreError> {
    let mut state = mutex_lock(&self.state);
    let Some(current) = state.views.get(id).cloned() else {
      return Ok(GraphViewReplaceResult::Missing);
    };
    if current.owner() != owner {
      return Ok(GraphViewReplaceResult::Missing);
    }
    if current.etag() != if_match.etag() {
      return Ok(GraphViewReplaceResult::PreconditionFailed {
        current_etag: current.etag().clone(),
      });
    }

    let replacement = current.replace_positions(next_etag, positions)?;
    state.views.insert(id.clone(), replacement.clone());
    Ok(GraphViewReplaceResult::Updated(replacement))
  }
}

#[derive(Default)]
struct GraphViewState {
  views: BTreeMap<PublicId, SavedGraphView>,
}

#[cfg(test)]
mod tests {
  use ulid::Ulid;

  use super::*;
  use crate::{
    domain::{
      canonical::CanonicalId,
      graph::{GraphNodeKey, GraphNodeKind},
      graph_view::GraphNodePosition,
    },
    ports::graph_view::GraphViewStore,
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn positions(value: &str) -> GraphViewPositions {
    let node = GraphNodeKey::new(GraphNodeKind::Sense, CanonicalId::new(value).unwrap());
    GraphViewPositions::new(vec![GraphNodePosition::new(node, 1.0, 2.0, 3.0).unwrap()]).unwrap()
  }

  fn view(id: u128, owner: LearnerId, etag: u128, node: &str) -> SavedGraphView {
    SavedGraphView::new(
      public_id(id),
      owner,
      GraphViewEtag::new(public_id(etag)),
      positions(node),
    )
  }

  #[tokio::test]
  async fn reads_hide_foreign_views_and_reopen_shares_private_state() {
    let store = InMemoryGraphViewStore::new();
    let first = owner("first-owner");
    let second = owner("second-owner");
    let saved = view(1, first.clone(), 2, "sense-a");
    assert!(matches!(
      store.create(saved.clone()).await.unwrap(),
      GraphViewCreateResult::Created(_)
    ));

    assert_eq!(store.find(&second, saved.id()).await.unwrap(), None);
    assert_eq!(
      store.reopen().find(&first, saved.id()).await.unwrap(),
      Some(saved)
    );
  }

  #[tokio::test]
  async fn failed_precondition_does_not_overwrite_positions() {
    let store = InMemoryGraphViewStore::new();
    let learner = owner("private-owner");
    let saved = view(10, learner.clone(), 11, "sense-a");
    store.create(saved.clone()).await.unwrap();

    let result = store
      .replace(
        &learner,
        saved.id(),
        &GraphViewIfMatch::new(GraphViewEtag::new(public_id(12))),
        GraphViewEtag::new(public_id(13)),
        positions("sense-b"),
      )
      .await
      .unwrap();
    assert_eq!(
      result,
      GraphViewReplaceResult::PreconditionFailed {
        current_etag: saved.etag().clone(),
      }
    );
    assert_eq!(store.find(&learner, saved.id()).await.unwrap(), Some(saved));
  }

  #[tokio::test]
  async fn duplicate_create_does_not_replace_the_existing_private_view() {
    let store = InMemoryGraphViewStore::new();
    let learner = owner("private-owner");
    let first = view(20, learner.clone(), 21, "sense-a");
    let duplicate_id = view(20, learner.clone(), 22, "sense-b");
    assert!(matches!(
      store.create(first.clone()).await.unwrap(),
      GraphViewCreateResult::Created(_)
    ));

    assert_eq!(
      store.create(duplicate_id).await.unwrap(),
      GraphViewCreateResult::IdConflict
    );
    assert_eq!(store.find(&learner, first.id()).await.unwrap(), Some(first));
  }
}
