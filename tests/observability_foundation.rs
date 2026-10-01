//! HTTP and privacy contract coverage for the observability foundation.

use axum::{
  body::Body,
  http::{Request, StatusCode},
};
use tower::ServiceExt;
use transnet::{app_router, AppState};

fn app() -> axum::Router {
  app_router(AppState::new())
}

#[tokio::test]
async fn valid_trace_parent_is_normalized_and_propagated() {
  let response = app()
    .oneshot(
      Request::post("/api/v1/health")
        .header("content-type", "application/json")
        .header(
          "traceparent",
          "00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01",
        )
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    response.headers()["traceparent"],
    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
  );
}

#[tokio::test]
async fn malformed_or_repeated_trace_parent_is_not_propagated() {
  let malformed = app()
    .clone()
    .oneshot(
      Request::post("/api/v1/health")
        .header("content-type", "application/json")
        .header("traceparent", "credential-secret")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert!(!malformed.headers().contains_key("traceparent"));

  let repeated = app()
    .oneshot(
      Request::post("/api/v1/health")
        .header("content-type", "application/json")
        .header(
          "traceparent",
          "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .header(
          "traceparent",
          "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbb-00",
        )
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert!(!repeated.headers().contains_key("traceparent"));
}
