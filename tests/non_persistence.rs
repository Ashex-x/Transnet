//! Repository-side proofs that request material stays out of Transnet diagnostics and telemetry.

use std::{
  io::{self, Write},
  sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;
use transnet::{
  app_router,
  application::translation::TranslationOrchestrator,
  domain::{
    live_retrieval::{LiveFetchedPage, LiveSearchQuery, LiveSearchResult},
    observability::{
      GraphOperation, LookupStage, MetricEvent, MetricOutcome, ModelValidationOutcome,
      TelemetryDropReason, VectorLagBucket,
    },
    translation_turn::{TranslationTurn, TranslationTurnRequest},
  },
  AppState, GenerationInput, GenerationOutput, GenerationPort, GenerationRequest,
  GenerationResponse, ModelOperationContext, ModelOperationError, ModelVersion,
};
use url::Url;

const TEXT_SENTINEL: &str = "np-text-sentinel-51d9";
const HISTORY_SENTINEL: &str = "np-history-sentinel-83ab";
const TERM_SENTINEL: &str = "np-term-sentinel-c4e2";
const SEGMENT_SENTINEL: &str = "np-segment-sentinel-774a";
const IMAGE_SENTINEL: &str = "np-image-sentinel-b91f";
const HEADER_SENTINEL: &str = "np-header-sentinel-67c3";
const PROVIDER_SENTINEL: &str = "np-provider-sentinel-f402";
const REASONING_SENTINEL: &str = "np-reasoning-sentinel-2a18";
const LIVE_QUERY_SENTINEL: &str = "np-live-query-sentinel-2f6c";
const LIVE_FRAGMENT_SENTINEL: &str = "np-live-fragment-sentinel-91ae";
const LIVE_SOURCE_SENTINEL: &str = "np-live-source-sentinel-77db";
const ONE_PIXEL_PNG: &str =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

const SENTINELS: &[&str] = &[
  TEXT_SENTINEL,
  HISTORY_SENTINEL,
  TERM_SENTINEL,
  SEGMENT_SENTINEL,
  IMAGE_SENTINEL,
  HEADER_SENTINEL,
  PROVIDER_SENTINEL,
  REASONING_SENTINEL,
  LIVE_QUERY_SENTINEL,
  LIVE_FRAGMENT_SENTINEL,
  LIVE_SOURCE_SENTINEL,
];

#[derive(Clone, Default)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl CaptureWriter {
  fn contents(&self) -> String {
    String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
  }
}

impl Write for CaptureWriter {
  fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
    self.0.lock().unwrap().extend_from_slice(bytes);
    Ok(bytes.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
  type Writer = Self;

  fn make_writer(&'a self) -> Self::Writer {
    self.clone()
  }
}

#[derive(Clone, Copy)]
struct FailingWriter;

impl Write for FailingWriter {
  fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
    Err(io::Error::other("synthetic telemetry writer failure"))
  }

  fn flush(&mut self) -> io::Result<()> {
    Err(io::Error::other("synthetic telemetry writer failure"))
  }
}

impl<'a> MakeWriter<'a> for FailingWriter {
  type Writer = Self;

  fn make_writer(&'a self) -> Self::Writer {
    *self
  }
}

#[derive(Clone)]
struct FixedGeneration(Result<String, ModelOperationError>);

#[async_trait]
impl GenerationPort for FixedGeneration {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    let output = self.0.clone()?;
    Ok(GenerationResponse {
      output: GenerationOutput::new(output).map_err(|_| ModelOperationError::InvalidOutput)?,
      model_version: ModelVersion::new("non-persistence-test-model-v1").unwrap(),
      prompt_version: request.prompt_version,
    })
  }
}

fn translated_app(output: Result<String, ModelOperationError>) -> axum::Router {
  let orchestrator = TranslationOrchestrator::new(Arc::new(FixedGeneration(output)));
  app_router(AppState::new().with_translation_orchestrator(Arc::new(orchestrator)))
}

fn post_translation(body: Value) -> Request<Body> {
  Request::post("/api/v1/translations")
    .header(header::CONTENT_TYPE, "application/json")
    .header("x-request-id", "non-persistence-proof")
    .body(Body::from(body.to_string()))
    .unwrap()
}

fn assert_content_free(rendered: &str) {
  for sentinel in SENTINELS {
    assert!(
      !rendered.contains(sentinel),
      "diagnostic sink leaked sentinel {sentinel}"
    );
    let fingerprint = format!("{:x}", Sha256::digest(sentinel.as_bytes()));
    assert!(
      !rendered.contains(&fingerprint),
      "diagnostic sink leaked deterministic fingerprint for {sentinel}"
    );
  }
}

