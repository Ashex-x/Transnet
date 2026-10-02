//! Tests for provider-neutral translation orchestration and request-local call policy.

use std::{
  collections::VecDeque,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
  time::Duration,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use time::OffsetDateTime;
use transnet::{
  application::translation::{
    TranslationOrchestrationError, TranslationOrchestrator, MAX_CONNECTED_CHUNK_CHARS,
    MAX_PARALLEL_GENERATIONS,
  },
  domain::translation_turn::{
    GuidanceAudience, GuidancePurpose, GuidanceRegister, TerminologyConstraint, TerminologyPolicy,
    TranslationGuidance, TranslationHistory, TranslationInput, TranslationResultKind,
    TranslationSegment, TranslationTurn, TranslationTurnRequest,
  },
  CancellationSignal, GenerationInput, GenerationOutput, GenerationPort, GenerationProfile,
  GenerationRequest, GenerationResponse, ModelOperationContext, ModelOperationError, ModelVersion,
  RequestContext, RequestId,
};

#[derive(Clone)]
struct RecordedCall {
  profile: GenerationProfile,
  input: String,
  deadline: OffsetDateTime,
  image_count: usize,
  images: Vec<Vec<u8>>,
}

struct FakeGeneration {
  outcomes: Mutex<VecDeque<Result<String, ModelOperationError>>>,
  calls: Mutex<Vec<RecordedCall>>,
  in_flight: AtomicUsize,
  peak: AtomicUsize,
  delay: Duration,
}

struct VersionedChunkGeneration;

struct SegmentGeneration {
  invalidate_fast: bool,
  reasoning_calls: AtomicUsize,
}

#[async_trait]
impl GenerationPort for SegmentGeneration {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    let encoded = request.input.as_str();
    let prompt: serde_json::Value =
      serde_json::from_str(encoded.split_once('\n').map_or(encoded, |(json, _)| json)).unwrap();
    let index = prompt["segment_index"].as_u64().unwrap_or(0);
    let output = match request.profile {
      GenerationProfile::Reasoning => {
        self.reasoning_calls.fetch_add(1, Ordering::AcqRel);
        connected("修复 **{name}**")
      }
      GenerationProfile::Fast if self.invalidate_fast => "invalid".into(),
      GenerationProfile::Fast if index == 0 => connected("发布 **{name}**"),
      GenerationProfile::Fast => connected("正文"),
    };
    Ok(GenerationResponse {
      output: GenerationOutput::new(output).unwrap(),
      model_version: ModelVersion::new("segment-test-v1").unwrap(),
      prompt_version: request.prompt_version,
    })
  }
}

#[async_trait]
impl GenerationPort for VersionedChunkGeneration {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    let (output, model) = match request.profile {
      GenerationProfile::Reasoning => (connected("修复一"), "repair-model"),
      GenerationProfile::Fast => {
        let prompt: serde_json::Value = serde_json::from_str(request.input.as_str()).unwrap();
        match prompt["chunk_index"].as_u64().unwrap() {
          0 => ("invalid-first".to_string(), "fast-model-0"),
          1 => (connected("二"), "fast-model-1"),
          other => panic!("unexpected chunk index {other}"),
        }
      }
    };
    Ok(GenerationResponse {
      output: GenerationOutput::new(output).unwrap(),
      model_version: ModelVersion::new(model).unwrap(),
      prompt_version: request.prompt_version,
    })
  }
}

impl FakeGeneration {
  fn new(outcomes: impl IntoIterator<Item = Result<String, ModelOperationError>>) -> Self {
    Self {
      outcomes: Mutex::new(outcomes.into_iter().collect()),
      calls: Mutex::new(Vec::new()),
      in_flight: AtomicUsize::new(0),
      peak: AtomicUsize::new(0),
      delay: Duration::ZERO,
    }
  }

  fn delayed(
    outcomes: impl IntoIterator<Item = Result<String, ModelOperationError>>,
    delay: Duration,
  ) -> Self {
    Self {
      delay,
      ..Self::new(outcomes)
    }
  }

  fn calls(&self) -> Vec<RecordedCall> {
    self.calls.lock().unwrap().clone()
  }
}

