//! Minimal HTTP interface for health and translation.

use std::sync::Arc;

use axum::{
  extract::{rejection::JsonRejection, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use tower_http::trace::TraceLayer;

use crate::{
  provider::{TranslationError, TranslationService},
  types::{ErrorResponse, HealthResponse, TranslateRequest},
};

/// Shared dependencies used by request handlers.
#[derive(Clone)]
pub struct AppState {
  service: Arc<TranslationService>,
}

impl AppState {
  /// Creates application state for a translation service.
  pub fn new(service: TranslationService) -> Self {
    Self {
      service: Arc::new(service),
    }
  }
}

/// Builds the complete Transnet HTTP router.
pub fn app_router(state: AppState) -> Router {
  Router::new()
    .route("/health", get(health))
    .route("/translate", post(translate))
    .layer(TraceLayer::new_for_http())
    .with_state(state)
}

async fn health() -> Json<HealthResponse> {
  Json(HealthResponse { status: "ok" })
}

async fn translate(
  State(state): State<AppState>,
  payload: Result<Json<TranslateRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(request) => request,
    Err(_) => return error(StatusCode::BAD_REQUEST, "invalid JSON request"),
  };

  match state.service.translate(request).await {
    Ok(response) => (StatusCode::OK, Json(response)).into_response(),
    Err(TranslationError::Validation(message)) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    Err(TranslationError::Provider) => error(
      StatusCode::SERVICE_UNAVAILABLE,
      "translation provider unavailable",
    ),
  }
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
  (
    status,
    Json(ErrorResponse {
      error: message.into(),
    }),
  )
    .into_response()
}
