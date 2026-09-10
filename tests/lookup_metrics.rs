//! Integration coverage for closed metric events on model-only lookups.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
  body::Body,
  http::{Request, StatusCode},
};
use tower::ServiceExt;
use transnet::{
  adapters::in_memory::InMemoryMetricsRecorder,
  app_router,
  domain::{
    observability::{LookupStage, MetricEvent, MetricOutcome, ModelValidationOutcome},
    translation::{Confidence, TranslationInput, TranslationResult},
  },
  ports::learning_model::{LearningModel, LearningModelError},
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
  app_router(
    AppState::new(translation_service())
      .with_learning_model(Arc::new(StubModel { outcome }))
      .with_metrics_recorder(Arc::new(recorder.clone())),
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
  let events = recorder.events().await;
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
  let events = recorder.events().await;
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
  let events = recorder.events().await;
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
  let events = recorder.events().await;
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
