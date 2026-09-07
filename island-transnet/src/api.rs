//! HTTP transport for Transnet's health and translation endpoints.
//!
//! Handlers translate HTTP input and domain errors but leave translation logic
//! to [`crate::llm::TranslationService`]. Authentication and persistence do not belong here.

use std::sync::Arc;

use axum::{
  extract::State,
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  Json, Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::llm::TranslationService;
use crate::types::{ErrorResponse, HealthData, SuccessResponse, TranslateRequest, TransnetError};

/// Shared dependencies used by HTTP request handlers.
#[derive(Clone)]
pub struct AppState {
  translation_service: Arc<TranslationService>,
}

impl AppState {
  /// Creates handler state backed by the supplied translation service.
  pub fn new(translation_service: TranslationService) -> Self {
    Self {
      translation_service: Arc::new(translation_service),
    }
  }
}

/// Builds the Transnet HTTP router.
///
/// The router exposes `GET /health` and `POST /translate` and installs permissive
/// CORS for compatibility with existing clients.
pub fn app_router(state: AppState) -> Router {
  Router::new()
    .route("/health", get(health))
    .route("/translate", post(translate))
    .layer(CorsLayer::permissive())
    .layer(TraceLayer::new_for_http())
    .with_state(state)
}

async fn health() -> Json<SuccessResponse<HealthData>> {
  Json(SuccessResponse::new(HealthData {
    status: "ready".to_string(),
    service: "transnet-backend".to_string(),
    // Readiness currently means the process can serve requests; provider health
    // is established only when a translation request is made.
    checks: None,
  }))
}

async fn translate(
  State(state): State<AppState>,
  Json(request): Json<TranslateRequest>,
) -> impl IntoResponse {
  match state.translation_service.translate(request).await {
    Ok(response) => (StatusCode::OK, Json(SuccessResponse::new(response))).into_response(),
    Err(error) => map_error(error),
  }
}

fn map_error(error: TransnetError) -> axum::response::Response {
  let (status, code) = match &error {
    TransnetError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
    TransnetError::Llm(_) => (StatusCode::SERVICE_UNAVAILABLE, "LLM_ERROR"),
    TransnetError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
    TransnetError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
  };

  (status, Json(ErrorResponse::new(code, error.to_string()))).into_response()
}

#[cfg(test)]
mod tests {
  use axum::{body::Body, http::Request};
  use tower::util::ServiceExt;

  use crate::types::LlmConfig;

  use super::*;

  fn test_router() -> Router {
    let service = TranslationService::new(LlmConfig {
      api_key: "test".to_string(),
      base_url: "http://localhost:13595/v1".to_string(),
      model: "ACTION".to_string(),
      timeout_seconds: 1,
      max_retries: 0,
      normal_lang_base_url: None,
      normal_lang_model: None,
    })
    .unwrap();

    app_router(AppState::new(service))
  }

  #[tokio::test]
  async fn test_health_endpoint_returns_ok() {
    let response = test_router()
      .oneshot(
        Request::builder()
          .uri("/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn test_translate_endpoint_rejects_empty_text() {
    let response = test_router()
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/translate")
          .header("content-type", "application/json")
          .body(Body::from(
            r#"{"text":" ","source_lang":"en","target_lang":"es"}"#,
          ))
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  }
}