#[test]
fn http_traces_and_failure_diagnostics_exclude_request_and_provider_material() {
  let captured = CaptureWriter::default();
  let captured_for_thread = captured.clone();
  let statuses = std::thread::spawn(move || {
    let subscriber = tracing_subscriber::fmt()
      .without_time()
      .with_ansi(false)
      .with_target(false)
      .with_max_level(tracing::Level::TRACE)
      .with_writer(captured_for_thread)
      .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    tracing::dispatcher::with_default(&dispatch, || {
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
          let success = translated_app(Ok(
            json!({"status":"complete","translation":"safe translated result"}).to_string(),
          ))
          .oneshot(post_translation(json!({
            "input":{"type":"text","text":format!(
              "This complete sentence includes {TEXT_SENTINEL}."
            )},
            "source_language":"en","target_language":"zh-CN","response_level":"full",
            "history":[{"source_text":HISTORY_SENTINEL,"translated_text":"safe prior result",
              "source_language":"en","target_language":"zh-CN"}],
            "guidance":{"purpose":"technical","audience":"specialist","register":"preserve",
              "terminology":[{"source":TERM_SENTINEL,"target":"safe-term","policy":"preferred"}],
              "annotations":[],"freshness":"offline"}
          })))
          .await
          .unwrap();

          let invalid_provider = translated_app(Ok(PROVIDER_SENTINEL.into()))
            .oneshot(post_translation(json!({
              "input":{"type":"text","text":"ordinary input sentence."},
              "source_language":"en","target_language":"zh-CN","response_level":"brief"
            })))
            .await
            .unwrap();
          let invalid_provider_status = invalid_provider.status();
          let invalid_provider_body = to_bytes(invalid_provider.into_body(), 16_384)
            .await
            .unwrap();
          assert_content_free(&String::from_utf8_lossy(&invalid_provider_body));

          let private_header = app_router(AppState::new())
            .oneshot(
              Request::post("/api/v1/translations")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-user-id", HEADER_SENTINEL)
                .body(Body::from("{}"))
                .unwrap(),
            )
            .await
            .unwrap();
          let private_header_status = private_header.status();
          let private_header_body = to_bytes(private_header.into_body(), 16_384).await.unwrap();
          assert_content_free(&String::from_utf8_lossy(&private_header_body));

          let segments = app_router(AppState::new())
            .oneshot(post_translation(json!({
              "input":{"type":"segments","segments":[{"segment_id":"segment-proof",
                "text":SEGMENT_SENTINEL,"role":"paragraph","format":"plain",
                "protected_ranges":[{"start":0,"end":SEGMENT_SENTINEL.chars().count()}]}]},
              "source_language":"en","target_language":"zh-CN","response_level":"brief"
            })))
            .await
            .unwrap();
          let segments_status = segments.status();
          let segments_body = to_bytes(segments.into_body(), 16_384).await.unwrap();
          assert_content_free(&String::from_utf8_lossy(&segments_body));

          let image = app_router(AppState::new())
            .oneshot(post_translation(json!({
              "input":{"type":"image_regions","images":[{"image_id":IMAGE_SENTINEL,
                "media_type":"image/png","data":ONE_PIXEL_PNG,
                "regions":[{"region_id":"region-proof","x":0.0,"y":0.0,
                  "width":1.0,"height":1.0}]}],
                "reading_order":[format!("{IMAGE_SENTINEL}:region-proof")]},
              "source_language":"auto","target_language":"zh-CN","response_level":"brief"
            })))
            .await
            .unwrap();
          let image_status = image.status();
          let image_body = to_bytes(image.into_body(), 16_384).await.unwrap();
          assert_content_free(&String::from_utf8_lossy(&image_body));

          let raw_path = app_router(AppState::new())
            .oneshot(
              Request::get(format!(
                "/unmatched/{TEXT_SENTINEL}?query={LIVE_QUERY_SENTINEL}"
              ))
              .body(Body::empty())
              .unwrap(),
            )
            .await
            .unwrap();
          let raw_path_status = raw_path.status();
          let raw_path_body = to_bytes(raw_path.into_body(), 16_384).await.unwrap();
          assert_content_free(&String::from_utf8_lossy(&raw_path_body));

          [
            success.status(),
            invalid_provider_status,
            private_header_status,
            segments_status,
            image_status,
            raw_path_status,
          ]
        })
    })
  })
  .join()
  .unwrap();

  assert_eq!(statuses[0], StatusCode::OK);
  assert_eq!(statuses[1], StatusCode::BAD_GATEWAY);
  assert_eq!(statuses[2], StatusCode::BAD_REQUEST);
  assert_eq!(statuses[3], StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(statuses[4], StatusCode::SERVICE_UNAVAILABLE);
  assert!(statuses[5].is_client_error());

  let trace = captured.contents();
  assert!(trace.contains("http.request"));
  assert!(trace.contains("/api/v1/translations"));
  assert!(trace.contains("unmatched"));
  assert_content_free(&trace);
}

