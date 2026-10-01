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
use time::OffsetDateTime;
use transnet::{
  application::translation::{
    TranslationOrchestrationError, TranslationOrchestrator, MAX_CONNECTED_CHUNK_CHARS,
    MAX_PARALLEL_GENERATIONS,
  },
  domain::translation_turn::{
    GuidanceAudience, GuidancePurpose, GuidanceRegister, TerminologyConstraint, TerminologyPolicy,
    TranslationGuidance, TranslationHistory, TranslationTurn, TranslationTurnRequest,
    TranslationUnit,
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
}

struct FakeGeneration {
  outcomes: Mutex<VecDeque<Result<String, ModelOperationError>>>,
  calls: Mutex<Vec<RecordedCall>>,
  in_flight: AtomicUsize,
  peak: AtomicUsize,
  delay: Duration,
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
    text: Some(text.to_string()),
    input: None,
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "full".to_string(),
    history: Vec::new(),
    guidance: None,
  })
  .unwrap()
}

fn guided_turn(text: &str) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    text: Some(text.to_string()),
    input: None,
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
  assert_eq!(word.translation.unit, TranslationUnit::Word);
  assert_eq!(sentence.translation.unit, TranslationUnit::Passage);
  assert!(fake
    .calls()
    .iter()
    .all(|call| call.profile == GenerationProfile::Fast));
  assert!(fake.calls()[0].input.contains("lexical_translation"));
  assert!(fake.calls()[1].input.contains("connected_translation"));
  assert!(!fake.calls()[0].input.contains("provider"));
}

#[tokio::test]
async fn validated_guidance_is_present_in_the_request_local_prompt() {
  let fake = Arc::new(FakeGeneration::new([Ok(lexical("扭矩"))]));
  let orchestrator = TranslationOrchestrator::new(fake.clone());
  orchestrator
    .translate(
      &context(30),
      Arc::new(CancellationSignal::default()),
      &guided_turn("torque"),
    )
    .await
    .unwrap();

  let prompt: serde_json::Value = serde_json::from_str(&fake.calls()[0].input).unwrap();
  assert_eq!(prompt["guidance"]["purpose"], "technical");
  assert_eq!(prompt["guidance"]["audience"], "specialist");
  assert_eq!(prompt["guidance"]["register"], "formal");
  assert_eq!(prompt["guidance"]["terminology"][0]["target"], "扭矩");
  assert_eq!(prompt["guidance"]["terminology"][0]["policy"], "required");
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
  assert_eq!(result.translation.translations[0].text, "一 二 三 四");
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
    text: Some("This is ready.".into()),
    input: None,
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
