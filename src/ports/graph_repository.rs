//! Read-only port for canonical graph topology.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::graph::{
  GraphContentVersion, GraphEdge, GraphEdgeOrderingKey, GraphFilter, GraphNode, GraphNodeKey,
  SemanticScale, StoredGraphRelation,
};

/// Typed failure from the canonical graph store.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphRepositoryError {
  /// The graph store could not complete the bounded read.
  #[error("graph repository unavailable")]
  Unavailable,
  /// Stored graph rows violated their canonical integrity contract.
  #[error("graph repository returned inconsistent data")]
  InconsistentData,
}

/// Bounded request for raw canonical adjacency records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphAdjacencyRequest {
  /// Exact graph content and aggregate versions selected before traversal begins.
  pub content: GraphContentVersion,
  /// Typed node whose adjacent canonical records are requested.
  pub node: GraphNodeKey,
  /// Relation filter that storage may apply before returning records.
  pub filter: GraphFilter,
  /// Hard upper bound on canonical relation and scale records returned by the adapter.
  pub record_limit: usize,
}

/// Canonical adjacency records from which the application projects graph edges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphAdjacency {
  /// Canonical stored relations touching the requested node.
  pub relations: Vec<StoredGraphRelation>,
  /// Ordered scales containing the requested sense, or selected by a scale root.
  pub scales: Vec<SemanticScale>,
}

/// Bounded request for one storage-ordered page of public direct-neighbor projections.
///
/// This is deliberately separate from [`GraphAdjacencyRequest`]. A repository that reaches a
/// raw-record cap cannot safely resume a rank-ordered public edge stream unless it applies the
/// public ordering key before limiting source records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNeighborPageRequest {
  /// Exact active graph content tuple selected before the page begins.
  pub content: GraphContentVersion,
  /// Typed node whose direct projections are requested.
  pub node: GraphNodeKey,
  /// Relation filter that storage may apply before ordering candidates.
  pub filter: GraphFilter,
  /// Last public ordering key accepted by the previous page, when resuming.
  pub after: Option<GraphEdgeOrderingKey>,
  /// Hard upper bound on projected edge candidates returned by this page.
  pub edge_limit: usize,
}

/// One deterministic page of direct-neighbor candidates from the canonical graph store.
///
/// The application validates endpoint completion before exposing any edge. `has_more` is true
/// only when a later public ordering key exists, including when the adapter needed to advance
/// beyond a raw canonical-record cap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphNeighborPage {
  /// Strictly ordered projected candidates after the request cursor.
  pub edges: Vec<GraphEdge>,
  /// Whether another candidate exists after every edge in `edges`.
  pub has_more: bool,
}

/// Reads immutable canonical graph records without exposing storage or query details.
///
/// Implementations must pin every record to the exact `GraphAdjacencyRequest::content` tuple
/// (release, ranking, and community-aggregate versions), apply release and public-publication
/// eligibility filters, and return no more than `record_limit` canonical records. The application
/// service repeats relation validation and endpoint completion checks before building a public
/// graph result.
#[async_trait]
pub trait GraphRepository: Send + Sync {
  /// Resolves the active graph release, ranker, and aggregate versions for a new read.
  async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError>;

  /// Loads a set of typed graph nodes from one pinned content version.
  ///
  /// Missing nodes are omitted so the application can distinguish an absent root from a truncated
  /// edge whose endpoint is no longer eligible.
  async fn load_nodes(
    &self,
    content: &GraphContentVersion,
    keys: &[GraphNodeKey],
  ) -> Result<Vec<GraphNode>, GraphRepositoryError>;

  /// Returns bounded canonical adjacency records for one typed node.
  async fn adjacency(
    &self,
    request: &GraphAdjacencyRequest,
  ) -> Result<GraphAdjacency, GraphRepositoryError>;

  /// Returns one globally ordered direct-neighbor candidate page.
  ///
  /// Implementations must pin candidates to the exact active request content tuple (release,
  /// ranking, and community-aggregate versions), apply public-publication eligibility before
  /// ordering or capping, apply `after` using the public graph-edge ordering before any raw
  /// source-record cap, return at most `edge_limit` strictly ordered candidates, and set
  /// `has_more` only when a later candidate exists. The default safely reports an unavailable
  /// dependency so existing adapters cannot accidentally provide unsafe cursor continuation.
  async fn neighbor_page(
    &self,
    _request: &GraphNeighborPageRequest,
  ) -> Result<GraphNeighborPage, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }
}