#[async_trait]
impl GenerationPort for FakeGeneration {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    self.calls.lock().unwrap().push(RecordedCall {
      profile: request.profile,
      input: request.input.as_str().to_string(),
      deadline: context.request.deadline_at(),
      image_count: request.images.len(),
      images: request
        .images
        .iter()
        .map(|image| image.bytes().to_vec())
        .collect(),
    });
    let current = self.in_flight.fetch_add(1, Ordering::AcqRel) + 1;
    self.peak.fetch_max(current, Ordering::AcqRel);
    if !self.delay.is_zero() {
      tokio::select! {
        _ = context.cancellation.cancelled() => {
          self.in_flight.fetch_sub(1, Ordering::AcqRel);
          return Err(ModelOperationError::Cancelled);
        }
        _ = tokio::time::sleep(self.delay) => {}
      }
    }
    self.in_flight.fetch_sub(1, Ordering::AcqRel);
    let output = self
      .outcomes
      .lock()
      .unwrap()
      .pop_front()
      .unwrap_or_else(|| Ok(connected("default")))?;
    Ok(GenerationResponse {
      output: GenerationOutput::new(output).unwrap(),
      model_version: ModelVersion::new("gemma4-test-r1").unwrap(),
      prompt_version: request.prompt_version,
    })
  }
}

fn context(seconds: i64) -> RequestContext {
  let now = OffsetDateTime::now_utc();
  let now = now
    .replace_nanosecond(now.nanosecond() / 1_000 * 1_000)
    .unwrap();
  RequestContext::new(
    RequestId::new("translation-orchestration-test").unwrap(),
    now + time::Duration::seconds(seconds),
    "transnet-service-v1",
    None,
  )
  .unwrap()
}

fn turn(text: &str) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Text {
      text: text.to_string(),
    },
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "full".to_string(),
    history: Vec::new(),
    guidance: None,
  })
  .unwrap()
}

fn same_language_turn(text: &str) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Text {
      text: text.to_string(),
    },
    source_language: "auto".to_string(),
    target_language: "en".to_string(),
    response_level: "full".to_string(),
    history: Vec::new(),
    guidance: None,
  })
  .unwrap()
}

fn image_turn() -> TranslationTurn {
  let request: TranslationTurnRequest = serde_json::from_value(serde_json::json!({
    "input":{"type":"image_regions","images":[{"image_id":"page",
      "media_type":"image/png","data":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      "regions":[{"region_id":"title","x":0.0,"y":0.0,"width":1.0,"height":1.0}]}],
      "reading_order":["page:title"]},
    "source_language":"auto","target_language":"zh-CN","response_level":"standard"
  }))
  .unwrap();
  TranslationTurn::new(request).unwrap()
}

fn split_color_image_turn() -> TranslationTurn {
  let mut source = image::RgbaImage::new(2, 1);
  source.put_pixel(0, 0, image::Rgba([12, 34, 56, 255]));
  source.put_pixel(1, 0, image::Rgba([250, 1, 2, 255]));
  let mut bytes = std::io::Cursor::new(Vec::new());
  image::DynamicImage::ImageRgba8(source)
    .write_to(&mut bytes, image::ImageFormat::Png)
    .unwrap();
  let request = serde_json::json!({
    "input":{"type":"image_regions","images":[{"image_id":"page",
      "media_type":"image/png","data":BASE64.encode(bytes.into_inner()),
      "regions":[{"region_id":"left","x":0.0,"y":0.0,"width":0.5,"height":1.0}]}],
      "reading_order":["page:left"]},
    "source_language":"auto","target_language":"zh-CN","response_level":"standard"
  });
  TranslationTurn::new(serde_json::from_value(request).unwrap()).unwrap()
}

fn guided_turn(text: &str) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Text {
      text: text.to_string(),
    },
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "full".to_string(),
    history: Vec::new(),
    guidance: Some(TranslationGuidance {
      purpose: Some(GuidancePurpose::Technical),
      audience: Some(GuidanceAudience::Specialist),
      register: Some(GuidanceRegister::Formal),
      terminology: vec![TerminologyConstraint {
        source: "torque".to_string(),
        target: "扭矩".to_string(),
        policy: TerminologyPolicy::Required,
      }],
      ..TranslationGuidance::default()
    }),
  })
  .unwrap()
}