#[test]
fn request_local_debug_and_metric_surfaces_are_content_free() {
  let text_turn = TranslationTurn::new(
    serde_json::from_value::<TranslationTurnRequest>(json!({
      "input":{"type":"text","text":TEXT_SENTINEL},
      "source_language":"en","target_language":"zh-CN","response_level":"brief",
      "history":[{"source_text":HISTORY_SENTINEL,"translated_text":"safe history",
        "source_language":"en","target_language":"zh-CN"}],
      "guidance":{"purpose":"technical","audience":"specialist","register":"preserve",
        "terminology":[{"source":TERM_SENTINEL,"target":"safe-term","policy":"preferred"}],
        "annotations":[],"freshness":"offline"}
    }))
    .unwrap(),
  )
  .unwrap();
  let segment_turn = TranslationTurn::new(
    serde_json::from_value::<TranslationTurnRequest>(json!({
      "input":{"type":"segments","segments":[{"segment_id":"segment-proof",
        "text":SEGMENT_SENTINEL,"role":"paragraph","format":"plain"}]},
      "source_language":"en","target_language":"zh-CN","response_level":"brief"
    }))
    .unwrap(),
  )
  .unwrap();
  let image_turn = TranslationTurn::new(
    serde_json::from_value::<TranslationTurnRequest>(json!({
      "input":{"type":"image_regions","images":[{"image_id":IMAGE_SENTINEL,
        "media_type":"image/png","data":ONE_PIXEL_PNG,
        "regions":[{"region_id":"region-proof","x":0.0,"y":0.0,
          "width":1.0,"height":1.0}]}],
        "reading_order":[format!("{IMAGE_SENTINEL}:region-proof")]},
      "source_language":"auto","target_language":"zh-CN","response_level":"brief"
    }))
    .unwrap(),
  )
  .unwrap();

  let query = LiveSearchQuery::new(LIVE_QUERY_SENTINEL).unwrap();
  let result = LiveSearchResult::new(
    Url::parse("https://example.com/non-persistence").unwrap(),
    LIVE_SOURCE_SENTINEL.into(),
    None,
    None,
  )
  .unwrap();
  let page = LiveFetchedPage::new(
    result,
    LIVE_FRAGMENT_SENTINEL.into(),
    "2026-10-03T00:00:00Z".into(),
    LIVE_FRAGMENT_SENTINEL.len(),
  )
  .unwrap();
  let provider_output = GenerationOutput::new(PROVIDER_SENTINEL).unwrap();
  let provider_input = GenerationInput::new(TEXT_SENTINEL).unwrap();
  let reasoning =
    transnet::provider::ProviderStreamEvent::ReasoningDelta(REASONING_SENTINEL.into());

  let rendered = format!(
    "{text_turn:?} {segment_turn:?} {image_turn:?} {query:?} {page:?} \
     {provider_input:?} {provider_output:?} {reasoning:?} \
     {:?} {:?}",
    ModelOperationError::InvalidOutput,
    transnet::domain::live_retrieval::LiveRetrievalError::Unavailable,
  );
  assert_content_free(&rendered);
  assert!(rendered.contains("REDACTED"));

  let events = [
    MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Rejected,
    },
    MetricEvent::ModelValidation {
      outcome: ModelValidationOutcome::Rejected,
    },
    MetricEvent::VectorLag {
      bucket: VectorLagBucket::Unavailable,
    },
    MetricEvent::GraphOperation {
      operation: GraphOperation::Traversal,
      outcome: MetricOutcome::Failed,
    },
    MetricEvent::TelemetryDropped {
      reason: TelemetryDropReason::Capacity,
    },
  ];
  let metrics = events
    .into_iter()
    .map(|event| {
      let labels = event
        .attributes()
        .labels()
        .iter()
        .map(|label| format!("{}={}", label.key(), label.value()))
        .collect::<Vec<_>>()
        .join(",");
      format!("{}:{labels}", event.metric_name().as_str())
    })
    .collect::<Vec<_>>()
    .join("\n");
  assert_content_free(&metrics);
}

#[test]
fn tracing_writer_failure_does_not_change_the_business_response() {
  let status = std::thread::spawn(|| {
    let subscriber = tracing_subscriber::fmt()
      .without_time()
      .with_ansi(false)
      .with_target(false)
      .with_max_level(tracing::Level::TRACE)
      .with_writer(FailingWriter)
      .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    tracing::dispatcher::with_default(&dispatch, || {
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
          translated_app(Ok(
            json!({"status":"complete","translation":"safe translated result"}).to_string(),
          ))
          .oneshot(post_translation(json!({
            "input":{"type":"text","text":"A complete sentence remains translatable."},
            "source_language":"en","target_language":"zh-CN","response_level":"brief"
          })))
          .await
          .unwrap()
          .status()
        })
    })
  })
  .join()
  .unwrap();

  assert_eq!(status, StatusCode::OK);
}
