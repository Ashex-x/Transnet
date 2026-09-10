use std::{
  collections::VecDeque,
  fmt::Write,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
  time::{Duration, Instant},
};

use axum::{
  extract::State,
  http::{header::RETRY_AFTER, HeaderValue, StatusCode},
  response::{IntoResponse, Response},
  routing::post,
  Json, Router,
};
use serde_json::{json, Value};
use tokio::sync::Notify;
use tracing::{field::Visit, Event, Subscriber};
use tracing_subscriber::{layer::Context, prelude::*, Layer};
use transnet::{
  ProviderConfig, ProviderPolicy, TranslateRequest, TranslationConfig, TranslationError,
  TranslationService,
};

#[derive(Clone)]
struct MockState {
  calls: Arc<AtomicUsize>,
  failures: usize,
  empty_response: bool,
  bodies: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
struct SequenceState {
  calls: Arc<AtomicUsize>,
  replies: Arc<Mutex<VecDeque<ProviderReply>>>,
}

#[derive(Clone)]
enum ProviderReply {
  Status {
    status: StatusCode,
    retry_after: Option<HeaderValue>,
  },
  Translation(String),
}

#[derive(Clone)]
struct BlockingState {
  calls: Arc<AtomicUsize>,
  started: Arc<Notify>,
  release: Arc<Notify>,
}

#[derive(Clone, Default)]
struct CapturedEvents {
  events: Arc<Mutex<Vec<String>>>,
}

impl CapturedEvents {
  fn joined(&self) -> String {
    match self.events.lock() {
      Ok(events) => events.join("\n"),
      Err(poisoned) => poisoned.into_inner().join("\n"),
    }
  }
}

impl<S> Layer<S> for CapturedEvents
where
  S: Subscriber,
{
  fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
    let mut visitor = FieldVisitor::default();
    event.record(&mut visitor);
    let mut record = event.metadata().name().to_string();
    let _ = write!(&mut record, " {}", visitor.fields);
    match self.events.lock() {
      Ok(mut events) => events.push(record),
      Err(poisoned) => poisoned.into_inner().push(record),
    }
  }
}

#[derive(Default)]
struct FieldVisitor {
  fields: String,
}

impl Visit for FieldVisitor {
  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    let _ = write!(&mut self.fields, "{}={value:?};", field.name());
  }
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

async fn sequence_completion(
  State(state): State<SequenceState>,
  Json(_body): Json<Value>,
) -> Response {
  state.calls.fetch_add(1, Ordering::SeqCst);
  let reply = match state.replies.lock() {
    Ok(mut replies) => replies.pop_front(),
    Err(poisoned) => poisoned.into_inner().pop_front(),
  }
  .unwrap_or_else(|| ProviderReply::Translation("translated".to_string()));
  match reply {
    ProviderReply::Status {
      status,
      retry_after,
    } => {
      let mut response = status.into_response();
      if let Some(retry_after) = retry_after {
        response.headers_mut().insert(RETRY_AFTER, retry_after);
      }
      response
    }
    ProviderReply::Translation(content) => {
      Json(json!({"choices": [{"message": {"content": content}}]})).into_response()
    }
  }
}

