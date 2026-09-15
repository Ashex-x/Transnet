//! Service-boundary regressions: private state is rejected before model or data work.

use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use tower::ServiceExt;
use transnet::{app_router, AppState, ProviderConfig, TranslationConfig, TranslationService};

fn router() -> axum::Router {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".into(),
    model: "unused".into(),
    api_key: "unused".into(),
  };
  app_router(AppState::new(
    TranslationService::new(
      TranslationConfig {
        long_text_chars: 4000,
        timeout_seconds: 1,
        max_retries: 0,
        retry_delay_ms: 0,
      },
      provider.clone(),
      provider,
    )
    .unwrap(),
  ))
}

#[tokio::test]
async fn private_headers_are_rejected_before_dispatch_without_echoing_values() {
  for path in [
    "/health",
    "/translate",
    "/v1/lookups",
    "/v1/graph",
    "/v1/senses/example",
  ] {
    for name in [
      "authorization",
      "proxy-authorization",
      "cookie",
      "cookie2",
      "x-user-id",
      "x-user-profile",
      "x-account-id",
      "x-account-profile",
      "x-learner-id",
      "x-learner-profile",
      "x-owner-id",
      "x-owner-profile",
      "x-session-id",
      "x-session-state",
      "x-authenticated-user",
      "x-forwarded-user",
      "remote-user",
      "x-api-key",
      "lookup-capability",
    ] {
      let response = router()
        .oneshot(
          Request::post(path)
            .header(name, "private-sentinel-8391")
            .header("x-request-id", "boundary-test")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
      assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}: {name}");
      assert_eq!(response.headers()["x-request-id"], "boundary-test");
      assert!(response.headers()[header::CACHE_CONTROL]
        .to_str()
        .unwrap()
        .contains("no-store"));
      let body = to_bytes(response.into_body(), 8192).await.unwrap();
      assert!(!String::from_utf8_lossy(&body).contains("private-sentinel-8391"));
    }
  }
}

#[tokio::test]
async fn translation_and_lookup_reject_unknown_and_private_fields() {
  for (path, base) in [
    (
      "/translate",
      serde_json::json!({"text":"hello","source_lang":"en","target_lang":"zh-CN"}),
    ),
    ("/v1/lookups", serde_json::json!({"query":"hello"})),
  ] {
    for field in [
      "user_id",
      "account_id",
      "learner_level",
      "history",
      "profile",
      "saved_items",
      "mastery",
      "persist",
      "incognito",
      "unknown_field",
    ] {
      let mut input = base.clone();
      input[field] = serde_json::json!("private-sentinel-8391");
      let response = router()
        .oneshot(
          Request::post(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(input.to_string()))
            .unwrap(),
        )
        .await
        .unwrap();
      assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "{path}: {field}"
      );
      let body = to_bytes(response.into_body(), 8192).await.unwrap();
      assert!(!String::from_utf8_lossy(&body).contains("private-sentinel-8391"));
    }
  }
  let response = router()
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"hello","include":["practice_preview"]}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn private_query_parameters_and_removed_routes_are_not_accepted() {
  for path in ["/health", "/v1/graph", "/v1/senses/example"] {
    let response = router()
      .oneshot(
        Request::get(path)
          .body(Body::from(r#"{"user_id":"private-sentinel"}"#))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  }
  for path in [
    "/health?user_id=private-sentinel",
    "/translate?persist=true",
    "/v1/senses/example?user_id=private-sentinel",
  ] {
    let response = router()
      .oneshot(Request::get(path).body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  }
  for path in [
    "/v1/lookup-jobs/example",
    "/v1/learners",
    "/v1/practice",
    "/v1/graph-views",
    "/v1/feedback",
  ] {
    let response = router()
      .oneshot(Request::get(path).body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }
}

#[test]
fn debug_diagnostics_redact_request_and_generated_text() {
  use transnet::domain::{
    canonical::LanguageTag,
    retrieval::RetrievalRequest,
    translation::{Confidence, EnglishDialect, TranslationInput, TranslationResult},
  };
  let secret = "private-sentinel-8391";
  let request = transnet::TranslateRequest {
    text: secret.into(),
    source_lang: secret.into(),
    target_lang: secret.into(),
  };
  let response = transnet::TranslateResponse {
    translation: secret.into(),
  };
  let input =
    TranslationInput::new(secret, "en", Some(secret), "en", EnglishDialect::American).unwrap();
  let result = TranslationResult {
    source_language: secret.into(),
    language_confidence: Confidence::High,
    entries: Vec::new(),
    warnings: vec![secret.into()],
  };
  let retrieval =
    RetrievalRequest::for_public_api(secret, LanguageTag::parse("en").unwrap()).unwrap();
  let debug = format!("{request:?}{response:?}{input:?}{result:?}{retrieval:?}");
  assert!(!debug.contains(secret));
  assert!(debug.contains("REDACTED"));
}
