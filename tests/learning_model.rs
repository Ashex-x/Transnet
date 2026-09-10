use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

use axum::{extract::State, routing::post, Json, Router};
use serde_json::{json, Value};
use transnet::{
  adapters::learning_model::OpenAiLearningModel,
  domain::translation::{EnglishDialect, TranslationInput},
  ports::learning_model::{LearningModel, LearningModelError},
  ProviderConfig, TranslationConfig,
};

#[derive(Clone)]
struct MockState {
  responses: Arc<Mutex<VecDeque<String>>>,
  bodies: Arc<Mutex<Vec<Value>>>,
}

async fn completion(State(state): State<MockState>, Json(body): Json<Value>) -> Json<Value> {
  state.bodies.lock().unwrap().push(body);
  let content = state.responses.lock().unwrap().pop_front().unwrap();
  Json(json!({"choices": [{"message": {"content": content}}]}))
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
    api_key: "test".to_string(),
  };
  (
    OpenAiLearningModel::new(&settings, provider).unwrap(),
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
