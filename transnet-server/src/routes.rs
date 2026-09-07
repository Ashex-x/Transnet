//! HTTP router and request handlers for the gateway API.
//!
//! This module serves JSON APIs only; browser assets belong to a separate deployment.

use std::sync::Arc;

use axum::{
  extract::{Path, Query, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{delete, get, post, put},
  Json, Router,
};
use serde::Deserialize;

use crate::{
  AboutData, BackendClient, ErrorResponse, FavoriteNoteRequest, FavoriteRequest, FavoriteResponse,
  FavoritesResponse, HealthData, HistoryResponse, LoginRequest, LoginResponse, Pagination,
  RefreshRequest, RegisterRequest, StatsData, SuccessResponse, TranslateRequest, TransnetError,
  User,
};

/// Shared dependencies available to request handlers.
#[derive(Clone)]
pub struct AppState {
  backend: Arc<BackendClient>,
}

impl AppState {
  /// Creates application state for a translation backend base URL.
  pub fn new(backend_url: impl Into<String>) -> Self {
    Self {
      backend: Arc::new(BackendClient::new(backend_url)),
    }
  }
}

/// Builds the JSON-only gateway router before application state is attached.
///
/// Unknown paths return Axum's standard 404 response; the gateway never falls back to HTML.
pub fn create_router() -> Router<AppState> {
  Router::new()
    .route("/api/about", get(get_about))
    .route("/api/stats", get(get_stats))
    .route("/api/health", get(get_health))
    .route("/api/account/register", post(register))
    .route("/api/account/login", post(login))
    .route("/api/account/logout", post(logout))
    .route("/api/account/refresh", post(refresh))
    .route("/api/account/change-password", post(change_password))
    .route("/translate", post(translate))
    .route("/api/transnet/translate", post(translate))
    .route("/api/transnet/history", get(get_history))
    .route("/api/transnet/history/:id", get(get_history_by_id))
    .route("/api/transnet/history/:id", delete(delete_history))
    .route("/api/transnet/favorites", post(add_favorite))
    .route("/api/transnet/favorites", get(get_favorites))
    .route("/api/transnet/favorites/:id", put(update_favorite))
    .route("/api/transnet/favorites/:id", delete(delete_favorite))
    .route("/api/profile", get(get_profile))
    .route("/api/profile", put(update_profile))
    .route("/health", get(health))
    .layer(tower_http::cors::CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> Response {
  match state.backend.health().await {
    Ok(data) => (StatusCode::OK, Json(SuccessResponse::new(data))).into_response(),
    Err(error) => map_error(error),
  }
}

async fn translate(
  State(state): State<AppState>,
  Json(request): Json<TranslateRequest>,
) -> Response {
  match state.backend.translate(request).await {
    Ok(response) => (StatusCode::OK, Json(SuccessResponse::new(response))).into_response(),
    Err(error) => map_error(error),
  }
}

async fn get_about() -> impl IntoResponse {
  Json(SuccessResponse::new(AboutData {
    name: "Transnet".to_string(),
    version: "1.0.0".to_string(),
    description: "AI-powered translation service with history and favorites".to_string(),
    features: [
      "translation",
      "history",
      "favorites",
      "user_profiles",
      "multi_language_support",
    ]
    .map(str::to_string)
    .to_vec(),
    supported_languages: ["en", "es", "fr", "de", "zh", "ja", "cn"]
      .map(str::to_string)
      .to_vec(),
    max_text_length: 5000,
  }))
}

async fn get_stats() -> impl IntoResponse {
  Json(SuccessResponse::new(StatsData {
    translations_today: 14253,
    active_users: 387,
    translations_this_hour: 1247,
    llm_api_status: "healthy".to_string(),
    database_status: "connected".to_string(),
    requests_per_minute: 245,
    database_size_mb: 142.3,
  }))
}

async fn get_health(State(state): State<AppState>) -> Response {
  match state.backend.health().await {
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
      Json(ErrorResponse::new(
        "SERVICE_UNAVAILABLE",
        "Backend service unavailable",
      )),
    )
      .into_response(),
  }
}

async fn register(Json(request): Json<RegisterRequest>) -> impl IntoResponse {
  (
    StatusCode::CREATED,
    Json(SuccessResponse::new(User {
      user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
      username: request.username,
      email: request.email,
    })),
  )
}

async fn login(Json(_request): Json<LoginRequest>) -> impl IntoResponse {
  let user_id = "550e8400-e29b-41d4-a716-446655440000";
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
  }))
}

async fn logout() -> impl IntoResponse {
  Json(SuccessResponse::new(serde_json::json!({
    "message": "Logged out successfully"
  })))
}

