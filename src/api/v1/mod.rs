//! Version 1 lexical-knowledge HTTP routing and common failures.

use std::sync::Arc;

use axum::{
  extract::Extension,
  http::StatusCode,
  response::Response,
  routing::{get, post},
  Router,
};

use super::{problem, request_id::RequestId, AppState};

pub(crate) mod basic_card;
pub(crate) mod capabilities;
pub(crate) mod graph;
pub(crate) mod knowledge_paths;
pub(crate) mod knowledge_views;
pub(crate) mod lookup;
pub(crate) mod probe;
pub(crate) mod sense;
pub(crate) mod translation;

/// Builds the target `/api/v1` routes implemented by the current loopback runtime.
pub(crate) fn target_router(
  knowledge: Option<knowledge_views::KnowledgeRouteDependencies>,
) -> Router<AppState> {
  let runtime_cancellation = knowledge
    .as_ref()
    .map(knowledge_views::KnowledgeRouteDependencies::runtime_cancellation)
    .unwrap_or_else(|| Arc::new(crate::domain::model_runtime::CancellationSignal::default()));
  let router = Router::new()
    .route("/capabilities", post(capabilities::get))
    .route("/health", post(probe::health))
    .route("/livez", post(probe::livez))
    .route("/readyz", post(probe::readyz))
    .route("/translations", post(translation::translate))
    .route("/basic-cards/lookup", post(basic_card::lookup))
    .route("/senses/get", post(basic_card::sense));
  let router = match knowledge {
    Some(dependencies) => router.merge(knowledge_views::routes(dependencies)),
    None => router,
  };
  router
    .fallback(not_found)
    .method_not_allowed_fallback(method_not_allowed)
    .layer(Extension(knowledge_paths::RequestCancellationFactory::new(
      runtime_cancellation,
    )))
}

/// Builds the versioned API router before application state is attached.
pub(crate) fn router(
  graph_enabled: bool,
  canonical_sense_details_enabled: bool,
) -> Router<AppState> {
  let router = Router::new().route("/lookups", post(lookup::lookup));
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