fn segment_turn() -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Segments {
      segments: vec![
        TranslationSegment {
          segment_id: "title".into(),
          text: "Launch **{name}**".into(),
          role: transnet::domain::translation_turn::SegmentRole::Title,
          format: transnet::domain::translation_turn::SegmentFormat::Markdown,
          protected_ranges: vec![transnet::domain::translation_turn::ProtectedRange {
            start: 9,
            end: 15,
          }],
        },
        TranslationSegment {
          segment_id: "body".into(),
          text: "Body".into(),
          role: transnet::domain::translation_turn::SegmentRole::Paragraph,
          format: transnet::domain::translation_turn::SegmentFormat::Plain,
          protected_ranges: Vec::new(),
        },
      ],
    },
    source_language: "en".into(),
    target_language: "zh-CN".into(),
    response_level: "standard".into(),
    history: Vec::new(),
    guidance: None,
  })
  .unwrap()
}

fn connected(value: &str) -> String {
  serde_json::json!({"status":"complete", "translation":value}).to_string()
}

fn lexical(value: &str) -> String {
  serde_json::json!({"status":"complete", "translations":[{"text":value,
    "meaning":"material meaning", "part_of_speech":"noun",
    "phrase_type":"established expression", "aliases":[], "examples":[],
    "usage_notes":[]}]})
  .to_string()
}