async fn blocking_completion(
  State(state): State<BlockingState>,
  Json(_body): Json<Value>,
) -> Response {
  state.calls.fetch_add(1, Ordering::SeqCst);
  state.started.notify_one();
  state.release.notified().await;
  Json(json!({"choices": [{"message": {"content": "translated"}}]})).into_response()
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

async fn sequence_provider(replies: Vec<ProviderReply>) -> (String, SequenceState) {
  let state = SequenceState {
    calls: Arc::new(AtomicUsize::new(0)),
    replies: Arc::new(Mutex::new(replies.into())),
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(sequence_completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
  (format!("http://{address}/v1"), state)
}

async fn blocking_provider() -> (String, BlockingState) {
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

fn policy(
  max_retries: u32,
  retry_delay: Duration,
  max_concurrent_requests: usize,
  circuit_failure_threshold: u32,
) -> ProviderPolicy {
  ProviderPolicy::new(
    Duration::from_secs(2),
    max_retries,
    retry_delay,
    Duration::from_secs(5),
    max_concurrent_requests,
    circuit_failure_threshold,
    Duration::from_secs(1),
  )
  .unwrap()
}

fn resilient_service(base_url: String, policy: ProviderPolicy) -> TranslationService {
  TranslationService::with_provider_policies(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 2,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider(base_url.clone(), "Gemma4"),
    policy.clone(),
    provider(base_url, "TranslateGemma"),
    policy,
  )
  .unwrap()
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
  assert_eq!(config.provider_resilience.gemma4.max_concurrent_requests, 8);
  assert_eq!(
    config
      .provider_resilience
      .translate_gemma
      .max_concurrent_requests,
    4
  );
}

#[tokio::test]
async fn does_not_retry_non_transient_provider_statuses() {
  let (url, state) = sequence_provider(vec![ProviderReply::Status {
    status: StatusCode::BAD_REQUEST,
    retry_after: None,
  }])
  .await;
  let service = resilient_service(url, policy(3, Duration::from_millis(0), 8, 5));

  assert!(matches!(
    service.translate(request("hello".to_string())).await,
    Err(TranslationError::Provider)
  ));
  assert_eq!(state.calls.load(Ordering::SeqCst), 1);
  assert_eq!(service.provider_metrics().gemma4.retried, 0);
}

#[tokio::test]
async fn retries_connection_establishment_failures() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  drop(listener);
  let service = resilient_service(
    format!("http://{address}/v1"),
    policy(1, Duration::from_millis(0), 8, 5),
  );

  assert!(matches!(
    service.translate(request("hello".to_string())).await,
    Err(TranslationError::Provider)
  ));
  let metrics = service.provider_metrics().gemma4;
  assert_eq!(metrics.attempts, 2);
  assert_eq!(metrics.retried, 1);
}

#[tokio::test]
async fn honors_retry_after_for_rate_limits() {
  let (url, state) = sequence_provider(vec![
    ProviderReply::Status {
      status: StatusCode::TOO_MANY_REQUESTS,
      retry_after: Some(HeaderValue::from_static("0")),
    },
    ProviderReply::Translation("translated".to_string()),
  ])
  .await;
  let service = resilient_service(url, policy(1, Duration::from_secs(1), 8, 5));

  let started = Instant::now();
  assert_eq!(
    service
      .translate(request("hello".to_string()))
      .await
      .unwrap()
      .translation,
    "translated"
  );
  assert!(started.elapsed() < Duration::from_millis(800));
  assert_eq!(state.calls.load(Ordering::SeqCst), 2);
  let metrics = service.provider_metrics().gemma4;
  assert_eq!(metrics.retried, 1);
  assert_eq!(metrics.rate_limited, 1);
}

#[tokio::test]
async fn opens_a_provider_circuit_without_another_http_attempt() {
  let (url, state) = sequence_provider(vec![ProviderReply::Status {
    status: StatusCode::BAD_GATEWAY,
    retry_after: None,
  }])
  .await;
  let service = resilient_service(url, policy(0, Duration::from_millis(0), 8, 1));

  assert!(matches!(
    service.translate(request("first".to_string())).await,
    Err(TranslationError::Provider)
  ));
  assert!(matches!(
    service.translate(request("second".to_string())).await,
    Err(TranslationError::Provider)
  ));
  assert_eq!(state.calls.load(Ordering::SeqCst), 1);
  assert_eq!(service.provider_metrics().gemma4.circuit_open, 1);
}

#[tokio::test]
async fn rejects_concurrent_calls_when_the_provider_bulkhead_is_full() {
  let (url, state) = blocking_provider().await;
  let service = resilient_service(url, policy(0, Duration::from_millis(0), 1, 5));
  let first_service = service.clone();
  let first =
    tokio::spawn(async move { first_service.translate(request("first".to_string())).await });

  state.started.notified().await;
  assert!(matches!(
    service.translate(request("second".to_string())).await,
    Err(TranslationError::Provider)
  ));
  state.release.notify_one();
  assert!(first.await.unwrap().is_ok());
  assert_eq!(state.calls.load(Ordering::SeqCst), 1);
  assert_eq!(service.provider_metrics().gemma4.bulkhead_rejected, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn provider_telemetry_excludes_payloads_credentials_and_identity_data() {
  let (url, _state) = sequence_provider(vec![ProviderReply::Translation(
    "ANSWER_SECRET BODY_SECRET".to_string(),
  )])
  .await;
  let captured = CapturedEvents::default();
  let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(captured.clone()));
  let _guard = tracing::dispatcher::set_default(&dispatch);
  let service = TranslationService::with_provider_policies(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 2,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    ProviderConfig {
      base_url: url.clone(),
      model: "model-secret-is-not-a-telemetry-field".to_string(),
      api_key: "CREDENTIAL_SECRET".to_string(),
    },
    policy(0, Duration::ZERO, 8, 5),
    ProviderConfig {
      base_url: url,
      model: "other-model".to_string(),
      api_key: "CREDENTIAL_SECRET".to_string(),
    },
    policy(0, Duration::ZERO, 8, 5),
  )
  .unwrap();

  let response = service
    .translate(request("QUERY_SECRET IDENTITY_SECRET".to_string()))
    .await
    .unwrap();
  assert_eq!(response.translation, "ANSWER_SECRET BODY_SECRET");
  let telemetry = captured.joined();
  let metrics = format!("{:?}", service.provider_metrics());
  for secret in [
    "QUERY_SECRET",
    "ANSWER_SECRET",
    "BODY_SECRET",
    "CREDENTIAL_SECRET",
    "IDENTITY_SECRET",
  ] {
    assert!(!telemetry.contains(secret), "telemetry leaked {secret}");
    assert!(!metrics.contains(secret), "metrics leaked {secret}");
  }
}
