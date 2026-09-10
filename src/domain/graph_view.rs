//! Private saved graph-layout values and optimistic replacement invariants.
//!
//! A saved view stores only a bounded set of typed node identifiers and finite three-dimensional
//! coordinates. It intentionally contains no graph topology, adjacency, node labels, filters,
//! ranking components, feedback, camera state, force-layout output, or semantic evidence.
//! Positions are private presentation hints: they cannot change a graph read or rank result.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::{
  domain::{graph::GraphNodeKey, learner::LearnerId},
  ports::public_id::PublicId,
};

/// Largest number of saved node positions in one private graph view.
pub const MAX_SAVED_GRAPH_VIEW_POSITIONS: usize = crate::domain::graph::MAX_GRAPH_NODE_LIMIT;
/// Largest absolute coordinate accepted for one saved node position.
pub const MAX_GRAPH_POSITION_ABS: f64 = 1_000_000.0;

/// Validation failure for a private saved graph-layout value.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphViewValidationError {
  /// A saved view would retain more positions than the bounded contract permits.
  #[error("saved graph view contains too many node positions")]
  TooManyPositions,
  /// A saved view attempted to store more than one position for the same typed node.
  #[error("saved graph view contains duplicate node positions")]
  DuplicateNodePosition,
  /// A coordinate was `NaN` or infinite.
  #[error("saved graph position coordinates must be finite")]
  NonFiniteCoordinate,
  /// A finite coordinate exceeded the supported presentation-only range.
  #[error("saved graph position coordinates exceed the supported range")]
  CoordinateOutOfBounds,
  /// A successful replacement did not issue a fresh opaque entity tag.
  #[error("saved graph view replacement must issue a fresh ETag")]
  ReusedEtag,
}

/// One finite three-dimensional presentation hint for a typed graph node.
///
/// The node key is an identity reference only. This value contains no edge, neighbor, filter,
/// rank, score, label, evidence, or graph-content data, so coordinates cannot alter topology or
/// ranking.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNodePosition {
  node: GraphNodeKey,
  x: f64,
  y: f64,
  z: f64,
}

impl GraphNodePosition {
  /// Creates a bounded finite presentation position for `node`.
  ///
  /// Signed zero is normalized to positive zero so otherwise equivalent positions compare
  /// deterministically.
  ///
  /// # Errors
  ///
  /// Returns [`GraphViewValidationError::NonFiniteCoordinate`] when a coordinate is `NaN` or
  /// infinite, or [`GraphViewValidationError::CoordinateOutOfBounds`] when a finite coordinate is
  /// outside `-MAX_GRAPH_POSITION_ABS..=MAX_GRAPH_POSITION_ABS`.
  pub fn new(node: GraphNodeKey, x: f64, y: f64, z: f64) -> Result<Self, GraphViewValidationError> {
    for coordinate in [x, y, z] {
      if !coordinate.is_finite() {
        return Err(GraphViewValidationError::NonFiniteCoordinate);
      }
      if coordinate.abs() > MAX_GRAPH_POSITION_ABS {
        return Err(GraphViewValidationError::CoordinateOutOfBounds);
      }
    }

    Ok(Self {
      node,
      x: normalize_zero(x),
      y: normalize_zero(y),
      z: normalize_zero(z),
    })
  }

  /// Returns the typed node identity this presentation hint refers to.
  pub fn node(&self) -> &GraphNodeKey {
    &self.node
  }

  /// Returns the bounded presentation x coordinate.
  pub const fn x(&self) -> f64 {
    self.x
  }

  /// Returns the bounded presentation y coordinate.
  pub const fn y(&self) -> f64 {
    self.y
  }

  /// Returns the bounded presentation z coordinate.
  pub const fn z(&self) -> f64 {
    self.z
  }
}

/// Validated deterministic set of presentation-only node positions for one saved view.
///
/// Entries are ordered by typed node key and have no topology or ranking semantics. Omitting a
/// node means only that no private position was saved for it; it does not hide, add, or re-rank a
/// node in a canonical graph response.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphViewPositions {
  entries: Vec<GraphNodePosition>,
}

