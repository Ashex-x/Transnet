//! Service-boundary regressions: private state is rejected before model or data work.

use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use tower::ServiceExt;
use transnet::{app_router, AppState};

const REMOVED_OBSOLETE_MODULES: &[&str] = &[
  "src/adapters/in_memory/feedback.rs",
  "src/adapters/in_memory/graph_view.rs",
  "src/adapters/in_memory/idempotency.rs",
  "src/adapters/in_memory/jobs.rs",
  "src/adapters/in_memory/learner_state.rs",
  "src/adapters/in_memory/lookup_jobs.rs",
  "src/adapters/in_memory/practice_state.rs",
  "src/adapters/in_memory/repository.rs",
  "src/api/v1/lookup_job.rs",
  "src/application/canonical_lookup_cache.rs",
  "src/application/durable_worker.rs",
  "src/application/feedback.rs",
  "src/application/graph_view.rs",
  "src/application/learner_state.rs",
  "src/application/lookup_job.rs",
  "src/application/practice_state.rs",
  "src/domain/canonical_lookup_cache.rs",
  "src/domain/feedback.rs",
  "src/domain/graph_view.rs",
  "src/domain/learner.rs",
  "src/domain/practice.rs",
  "src/ports/durable_job.rs",
  "src/ports/graph_feedback.rs",
  "src/ports/graph_view.rs",
  "src/ports/idempotency.rs",
  "src/ports/learner_state.rs",
  "src/ports/lookup_job.rs",
  "src/ports/practice_state.rs",
  "src/ports/repository.rs",
  "src/domain/translation.rs",
  "src/application/lookup.rs",
  "src/application/retrieval.rs",
  "src/application/graph.rs",
  "src/application/graph_topology_cache.rs",
  "src/adapters/learning_model.rs",
  "src/adapters/in_memory_retrieval.rs",
  "src/ports/learning_model.rs",
  "src/ports/translation_model.rs",
  "src/ports/graph_repository.rs",
  "src/ports/vector_retriever.rs",
];

fn router() -> axum::Router {
  app_router(AppState::new())
}

#[test]
fn obsolete_private_state_modules_are_absent_from_the_source_surface() {
  let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

  for relative_path in REMOVED_OBSOLETE_MODULES {
    assert!(
      !manifest.join(relative_path).exists(),
      "obsolete private-state module returned: {relative_path}"
    );
  }
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
      "/api/v1/translations",
      serde_json::json!({"input":{"type":"text","text":"hello"},"source_language":"en","target_language":"zh-CN","response_level":"brief"}),
    ),
    (
      "/api/v1/basic-cards/lookup",
      serde_json::json!({"query":"hello","source_language":"en","target_language":"zh-CN"}),
    ),
  ] {
    for field in [
      "user_id",
      "account_id",
      "learner_level",
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
      Request::post("/api/v1/basic-cards/lookup")
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
    translation_turn::{TranslationTurn, TranslationTurnRequest},
  };
  let secret = "private-sentinel-8391";
  let input: TranslationTurnRequest = serde_json::from_value(serde_json::json!({
    "input":{"type":"text","text":secret},
    "source_language":"en","target_language":"zh-CN","response_level":"brief"
  }))
  .unwrap();
  let turn = TranslationTurn::new(input).unwrap();
  let retrieval =
    RetrievalRequest::for_public_api(secret, LanguageTag::parse("en").unwrap()).unwrap();
  let debug = format!("{turn:?}{retrieval:?}");
  assert!(!debug.contains(secret));
  assert!(debug.contains("REDACTED"));
}