#[tokio::test]
async fn routing_uses_one_fast_profile_and_no_provider_selector() {
  let fake = Arc::new(FakeGeneration::new([
    Ok(lexical("译文")),
    Ok(connected("句子译文")),
  ]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let cancellation = Arc::new(CancellationSignal::default());
  let request = context(30);
  let word = orchestrator
    .translate(&request, cancellation.clone(), &turn("hot"))
    .await
    .unwrap();
  let sentence = orchestrator
    .translate(&request, cancellation, &turn("The service is ready."))
    .await
    .unwrap();
  assert_eq!(word.translation.kind(), TranslationResultKind::Word);
  assert_eq!(sentence.translation.kind(), TranslationResultKind::Passage);
  assert!(fake
    .calls()
    .iter()
    .all(|call| call.profile == GenerationProfile::Fast));
  assert!(fake.calls()[0].input.contains("lexical_translation"));
  assert!(fake.calls()[1].input.contains("connected_translation"));
  assert!(!fake.calls()[0].input.contains("provider"));
}

#[tokio::test]
async fn same_language_text_is_returned_without_generation() {
  let fake = Arc::new(FakeGeneration::new([]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let result = orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &same_language_turn("hot"),
    )
    .await
    .unwrap();
  let value = serde_json::to_value(result).unwrap();
  assert_eq!(value["translation"]["translations"][0]["text"], "hot");
  assert_eq!(value["translation"]["detected_source_language"], "en");
  assert_eq!(
    value["metadata"]["inference_profiles"],
    serde_json::json!([])
  );
  assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn text_guidance_is_executed_and_checked() {
  let fake = Arc::new(FakeGeneration::new([Ok(lexical("扭矩"))]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let result = orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &guided_turn("torque"),
    )
    .await
    .unwrap();
  assert_eq!(result.translation.kind(), TranslationResultKind::Word);
  assert_eq!(fake.calls().len(), 1);
  assert!(fake.calls()[0].input.contains("technical"));
}

#[tokio::test]
async fn terminology_violation_uses_only_one_repair_then_fails() {
  let fake = Arc::new(FakeGeneration::new([
    Ok(lexical("错误")),
    Ok(lexical("仍然错误")),
  ]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  assert_eq!(
    orchestrator
      .translate(
        &context(30),
        Arc::new(CancellationSignal::default()),
        &guided_turn("torque"),
      )
      .await
      .unwrap_err(),
    TranslationOrchestrationError::GuidanceViolation
  );
  assert_eq!(fake.calls().len(), 2);
  assert_eq!(fake.calls()[1].profile, GenerationProfile::Reasoning);
}

#[tokio::test]
async fn segments_preserve_request_order_ids_protected_content_and_format() {
  let generation = Arc::new(SegmentGeneration {
    invalidate_fast: false,
    reasoning_calls: AtomicUsize::new(0),
  });
  let orchestrator = TranslationOrchestrator::new(generation);
  let turn = segment_turn();
  let result = orchestrator
    .translate(&context(30), Arc::new(CancellationSignal::default()), &turn)
    .await
    .unwrap();
  result.validate_for_turn(&turn).unwrap();
  let encoded = serde_json::to_value(result.translation).unwrap();
  assert_eq!(encoded["unit"], "segment");
  assert_eq!(encoded["segments"][0]["segment_id"], "title");
  assert_eq!(encoded["segments"][0]["order"], 0);
  assert_eq!(
    encoded["segments"][0]["translations"][0]["text"],
    "发布 **{name}**"
  );
  assert_eq!(encoded["segments"][1]["segment_id"], "body");
  assert_eq!(encoded["segments"][1]["order"], 1);
  assert_eq!(encoded["segments"][1]["translations"][0]["text"], "正文");
}

#[tokio::test]
async fn segment_guidance_is_prompted_enforced_and_bound_to_the_result() {
  let generation = Arc::new(FakeGeneration::new([Ok(connected("Measure torque."))]));
  let orchestrator = TranslationOrchestrator::new(generation.clone());
  let turn = TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Segments {
      segments: vec![TranslationSegment {
        segment_id: "technical".into(),
        text: "测量扭矩值".into(),
        role: transnet::domain::translation_turn::SegmentRole::Paragraph,
        format: transnet::domain::translation_turn::SegmentFormat::Plain,
        protected_ranges: Vec::new(),
      }],
    },
    source_language: "zh-CN".into(),
    target_language: "en".into(),
    response_level: "standard".into(),
    history: Vec::new(),
    guidance: Some(TranslationGuidance {
      purpose: Some(GuidancePurpose::Technical),
      terminology: vec![TerminologyConstraint {
        source: "扭矩".into(),
        target: "torque".into(),
        policy: TerminologyPolicy::Required,
      }],
      ..TranslationGuidance::default()
    }),
  })
  .unwrap();

  let result = orchestrator
    .translate(&context(30), Arc::new(CancellationSignal::default()), &turn)
    .await
    .unwrap();
  result.validate_for_turn(&turn).unwrap();
  let encoded = serde_json::to_value(result.translation).unwrap();
  assert_eq!(encoded["terminology_decisions"][0]["source"], "扭矩");
  assert_eq!(encoded["terminology_decisions"][0]["target"], "torque");
  let prompt: serde_json::Value = serde_json::from_str(&generation.calls()[0].input).unwrap();
  assert_eq!(prompt["guidance"]["purpose"], "technical");
  assert_eq!(prompt["guidance"]["terminology"][0]["source"], "扭矩");
}

#[tokio::test]
async fn segment_terminology_violation_uses_only_one_repair_then_fails_closed() {
  let generation = Arc::new(FakeGeneration::new([
    Ok(connected("错误")),
    Ok(connected("仍然错误")),
  ]));
  let orchestrator = TranslationOrchestrator::new(generation.clone());
  let mut request: TranslationTurnRequest = serde_json::from_value(serde_json::json!({
    "input":{"type":"segments","segments":[{"segment_id":"s","text":"torque",
      "role":"paragraph","format":"plain"}]},
    "source_language":"en","target_language":"zh-CN","response_level":"standard"
  }))
  .unwrap();
  request.guidance = Some(TranslationGuidance {
    terminology: vec![TerminologyConstraint {
      source: "torque".into(),
      target: "扭矩".into(),
      policy: TerminologyPolicy::Required,
    }],
    ..TranslationGuidance::default()
  });
  let turn = TranslationTurn::new(request).unwrap();
  assert_eq!(
    orchestrator
      .translate(&context(30), Arc::new(CancellationSignal::default()), &turn)
      .await
      .unwrap_err(),
    TranslationOrchestrationError::GuidanceViolation
  );
  assert_eq!(generation.calls().len(), 2);
}

#[tokio::test]
async fn segment_postcondition_failures_share_one_reasoning_repair() {
  let generation = Arc::new(SegmentGeneration {
    invalidate_fast: true,
    reasoning_calls: AtomicUsize::new(0),
  });
  let orchestrator = TranslationOrchestrator::new(generation.clone());
  assert_eq!(
    orchestrator
      .translate(
        &context(30),
        Arc::new(CancellationSignal::default()),
        &segment_turn(),
      )
      .await
      .unwrap_err(),
    TranslationOrchestrationError::InvalidModelOutput
  );
  assert_eq!(generation.reasoning_calls.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn segment_deadline_and_cancellation_propagate_without_partial_results() {
  let expired = Arc::new(FakeGeneration::new([
    Ok(connected("unused")),
    Ok(connected("unused")),
  ]));
  let orchestrator = TranslationOrchestrator::new(expired.clone());
  assert_eq!(
    orchestrator
      .translate(
        &context(-1),
        Arc::new(CancellationSignal::default()),
        &segment_turn(),
      )
      .await
      .unwrap_err(),
    TranslationOrchestrationError::DeadlineExceeded
  );
  assert!(expired.calls().is_empty());

  let delayed = Arc::new(FakeGeneration::delayed(
    [Ok(connected("unused")), Ok(connected("unused"))],
    Duration::from_secs(2),
  ));
  let orchestrator = TranslationOrchestrator::new(delayed);
  let cancellation = Arc::new(CancellationSignal::default());
  let cancel = cancellation.clone();
  let context = context(30);
  let turn = segment_turn();
  let operation = orchestrator.translate(&context, cancellation, &turn);
  tokio::pin!(operation);
  tokio::select! {
    result = &mut operation => panic!("segment operation completed unexpectedly: {result:?}"),
    _ = tokio::time::sleep(Duration::from_millis(20)) => cancel.cancel(),
  }
  assert_eq!(
    operation.await.unwrap_err(),
    TranslationOrchestrationError::Cancelled
  );
}

#[tokio::test]
async fn long_chunks_run_in_bounded_parallel_and_reassemble_in_source_order() {
  let source = format!(
    "{} {} {} {}",
    "a".repeat(MAX_CONNECTED_CHUNK_CHARS - 20),
    "b".repeat(MAX_CONNECTED_CHUNK_CHARS - 20),
    "c".repeat(MAX_CONNECTED_CHUNK_CHARS - 20),
    "d".repeat(MAX_CONNECTED_CHUNK_CHARS - 20)
  );
  let fake = Arc::new(FakeGeneration::delayed(
    [
      Ok(connected("一")),
      Ok(connected("二")),
      Ok(connected("三")),
      Ok(connected("四")),
    ],
    Duration::from_millis(25),
  ));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let result = orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &turn(&source),
    )
    .await
    .unwrap();
  assert_eq!(result.translation.translations()[0].text, "一 二 三 四");
  assert!(fake.peak.load(Ordering::Acquire) > 1);
  assert!(fake.peak.load(Ordering::Acquire) <= MAX_PARALLEL_GENERATIONS);
}

#[tokio::test]
async fn repeated_terms_form_one_request_local_ledger_and_are_not_retained() {
  let secret = "RequestLocalTerm991";
  let source = format!(
    "{} {secret}. {} {secret}.",
    "x".repeat(MAX_CONNECTED_CHUNK_CHARS - 40),
    "y".repeat(MAX_CONNECTED_CHUNK_CHARS - 40)
  );
  let fake = Arc::new(FakeGeneration::new([
    Ok(connected("甲")),
    Ok(connected("乙")),
  ]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &turn(&source),
    )
    .await
    .unwrap();
  assert!(fake.calls().iter().all(|call| call.input.contains(secret)));
  assert!(!format!("{orchestrator:?}").contains(secret));
}

#[tokio::test]
async fn first_invalid_output_gets_the_only_reasoning_repair() {
  let fake = Arc::new(FakeGeneration::new([
    Ok("not-json".to_string()),
    Ok(lexical("修复译文")),
  ]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let result = orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &turn("hot"),
    )
    .await
    .unwrap();
  let calls = fake.calls();
  assert_eq!(calls.len(), 2);
  assert_eq!(calls[0].profile, GenerationProfile::Fast);
  assert_eq!(calls[1].profile, GenerationProfile::Reasoning);
  assert!(calls[1].input.contains("Never return analysis"));
  assert!(calls[1].input.contains("do not include analysis"));
  assert!(result.metadata.reasoning_escalated);
  assert_eq!(
    result.metadata.inference_profiles,
    [GenerationProfile::Fast, GenerationProfile::Reasoning]
  );
}

#[tokio::test]
async fn two_invalid_chunks_never_consume_two_reasoning_calls() {
  let source = format!(
    "{}. {}.",
    "a".repeat(MAX_CONNECTED_CHUNK_CHARS - 2),
    "b".repeat(MAX_CONNECTED_CHUNK_CHARS - 2)
  );
  let fake = Arc::new(FakeGeneration::new([
    Ok("invalid-one".to_string()),
    Ok("invalid-two".to_string()),
    Ok(connected("修复一")),
  ]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  assert_eq!(
    orchestrator
      .translate(
        &context(30),
        Arc::new(CancellationSignal::default()),
        &turn(&source)
      )
      .await
      .unwrap_err(),
    TranslationOrchestrationError::InvalidModelOutput
  );
  assert_eq!(
    fake
      .calls()
      .iter()
      .filter(|call| call.profile == GenerationProfile::Reasoning)
      .count(),
    1
  );
}

#[tokio::test]
async fn all_parallel_fast_versions_precede_later_reasoning_repair() {
  let source = format!(
    "{}. {}.",
    "a".repeat(MAX_CONNECTED_CHUNK_CHARS - 2),
    "b".repeat(MAX_CONNECTED_CHUNK_CHARS - 2)
  );
  let orchestrator = TranslationOrchestrator::new(Arc::new(VersionedChunkGeneration));

  let result = orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &turn(&source),
    )
    .await
    .unwrap();

  assert_eq!(result.translation.translations()[0].text, "修复一 二");
  assert_eq!(
    result.metadata.model_versions,
    ["fast-model-0", "fast-model-1", "repair-model"]
  );
  assert_eq!(
    result.metadata.inference_profiles,
    [GenerationProfile::Fast, GenerationProfile::Reasoning]
  );
}

#[tokio::test]
async fn deadline_and_cancellation_propagate_without_content_in_errors() {
  let fake = Arc::new(FakeGeneration::delayed(
    [Ok(connected("unused"))],
    Duration::from_secs(2),
  ));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  assert_eq!(
    orchestrator
      .translate(
        &context(-1),
        Arc::new(CancellationSignal::default()),
        &turn("deadline-secret")
      )
      .await
      .unwrap_err(),
    TranslationOrchestrationError::DeadlineExceeded
  );
  let cancellation = Arc::new(CancellationSignal::default());
  let cancel_for_task = cancellation.clone();
  let request = context(30);
  let input = turn("cancellation-secret sentence.");
  let operation = orchestrator.translate(&request, cancellation, &input);
  tokio::pin!(operation);
  tokio::select! {
    result = &mut operation => panic!("operation completed unexpectedly: {result:?}"),
    _ = tokio::time::sleep(Duration::from_millis(20)) => cancel_for_task.cancel(),
  }
  let error = operation.await.unwrap_err();
  assert_eq!(error, TranslationOrchestrationError::Cancelled);
  assert!(!format!("{error:?} {error}").contains("secret"));
  assert!(fake
    .calls()
    .iter()
    .all(|call| call.deadline > OffsetDateTime::now_utc()));
}

#[test]
fn generation_input_debug_is_content_free() {
  let secret = "generation-secret-114";
  let input = GenerationInput::new(secret).unwrap();
  assert!(!format!("{input:?}").contains(secret));
}

#[test]
fn history_shape_remains_request_local() {
  let request = TranslationTurn::new(TranslationTurnRequest {
    input: TranslationInput::Text {
      text: "This is ready.".into(),
    },
    source_language: "en".into(),
    target_language: "zh-CN".into(),
    response_level: "brief".into(),
    history: vec![TranslationHistory {
      source_text: "prior source".into(),
      translated_text: "先前译文".into(),
      source_language: "en".into(),
      target_language: "zh-CN".into(),
    }],
    guidance: None,
  })
  .unwrap();
  assert_eq!(request.history().len(), 1);
}

#[tokio::test]
async fn image_regions_use_one_bounded_vlm_call_and_preserve_reading_order() {
  let fake = Arc::new(FakeGeneration::new([Ok(
    serde_json::json!({
      "regions":[{"image_id":"page","region_id":"title",
        "detected_source_language":"en","translation":"标题"}]
    })
    .to_string(),
  )]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  let result = orchestrator
    .translate(
      &context(5),
      Arc::new(CancellationSignal::default()),
      &image_turn(),
    )
    .await
    .unwrap();
  assert_eq!(
    result.translation.kind(),
    TranslationResultKind::ImageRegion
  );
  let value = serde_json::to_value(result.translation).unwrap();
  assert_eq!(value["regions"][0]["image_id"], "page");
  assert_eq!(value["regions"][0]["region_id"], "title");
  assert_eq!(value["regions"][0]["order"], 0);
  assert_eq!(value["regions"][0]["translations"][0]["text"], "标题");
  let calls = fake.calls();
  assert_eq!(calls.len(), 1);
  assert_eq!(calls[0].image_count, 1);
  assert!(!calls[0].input.contains("iVBOR"));
  let crop = image::load_from_memory(&calls[0].images[0]).unwrap();
  assert_eq!((crop.width(), crop.height()), (1, 1));
}

#[tokio::test]
async fn image_region_attachment_excludes_pixels_outside_the_declared_rectangle() {
  let fake = Arc::new(FakeGeneration::new([Ok(
    serde_json::json!({"regions":[{"image_id":"page","region_id":"left",
      "detected_source_language":"en","translation":"安全"}]})
    .to_string(),
  )]));
  TranslationOrchestrator::new(fake.clone())
    .translate(
      &context(5),
      Arc::new(CancellationSignal::default()),
      &split_color_image_turn(),
    )
    .await
    .unwrap();
  let calls = fake.calls();
  let crop = image::load_from_memory(&calls[0].images[0])
    .unwrap()
    .to_rgba8();
  assert_eq!((crop.width(), crop.height()), (1, 1));
  assert_eq!(crop.get_pixel(0, 0).0, [12, 34, 56, 255]);
  assert!(!calls[0].input.contains("250"));
}

#[tokio::test]
async fn image_guidance_is_rejected_before_pixels_reach_the_model() {
  let fake = Arc::new(FakeGeneration::new([Ok("unused".into())]));
  let request = serde_json::json!({
    "input":{"type":"image_regions","images":[{"image_id":"page",
      "media_type":"image/png","data":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      "regions":[{"region_id":"title","x":0.0,"y":0.0,"width":1.0,"height":1.0}]}],
      "reading_order":["page:title"]},
    "source_language":"auto","target_language":"zh-CN","response_level":"standard",
    "guidance":{"register":"formal"}
  });
  let turn = TranslationTurn::new(serde_json::from_value(request).unwrap()).unwrap();
  assert_eq!(
    TranslationOrchestrator::new(fake.clone())
      .translate(&context(5), Arc::new(CancellationSignal::default()), &turn,)
      .await
      .unwrap_err(),
    TranslationOrchestrationError::UnsupportedImageGuidance
  );
  assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn image_output_identity_drift_and_cancellation_fail_content_free() {
  let fake = Arc::new(FakeGeneration::new([Ok(
    serde_json::json!({
      "regions":[{"image_id":"page","region_id":"wrong",
        "detected_source_language":"en","translation":"private-output"}]
    })
    .to_string(),
  )]));
  let error = TranslationOrchestrator::new(fake)
    .translate(
      &context(5),
      Arc::new(CancellationSignal::default()),
      &image_turn(),
    )
    .await
    .unwrap_err();
  assert_eq!(error, TranslationOrchestrationError::InvalidModelOutput);
  assert!(!format!("{error:?} {error}").contains("private-output"));

  let fake = Arc::new(FakeGeneration::delayed(
    [Ok("unused".into())],
    Duration::from_secs(1),
  ));
  let cancellation = Arc::new(CancellationSignal::default());
  let cancel = cancellation.clone();
  let orchestrator = TranslationOrchestrator::new(fake);
  let request_context = context(5);
  let turn = image_turn();
  let operation = orchestrator.translate(&request_context, cancellation, &turn);
  tokio::pin!(operation);
  tokio::select! {
    result = &mut operation => panic!("image operation completed early: {result:?}"),
    _ = tokio::time::sleep(Duration::from_millis(20)) => cancel.cancel(),
  }
  assert_eq!(
    operation.await.unwrap_err(),
    TranslationOrchestrationError::Cancelled
  );
}
