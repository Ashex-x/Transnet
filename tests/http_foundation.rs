//! HTTP platform foundation integration tests.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Method, Request, StatusCode},
  Router,
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  app_router, app_router_with_http_config, AppState, HttpConfig, HttpConfigError, ProviderConfig,
  Readiness, TranslationConfig, TranslationService,
};

fn service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string(),
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

#[derive(Debug)]
struct FixedReadiness(bool);

#[async_trait]
impl Readiness for FixedReadiness {
  async fn is_ready(&self) -> bool {
    self.0
  }
}

#[tokio::test]
async fn livez_reports_process_liveness() {
  let response = app()
    .oneshot(Request::get("/livez").body(Body::empty()).unwrap())
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert!(is_safe_request_id(
    response.headers()["x-request-id"].to_str().unwrap()
  ));
  assert_eq!(json(response).await, serde_json::json!({"status": "ok"}));
}

#[tokio::test]
async fn readyz_uses_the_injected_readiness_probe() {
  let response =
    app_router(AppState::new(service()).with_readiness(Arc::new(FixedReadiness(false))))
      .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
      .await
      .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    json(response).await,
    serde_json::json!({"status": "unavailable"})
  );
}

#[tokio::test]
async fn request_ids_preserve_safe_values_and_replace_unsafe_values() {
  let safe_response = app()
    .oneshot(
      Request::get("/livez")
        .header("x-request-id", "edge-42.request_id")
        .body(Body::empty())
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
      Request::get("/livez")
        .header("x-request-id", "unsafe value")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  let generated = unsafe_response.headers()["x-request-id"].to_str().unwrap();
  assert_ne!(generated, "unsafe value");
  assert!(is_safe_request_id(generated));
}

#[tokio::test]
async fn versioned_route_errors_use_problem_details_and_the_same_request_id() {
  let response = app()
    .oneshot(
      Request::get("/v1/not-implemented")
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
async fn payload_limits_keep_translate_compatibility_and_use_v1_problems() {
  let config = HttpConfig {
    max_request_body_bytes: 16,
    allowed_origins: Vec::new(),
    allow_credentials: false,
  };
  let payload = r#"{"query":"this body is intentionally larger than sixteen bytes"}"#;
  let response = configured_app(config.clone())
    .oneshot(
      Request::post("/v1/lookups")
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

  let response = configured_app(config)
    .oneshot(
      Request::post("/translate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "request body too large"})
  );
}

#[tokio::test]
async fn cors_allows_only_configured_origins_and_never_uses_a_wildcard() {
  let config = HttpConfig {
    max_request_body_bytes: 1_024,
    allowed_origins: vec!["https://app.example.test".to_string()],
    allow_credentials: true,
  };
  let response = configured_app(config.clone())
    .oneshot(
      Request::builder()
        .method(Method::OPTIONS)
        .uri("/translate")
        .header(header::ORIGIN, "https://app.example.test")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
          header::ACCESS_CONTROL_REQUEST_HEADERS,
          "content-type, x-request-id",
        )
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert!(response.status().is_success());
  assert_eq!(
    response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
    "https://app.example.test"
  );
  assert_eq!(
    response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
    "true"
  );
  assert!(response.headers().contains_key("x-request-id"));

  let response = configured_app(config)
    .oneshot(
      Request::get("/livez")
        .header(header::ORIGIN, "https://other.example.test")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert!(!response
    .headers()
    .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
  assert!(!response
    .headers()
    .contains_key(header::ACCESS_CONTROL_ALLOW_CREDENTIALS));

  let wildcard = HttpConfig {
    max_request_body_bytes: 1_024,
    allowed_origins: vec!["*".to_string()],
    allow_credentials: true,
  };
  assert!(matches!(
    app_router_with_http_config(AppState::new(service()), &wildcard),
    Err(HttpConfigError::InvalidOrigin(_))
  ));
}

fn is_safe_request_id(value: &str) -> bool {
  (1..=128).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
