//! Tests for unified translation orchestration, conservative intent routing, and request disposal.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use transnet::{
  application::translation::{TranslationOrchestrationError, TranslationOrchestrator},
  domain::translation_turn::{
    LexicalMeaningDraft, LexicalTurnDraft, TranslationHistory, TranslationTurn,
    TranslationTurnRequest, TranslationUnit, TurnExample, TurnLanguage, NORMALIZER_VERSION,
    PROJECTION_VERSION, TRANSLATION_RESULT_SCHEMA_VERSION,
  },
  ports::translation_model::{
    ConnectedTextModel, ConnectedTextOutput, ConnectedTextRequest, LexicalDraftModel,
    LexicalDraftOutput, ModelOperationVersions, TranslationModelError,
  },
};

#[derive(Default)]
struct Calls {
  connected: Vec<usize>,
  segments: Vec<String>,
  terminology: Vec<Vec<String>>,
  preceding: Vec<Option<String>>,
  lexical: Vec<(TranslationUnit, usize)>,
}

struct FakeConnected {
  calls: Arc<Mutex<Calls>>,
  outcome: Result<String, TranslationModelError>,
  echo: bool,
  fail_on_call: Option<usize>,
}

#[async_trait]
impl ConnectedTextModel for FakeConnected {
  async fn translate_connected_text(
    &self,
    request: ConnectedTextRequest<'_>,
    _source_language: TurnLanguage,
  ) -> Result<ConnectedTextOutput, TranslationModelError> {
    let mut calls = self.calls.lock().unwrap();
    let call_number = calls.connected.len() + 1;
    calls.connected.push(request.turn.history().len());
    calls.segments.push(request.text.to_string());
    calls.terminology.push(request.terminology.to_vec());
    calls
      .preceding
      .push(request.preceding_translation.map(str::to_string));
    drop(calls);
    if self.fail_on_call == Some(call_number) {
      return Err(TranslationModelError::Unavailable);
    }
    if self.echo {
      return Ok(connected_output(request.text.to_string()));
    }
    self.outcome.clone().map(connected_output)
  }
}

struct FakeLexical {
  calls: Arc<Mutex<Calls>>,
  outcome: Result<LexicalTurnDraft, TranslationModelError>,
}

#[async_trait]
impl LexicalDraftModel for FakeLexical {
  async fn generate_lexical_draft(
    &self,
    turn: &TranslationTurn,
    unit: TranslationUnit,
    _source_language: TurnLanguage,
  ) -> Result<LexicalDraftOutput, TranslationModelError> {
    self
      .calls
      .lock()
      .unwrap()
      .lexical
      .push((unit, turn.history().len()));
    self.outcome.clone().map(|draft| LexicalDraftOutput {
      draft,
      versions: versions("fake-lexical-v1", "lexical-draft-prompt-v1"),
    })
  }
}

fn versions(model: &str, prompt: &'static str) -> ModelOperationVersions {
  ModelOperationVersions {
    model_version: model.to_string(),
    prompt_version: prompt,
  }
}

fn connected_output(translation: String) -> ConnectedTextOutput {
  ConnectedTextOutput {
    translation,
    versions: versions("fake-connected-v1", "connected-text-prompt-v1"),
  }
}

fn lexical_draft() -> LexicalTurnDraft {
  LexicalTurnDraft {
    translations: vec![LexicalMeaningDraft {
      text: "译文".to_string(),
      meaning: "material meaning".to_string(),
      part_of_speech: "noun".to_string(),
      phrase_type: "established expression".to_string(),
      aliases: Vec::new(),
      examples: Vec::new(),
      usage_notes: Vec::new(),
    }],
  }
}

fn turn(text: &str) -> TranslationTurn {
  turn_with_history(text, Vec::new())
}

fn turn_at_level(text: &str, response_level: &str) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    text: text.to_string(),
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: response_level.to_string(),
    history: Vec::new(),
  })
  .unwrap()
}

fn turn_with_history(text: &str, history: Vec<TranslationHistory>) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    text: text.to_string(),
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "full".to_string(),
    history,
  })
  .unwrap()
}

fn orchestrator(
  connected_outcome: Result<String, TranslationModelError>,
  lexical_outcome: Result<LexicalTurnDraft, TranslationModelError>,
) -> (TranslationOrchestrator, Arc<Mutex<Calls>>) {
  let calls = Arc::new(Mutex::new(Calls::default()));
  (
    TranslationOrchestrator::new(
      Arc::new(FakeConnected {
        calls: Arc::clone(&calls),
        outcome: connected_outcome,
        echo: false,
        fail_on_call: None,
      }),
      Arc::new(FakeLexical {
        calls: Arc::clone(&calls),
        outcome: lexical_outcome,
      }),
    ),
    calls,
  )
}

