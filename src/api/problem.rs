//! Shared RFC 9457-style problem responses for versioned HTTP routes.

use axum::{
  http::{header, HeaderValue, StatusCode},
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;

use super::request_id::RequestId;

/// One field-specific validation failure in a problem response.
#[derive(Debug, Serialize)]
pub(crate) struct FieldError {
  field: &'static str,
  message: String,
}

impl FieldError {
  /// Creates a field-specific validation failure.
  pub(crate) fn new(field: &'static str, message: impl Into<String>) -> Self {
    Self {
      field,
      message: message.into(),
    }
  }
}

/// Creates a versioned API problem response without echoing request content.
pub(crate) fn response(
  status: StatusCode,
  code: &'static str,
  title: &'static str,
  detail: impl Into<String>,
  request_id: &RequestId,
  retryable: bool,
  errors: Vec<FieldError>,
) -> Response {
  let mut response = (
    status,
    Json(ProblemResponse {
      problem_type: "about:blank",
      title,
      status: status.as_u16(),
      code,
      detail: detail.into(),
      request_id: request_id.as_str().to_string(),
      retryable,
      errors,
    }),
  )
    .into_response();
  response.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/problem+json"),
  );
  no_store(response)
}

/// Creates the standard versioned API response for an oversized payload.
pub(crate) fn payload_too_large(request_id: &RequestId) -> Response {
  response(
    StatusCode::PAYLOAD_TOO_LARGE,
    "payload_too_large",
    "Request payload too large",
    "The request body exceeds the configured size limit.",
    request_id,
    false,
    Vec::new(),
  )
}

/// Adds the no-store policy required for private or error API responses.
pub(crate) fn no_store(mut response: Response) -> Response {
  response
    .headers_mut()
    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
  response
}

#[derive(Debug, Serialize)]
struct ProblemResponse {
  #[serde(rename = "type")]
  problem_type: &'static str,
  title: &'static str,
  status: u16,
  code: &'static str,
  detail: String,
  request_id: String,
  retryable: bool,
  errors: Vec<FieldError>,
}
