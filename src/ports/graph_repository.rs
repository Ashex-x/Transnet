//! Read-only port for canonical graph topology.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::graph::{
  GraphContentVersion, GraphFilter, GraphNode, GraphNodeKey, SemanticScale, StoredGraphRelation,
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

/// Reads immutable canonical graph records without exposing storage or query details.
///
/// Implementations must pin every record to `GraphAdjacencyRequest::content.release_id`, apply
/// release and publication eligibility filters, and return no more than `record_limit` canonical
/// records. The application service repeats relation validation and endpoint completion checks
/// before building a public graph result.
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
}