fn chunking_orchestrator(
  fail_on_call: Option<usize>,
) -> (TranslationOrchestrator, Arc<Mutex<Calls>>) {
  let calls = Arc::new(Mutex::new(Calls::default()));
  (
    TranslationOrchestrator::new(
      Arc::new(FakeConnected {
        calls: Arc::clone(&calls),
        outcome: Ok(String::new()),
        echo: true,
        fail_on_call,
      }),
      Arc::new(FakeLexical {
        calls: Arc::clone(&calls),
        outcome: Ok(lexical_draft()),
      }),
    ),
    calls,
  )
}

#[tokio::test]
async fn high_confidence_words_terms_and_phrases_use_only_lexical_model() {
  let (service, calls) = orchestrator(Ok("unused".to_string()), Ok(lexical_draft()));
  let cases = [
    ("hot", TranslationUnit::Word),
    ("C", TranslationUnit::Word),
    ("C++", TranslationUnit::Word),
    ("C#", TranslationUnit::Word),
    ("Node.js", TranslationUnit::Word),
    ("machine learning", TranslationUnit::Phrase),
    ("up in the air", TranslationUnit::Phrase),
    (
      "this_is_one_lexical_identifier_with_a_length_that_is_independent_of_any_provider_threshold",
      TranslationUnit::Word,
    ),
  ];

  for (text, expected) in cases {
    let result = service.translate(&turn(text)).await.unwrap();
    assert_eq!(result.translation.unit, expected);
  }

  let calls = calls.lock().unwrap();
  assert!(calls.connected.is_empty());
  assert_eq!(calls.lexical.len(), cases.len());
}

#[tokio::test]
async fn clauses_sentences_passages_and_ambiguous_fragments_use_only_connected_model() {
  let (service, calls) = orchestrator(Ok("连续译文".to_string()), Ok(lexical_draft()));
  let cases = [
    "Can run",
    "The service is ready.",
    "First paragraph.\n\nSecond paragraph.",
    "This is a deliberately long passage whose structure clearly exceeds one lexical unit and must remain connected text even without relying on provider selection thresholds.",
  ];

  for text in cases {
    let result = service.translate(&turn(text)).await.unwrap();
    assert_eq!(result.translation.unit, TranslationUnit::Passage);
  }

  let calls = calls.lock().unwrap();
  assert!(calls.lexical.is_empty());
  assert_eq!(calls.connected.len(), cases.len());
}

#[tokio::test]
async fn request_history_is_observed_only_on_its_own_call() {
  let (service, calls) = orchestrator(Ok("连续译文".to_string()), Ok(lexical_draft()));
  let history = vec![TranslationHistory {
    source_text: "private prior source".to_string(),
    translated_text: "private prior translation".to_string(),
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
  }];

  service
    .translate(&turn_with_history("This is ready.", history))
    .await
    .unwrap();
  service.translate(&turn("This is fresh.")).await.unwrap();

  assert_eq!(calls.lock().unwrap().connected, vec![1, 0]);
}

#[tokio::test]
async fn model_failures_are_stable_and_redacted() {
  let secret = "private-input-8127";
  let credential = "credential-9931";
  let (unavailable, _) = orchestrator(Err(TranslationModelError::Unavailable), Ok(lexical_draft()));
  let error = unavailable
    .translate(&turn(&format!("{secret}.")))
    .await
    .unwrap_err();
  assert_eq!(error, TranslationOrchestrationError::ModelUnavailable);

  let (invalid, _) = orchestrator(
    Ok("unused".to_string()),
    Err(TranslationModelError::InvalidOutput),
  );
  let error = invalid.translate(&turn("lexeme")).await.unwrap_err();
  assert_eq!(error, TranslationOrchestrationError::InvalidModelOutput);

  let rendered = format!("{error:?} {error}");
  assert!(!rendered.contains(secret));
  assert!(!rendered.contains(credential));
  assert!(!rendered.contains("Gemma"));
}

#[test]
fn callers_cannot_select_provider_model_or_workflow() {
  for forbidden in ["provider", "model", "workflow"] {
    let mut request = serde_json::json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief"
    });
    request[forbidden] = serde_json::json!("caller-choice");
    assert!(serde_json::from_value::<TranslationTurnRequest>(request).is_err());
  }
}

#[tokio::test]
async fn short_connected_text_uses_one_direct_segment_without_a_ledger() {
  let (service, calls) = chunking_orchestrator(None);
  let source = "This sentence remains one direct model operation.";

  let result = service.translate(&turn(source)).await.unwrap();

  assert_eq!(result.translation.translations[0].text, source);
  let calls = calls.lock().unwrap();
  assert_eq!(calls.segments, [source]);
  assert!(calls.terminology[0].is_empty());
  assert_eq!(calls.preceding, [None]);
}

