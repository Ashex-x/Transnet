use std::{env, path::PathBuf, sync::Arc};

use axum::{
  extract::{Path, Query, State},
  http::StatusCode,
  response::IntoResponse,
  routing::{delete, get, post, put},
  Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
  Paragraph,
  Essay,
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
  pub translation_id: u64,
  pub text: String,
  pub source_lang: String,
  pub target_lang: String,
  pub input_type: InputType,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user_id: Option<String>,
  pub translation: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  pub status: String,
  pub service: String,
}

// System info types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutData {
  pub name: String,
  pub version: String,
  pub description: String,
  pub features: Vec<String>,
  pub supported_languages: Vec<String>,
  pub max_text_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsData {
  pub translations_today: u64,
  pub active_users: u64,
  pub translations_this_hour: u64,
  pub llm_api_status: String,
  pub database_status: String,
  pub requests_per_minute: u64,
  pub database_size_mb: f64,
}

// Account types
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
  pub username: String,
  pub email: String,
  pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
  pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
  pub current_password: String,
  pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
  pub user_id: String,
  pub username: String,
  pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginResponse {
  pub access_token: String,
  pub refresh_token: String,
  pub token_type: String,
  pub expires_in: u64,
  pub user: User,
}

// History types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryItem {
  pub translation_id: u64,
  pub text: String,
  pub source_lang: String,
  pub target_lang: String,
  pub input_type: InputType,
  pub provider: String,
  pub model: String,
  pub translation: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryResponse {
  pub translations: Vec<HistoryItem>,
  pub pagination: Pagination,
}

// Favorites types
#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteRequest {
  pub translation_id: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteNoteRequest {
  pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteItem {
  pub translation_id: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<String>,
  pub translation: HistoryItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoritesResponse {
  pub favorites: Vec<FavoriteItem>,
  pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteResponse {
  pub user_id: String,
  pub translation_id: u64,
  pub note: String,
  pub updated_at: String,
}

// Profile types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileStats {
  pub total_translations: u64,
  pub total_favorites: u64,
  pub languages_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileData {
  pub user_id: String,
  pub username: String,
  pub email: String,
  pub updated_at: String,
  pub stats: ProfileStats,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileUpdateRequest {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub username: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email: Option<String>,
}

// Common pagination type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pagination {
  pub page: u64,
  pub limit: u64,
  pub total: u64,
  pub total_pages: u64,
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
    // System endpoints
    .route("/api/about", get(get_about))
    .route("/api/stats", get(get_stats))
    .route("/api/health", get(get_health))
    // Account endpoints
    .route("/api/account/register", post(register))
    .route("/api/account/login", post(login))
    .route("/api/account/logout", post(logout))
    .route("/api/account/refresh", post(refresh))
    .route("/api/account/change-password", post(change_password))
    // Translation endpoints
    .route("/translate", post(translate))
    .route("/api/transnet/translate", post(translate))
    .route("/api/transnet/history", get(get_history))
    .route("/api/transnet/history/:id", get(get_history_by_id))
    .route("/api/transnet/history/:id", delete(delete_history))
    // Favorites endpoints
    .route("/api/transnet/favorites", post(add_favorite))
    .route("/api/transnet/favorites", get(get_favorites))
    .route("/api/transnet/favorites/:id", put(update_favorite))
    .route("/api/transnet/favorites/:id", delete(delete_favorite))
    // Profile endpoints
    .route("/api/profile", get(get_profile))
    .route("/api/profile", put(update_profile))
    // Legacy and static routes
    .route("/health", get(health))
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
    Ok(mut response) => {
      // For now, no JWT validation, so user_id is None
      response.user_id = None;
      (
        StatusCode::OK,
        Json(SuccessResponse::new(response)),
      )
        .into_response()
    }
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

// System endpoints
async fn get_about() -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(AboutData {
      name: "Transnet".to_string(),
      version: "1.0.0".to_string(),
      description: "AI-powered translation service with history and favorites".to_string(),
      features: vec![
        "translation".to_string(),
        "history".to_string(),
        "favorites".to_string(),
        "user_profiles".to_string(),
        "multi_language_support".to_string(),
      ],
      supported_languages: vec!["en", "es", "fr", "de", "zh", "ja", "cn"]
        .iter()
        .map(|s| s.to_string())
        .collect(),
      max_text_length: 5000,
    })),
  )
    .into_response()
}

async fn get_stats() -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(StatsData {
      translations_today: 14253,
      active_users: 387,
      translations_this_hour: 1247,
      llm_api_status: "healthy".to_string(),
      database_status: "connected".to_string(),
      requests_per_minute: 245,
      database_size_mb: 142.3,
    })),
  )
    .into_response()
}

async fn get_health(State(state): State<AppState>) -> impl IntoResponse {
  match state.backend_client.health().await {
    Ok(_) => (
      StatusCode::OK,
      Json(SuccessResponse::new(HealthData {
        status: "ready".to_string(),
        service: "transnet".to_string(),
      })),
    )
      .into_response(),
    Err(_) => (
      StatusCode::SERVICE_UNAVAILABLE,
      Json(ErrorResponse::new("SERVICE_UNAVAILABLE", "Backend service unavailable")),
    )
      .into_response(),
  }
}

