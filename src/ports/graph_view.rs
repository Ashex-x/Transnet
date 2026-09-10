//! Private saved graph-view persistence contract.
//!
//! This port preserves owner-safe missing behavior and atomic opaque `If-Match` replacement. It
//! stores presentation-only node positions, never graph topology or ranking. It does not
//! authenticate callers, implement HTTP headers, provide durable persistence, or coordinate
//! updates with canonical graph content.

use async_trait::async_trait;
use thiserror::Error;

use crate::{
  domain::{
    graph_view::{
      GraphViewEtag, GraphViewIfMatch, GraphViewPositions, GraphViewValidationError, SavedGraphView,
    },
    learner::LearnerId,
  },
  ports::public_id::PublicId,
};

/// Failure returned by a private saved graph-view storage dependency.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphViewStoreError {
  /// A store could not construct a valid next saved-view snapshot.
  #[error(transparent)]
  Validation(#[from] GraphViewValidationError),
  /// The private saved-view dependency could not complete the operation.
  #[error("saved graph view store is unavailable")]
  Unavailable,
}

/// Result of atomically creating one private saved graph view.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphViewCreateResult {
  /// The new private view was stored.
  Created(SavedGraphView),
  /// The generated stable ID already exists, so no view was written.
  IdConflict,
}

/// Result of atomically replacing one owned saved graph view.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphViewReplaceResult {
  /// The owned view was replaced and received a fresh opaque entity tag.
  Updated(SavedGraphView),
  /// The view is absent or belongs to a different owner.
  Missing,
  /// The owned view changed after the caller obtained its `If-Match` value.
  ///
  /// The current tag is exposed only after the ownership check succeeds, allowing an authorized
  /// caller to refresh its snapshot without revealing a foreign view's existence.
  PreconditionFailed {
    /// Current opaque tag for the owned view.
    current_etag: GraphViewEtag,
  },
}

/// Stores owner-scoped private graph-layout snapshots.
///
/// Implementations must use one atomic operation for owner matching, exact opaque-tag comparison,
/// and replacement. `Missing` must cover absent and foreign view IDs. A successful replacement
/// must write only validated presentation positions, preserve the view's owner and ID, and issue a
/// fresh [`GraphViewEtag`]. This port never authorizes the owner or changes canonical topology,
/// graph content versions, filters, ranks, scores, evidence, or feedback.
#[async_trait]
pub trait GraphViewStore: Send + Sync {
  /// Creates one fully validated private view unless its stable ID already exists.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot atomically persist the view.
  async fn create(
    &self,
    view: SavedGraphView,
  ) -> Result<GraphViewCreateResult, GraphViewStoreError>;

  /// Finds a private view only when it is owned by `owner`.
  ///
  /// `None` covers absent and foreign view IDs.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot serve the private read.
  async fn find(
    &self,
    owner: &LearnerId,
    id: &PublicId,
  ) -> Result<Option<SavedGraphView>, GraphViewStoreError>;

  /// Atomically replaces presentation positions when the exact `If-Match` tag is current.
  ///
  /// The store must compare ownership and `if_match` before accepting `next_etag` and positions.
  /// It must return `Missing` before exposing a precondition result for a foreign ID. The supplied
  /// new tag must differ from the stored tag, and a successful replacement must not alter the
  /// stable view ID or owner.
  ///
  /// # Errors
  ///
  /// Returns an error when the store cannot apply the atomic private replacement or the candidate
  /// next snapshot violates a domain invariant.
  async fn replace(
    &self,
    owner: &LearnerId,
    id: &PublicId,
    if_match: &GraphViewIfMatch,
    next_etag: GraphViewEtag,
    positions: GraphViewPositions,
  ) -> Result<GraphViewReplaceResult, GraphViewStoreError>;
}
