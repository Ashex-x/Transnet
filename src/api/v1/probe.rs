//! Target health, liveness, and dependency-readiness probe handlers.

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;
use serde_json::Value;

use crate::domain::request_context::RequestContext;

use super::super::{envelope::SuccessEnvelope, problem, AppState};

const PROBE_SCHEMA_VERSION: &str = "probe-v1";

#[derive(Debug, Serialize)]
struct ProbeData {
  status: &'static str,
}

pub(crate) async fn health(
  Extension(context): Extension<RequestContext>,
  payload: Result<Json<Value>, JsonRejection>,
) -> Response {
  probe_success(context, payload, "ok")
}

pub(crate) async fn livez(
  Extension(context): Extension<RequestContext>,
  payload: Result<Json<Value>, JsonRejection>,
) -> Response {
  probe_success(context, payload, "alive")
}

pub(crate) async fn readyz(
  State(state): State<AppState>,
  Extension(context): Extension<RequestContext>,
  payload: Result<Json<Value>, JsonRejection>,
) -> Response {
  if let Some(response) = invalid_payload(&context, payload) {
    return response;
  }
  if !state.readiness.is_ready().await {
    return problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "dependency_unavailable",
      "Required dependency unavailable",
      "A required dependency cannot safely serve new work.",
      context.request_id(),
      true,
      Vec::new(),
    );
  }
  success(&context, "ready")
}

fn probe_success(
  context: RequestContext,
  payload: Result<Json<Value>, JsonRejection>,
  status: &'static str,
) -> Response {
  if let Some(response) = invalid_payload(&context, payload) {
    return response;
  }
  success(&context, status)
}

fn invalid_payload(
  context: &RequestContext,
  payload: Result<Json<Value>, JsonRejection>,
) -> Option<Response> {
  if matches!(&payload, Ok(Json(Value::Object(object))) if object.is_empty()) {
    return None;
  }
  Some(problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_probe_request",
    "Invalid probe request",
    "The request must use application/json and contain exactly an empty object.",
    context.request_id(),
    false,
    Vec::new(),
  ))
}

fn success(context: &RequestContext, status: &'static str) -> Response {
  problem::no_store(
    Json(SuccessEnvelope::new(
      ProbeData { status },
      context,
      PROBE_SCHEMA_VERSION,
    ))
    .into_response(),
  )
}