async fn refresh(Json(_request): Json<RefreshRequest>) -> impl IntoResponse {
  Json(SuccessResponse::new(serde_json::json!({
    "access_token": generate_mock_jwt("550e8400-e29b-41d4-a716-446655440000"),
    "token_type": "Bearer",
    "expires_in": 3600
  })))
}

async fn change_password() -> impl IntoResponse {
  Json(SuccessResponse::new(serde_json::json!({
    "message": "Password changed successfully"
  })))
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
  #[allow(dead_code)]
  page: Option<u64>,
  #[allow(dead_code)]
  limit: Option<u64>,
  #[allow(dead_code)]
  source_lang: Option<String>,
  #[allow(dead_code)]
  target_lang: Option<String>,
  #[allow(dead_code)]
  input_type: Option<String>,
}

async fn get_history(Query(_query): Query<HistoryQuery>) -> impl IntoResponse {
  Json(SuccessResponse::new(HistoryResponse {
    translations: vec![],
    pagination: empty_pagination(),
  }))
}

async fn get_history_by_id(Path(_id): Path<u64>) -> impl IntoResponse {
  (
    StatusCode::NOT_FOUND,
    Json(ErrorResponse::new("NOT_FOUND", "Translation not found")),
  )
}

async fn delete_history(Path(_id): Path<u64>) -> impl IntoResponse {
  Json(SuccessResponse::new(serde_json::json!({
    "message": "Translation deleted successfully"
  })))
}

async fn add_favorite(Json(_request): Json<FavoriteRequest>) -> impl IntoResponse {
  Json(SuccessResponse::new(mock_favorite_response("")))
}

#[derive(Debug, Deserialize)]
struct FavoritesQuery {
  #[allow(dead_code)]
  page: Option<u64>,
  #[allow(dead_code)]
  limit: Option<u64>,
}

async fn get_favorites(Query(_query): Query<FavoritesQuery>) -> impl IntoResponse {
  Json(SuccessResponse::new(FavoritesResponse {
    favorites: vec![],
    pagination: empty_pagination(),
  }))
}

async fn update_favorite(
  Path(_id): Path<u64>,
  Json(_request): Json<FavoriteNoteRequest>,
) -> impl IntoResponse {
  Json(SuccessResponse::new(mock_favorite_response("Updated note")))
}

async fn delete_favorite(Path(_id): Path<u64>) -> impl IntoResponse {
  Json(SuccessResponse::new(serde_json::json!({
    "message": "Favorite removed successfully"
  })))
}

async fn get_profile() -> impl IntoResponse {
  (
    StatusCode::UNAUTHORIZED,
    Json(ErrorResponse::new(
      "UNAUTHORIZED",
      "Authentication required",
    )),
  )
}

async fn update_profile() -> impl IntoResponse {
  (
    StatusCode::UNAUTHORIZED,
    Json(ErrorResponse::new(
      "UNAUTHORIZED",
      "Authentication required",
    )),
  )
}

fn empty_pagination() -> Pagination {
  Pagination {
    page: 1,
    limit: 20,
    total: 0,
    total_pages: 0,
  }
}

fn mock_favorite_response(note: &str) -> FavoriteResponse {
  FavoriteResponse {
    user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    translation_id: 12345,
    note: note.to_string(),
    updated_at: "2024-01-15T12:30:00Z".to_string(),
  }
}

fn map_error(error: TransnetError) -> Response {
  let (status, code) = match &error {
    TransnetError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
    TransnetError::Backend(_) => (StatusCode::SERVICE_UNAVAILABLE, "BACKEND_ERROR"),
    TransnetError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
    TransnetError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
  };
  (status, Json(ErrorResponse::new(code, error.to_string()))).into_response()
}

fn generate_mock_jwt(user_id: &str) -> String {
  format!(
    "mock_jwt_token_{}_{}",
    user_id,
    chrono::Utc::now().timestamp()
  )
}

fn generate_mock_refresh_token(user_id: &str) -> String {
  format!(
    "mock_refresh_token_{}_{}",
    user_id,
    chrono::Utc::now().timestamp()
  )
}

#[cfg(test)]
mod tests {
  use axum::{body::Body, http::Request};
  use tower::ServiceExt;

  use super::*;

  #[tokio::test]
  async fn test_router_root_returns_not_found_without_web_ui_fallback() {
    let app = create_router().with_state(AppState::new("http://127.0.0.1:1"));
    let response = app
      .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
      .await
      .expect("router should respond");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[tokio::test]
  async fn test_router_static_asset_path_returns_not_found() {
    let app = create_router().with_state(AppState::new("http://127.0.0.1:1"));
    let response = app
      .oneshot(
        Request::builder()
          .uri("/assets/app.js")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .expect("router should respond");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }
}
