//! Version 1 learning HTTP API routing and common failures.

use axum::{
  extract::Extension,
  http::StatusCode,
  response::Response,
  routing::{get, post},
  Router,
};

use super::{problem, request_id::RequestId, AppState};

pub(crate) mod graph;
pub(crate) mod lookup;
pub(crate) mod lookup_job;
pub(crate) mod sense;

/// Builds the versioned API router before application state is attached.
pub(crate) fn router(
  lookup_jobs_enabled: bool,
  graph_enabled: bool,
  canonical_sense_details_enabled: bool,
) -> Router<AppState> {
  let router = Router::new().route("/lookups", post(lookup::lookup));
  let router = if lookup_jobs_enabled {
    router.route("/lookup-jobs/:job_id", get(lookup_job::poll))
  } else {
    router
  };
  let router = if graph_enabled {
    router.route("/graph", get(graph::read)).route(
      "/graph/nodes/:node_kind/:node_id/neighbors",
      get(graph::neighbors),
    )
  } else {
    router
  };
  let router = if canonical_sense_details_enabled {
    router.route("/senses/:sense_id", get(sense::read))
  } else {
    router
  };

  router
    .fallback(not_found)
    .method_not_allowed_fallback(method_not_allowed)
}

async fn not_found(Extension(request_id): Extension<RequestId>) -> Response {
  problem::response(
    StatusCode::NOT_FOUND,
    "not_found",
    "Versioned API route not found",
    "The requested versioned API route does not exist.",
    &request_id,
    false,
    Vec::new(),
  )
}

async fn method_not_allowed(Extension(request_id): Extension<RequestId>) -> Response {
  problem::response(
    StatusCode::METHOD_NOT_ALLOWED,
    "method_not_allowed",
    "Versioned API method not allowed",
    "The requested HTTP method is not supported for this versioned API route.",
    &request_id,
    false,
    Vec::new(),
  )
}
