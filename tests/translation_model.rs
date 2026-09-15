//! Contract tests for application-facing translation model ports and provider adapters.

use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
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
  adapters::learning_model::OpenAiLearningModel,
  domain::translation_turn::{
    TranslationTurn, TranslationTurnRequest, TranslationUnit, TurnLanguage,
  },
  ports::translation_model::{ConnectedTextModel, LexicalDraftModel, TranslationModelError},
  ProviderConfig, TranslationConfig, TranslationService,
};

#[derive(Clone)]
struct MockState {
  replies: Arc<Mutex<VecDeque<Reply>>>,
  bodies: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
enum Reply {
  Content(String),
  Status(StatusCode),
}

async fn completion(State(state): State<MockState>, Json(body): Json<Value>) -> Response {
  state.bodies.lock().unwrap().push(body);
  match state.replies.lock().unwrap().pop_front().unwrap() {
    Reply::Content(content) => {
      Json(json!({"choices": [{"message": {"content": content}}]})).into_response()
    }
    Reply::Status(status) => status.into_response(),
  }
}

async fn provider(replies: Vec<Reply>, credential: &str) -> (ProviderConfig, MockState) {
  let state = MockState {
    replies: Arc::new(Mutex::new(replies.into())),
    bodies: Arc::new(Mutex::new(Vec::new())),
  };
  let app = Router::new()
    .route("/v1/chat/completions", post(completion))
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
  (
    ProviderConfig {
      base_url: format!("http://{address}/v1"),
      model: "private-model-name".to_string(),
      api_key: credential.into(),
    },
    state,
  )
}

fn settings(boundary: usize) -> TranslationConfig {
  TranslationConfig {
    long_text_chars: boundary,
    timeout_seconds: 2,
    max_retries: 0,
    retry_delay_ms: 0,
  }
}

fn turn(text: String) -> TranslationTurn {
  TranslationTurn::new(TranslationTurnRequest {
    text,
    source_language: "en".to_string(),
    target_language: "zh-CN".to_string(),
    response_level: "standard".to_string(),
    history: Vec::new(),
  })
  .unwrap()
}

fn lexical_output() -> String {
  json!({
    "translations": [{
      "text": "热的",
      "meaning": "having a high temperature",
      "part_of_speech": "adjective",
      "phrase_type": "",
      "aliases": [],
      "examples": [{"source_text": "hot tea", "translated_text": "热茶"}],
      "usage_notes": ["Used for temperature."]
    }]
  })
  .to_string()
}

#[tokio::test]
async fn connected_text_port_keeps_length_selection_inside_the_adapter() {
  let (short_provider, short_state) =
    provider(vec![Reply::Content("短".to_string())], "short-secret").await;
  let (long_provider, long_state) =
    provider(vec![Reply::Content("长".to_string())], "long-secret").await;
  let model: Arc<dyn ConnectedTextModel> =
    Arc::new(TranslationService::new(settings(4), short_provider, long_provider).unwrap());

  assert_eq!(
    model
      .translate_connected_text(&turn("four".to_string()), TurnLanguage::English)
      .await
      .unwrap(),
    "短"
  );
  assert_eq!(
    model
      .translate_connected_text(&turn("longer".to_string()), TurnLanguage::English)
      .await
      .unwrap(),
    "长"
  );
  assert_eq!(short_state.bodies.lock().unwrap().len(), 1);
  assert_eq!(long_state.bodies.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn lexical_draft_port_uses_strict_structured_output() {
  let (provider, state) = provider(
    vec![Reply::Content(lexical_output())],
    "lexical-credential-secret",
  )
  .await;
  let model: Arc<dyn LexicalDraftModel> =
    Arc::new(OpenAiLearningModel::new(&settings(4_000), provider).unwrap());

  let draft = model
    .generate_lexical_draft(
      &turn("hot".to_string()),
      TranslationUnit::Word,
      TurnLanguage::English,
    )
    .await
    .unwrap();
  assert!(draft.is_valid(TranslationUnit::Word));
  let bodies = state.bodies.lock().unwrap();
  assert_eq!(bodies[0]["response_format"]["type"], "json_schema");
  assert_eq!(
    bodies[0]["response_format"]["json_schema"]["name"],
    "translation_lexical_draft"
  );
  assert_eq!(bodies[0]["response_format"]["json_schema"]["strict"], true);
}

#[tokio::test]
async fn malformed_lexical_output_is_repaired_once_then_rejected() {
  let (provider, state) = provider(
    vec![
      Reply::Content("not json".to_string()),
      Reply::Content("still not json".to_string()),
    ],
    "credential-secret",
  )
  .await;
  let model = OpenAiLearningModel::new(&settings(4_000), provider).unwrap();

  assert!(matches!(
    model
      .generate_lexical_draft(
        &turn("hot".to_string()),
        TranslationUnit::Word,
        TurnLanguage::English,
      )
      .await,
    Err(TranslationModelError::InvalidOutput)
  ));
  assert_eq!(state.bodies.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn provider_failure_maps_to_a_stable_redacted_port_error() {
  let input_secret = "request-secret-8172";
  let credential_secret = "credential-secret-4815";
  let (failed, _) = provider(
    vec![Reply::Status(StatusCode::SERVICE_UNAVAILABLE)],
    credential_secret,
  )
  .await;
  let (unused, _) = provider(vec![Reply::Content("unused".to_string())], "unused").await;
  let model = TranslationService::new(settings(4_000), failed, unused).unwrap();

  let error = model
    .translate_connected_text(&turn(input_secret.to_string()), TurnLanguage::English)
    .await
    .unwrap_err();
  assert_eq!(error, TranslationModelError::Unavailable);
  let rendered = format!("{error:?} {error}");
  assert!(!rendered.contains(input_secret));
  assert!(!rendered.contains(credential_secret));
  assert!(!rendered.contains("Gemma"));
}
