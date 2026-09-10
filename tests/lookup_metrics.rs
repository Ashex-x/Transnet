//! Integration coverage for closed metric events on model-only lookups.

use std::{
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  time::Duration,
};

use async_trait::async_trait;
use axum::{
  body::Body,
  http::{Request, StatusCode},
};
use tokio::{
  sync::{Notify, Semaphore},
  time::timeout,
};
use tower::ServiceExt;
use transnet::{
  adapters::in_memory::InMemoryMetricsRecorder,
  app_router,
  domain::{
    observability::{LookupStage, MetricEvent, MetricOutcome, ModelValidationOutcome},
    translation::{Confidence, TranslationInput, TranslationResult},
  },
  ports::{
    learning_model::{LearningModel, LearningModelError},
    metrics::MetricsRecorder,
  },
  AppState, ProviderConfig, TranslationConfig, TranslationService,
};

#[derive(Clone, Copy)]
enum StubOutcome {
  Success,
  Unavailable,
  InvalidOutput,
}

struct StubModel {
  outcome: StubOutcome,
}

#[async_trait]
impl LearningModel for StubModel {
  async fn generate(
    &self,
    input: &TranslationInput,
  ) -> Result<TranslationResult, LearningModelError> {
    match self.outcome {
      StubOutcome::Success => Ok(TranslationResult {
        source_language: input.source_language.clone(),
        language_confidence: Confidence::High,
        entries: Vec::new(),
        warnings: Vec::new(),
      }),
      StubOutcome::Unavailable => Err(LearningModelError::Unavailable),
      StubOutcome::InvalidOutput => Err(LearningModelError::InvalidOutput),
    }
  }
}

fn model_app(outcome: StubOutcome, recorder: &InMemoryMetricsRecorder) -> axum::Router {
  model_app_with_recorder(outcome, Arc::new(recorder.clone()))
}

fn model_app_with_recorder(
  outcome: StubOutcome,
  recorder: Arc<dyn MetricsRecorder>,
) -> axum::Router {
  app_router(
    AppState::new(translation_service())
      .with_learning_model(Arc::new(StubModel { outcome }))
      .with_metrics_recorder(recorder),
  )
}

fn translation_service() -> TranslationService {
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
  .expect("test provider configuration is valid")
}

fn lookup_request(body: &'static str) -> Request<Body> {
  Request::post("/v1/lookups")
    .header("content-type", "application/json")
    .header("x-request-id", "request-id-secret-8172")
    .body(Body::from(body))
    .expect("lookup request is valid")
}

async fn recorded_events(
  recorder: &InMemoryMetricsRecorder,
  expected_count: usize,
) -> Vec<MetricEvent> {
  timeout(Duration::from_secs(1), async {
    loop {
      let events = recorder.events().await;
      if events.len() >= expected_count {
        return events;
      }
      tokio::task::yield_now().await;
    }
  })
  .await
  .expect("metrics recorder received the expected events")
}

#[derive(Clone)]
struct BlockingMetricsRecorder {
  started: Arc<AtomicUsize>,
  completed: Arc<AtomicUsize>,
  started_notify: Arc<Notify>,
  completed_notify: Arc<Notify>,
  gate: Arc<Semaphore>,
}

impl BlockingMetricsRecorder {
  fn new() -> Self {
    Self {
      started: Arc::new(AtomicUsize::new(0)),
      completed: Arc::new(AtomicUsize::new(0)),
      started_notify: Arc::new(Notify::new()),
      completed_notify: Arc::new(Notify::new()),
      gate: Arc::new(Semaphore::new(0)),
    }
  }

  fn release(&self, records: usize) {
    self.gate.add_permits(records);
  }

  fn started_count(&self) -> usize {
    self.started.load(Ordering::SeqCst)
  }

  fn completed_count(&self) -> usize {
    self.completed.load(Ordering::SeqCst)
  }

  async fn wait_for_started(&self, expected_count: usize) {
    wait_for_count(
      &self.started,
      &self.started_notify,
      expected_count,
      "blocking recorder started expected records",
    )
    .await;
  }

  async fn wait_for_completed(&self, expected_count: usize) {
    wait_for_count(
      &self.completed,
      &self.completed_notify,
      expected_count,
      "blocking recorder completed expected records",
    )
    .await;
  }
}

#[async_trait]
impl MetricsRecorder for BlockingMetricsRecorder {
  async fn record(&self, _event: MetricEvent) {
    self.started.fetch_add(1, Ordering::SeqCst);
    self.started_notify.notify_waiters();
    let permit = self
      .gate
      .acquire()
      .await
      .expect("blocking recorder gate is open");
    drop(permit);
    self.completed.fetch_add(1, Ordering::SeqCst);
    self.completed_notify.notify_waiters();
  }
}

async fn wait_for_count(
  count: &AtomicUsize,
  notify: &Notify,
  expected_count: usize,
  description: &str,
) {
  timeout(Duration::from_secs(1), async {
    loop {
      let notified = notify.notified();
      if count.load(Ordering::SeqCst) >= expected_count {
        return;
      }
      notified.await;
    }
  })
  .await
  .unwrap_or_else(|_| panic!("timed out while waiting for {description}"));
}