impl GraphViewPositions {
  /// Creates a bounded, duplicate-free set of saved node positions.
  ///
  /// # Errors
  ///
  /// Returns [`GraphViewValidationError::TooManyPositions`] when `entries` exceeds the fixed
  /// maximum, or [`GraphViewValidationError::DuplicateNodePosition`] when a typed node appears
  /// more than once.
  pub fn new(mut entries: Vec<GraphNodePosition>) -> Result<Self, GraphViewValidationError> {
    if entries.len() > MAX_SAVED_GRAPH_VIEW_POSITIONS {
      return Err(GraphViewValidationError::TooManyPositions);
    }

    let mut nodes = BTreeSet::new();
    if !entries
      .iter()
      .all(|position| nodes.insert(position.node.clone()))
    {
      return Err(GraphViewValidationError::DuplicateNodePosition);
    }

    entries.sort_by(|left, right| left.node.cmp(&right.node));
    Ok(Self { entries })
  }

  /// Creates an empty presentation-only layout.
  pub const fn empty() -> Self {
    Self {
      entries: Vec::new(),
    }
  }

  /// Returns positions in deterministic typed-node-key order.
  pub fn entries(&self) -> &[GraphNodePosition] {
    &self.entries
  }
}

impl Default for GraphViewPositions {
  fn default() -> Self {
    Self::empty()
  }
}

/// Opaque strong entity tag for one exact private saved-view revision.
///
/// A transport boundary can emit this token as a quoted `ETag` header and require an exact value
/// in `If-Match`. Callers must compare it only for equality: it is not a topology, ranking, graph
/// content, or numeric revision value. Each successful replacement must use a freshly generated
/// tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphViewEtag(PublicId);

impl GraphViewEtag {
  /// Wraps a generated public identifier as an opaque view-revision tag.
  pub fn new(value: PublicId) -> Self {
    Self(value)
  }

  /// Returns the opaque token for exact comparison or future HTTP header serialization.
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

/// Exact optimistic precondition supplied when replacing one saved graph view.
///
/// This is the typed equivalent of a single strong `If-Match` value. Wildcards and weak tags are
/// intentionally not representable, so a replacement can proceed only from a snapshot's exact
/// current [`GraphViewEtag`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphViewIfMatch(GraphViewEtag);

impl GraphViewIfMatch {
  /// Creates an exact replace precondition from the view's last observed entity tag.
  pub fn new(etag: GraphViewEtag) -> Self {
    Self(etag)
  }

  /// Returns the exact entity tag that must still be current for a replacement to proceed.
  pub fn etag(&self) -> &GraphViewEtag {
    &self.0
  }
}

/// Immutable private snapshot of one saved graph-layout view.
///
/// The owner is an opaque authenticated-principal reference and is redacted in debug output by
/// [`LearnerId`]. The snapshot deliberately exposes only a public view ID, an opaque revision tag,
/// and presentation-only positions; it cannot carry graph topology or ranking data.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedGraphView {
  id: PublicId,
  owner: LearnerId,
  etag: GraphViewEtag,
  positions: GraphViewPositions,
}

impl SavedGraphView {
  /// Creates an immutable owner-scoped graph-layout snapshot.
  pub fn new(
    id: PublicId,
    owner: LearnerId,
    etag: GraphViewEtag,
    positions: GraphViewPositions,
  ) -> Self {
    Self {
      id,
      owner,
      etag,
      positions,
    }
  }

  /// Returns the stable public identifier for this saved view.
  pub fn id(&self) -> &PublicId {
    &self.id
  }

  /// Returns the opaque owner reference used only at a protected private-state boundary.
  pub fn owner(&self) -> &LearnerId {
    &self.owner
  }

  /// Returns the exact opaque entity tag required for the next replacement.
  pub fn etag(&self) -> &GraphViewEtag {
    &self.etag
  }

