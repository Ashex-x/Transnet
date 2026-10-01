//! HTTP platform foundation integration tests.

use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
  Router,
};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, Duration as TimeDuration, OffsetDateTime};
use tower::ServiceExt;
use transnet::{
  app_router, app_router_with_http_config, AppState, HttpConfig, ProviderConfig, TranslationConfig,
  TranslationService,
};

fn service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap()
}

fn app() -> Router {
  app_router(AppState::new(service()))
}

fn configured_app(config: HttpConfig) -> Router {
  app_router_with_http_config(AppState::new(service()), &config).unwrap()
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn transitional_public_routes_are_absent() {
  for (method, path) in [
    ("GET", "/health"),
    ("GET", "/livez"),
    ("GET", "/readyz"),
    ("POST", "/translate"),
    ("POST", "/v1/lookups"),
    ("GET", "/v1/graph"),
    ("GET", "/v1/graph/nodes/sense/example/neighbors"),
    ("GET", "/v1/senses/example"),
  ] {
    let response = app()
      .oneshot(
        Request::builder()
          .method(method)
          .uri(path)
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
  }
}

#[tokio::test]
async fn target_probes_require_exact_empty_json_and_use_envelopes() {
  for (path, status) in [
    ("/api/v1/health", "ok"),
    ("/api/v1/livez", "alive"),
    ("/api/v1/readyz", "ready"),
  ] {
    let response = app()
      .oneshot(
        Request::post(path)
          .header(header::CONTENT_TYPE, "application/json")
          .header("x-request-id", "probe-contract-1")
          .body(Body::from("{}"))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["data"]["status"], status);
    if path == "/api/v1/readyz" {
      assert_eq!(
        body["data"]["components"],
        serde_json::json!({
          "canonical_data": "disabled",
          "retrieval_data": "disabled",
          "knowledge_projection": "disabled"
        })
      );
    }
    assert_eq!(body["meta"]["request_id"], "probe-contract-1");
    assert_eq!(body["meta"]["schema_version"], "probe-v1");
  }

  for body in ["", r#"{"extra":true}"#, "[]"] {
    let response = app()
      .oneshot(
        Request::post("/api/v1/health")
          .header(header::CONTENT_TYPE, "application/json")
          .body(Body::from(body))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(json(response).await["code"], "invalid_probe_request");
  }
}

#[tokio::test]
async fn request_ids_preserve_safe_values_and_replace_unsafe_values() {
  let safe_response = app()
    .oneshot(
      Request::post("/api/v1/livez")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "edge-42.request_id")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(
    safe_response.headers()["x-request-id"],
    "edge-42.request_id"
  );

  let unsafe_response = app()
    .oneshot(
      Request::post("/api/v1/livez")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "unsafe value")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  let generated = unsafe_response.headers()["x-request-id"].to_str().unwrap();
  assert_ne!(generated, "unsafe value");
  assert!(is_safe_request_id(generated));
}

#[tokio::test]
async fn target_deadlines_are_bounded_before_handler_work() {
  let expired = app()
    .oneshot(
      Request::get("/api/v1/not-implemented")
        .header("x-request-id", "deadline-expired-1")
        .header("x-deadline-at", "2020-01-01T00:00:00.000000Z")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(expired.status(), StatusCode::GATEWAY_TIMEOUT);
  assert_eq!(expired.headers()["x-request-id"], "deadline-expired-1");
  assert_eq!(json(expired).await["code"], "deadline_exceeded");

  let too_distant = (OffsetDateTime::now_utc() + TimeDuration::minutes(3))
    .format(&Rfc3339)
    .unwrap();
  let invalid = app()
    .oneshot(
      Request::get("/api/v1/not-implemented")
        .header("x-deadline-at", too_distant)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
  assert_eq!(json(invalid).await["code"], "invalid_deadline");

  let valid = (OffsetDateTime::now_utc() + TimeDuration::seconds(30))
    .replace_nanosecond(0)
    .unwrap()
    .format(&Rfc3339)
    .unwrap();
  let admitted = app()
    .oneshot(
      Request::get("/api/v1/not-implemented")
        .header("x-deadline-at", valid)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(admitted.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn versioned_route_errors_use_problem_details_and_the_same_request_id() {
  let response = app()
    .oneshot(
      Request::get("/api/v1/not-implemented")
        .header("x-request-id", "gateway-7")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  let request_id = response.headers()["x-request-id"]
    .to_str()
    .unwrap()
    .to_string();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["type"], "about:blank");
  assert_eq!(body["status"], StatusCode::NOT_FOUND.as_u16());
  assert_eq!(body["code"], "not_found");
  assert_eq!(body["request_id"], request_id);
  assert_eq!(body["request_id"], "gateway-7");
  assert_eq!(body["retryable"], false);
}

#[tokio::test]
async fn payload_limits_use_target_problem_responses() {
  let config = HttpConfig {
    max_request_body_bytes: 16,
  };
  let payload = r#"{"query":"this body is intentionally larger than sixteen bytes"}"#;
  let response = configured_app(config.clone())
    .oneshot(
      Request::post("/api/v1/translations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
  let body = json(response).await;
  assert_eq!(body["code"], "payload_too_large");
  assert_eq!(body["status"], StatusCode::PAYLOAD_TOO_LARGE.as_u16());
}

fn is_safe_request_id(value: &str) -> bool {
  (1..=128).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
