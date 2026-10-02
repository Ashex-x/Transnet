//! Version 1 lexical-knowledge HTTP routing and common failures.

use std::sync::Arc;

use axum::{extract::Extension, http::StatusCode, response::Response, routing::post, Router};

use super::{problem, request_id::RequestId, AppState};

pub(crate) mod basic_card;
pub(crate) mod capabilities;
pub(crate) mod knowledge_paths;
pub(crate) mod knowledge_views;
pub(crate) mod probe;
pub(crate) mod sense;
pub(crate) mod translation;

/// Builds the target `/api/v1` routes served by the Unix-domain-socket runtime.
pub(crate) fn target_router(
  knowledge: Option<knowledge_views::KnowledgeRouteDependencies>,
  runtime_cancellation: Arc<crate::domain::model_runtime::CancellationSignal>,
) -> Router<AppState> {
  let router = Router::new()
    .route("/capabilities", post(capabilities::get))
    .route("/health", post(probe::health))
    .route("/livez", post(probe::livez))
    .route("/readyz", post(probe::readyz))
    .route("/translations", post(translation::translate))
    .route("/translations/stream", post(translation::translate_stream))
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