fn assert_redacted(events: &[MetricEvent]) {
  let rendered = format!("{events:?}");
  for private_value in [
    "query-secret-8172",
    "context-secret-8172",
    "request-id-secret-8172",
  ] {
    assert!(
      !rendered.contains(private_value),
      "metric event leaked request data: {private_value}"
    );
  }

  for event in events {
    let attributes = event.attributes();
    for label in attributes.labels() {
      assert!(matches!(label.key(), "stage" | "outcome"));
      assert!(matches!(
        label.value(),
        "request_validation" | "response_assembly" | "succeeded" | "rejected"
      ));
    }
  }
}

#[tokio::test]
async fn successful_model_lookup_records_closed_validation_and_assembly_events() {
  let recorder = InMemoryMetricsRecorder::new();
  let response = model_app(StubOutcome::Success, &recorder)
    .oneshot(lookup_request(
      r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
    ))
    .await
    .expect("router response");

  assert_eq!(response.status(), StatusCode::OK);
  let events = recorded_events(&recorder, 2).await;
  assert_eq!(
    events,
    vec![
      MetricEvent::LookupStage {
        stage: LookupStage::RequestValidation,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::LookupStage {
        stage: LookupStage::ResponseAssembly,
        outcome: MetricOutcome::Succeeded,
      },
    ]
  );
  assert_redacted(&events);
}

#[tokio::test]
async fn rejected_lookup_input_records_only_a_closed_validation_event() {
  let recorder = InMemoryMetricsRecorder::new();
  let response = model_app(StubOutcome::Success, &recorder)
    .oneshot(lookup_request(
      r#"{"query":"query-secret-8172","source_language":"es","target_language":"zh-CN","context":"context-secret-8172"}"#,
    ))
    .await
    .expect("router response");

  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let events = recorded_events(&recorder, 1).await;
  assert_eq!(
    events,
    vec![MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Rejected,
    }]
  );
  assert_redacted(&events);
}

#[tokio::test]
async fn unavailable_model_does_not_mislabel_model_work_as_canonical_retrieval() {
  let recorder = InMemoryMetricsRecorder::new();
  let response = model_app(StubOutcome::Unavailable, &recorder)
    .oneshot(lookup_request(
      r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
    ))
    .await
    .expect("router response");

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  let events = recorded_events(&recorder, 1).await;
  assert_eq!(
    events,
    vec![MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Succeeded,
    },]
  );
  assert_redacted(&events);
}

#[tokio::test]
async fn invalid_model_output_records_rejected_validation_without_request_data() {
  let recorder = InMemoryMetricsRecorder::new();
  let response = model_app(StubOutcome::InvalidOutput, &recorder)
    .oneshot(lookup_request(
      r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
    ))
    .await
    .expect("router response");

  assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
  let events = recorded_events(&recorder, 2).await;
  assert_eq!(
    events,
    vec![
      MetricEvent::LookupStage {
        stage: LookupStage::RequestValidation,
        outcome: MetricOutcome::Succeeded,
      },
      MetricEvent::ModelValidation {
        outcome: ModelValidationOutcome::Rejected,
      },
    ]
  );
  assert_redacted(&events);
}

#[tokio::test]
async fn translator_route_does_not_emit_model_lookup_metrics() {
  let recorder = InMemoryMetricsRecorder::new();
  let response = app_router(
    AppState::new(translation_service()).with_metrics_recorder(Arc::new(recorder.clone())),
  )
  .oneshot(
    Request::post("/translate")
      .header("content-type", "application/json")
      .body(Body::from(
        r#"{"text":"query-secret-8172","source_lang":"en","target_lang":"zh-CN"}"#,
      ))
      .expect("translation request is valid"),
  )
  .await
  .expect("router response");

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert!(recorder.events().await.is_empty());
}

#[tokio::test]
async fn lookup_response_does_not_wait_for_a_blocking_metrics_recorder() {
  let recorder = BlockingMetricsRecorder::new();
  let response = timeout(
    Duration::from_secs(1),
    model_app_with_recorder(StubOutcome::Success, Arc::new(recorder.clone())).oneshot(
      lookup_request(
        r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
      ),
    ),
  )
  .await
  .expect("lookup response does not await blocked telemetry")
  .expect("router response");

  assert_eq!(response.status(), StatusCode::OK);
  recorder.wait_for_started(2).await;
  assert_eq!(recorder.completed_count(), 0);

  recorder.release(2);
  recorder.wait_for_completed(2).await;
}

#[tokio::test]
async fn saturated_metrics_dispatch_drops_events_without_queuing_more_tasks() {
  const MAX_IN_FLIGHT_RECORDS: usize = 16;

  let recorder = BlockingMetricsRecorder::new();
  let app = model_app_with_recorder(StubOutcome::Success, Arc::new(recorder.clone()));
  for _ in 0..(MAX_IN_FLIGHT_RECORDS / 2) {
    let response = timeout(
      Duration::from_secs(1),
      app.clone().oneshot(lookup_request(
        r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
      )),
    )
    .await
    .expect("lookup response does not await blocked telemetry")
    .expect("router response");
    assert_eq!(response.status(), StatusCode::OK);
  }
  recorder.wait_for_started(MAX_IN_FLIGHT_RECORDS).await;

  let response = app
    .oneshot(lookup_request(
      r#"{"query":"query-secret-8172","source_language":"es","context":"context-secret-8172"}"#,
    ))
    .await
    .expect("router response");
  assert_eq!(response.status(), StatusCode::OK);
  tokio::task::yield_now().await;
  assert_eq!(recorder.started_count(), MAX_IN_FLIGHT_RECORDS);

  recorder.release(MAX_IN_FLIGHT_RECORDS);
  recorder.wait_for_completed(MAX_IN_FLIGHT_RECORDS).await;
}
