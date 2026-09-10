//! Version 1 learning HTTP API routing and common failures.

use axum::{extract::Extension, http::StatusCode, response::Response, routing::post, Router};

use super::{problem, request_id::RequestId, AppState};

pub(crate) mod lookup;

/// Builds the versioned API router before application state is attached.
pub(crate) fn router() -> Router<AppState> {
  Router::new()
    .route("/lookups", post(lookup::lookup))
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