// Account endpoints
async fn register(Json(request): Json<RegisterRequest>) -> impl IntoResponse {
  (
    StatusCode::CREATED,
    Json(SuccessResponse::new(User {
      user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
      username: request.username,
      email: request.email,
    })),
  )
    .into_response()
}

async fn login(Json(_request): Json<LoginRequest>) -> impl IntoResponse {
  let user_id = "550e8400-e29b-41d4-a716-446655440000";
  (
    StatusCode::OK,
    Json(SuccessResponse::new(LoginResponse {
      access_token: generate_mock_jwt(user_id),
      refresh_token: generate_mock_refresh_token(user_id),
      token_type: "Bearer".to_string(),
      expires_in: 3600,
      user: User {
        user_id: user_id.to_string(),
        username: "johndoe".to_string(),
        email: "john@example.com".to_string(),
      },
    })),
  )
    .into_response()
}

async fn logout() -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(serde_json::json!({
      "message": "Logged out successfully"
    }))),
  )
    .into_response()
}

async fn refresh(Json(_request): Json<RefreshRequest>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(serde_json::json!({
      "access_token": generate_mock_jwt("550e8400-e29b-41d4-a716-446655440000"),
      "token_type": "Bearer",
      "expires_in": 3600
    }))),
  )
    .into_response()
}

async fn change_password() -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(serde_json::json!({
      "message": "Password changed successfully"
    }))),
  )
    .into_response()
}

// History endpoints
#[derive(Debug, Deserialize)]
struct HistoryQuery {
  #[allow(dead_code)]
  _page: Option<u64>,
  #[allow(dead_code)]
  _limit: Option<u64>,
  #[allow(dead_code)]
  _source_lang: Option<String>,
  #[allow(dead_code)]
  _target_lang: Option<String>,
  #[allow(dead_code)]
  _input_type: Option<String>,
}

async fn get_history(Query(_params): Query<HistoryQuery>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(HistoryResponse {
      translations: vec![],
      pagination: Pagination {
        page: 1,
        limit: 20,
        total: 0,
        total_pages: 0,
      },
    })),
  )
    .into_response()
}

async fn get_history_by_id(Path(_id): Path<u64>) -> impl IntoResponse {
  (
    StatusCode::NOT_FOUND,
    Json(ErrorResponse::new("NOT_FOUND", "Translation not found")),
  )
    .into_response()
}

async fn delete_history(Path(_id): Path<u64>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(serde_json::json!({
      "message": "Translation deleted successfully"
    }))),
  )
    .into_response()
}

// Favorites endpoints
async fn add_favorite(Json(_request): Json<FavoriteRequest>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(FavoriteResponse {
      user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
      translation_id: 12345,
      note: "".to_string(),
      updated_at: "2024-01-15T12:30:00Z".to_string(),
    })),
  )
    .into_response()
}

#[derive(Debug, Deserialize)]
struct FavoritesQuery {
  #[allow(dead_code)]
  _page: Option<u64>,
  #[allow(dead_code)]
  _limit: Option<u64>,
}

async fn get_favorites(Query(_params): Query<FavoritesQuery>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(FavoritesResponse {
      favorites: vec![],
      pagination: Pagination {
        page: 1,
        limit: 20,
        total: 0,
        total_pages: 0,
      },
    })),
  )
    .into_response()
}

async fn update_favorite(Path(_id): Path<u64>, Json(_request): Json<FavoriteNoteRequest>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(FavoriteResponse {
      user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
      translation_id: 12345,
      note: "Updated note".to_string(),
      updated_at: "2024-01-15T12:30:00Z".to_string(),
    })),
  )
    .into_response()
}

async fn delete_favorite(Path(_id): Path<u64>) -> impl IntoResponse {
  (
    StatusCode::OK,
    Json(SuccessResponse::new(serde_json::json!({
      "message": "Favorite removed successfully"
    }))),
  )
    .into_response()
}

// Profile endpoints
async fn get_profile() -> impl IntoResponse {
  (
    StatusCode::UNAUTHORIZED,
    Json(ErrorResponse::new("UNAUTHORIZED", "Authentication required")),
  )
    .into_response()
}

async fn update_profile() -> impl IntoResponse {
  (
    StatusCode::UNAUTHORIZED,
    Json(ErrorResponse::new("UNAUTHORIZED", "Authentication required")),
  )
    .into_response()
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

// Mock JWT generation for demo purposes
fn generate_mock_jwt(user_id: &str) -> String {
  // Simple mock token - in production use a real JWT library
  format!("mock_jwt_token_{}_{}", user_id, chrono::Utc::now().timestamp())
}

fn generate_mock_refresh_token(user_id: &str) -> String {
  format!("mock_refresh_token_{}_{}", user_id, chrono::Utc::now().timestamp())
}
