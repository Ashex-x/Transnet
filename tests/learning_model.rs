use std::{
  collections::VecDeque,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
  time::Duration,
};

use axum::{
  extract::State,
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::post,
  Json, Router,
};
use serde_json::{json, Value};
use tokio::sync::Notify;
use transnet::{
  adapters::learning_model::OpenAiLearningModel,
  domain::translation::{EnglishDialect, TranslationInput},
  ports::learning_model::{LearningModel, LearningModelError},
  ProviderConfig, ProviderPolicy, TranslationConfig,
};

#[derive(Clone)]
struct MockState {
  responses: Arc<Mutex<VecDeque<String>>>,
  bodies: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
struct StatusState {
  calls: Arc<AtomicUsize>,
  status: StatusCode,
}

#[derive(Clone)]
struct BlockingState {
  calls: Arc<AtomicUsize>,
  started: Arc<Notify>,
  release: Arc<Notify>,
}

async fn completion(State(state): State<MockState>, Json(body): Json<Value>) -> Json<Value> {
  state.bodies.lock().unwrap().push(body);
  let content = state.responses.lock().unwrap().pop_front().unwrap();
  Json(json!({"choices": [{"message": {"content": content}}]}))
}

async fn status_completion(State(state): State<StatusState>, Json(_body): Json<Value>) -> Response {
  state.calls.fetch_add(1, Ordering::SeqCst);
  state.status.into_response()
}

async fn blocking_completion(
  State(state): State<BlockingState>,
  Json(_body): Json<Value>,
) -> Response {
  state.calls.fetch_add(1, Ordering::SeqCst);
  state.started.notify_one();
  state.release.notified().await;
  Json(json!({"choices": [{"message": {"content": output()}}]})).into_response()
}

async fn model(responses: Vec<String>) -> (OpenAiLearningModel, MockState) {
  let state = MockState {
    responses: Arc::new(Mutex::new(responses.into())),
    bodies: Arc::new(Mutex::new(Vec::new())),
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

  let settings = TranslationConfig {
    long_text_chars: 4_000,
    timeout_seconds: 2,
    max_retries: 0,
    retry_delay_ms: 0,
  };
  let provider = ProviderConfig {
    base_url: format!("http://{address}/v1"),
    model: "Gemma4".to_string(),
    api_key: "test".into(),
  };
  (
    OpenAiLearningModel::new(&settings, provider).unwrap(),
    state,
  )
}

fn policy(
  max_retries: u32,
  max_concurrent_requests: usize,
  circuit_failure_threshold: u32,
) -> ProviderPolicy {
  ProviderPolicy::new(
    Duration::from_secs(2),
    max_retries,
    Duration::ZERO,
    Duration::from_secs(1),
    max_concurrent_requests,
    circuit_failure_threshold,
    Duration::from_secs(1),
  )
  .unwrap()
}

async fn status_model(
  status: StatusCode,
  policy: ProviderPolicy,
) -> (OpenAiLearningModel, StatusState) {
  let state = StatusState {
    calls: Arc::new(AtomicUsize::new(0)),
    status,
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(status_completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

  (
    OpenAiLearningModel::with_provider_policy(
      ProviderConfig {
        base_url: format!("http://{address}/v1"),
        model: "Gemma4".to_string(),
        api_key: "test".into(),
      },
      policy,
    )
    .unwrap(),
    state,
  )
}

async fn blocking_model(policy: ProviderPolicy) -> (OpenAiLearningModel, BlockingState) {
  let state = BlockingState {
    calls: Arc::new(AtomicUsize::new(0)),
    started: Arc::new(Notify::new()),
    release: Arc::new(Notify::new()),
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(blocking_completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

  (
    OpenAiLearningModel::with_provider_policy(
      ProviderConfig {
        base_url: format!("http://{address}/v1"),
        model: "Gemma4".to_string(),
        api_key: "test".into(),
      },
      policy,
    )
    .unwrap(),
    state,
  )
}

fn input() -> TranslationInput {
  TranslationInput::new(
    "caliente",
    "es",
    Some("La sopa está caliente."),
    "zh-CN",
    EnglishDialect::American,
    None,
  )
  .unwrap()
}

fn output() -> String {
  json!({
    "source_language": "es",
    "language_confidence": "high",
    "entries": [{
      "lemma": "hot",
      "part_of_speech": "adjective",
      "definition": "having a high temperature",
      "localized_gloss": "温度高的",
      "confidence": "high",
      "pronunciations": [],
      "forms": [{"form": "hotter", "label": "comparative"}],
      "usage_notes": [{"kind": "habit", "text": "Common before nouns."}],
      "examples": [{"english": "The soup is hot.", "localized": "汤很烫。"}],
      "etymology": null,
      "related_words": [{"lemma": "warm", "relation": "lower_degree", "note": null}]
    }],
    "warnings": []
  })
  .to_string()
}

#[tokio::test]
async fn sends_protected_input_and_strict_schema() {
  let (model, state) = model(vec![output()]).await;

  let result = model.generate(&input()).await.unwrap();

  assert_eq!(result.entries[0].lemma, "hot");
  let bodies = state.bodies.lock().unwrap();
  assert_eq!(bodies.len(), 1);
  assert_eq!(bodies[0]["response_format"]["type"], "json_schema");
  assert_eq!(bodies[0]["response_format"]["json_schema"]["strict"], true);
  let user_message = bodies[0]["messages"][1]["content"].as_str().unwrap();
  assert!(user_message.contains("\"query\":\"caliente\""));
  assert!(user_message.contains("\"context\":\"La sopa está caliente.\""));
}

#[tokio::test]
async fn repairs_invalid_output_once() {
  let (model, state) = model(vec!["not json".to_string(), output()]).await;

  assert!(model.generate(&input()).await.is_ok());

  let bodies = state.bodies.lock().unwrap();
  assert_eq!(bodies.len(), 2);
  assert_eq!(bodies[1]["messages"][2]["role"], "assistant");
  assert_eq!(bodies[1]["messages"][2]["content"], "not json");
}

#[tokio::test]
async fn rejects_output_after_one_failed_repair() {
  let (model, state) = model(vec!["bad".to_string(), "still bad".to_string()]).await;

  assert_eq!(
    model.generate(&input()).await,
    Err(LearningModelError::InvalidOutput)
  );
  assert_eq!(state.bodies.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn learning_model_circuit_rejects_calls_without_more_provider_attempts() {
  let (model, state) = status_model(StatusCode::BAD_GATEWAY, policy(0, 8, 1)).await;

  assert_eq!(
    model.generate(&input()).await,
    Err(LearningModelError::Unavailable)
  );
  assert_eq!(
    model.generate(&input()).await,
    Err(LearningModelError::Unavailable)
  );
  assert_eq!(state.calls.load(Ordering::SeqCst), 1);
  assert_eq!(model.provider_metrics().circuit_open, 1);
}

#[tokio::test]
async fn learning_model_bulkhead_rejects_parallel_generation() {
  let (model, state) = blocking_model(policy(0, 1, 5)).await;
  let first_model = model.clone();
  let first_input = input();
  let first = tokio::spawn(async move { first_model.generate(&first_input).await });

  state.started.notified().await;
  assert_eq!(
    model.generate(&input()).await,
    Err(LearningModelError::Unavailable)
  );
  state.release.notify_one();
  assert!(first.await.unwrap().is_ok());
  assert_eq!(state.calls.load(Ordering::SeqCst), 1);
  assert_eq!(model.provider_metrics().bulkhead_rejected, 1);
}