#[tokio::test]
async fn long_text_splits_at_paragraphs_and_reassembles_without_loss_or_reordering() {
  let first = "Database consistency matters. ".repeat(220);
  let second = "Database terminology remains stable. ".repeat(220);
  let source = format!("{first}\n\n{second}");
  let (service, calls) = chunking_orchestrator(None);

  let result = service.translate(&turn(&source)).await.unwrap();

  assert_eq!(result.translation.translations[0].text, source);
  let calls = calls.lock().unwrap();
  assert_eq!(calls.segments.len(), 2);
  assert!(calls
    .segments
    .iter()
    .all(|segment| segment.chars().count() <= 8_192));
  assert!(calls.terminology.iter().all(|ledger| {
    ledger == &calls.terminology[0] && ledger.iter().any(|term| term == "Database")
  }));
  assert_eq!(calls.preceding[0], None);
  assert_eq!(
    calls.preceding[1].as_deref(),
    Some(calls.segments[0].as_str())
  );
}

#[tokio::test]
async fn unbreakable_oversized_text_fails_before_model_work() {
  let source = "x".repeat(8_193);
  let (service, calls) = chunking_orchestrator(None);

  assert_eq!(
    service.translate(&turn(&source)).await.unwrap_err(),
    TranslationOrchestrationError::ChunkPlanLimit
  );
  assert!(calls.lock().unwrap().connected.is_empty());
}

#[tokio::test]
async fn excessive_natural_segments_fail_the_chunk_count_bound_before_model_work() {
  let segment = format!("{} ", "x".repeat(4_097));
  let source = segment.repeat(129);
  let (service, calls) = chunking_orchestrator(None);

  assert_eq!(
    service.translate(&turn(&source)).await.unwrap_err(),
    TranslationOrchestrationError::ChunkPlanLimit
  );
  assert!(calls.lock().unwrap().connected.is_empty());
}

#[tokio::test]
async fn terminology_ledger_is_bounded_and_rebuilt_for_each_request() {
  let terms = (0..80)
    .map(|index| format!("Term_{index:02}"))
    .collect::<Vec<_>>();
  let repeated = terms.join(" ");
  let source = format!("{} {}.\n\n{} {}.", repeated, repeated, repeated, repeated);
  let source = format!("{} {}", source.repeat(20), "closing sentence.");
  let (service, calls) = chunking_orchestrator(None);

  service.translate(&turn(&source)).await.unwrap();
  service
    .translate(&turn(&format!("{}.", "AnotherTerm ".repeat(800))))
    .await
    .unwrap();

  let calls = calls.lock().unwrap();
  assert!(calls.terminology.iter().all(|ledger| ledger.len() <= 64));
  let second_request_ledger = calls.terminology.last().unwrap();
  assert!(!second_request_ledger.iter().any(|term| term == "Term_00"));
}

#[tokio::test]
async fn any_chunk_failure_fails_closed_without_a_partial_result() {
  let source = format!(
    "{}\n\n{}",
    "First repeated terminology sentence. ".repeat(220),
    "Second repeated terminology sentence. ".repeat(220)
  );
  let (service, calls) = chunking_orchestrator(Some(2));

  assert_eq!(
    service.translate(&turn(&source)).await.unwrap_err(),
    TranslationOrchestrationError::ModelUnavailable
  );
  assert_eq!(calls.lock().unwrap().connected.len(), 2);
}

#[tokio::test]
async fn auto_detection_rejects_symbol_only_input_without_calling_a_model() {
  let (service, calls) = orchestrator(Ok("unused".to_string()), Ok(lexical_draft()));
  let turn = TranslationTurn::new(TranslationTurnRequest {
    text: "+++".to_string(),
    source_language: "auto".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "brief".to_string(),
    history: Vec::new(),
  })
  .unwrap();

  assert_eq!(
    service.translate(&turn).await.unwrap_err(),
    TranslationOrchestrationError::UnsupportedSourceLanguage
  );
  let calls = calls.lock().unwrap();
  assert!(calls.connected.is_empty());
  assert!(calls.lexical.is_empty());
}

