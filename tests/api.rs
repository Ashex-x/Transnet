use axum::{
  body::{to_bytes, Body},
  extract::State,
  http::{Request, StatusCode},
  routing::post,
  Json, Router,
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{app_router, AppState, ProviderConfig, TranslationConfig, TranslationService};

fn app() -> axum::Router {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string(),
  };
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap();
  app_router(AppState::new(service))
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn completion(State(translation): State<&'static str>) -> Json<Value> {
  Json(serde_json::json!({
    "choices": [{"message": {"content": translation}}]
  }))
}

async fn app_with_provider() -> axum::Router {
  let provider = Router::new()
    .route("/v1/chat/completions", post(completion))
    .with_state("translated");
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });

  let provider = ProviderConfig {
    base_url: format!("http://{address}/v1"),
    model: "Gemma4".to_string(),
    api_key: "test".to_string(),
  };
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap();
  app_router(AppState::new(service))
}

#[tokio::test]
async fn health_returns_direct_status() {
  let response = app()
    .oneshot(Request::get("/health").body(Body::empty()).unwrap())
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(json(response).await, serde_json::json!({"status": "ok"}));
}

#[tokio::test]
async fn translate_returns_direct_translation() {
  let response = app_with_provider()
    .await
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":"hello","source_lang":"en","target_lang":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    json(response).await,
    serde_json::json!({"translation": "translated"})
  );
}

#[tokio::test]
async fn translate_maps_provider_failure_to_service_unavailable() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":"hello","source_lang":"en","target_lang":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "translation provider unavailable"})
  );
}

#[tokio::test]
async fn translate_rejects_invalid_input_with_direct_error() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":" ","source_lang":"English","target_lang":"zh_CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "text must not be blank"})
  );
}

#[tokio::test]
async fn translate_rejects_malformed_json() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "invalid JSON request"})
  );
}

#[tokio::test]
async fn removed_routes_return_not_found() {
  for path in [
    "/api/account/login",
    "/api/profile",
    "/api/transnet/history",
    "/api/transnet/favorites",
    "/api/about",
    "/api/stats",
  ] {
    let response = app()
      .oneshot(Request::get(path).body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "path: {path}");
  }
}
