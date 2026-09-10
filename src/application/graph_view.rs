//! Private saved graph-layout orchestration without HTTP or rendering dependencies.
//!
//! The service accepts a trusted opaque [`crate::domain::learner::LearnerId`] and coordinates
//! owner-safe reads plus exact-tag layout replacements. It does not authenticate callers, parse
//! HTTP headers, retain topology or ranking, persist to MySQL, or implement WebUI behavior.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  domain::{
    graph_view::{GraphViewEtag, GraphViewIfMatch, GraphViewPositions, SavedGraphView},
    learner::LearnerId,
  },
  ports::{
    graph_view::{
      GraphViewCreateResult, GraphViewReplaceResult, GraphViewStore, GraphViewStoreError,
    },
    public_id::{PublicId, PublicIdGenerationError, PublicIdGenerator},
  },
};

/// Input for creating one private graph-layout view.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSavedGraphView {
  positions: GraphViewPositions,
}

impl CreateSavedGraphView {
  /// Creates a request containing validated presentation-only node positions.
  pub fn new(positions: GraphViewPositions) -> Self {
    Self { positions }
  }

  /// Returns the bounded position set to save.
  pub fn positions(&self) -> &GraphViewPositions {
    &self.positions
  }
}

/// Input for replacing an existing private graph-layout view.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceSavedGraphView {
  id: PublicId,
  if_match: GraphViewIfMatch,
  positions: GraphViewPositions,
}

impl ReplaceSavedGraphView {
  /// Creates an exact-tag replacement request with validated presentation-only positions.
  pub fn new(id: PublicId, if_match: GraphViewIfMatch, positions: GraphViewPositions) -> Self {
    Self {
      id,
      if_match,
      positions,
    }
  }

  /// Returns the stable public identifier of the owned view to replace.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the exact opaque tag that must still be current.
  pub fn if_match(&self) -> &GraphViewIfMatch {
    &self.if_match
  }

  /// Returns the bounded replacement positions.
  pub fn positions(&self) -> &GraphViewPositions {
    &self.positions
  }
}

/// Ownership-safe result of replacing a private graph-layout view.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceSavedGraphViewOutcome {
  /// The owned view was replaced with a fresh opaque entity tag.
  Updated(SavedGraphView),
  /// The view is absent or is owned by another learner.
  Missing,
  /// The owned view no longer has the request's exact entity tag.
  PreconditionFailed {
    /// Current opaque tag visible only to the verified owner.
    current_etag: GraphViewEtag,
  },
}

/// Coordinates owner-scoped saved graph-layout views through explicit storage and ID ports.
///
/// The caller must derive `LearnerId` from a verified authentication principal. This service does
/// not make authorization decisions or attach saved coordinates to a graph read. Replacements are
/// atomic at the storage port, so concurrent `If-Match` requests cannot both succeed.
#[derive(Clone)]
pub struct GraphViewService {
  store: Arc<dyn GraphViewStore>,
  ids: Arc<dyn PublicIdGenerator>,
}

impl GraphViewService {
  /// Creates saved-view orchestration from explicit private storage and public-ID dependencies.
  pub fn new(store: Arc<dyn GraphViewStore>, ids: Arc<dyn PublicIdGenerator>) -> Self {
    Self { store, ids }
  }

  /// Creates one private saved graph-layout view for `owner`.
  ///
  /// The view ID and initial opaque entity tag are generated independently. A collision leaves no
  /// partial write; production adapters should make it vanishingly unlikely with globally unique
  /// IDs.
  ///
  /// # Errors
  ///
  /// Returns an error when an ID cannot be generated, its improbable collision is reported, or
  /// the private storage dependency cannot create the view.
  pub async fn create(
    &self,
    owner: LearnerId,
    request: CreateSavedGraphView,
  ) -> Result<SavedGraphView, GraphViewServiceError> {
    let view = SavedGraphView::new(
      self.ids.generate()?,
      owner,
      GraphViewEtag::new(self.ids.generate()?),
      request.positions().clone(),
    );
    match self.store.create(view).await? {
      GraphViewCreateResult::Created(view) => Ok(view),
      GraphViewCreateResult::IdConflict => Err(GraphViewServiceError::IdConflict),
    }
  }

