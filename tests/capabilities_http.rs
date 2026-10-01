//! HTTP contract tests for current, content-free service capability discovery.

use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  app_router_with_http_config, AppState, HttpConfig, ProviderConfig, TranslationConfig,
  TranslationService,
};

fn app(max_request_body_bytes: usize) -> axum::Router {
  let provider = ProviderConfig {
    base_url: "http://provider-secret.invalid/v1".to_string(),
    model: "private-model-name".to_string(),
    api_key: "private-capability-secret".into(),
  };
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap();
  app_router_with_http_config(
    AppState::new(service),
    &HttpConfig {
      max_request_body_bytes,
      allowed_origins: Vec::new(),
      allow_credentials: false,
    },
  )
  .unwrap()
}

async fn body(response: axum::response::Response) -> (String, Value) {
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let raw = String::from_utf8(bytes.to_vec()).unwrap();
  let json = serde_json::from_str(&raw).unwrap();
  (raw, json)
}

fn post(payload: &'static str) -> Request<Body> {
  Request::post("/api/v1/capabilities")
    .header(header::CONTENT_TYPE, "application/json")
    .header("x-request-id", "capability-test")
    .body(Body::from(payload))
    .unwrap()
}

#[tokio::test]
async fn reports_only_implemented_content_free_capabilities() {
  let response = app(8_192).oneshot(post("{}")).await.unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(response.headers()["x-request-id"], "capability-test");
  let (raw, json) = body(response).await;
  assert_eq!(json["meta"]["request_id"], "capability-test");
  assert_eq!(
    json["data"]["source_languages"],
    serde_json::json!(["auto", "en", "zh-CN"])
  );
  assert_eq!(
    json["data"]["target_languages"],
    serde_json::json!(["en", "zh-CN"])
  );
  assert_eq!(json["data"]["input_types"], serde_json::json!(["text"]));
  assert_eq!(json["data"]["image_media_types"], serde_json::json!([]));
  assert_eq!(json["data"]["purposes"], serde_json::json!([]));
  assert_eq!(json["data"]["annotation_families"], serde_json::json!([]));
  assert_eq!(json["data"]["knowledge_lenses"], serde_json::json!([]));
  assert_eq!(json["data"]["limits"]["max_request_body_bytes"], 8_192);
  assert_eq!(
    json["data"]["live_retrieval"],
    serde_json::json!({"available": false, "default": "offline"})
  );
  assert_eq!(
    json["data"]["generation_profiles"],
    serde_json::json!(["fast"])
  );
  assert_eq!(
    json["data"]["schema_versions"],
    serde_json::json!(["translation-result-v1"])
  );
  for forbidden in [
    "provider-secret",
    "private-model-name",
    "private-capability-secret",
    "socket_path",
    "concurrency",
    "base_url",
  ] {
    assert!(!raw.contains(forbidden), "response leaked `{forbidden}`");
  }
}

#[tokio::test]
async fn rejects_nonempty_or_malformed_requests_without_caching() {
  for payload in [r#"{"extra":true}"#, "null", "[]", "", "not-json"] {
    let response = app(8_192).oneshot(post(payload)).await.unwrap();
    assert_eq!(
      response.status(),
      StatusCode::BAD_REQUEST,
      "payload: {payload}"
    );
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let (_, json) = body(response).await;
    assert_eq!(json["code"], "invalid_json");
    assert_eq!(json["request_id"], "capability-test");
  }
}

#[tokio::test]
async fn requires_json_content_type_and_post_method() {
  let missing_content_type = app(8_192)
    .oneshot(
      Request::post("/api/v1/capabilities")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(missing_content_type.status(), StatusCode::BAD_REQUEST);
  assert_eq!(
    missing_content_type.headers()[header::CACHE_CONTROL],
    "no-store"
  );

  let wrong_method = app(8_192)
    .oneshot(
      Request::get("/api/v1/capabilities")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
  assert_eq!(wrong_method.headers()[header::CACHE_CONTROL], "no-store");
}
