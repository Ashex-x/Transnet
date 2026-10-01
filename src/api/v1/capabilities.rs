//! `POST /api/v1/capabilities` deployment capability discovery.

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;
use serde_json::Value;

use crate::domain::capabilities::ServiceCapabilities;

use super::super::{problem, request_id::RequestId, AppState};

pub(crate) async fn get(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  payload: Result<Json<Value>, JsonRejection>,
) -> Response {
  if !matches!(payload, Ok(Json(Value::Object(fields))) if fields.is_empty()) {
    return problem::response(
      StatusCode::BAD_REQUEST,
      "invalid_json",
      "Invalid JSON request",
      "The request body must be exactly one empty JSON object.",
      &request_id,
      false,
      Vec::new(),
    );
  }

  problem::no_store(
    Json(CapabilitiesResponse {
      data: state.capabilities().clone(),
      meta: CapabilitiesMeta {
        request_id: request_id.as_str().to_string(),
      },
    })
    .into_response(),
  )
}

#[derive(Serialize)]
struct CapabilitiesResponse {
  data: ServiceCapabilities,
  meta: CapabilitiesMeta,
}

#[derive(Serialize)]
struct CapabilitiesMeta {
  request_id: String,
}
