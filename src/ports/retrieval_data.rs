//! Outbound-only, release-pinned retrieval-data operations.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  request_context::RequestContext,
  retrieval_data::{
    EdgeSearchRequest, EdgeSearchResult, NeighborSearchRequest, NeighborSearchResult,
    NodeSearchRequest, NodeSearchResult, ScaleSearchRequest, ScaleSearchResult,
  },
};

/// Closed content-free failures exposed by the retrieval-data boundary.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RetrievalDataError {
  /// Local request bounds, schema, release, or relation validation failed.
  #[error("retrieval-data request is invalid")]
  InvalidRequest,
  /// No eligible value exists in the pinned release.
  #[error("retrieval-data value was not found")]
  NotFound,
  /// The selected immutable release is no longer readable.
  #[error("retrieval-data release is unavailable")]
  ContentReleaseUnavailable,
  /// The peer does not implement the required retrieval-data schema.
  #[error("retrieval-data schema is incompatible")]
  SchemaIncompatible,
  /// The local island-port dependency could not complete the operation.
  #[error("retrieval-data dependency is unavailable")]
  Unavailable,
  /// The request deadline was exhausted.
  #[error("retrieval-data request timed out")]
  Timeout,
  /// The peer response was malformed, oversized, or contradicted the request.
  #[error("retrieval-data response is inconsistent")]
  InconsistentData,
}

/// Storage-neutral candidate and direct-neighbor reads from immutable projections.
#[async_trait]
pub trait RetrievalDataPort: Send + Sync {
  /// Nominates canonical nodes without treating similarity as factual proof.
  async fn search_nodes(
    &self,
    context: &RequestContext,
    request: NodeSearchRequest,
  ) -> Result<NodeSearchResult, RetrievalDataError>;

  /// Finds complete semantic-scale pointers containing one explicit member.
  async fn search_scales(
    &self,
    context: &RequestContext,
    request: ScaleSearchRequest,
  ) -> Result<ScaleSearchResult, RetrievalDataError>;

  /// Nominates typed canonical relationships requiring authoritative hydration.
  async fn search_edges(
    &self,
    context: &RequestContext,
    request: EdgeSearchRequest,
  ) -> Result<EdgeSearchResult, RetrievalDataError>;

  /// Reads one bounded page of direct canonical neighbors.
  async fn search_neighbors(
    &self,
    context: &RequestContext,
    request: NeighborSearchRequest,
  ) -> Result<NeighborSearchResult, RetrievalDataError>;
}
