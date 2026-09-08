use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc, Mutex,
};

use axum::{
  extract::State,
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::post,
  Json, Router,
};
use serde_json::{json, Value};
use transnet::{
  ProviderConfig, TranslateRequest, TranslationConfig, TranslationError, TranslationService,
};

#[derive(Clone)]
struct MockState {
  calls: Arc<AtomicUsize>,
  failures: usize,
  empty_response: bool,
  bodies: Arc<Mutex<Vec<Value>>>,
}

async fn completion(State(state): State<MockState>, Json(body): Json<Value>) -> Response {
  state.bodies.lock().unwrap().push(body);
  let call = state.calls.fetch_add(1, Ordering::SeqCst);
  if call < state.failures {
    return StatusCode::BAD_GATEWAY.into_response();
  }
  if state.empty_response {
    return Json(json!({"choices": []})).into_response();
  }
  Json(json!({"choices": [{"message": {"content": " translated "}}]})).into_response()
}

async fn mock_provider(failures: usize, empty_response: bool) -> (String, MockState) {
  let state = MockState {
    calls: Arc::new(AtomicUsize::new(0)),
    failures,
    empty_response,
    bodies: Arc::new(Mutex::new(Vec::new())),
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
  (format!("http://{address}/v1"), state)
}

fn provider(base_url: String, model: &str) -> ProviderConfig {
  ProviderConfig {
    base_url,
    model: model.to_string(),
    api_key: "test-key".to_string(),
  }
}

fn request(text: String) -> TranslateRequest {
  TranslateRequest {
    text,
    source_lang: "en".to_string(),
    target_lang: "zh-CN".to_string(),
  }
}

#[tokio::test]
async fn routes_at_character_boundary_and_uses_provider_schemas() {
  let (gemma_url, gemma) = mock_provider(0, false).await;
  let (translate_url, translate) = mock_provider(0, false).await;
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 2,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider(gemma_url, "Gemma4"),
    provider(translate_url, "TranslateGemma"),
  )
  .unwrap();

  let short = service.translate(request("界".repeat(4000))).await.unwrap();
  let long = service.translate(request("界".repeat(4001))).await.unwrap();
  assert_eq!(short.translation, "translated");
  assert_eq!(long.translation, "translated");
  assert_eq!(gemma.calls.load(Ordering::SeqCst), 1);
  assert_eq!(translate.calls.load(Ordering::SeqCst), 1);

  let gemma_body = &gemma.bodies.lock().unwrap()[0];
  assert_eq!(gemma_body["model"], "Gemma4");
  assert!(gemma_body["messages"][0]["content"].is_string());

  let translate_body = &translate.bodies.lock().unwrap()[0];
  assert_eq!(translate_body["model"], "TranslateGemma");
  assert_eq!(translate_body["messages"][0]["content"][0]["type"], "text");
  assert_eq!(
    translate_body["messages"][0]["content"][0]["source_lang_code"],
    "en"
  );
  assert_eq!(
    translate_body["messages"][0]["content"][0]["target_lang_code"],
    "zh-CN"
  );
}

#[tokio::test]
async fn retries_provider_failures_and_returns_success() {
  let (gemma_url, gemma) = mock_provider(2, false).await;
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 2,
      max_retries: 2,
      retry_delay_ms: 0,
    },
    provider(gemma_url.clone(), "Gemma4"),
    provider(gemma_url, "TranslateGemma"),
  )
  .unwrap();

  assert!(service
    .translate(request("hello".to_string()))
    .await
    .is_ok());
  assert_eq!(gemma.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn returns_provider_error_after_retries_are_exhausted() {
  let (gemma_url, gemma) = mock_provider(3, false).await;
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 2,
      max_retries: 1,
      retry_delay_ms: 0,
    },
    provider(gemma_url.clone(), "Gemma4"),
    provider(gemma_url, "TranslateGemma"),
  )
  .unwrap();

  let result = service.translate(request("hello".to_string())).await;
  assert!(matches!(result, Err(TranslationError::Provider)));
  assert_eq!(gemma.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn retries_empty_provider_envelopes() {
  let (gemma_url, gemma) = mock_provider(0, true).await;
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 2,
      max_retries: 1,
      retry_delay_ms: 0,
    },
    provider(gemma_url.clone(), "Gemma4"),
    provider(gemma_url, "TranslateGemma"),
  )
  .unwrap();

  assert!(matches!(
    service.translate(request("hello".to_string())).await,
    Err(TranslationError::Provider)
  ));
  assert_eq!(gemma.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn checked_in_configuration_uses_expected_provider_defaults() {
  let config: transnet::AppConfig =
    toml::from_str(include_str!("../config/transnet.toml")).unwrap();
  assert_eq!(config.gemma4.base_url, "http://127.0.0.1:18011/v1");
  assert_eq!(config.gemma4.model, "Gemma4");
  assert_eq!(config.translate_gemma.base_url, "http://127.0.0.1:18007/v1");
  assert_eq!(config.translate_gemma.model, "TranslateGemma");
}
