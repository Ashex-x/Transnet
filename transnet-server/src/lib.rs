use std::{env, path::PathBuf, sync::Arc};

use axum::{
  extract::State,
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower_http::{cors::CorsLayer, services::ServeDir};

mod backend_client;

pub use backend_client::BackendClient;

fn get_static_dir() -> PathBuf {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
  PathBuf::from(manifest_dir)
    .parent()
    .expect("failed to find project root")
    .join("static")
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  #[default]
  Auto,
  Word,
  Phrase,
  Sentence,
  Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
  pub text: String,
  pub source_lang: String,
  pub target_lang: String,
  pub mode: Option<String>,
  pub input_type: Option<InputType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateResponse {
  pub translation: String,
  pub source_lang: String,
  pub target_lang: String,
  pub input_type: InputType,
  pub provider: String,
  pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  pub status: String,
  pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuccessResponse<T> {
  pub success: bool,
  pub data: T,
}

impl<T> SuccessResponse<T> {
  pub fn new(data: T) -> Self {
    Self {
      success: true,
      data,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
  pub code: String,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
  pub success: bool,
  pub error: ErrorInfo,
}

impl ErrorResponse {
  pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      success: false,
      error: ErrorInfo {
        code: code.into(),
        message: message.into(),
      },
    }
  }
}

#[derive(Debug, Error)]
pub enum TransnetError {
  #[error("{0}")]
  Validation(String),
  #[error("backend request failed: {0}")]
  Backend(String),
  #[error("configuration error: {0}")]
  Config(String),
  #[error("internal error: {0}")]
  Internal(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  pub host: String,
  pub port: u16,
  pub workers: usize,
  pub log_level: String,
}

impl ServerConfig {
  pub fn from_env() -> Result<Self, anyhow::Error> {
    let host = std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("GATEWAY_PORT")
      .unwrap_or_else(|_| "8080".to_string())
      .parse()?;
    let workers = std::env::var("WORKERS")
      .unwrap_or_else(|_| "4".to_string())
      .parse()?;
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    Ok(Self {
      host,
      port,
      workers,
      log_level,
    })
  }
}

#[derive(Clone)]
pub struct AppState {
  backend_client: Arc<BackendClient>,
}

impl AppState {
  pub fn new(backend_url: String) -> Self {
    let client = BackendClient::new(backend_url);
    Self {
      backend_client: Arc::new(client),
    }
  }
}

pub fn create_router() -> Router<AppState> {
  let static_dir = get_static_dir();

  Router::new()
    .route("/health", get(health))
    .route("/translate", post(translate))
    .nest_service("/assets", ServeDir::new(static_dir.join("dist")))
    .nest_service("/resource", ServeDir::new(static_dir.join("resource")))
    .route("/index.html", get(serve_index_html))
    .fallback(serve_index_html)
    .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
  match state.backend_client.health().await {
    Ok(health_data) => (
      StatusCode::OK,
      Json(SuccessResponse::new(health_data)),
    )
      .into_response(),
    Err(error) => map_error(error),
  }
}

async fn translate(
  State(state): State<AppState>,
  Json(request): Json<TranslateRequest>,
) -> impl IntoResponse {
  match state.backend_client.translate(request).await {
    Ok(response) => (
      StatusCode::OK,
      Json(SuccessResponse::new(response)),
    )
      .into_response(),
    Err(error) => map_error(error),
  }
}

async fn serve_index_html() -> impl IntoResponse {
  let static_dir = get_static_dir();
  let index_path = static_dir.join("index.html");

  match tokio::fs::read_to_string(&index_path).await {
    Ok(content) => (
      [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
      content,
    )
      .into_response(),
    Err(e) => {
      tracing::error!(path = %index_path.display(), error = %e, "failed to read index.html");
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load index.html",
      )
        .into_response()
    }
  }
}

fn map_error(error: TransnetError) -> axum::response::Response {
  let (status, code) = match &error {
    TransnetError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
    TransnetError::Backend(_) => (StatusCode::SERVICE_UNAVAILABLE, "BACKEND_ERROR"),
    TransnetError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
    TransnetError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
  };

  (
    status,
    Json(ErrorResponse::new(code, error.to_string())),
  )
    .into_response()
}
