//! Regression tests proving durable lookup jobs remain outside the service boundary.

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
        long_text_chars: 4_000,
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
async fn lookup_job_routes_are_permanently_absent() {
  for method in ["GET", "POST"] {
    let response = router()
      .oneshot(
        Request::builder()
          .method(method)
          .uri("/v1/lookup-jobs/job-example")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method}");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = to_bytes(response.into_body(), 8_192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["code"], "not_found");
  }
}

#[tokio::test]
async fn removed_job_credentials_are_rejected_before_route_fallback() {
  let secret = "private-capability-sentinel-8391";
  let response = router()
    .oneshot(
      Request::get("/v1/lookup-jobs/job-example")
        .header("lookup-capability", secret)
        .header("x-request-id", "removed-job-boundary")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  assert_eq!(response.headers()["x-request-id"], "removed-job-boundary");
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = to_bytes(response.into_body(), 8_192).await.unwrap();
  assert!(!String::from_utf8_lossy(&body).contains(secret));
}