  /// Returns the bounded presentation-only node positions in deterministic order.
  pub fn positions(&self) -> &GraphViewPositions {
    &self.positions
  }

  /// Creates the next immutable snapshot after a successful exact-tag replacement.
  ///
  /// # Errors
  ///
  /// Returns [`GraphViewValidationError::ReusedEtag`] when `next_etag` does not advance the
  /// opaque revision token.
  pub fn replace_positions(
    &self,
    next_etag: GraphViewEtag,
    positions: GraphViewPositions,
  ) -> Result<Self, GraphViewValidationError> {
    if self.etag == next_etag {
      return Err(GraphViewValidationError::ReusedEtag);
    }

    Ok(Self::new(
      self.id.clone(),
      self.owner.clone(),
      next_etag,
      positions,
    ))
  }
}

fn normalize_zero(value: f64) -> f64 {
  if value == 0.0 {
    0.0
  } else {
    value
  }
}

#[cfg(test)]
mod tests {
  use ulid::Ulid;

  use super::*;
  use crate::domain::{
    canonical::CanonicalId,
    graph::{GraphNodeKey, GraphNodeKind},
  };

  fn public_id(random: u128) -> PublicId {
    PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
  }

  fn node(value: &str) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Sense, CanonicalId::new(value).unwrap())
  }

  fn position(value: &str, x: f64) -> GraphNodePosition {
    GraphNodePosition::new(node(value), x, 0.0, 1.0).unwrap()
  }

  #[test]
  fn positions_reject_nonfinite_and_out_of_range_coordinates() {
    assert_eq!(
      GraphNodePosition::new(node("a"), f64::NAN, 0.0, 0.0),
      Err(GraphViewValidationError::NonFiniteCoordinate)
    );
    assert_eq!(
      GraphNodePosition::new(node("a"), 0.0, f64::INFINITY, 0.0),
      Err(GraphViewValidationError::NonFiniteCoordinate)
    );
    assert_eq!(
      GraphNodePosition::new(node("a"), MAX_GRAPH_POSITION_ABS + 1.0, 0.0, 0.0),
      Err(GraphViewValidationError::CoordinateOutOfBounds)
    );
  }

  #[test]
  fn position_collections_are_bounded_unique_and_deterministically_ordered() {
    let duplicate = GraphViewPositions::new(vec![position("b", 1.0), position("b", 2.0)]);
    assert_eq!(
      duplicate,
      Err(GraphViewValidationError::DuplicateNodePosition)
    );

    let entries = (0..=MAX_SAVED_GRAPH_VIEW_POSITIONS)
      .map(|index| position(&format!("node-{index}"), f64::from(index as u32)))
      .collect();
    assert_eq!(
      GraphViewPositions::new(entries),
      Err(GraphViewValidationError::TooManyPositions)
    );

    let positions = GraphViewPositions::new(vec![position("z", 1.0), position("a", 2.0)]).unwrap();
    assert_eq!(positions.entries()[0].node().id.as_str(), "a");
    assert_eq!(positions.entries()[1].node().id.as_str(), "z");
  }

  #[test]
  fn replacement_requires_a_fresh_opaque_etag() {
    let etag = GraphViewEtag::new(public_id(1));
    let view = SavedGraphView::new(
      public_id(2),
      LearnerId::new("private-owner").unwrap(),
      etag.clone(),
      GraphViewPositions::new(vec![position("a", 1.0)]).unwrap(),
    );

    assert_eq!(
      view.replace_positions(etag, GraphViewPositions::empty()),
      Err(GraphViewValidationError::ReusedEtag)
    );
    let replaced = view
      .replace_positions(
        GraphViewEtag::new(public_id(3)),
        GraphViewPositions::empty(),
      )
      .unwrap();
    assert_ne!(replaced.etag(), view.etag());
    assert!(replaced.positions().entries().is_empty());
  }
}