  /// Finds one private saved view only when it is owned by `owner`.
  ///
  /// `None` deliberately covers absent and foreign IDs.
  ///
  /// # Errors
  ///
  /// Returns an error when the private saved-view store cannot serve the read.
  pub async fn find(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedGraphView>, GraphViewServiceError> {
    Ok(self.store.find(owner, id).await?)
  }

  /// Replaces one owned saved layout when its exact opaque `If-Match` tag is current.
  ///
  /// A candidate next tag is allocated before the atomic store operation. It is intentionally
  /// discarded on a missing or failed-precondition result, preventing a read-then-write race.
  ///
  /// # Errors
  ///
  /// Returns an error when a fresh entity tag cannot be generated or the private store cannot
  /// apply the owner-safe atomic replacement.
  pub async fn replace(
    &self,
    owner: &LearnerId,
    request: ReplaceSavedGraphView,
  ) -> Result<ReplaceSavedGraphViewOutcome, GraphViewServiceError> {
    let next_etag = GraphViewEtag::new(self.ids.generate()?);
    match self
      .store
      .replace(
        owner,
        request.id(),
        request.if_match(),
        next_etag,
        request.positions().clone(),
      )
      .await?
    {
      GraphViewReplaceResult::Updated(view) => Ok(ReplaceSavedGraphViewOutcome::Updated(view)),
      GraphViewReplaceResult::Missing => Ok(ReplaceSavedGraphViewOutcome::Missing),
      GraphViewReplaceResult::PreconditionFailed { current_etag } => {
        Ok(ReplaceSavedGraphViewOutcome::PreconditionFailed { current_etag })
      }
    }
  }
}

/// Failure returned by private saved graph-view orchestration.
#[derive(Debug, Error)]
pub enum GraphViewServiceError {
  /// The private saved-view storage dependency could not complete the operation.
  #[error(transparent)]
  Store(#[from] GraphViewStoreError),
  /// A stable view ID or fresh opaque entity tag could not be generated.
  #[error("could not generate saved graph view identifier: {0}")]
  IdGeneration(#[from] PublicIdGenerationError),
  /// A generated stable view ID collided with an existing view before any write occurred.
  #[error("saved graph view identifier already exists")]
  IdConflict,
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use tokio::join;
  use ulid::Ulid;

  use super::*;
  use crate::{
    adapters::{in_memory::InMemoryGraphViewStore, public_id::SequencePublicIdGenerator},
    domain::{
      canonical::CanonicalId,
      graph::{GraphNodeKey, GraphNodeKind},
      graph_view::GraphNodePosition,
    },
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn owner(value: &str) -> LearnerId {
    LearnerId::new(value).unwrap()
  }

  fn positions(value: &str, x: f64) -> GraphViewPositions {
    let node = GraphNodeKey::new(GraphNodeKind::Sense, CanonicalId::new(value).unwrap());
    GraphViewPositions::new(vec![GraphNodePosition::new(node, x, 0.0, 0.0).unwrap()]).unwrap()
  }

  fn service(ids: impl IntoIterator<Item = PublicId>) -> GraphViewService {
    GraphViewService::new(
      Arc::new(InMemoryGraphViewStore::new()),
      Arc::new(SequencePublicIdGenerator::new(ids)),
    )
  }

  #[tokio::test]
  async fn create_and_exact_tag_replace_issue_a_fresh_snapshot() {
    let service = service([public_id(1), public_id(2), public_id(3)]);
    let learner = owner("private-owner");
    let created = service
      .create(
        learner.clone(),
        CreateSavedGraphView::new(positions("sense-a", 1.0)),
      )
      .await
      .unwrap();

    let outcome = service
      .replace(
        &learner,
        ReplaceSavedGraphView::new(
          created.id().clone(),
          GraphViewIfMatch::new(created.etag().clone()),
          positions("sense-b", 2.0),
        ),
      )
      .await
      .unwrap();
    let ReplaceSavedGraphViewOutcome::Updated(replaced) = outcome else {
      panic!("the matching tag must replace the owned view");
    };
    assert_ne!(created.etag(), replaced.etag());
    assert_eq!(
      replaced.positions().entries()[0].node().id.as_str(),
      "sense-b"
    );
  }

  #[tokio::test]
  async fn foreign_reads_and_replacements_are_missing() {
    let service = service([public_id(10), public_id(11), public_id(12)]);
    let owner_one = owner("owner-one");
    let owner_two = owner("owner-two");
    let created = service
      .create(
        owner_one.clone(),
        CreateSavedGraphView::new(positions("sense-a", 1.0)),
      )
      .await
      .unwrap();

    assert_eq!(service.find(&owner_two, created.id()).await.unwrap(), None);
    assert_eq!(
      service
        .replace(
          &owner_two,
          ReplaceSavedGraphView::new(
            created.id().clone(),
            GraphViewIfMatch::new(created.etag().clone()),
            positions("sense-b", 2.0),
          ),
        )
        .await
        .unwrap(),
      ReplaceSavedGraphViewOutcome::Missing
    );
  }

  #[tokio::test]
  async fn concurrent_exact_tag_replacements_allow_only_one_winner() {
    let service = service([public_id(20), public_id(21), public_id(22), public_id(23)]);
    let learner = owner("private-owner");
    let created = service
      .create(
        learner.clone(),
        CreateSavedGraphView::new(positions("sense-a", 1.0)),
      )
      .await
      .unwrap();
    let request_one = ReplaceSavedGraphView::new(
      created.id().clone(),
      GraphViewIfMatch::new(created.etag().clone()),
      positions("sense-b", 2.0),
    );
    let request_two = ReplaceSavedGraphView::new(
      created.id().clone(),
      GraphViewIfMatch::new(created.etag().clone()),
      positions("sense-c", 3.0),
    );

    let (first, second) = join!(
      service.replace(&learner, request_one),
      service.replace(&learner, request_two),
    );
    let outcomes = [first.unwrap(), second.unwrap()];
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplaceSavedGraphViewOutcome::Updated(_)))
        .count(),
      1
    );
    assert_eq!(
      outcomes
        .iter()
        .filter(|outcome| {
          matches!(
            outcome,
            ReplaceSavedGraphViewOutcome::PreconditionFailed { .. }
          )
        })
        .count(),
      1
    );
  }
}