#[tokio::test]
async fn lexical_levels_project_one_superset_without_changing_core_semantics() {
  let draft = LexicalTurnDraft {
    translations: vec![
      LexicalMeaningDraft {
        text: "热的".to_string(),
        meaning: "having a high temperature".to_string(),
        part_of_speech: "adjective".to_string(),
        phrase_type: "".to_string(),
        aliases: vec!["高温的".to_string()],
        examples: vec![
          TurnExample {
            source_text: "hot tea".to_string(),
            translated_text: "热茶".to_string(),
          },
          TurnExample {
            source_text: "a hot day".to_string(),
            translated_text: "炎热的一天".to_string(),
          },
        ],
        usage_notes: vec![
          "temperature".to_string(),
          "literal use".to_string(),
          "additional full detail".to_string(),
        ],
      },
      LexicalMeaningDraft {
        text: "热门的".to_string(),
        meaning: "currently popular".to_string(),
        part_of_speech: "adjective".to_string(),
        phrase_type: "".to_string(),
        aliases: vec!["流行的".to_string()],
        examples: Vec::new(),
        usage_notes: vec!["figurative use".to_string()],
      },
    ],
  };
  let (service, calls) = orchestrator(Ok("unused".to_string()), Ok(draft));

  let brief = service
    .translate(&turn_at_level("hot", "brief"))
    .await
    .unwrap();
  let standard = service
    .translate(&turn_at_level("hot", "standard"))
    .await
    .unwrap();
  let full = service
    .translate(&turn_at_level("hot", "full"))
    .await
    .unwrap();

  let core = |result: &transnet::domain::translation_turn::ProjectedTranslationResult| {
    result
      .translation
      .translations
      .iter()
      .map(|item| (item.text.clone(), item.meaning.clone()))
      .collect::<Vec<_>>()
  };
  assert_eq!(core(&brief), core(&standard));
  assert_eq!(core(&standard), core(&full));
  assert_eq!(core(&full).len(), 2);
  assert!(brief
    .translation
    .translations
    .iter()
    .all(|item| item.details.is_none()));
  let standard_details = standard.translation.translations[0]
    .details
    .as_ref()
    .unwrap();
  assert!(standard_details.aliases.is_empty());
  assert_eq!(standard_details.examples.len(), 1);
  assert_eq!(standard_details.usage_notes.len(), 2);
  let full_details = full.translation.translations[0].details.as_ref().unwrap();
  assert_eq!(full_details.aliases.len(), 1);
  assert_eq!(full_details.examples.len(), 2);
  assert_eq!(full_details.usage_notes.len(), 3);
  assert_eq!(calls.lock().unwrap().lexical.len(), 3);
}

#[tokio::test]
async fn projected_metadata_reports_only_components_that_participated() {
  let (service, _) = orchestrator(Ok("连续译文".to_string()), Ok(lexical_draft()));
  let lexical = service
    .translate(&turn_at_level("hot", "full"))
    .await
    .unwrap();
  let passage = service
    .translate(&turn_at_level("This is ready.", "brief"))
    .await
    .unwrap();

  for result in [&lexical, &passage] {
    assert_eq!(
      result.metadata.schema_version,
      TRANSLATION_RESULT_SCHEMA_VERSION
    );
    assert_eq!(result.metadata.normalizer_version, NORMALIZER_VERSION);
    assert_eq!(result.metadata.projection_version, PROJECTION_VERSION);
    assert_eq!(result.metadata.retrieval_version, None);
    assert_eq!(result.metadata.content_release, None);
    let json = serde_json::to_value(result).unwrap();
    assert!(json["metadata"].get("retrieval_version").is_none());
    assert!(json["metadata"].get("content_release").is_none());
  }
  assert_eq!(lexical.metadata.model_versions, ["fake-lexical-v1"]);
  assert_eq!(
    lexical.metadata.prompt_versions,
    ["lexical-draft-prompt-v1"]
  );
  assert_eq!(passage.metadata.model_versions, ["fake-connected-v1"]);
  assert_eq!(
    passage.metadata.prompt_versions,
    ["connected-text-prompt-v1"]
  );
  assert_eq!(passage.translation.translations[0].text, "连续译文");
}

#[tokio::test]
async fn passage_levels_preserve_the_same_translation_without_extra_calls() {
  let (service, calls) = orchestrator(Ok("同一段落译文".to_string()), Ok(lexical_draft()));
  let mut translations = Vec::new();

  for level in ["brief", "standard", "full"] {
    let result = service
      .translate(&turn_at_level("This is a complete sentence.", level))
      .await
      .unwrap();
    assert_eq!(result.translation.unit, TranslationUnit::Passage);
    assert_eq!(result.translation.translations.len(), 1);
    assert!(result.translation.translations[0].details.is_none());
    translations.push(result.translation.translations[0].text.clone());
  }

  assert_eq!(translations, ["同一段落译文"; 3]);
  assert_eq!(calls.lock().unwrap().connected.len(), 3);
}

#[tokio::test]
async fn outcome_debug_and_metadata_do_not_expose_request_content() {
  let secret = "projection-secret-6019";
  let (service, _) = orchestrator(Ok("安全译文".to_string()), Ok(lexical_draft()));
  let outcome = service
    .translate(&turn_at_level(
      &format!("This contains {secret}."),
      "standard",
    ))
    .await
    .unwrap();

  let rendered = format!("{outcome:?} {:?}", outcome.metadata);
  assert!(!rendered.contains(secret));
  assert!(!rendered.contains("安全译文"));
  assert!(!rendered.contains("credential"));
}
