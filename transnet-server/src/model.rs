//! JSON request, response, and envelope types exposed by the gateway.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Classification used to select the backend translation prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  /// Lets the backend infer the input type.
  #[default]
  Auto,
  /// A single word.
  Word,
  /// A short phrase.
  Phrase,
  /// A complete sentence.
  Sentence,
  /// A paragraph.
  Paragraph,
  /// A multi-paragraph essay.
  Essay,
}

/// JSON body accepted by translation endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
  pub text: String,
  pub source_lang: String,
  pub target_lang: String,
  pub mode: Option<String>,
  pub input_type: Option<InputType>,
}

/// Translation payload returned by the gateway.
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

/// Backend liveness data returned from the legacy health endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  pub status: String,
  pub service: String,
}

/// Product metadata returned by `/api/about`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutData {
  pub name: String,
  pub version: String,
  pub description: String,
  pub features: Vec<String>,
  pub supported_languages: Vec<String>,
  pub max_text_length: usize,
}

/// Operational counters returned by the current stub stats endpoint.
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

/// Registration input accepted by the account API.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
  pub username: String,
  pub email: String,
  pub password: String,
}

/// Login credentials accepted by the account API.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
}

/// Refresh token input accepted by the account API.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
  pub refresh_token: String,
}

/// Password rotation input accepted by the account API.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
  pub current_password: String,
  pub new_password: String,
}

/// Public account profile included in login and registration responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
  pub user_id: String,
  pub username: String,
  pub email: String,
}

/// Tokens and user profile returned after login.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginResponse {
  pub access_token: String,
  pub refresh_token: String,
  pub token_type: String,
  /// Access-token lifetime in seconds.
  pub expires_in: u64,
  pub user: User,
}

/// One translation record returned by history endpoints.
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

/// Paginated translation history response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryResponse {
  pub translations: Vec<HistoryItem>,
  pub pagination: Pagination,
}

/// Request to add a translation to favorites.
#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteRequest {
  pub translation_id: u64,
  pub note: Option<String>,
}

/// Request to replace the note on a favorite.
#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteNoteRequest {
  pub note: String,
}

/// Favorite metadata joined with its translation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteItem {
  pub translation_id: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
  /// RFC3339 timestamp when the favorite was last changed.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<String>,
  pub translation: HistoryItem,
}

/// Paginated favorites response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoritesResponse {
  pub favorites: Vec<FavoriteItem>,
  pub pagination: Pagination,
}

/// Result of adding or updating a favorite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteResponse {
  pub user_id: String,
  pub translation_id: u64,
  pub note: String,
  /// RFC3339 timestamp when the favorite was last changed.
  pub updated_at: String,
}

/// Aggregate usage fields associated with a profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileStats {
  pub total_translations: u64,
  pub total_favorites: u64,
  pub languages_used: Vec<String>,
}

/// User profile returned by the profile API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileData {
  pub user_id: String,
  pub username: String,
  pub email: String,
  /// RFC3339 timestamp when the profile was last changed.
  pub updated_at: String,
  pub stats: ProfileStats,
}

/// Optional profile fields accepted by profile updates.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileUpdateRequest {
  pub username: Option<String>,
  pub email: Option<String>,
}

/// Pagination metadata attached to list responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pagination {
  /// One-based result page.
  pub page: u64,
  /// Maximum number of records on a page.
  pub limit: u64,
  pub total: u64,
  pub total_pages: u64,
}

/// Standard success envelope used by gateway JSON endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuccessResponse<T> {
  pub success: bool,
  pub data: T,
}

impl<T> SuccessResponse<T> {
  /// Wraps data in an envelope whose `success` flag is true.
  pub fn new(data: T) -> Self {
    Self {
      success: true,
      data,
    }
  }
}

/// Machine-readable and human-readable details for an API failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
  pub code: String,
  pub message: String,
}

/// Standard error envelope used by gateway JSON endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
  pub success: bool,
  pub error: ErrorInfo,
}

impl ErrorResponse {
  /// Creates an error envelope whose `success` flag is false.
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_success_response_serializes_standard_envelope() {
    let response = SuccessResponse::new(HealthData {
      status: "ok".to_string(),
      service: "transnet".to_string(),
    });
    let json = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "ok");
  }

  #[test]
  fn test_translate_response_without_user_omits_user_id() {
    let response = TranslateResponse {
      translation_id: 1,
      text: "hello".to_string(),
      source_lang: "en".to_string(),
      target_lang: "zh".to_string(),
      input_type: InputType::Word,
      user_id: None,
      translation: serde_json::json!({"text": "你好"}),
    };
    let json = serde_json::to_value(response).expect("response should serialize");
    assert!(json.get("user_id").is_none());
  }
}
