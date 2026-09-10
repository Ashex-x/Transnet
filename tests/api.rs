use axum::{
  body::{to_bytes, Body},
  extract::State,
  http::{Request, StatusCode},
  routing::post,
  Json, Router,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use transnet::{
  app_router, AppState, OpenAiLearningModel, ProviderConfig, TranslationConfig, TranslationService,
};

fn app() -> axum::Router {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string(),
  };
  let service = TranslationService::new(
    TranslationConfig {
      long_text_chars: 4000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap();
  app_router(AppState::new(service))
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn completion(
  State(translation): State<&'static str>,
  Json(body): Json<Value>,
) -> Json<Value> {
  if body.get("response_format").is_some() {
    return Json(serde_json::json!({
      "choices": [{"message": {"content": serde_json::json!({
        "source_language": "es",
        "language_confidence": "high",
        "entries": [{
          "lemma": "hot",
          "part_of_speech": "adjective",
          "definition": "having a high temperature",
          "localized_gloss": "温度高的",
          "confidence": "high",
          "pronunciations": [{"value": "hɑt", "notation": "ipa", "dialect": "en-US"}],
          "forms": [{"form": "hotter", "label": "comparative"}],
          "usage_notes": [{"kind": "collocation", "text": "hot soup"}],
          "examples": [{"english": "The soup is hot.", "localized": "汤很烫。"}],
          "etymology": "From Old English.",
          "related_words": [{"lemma": "warm", "relation": "lower_degree", "note": null}]
        }],
        "warnings": []
      }).to_string()}}]
    }));
  }
  Json(serde_json::json!({
    "choices": [{"message": {"content": translation}}]
  }))
}

async fn app_with_provider() -> axum::Router {
  let provider = Router::new()
    .route("/v1/chat/completions", post(completion))
    .with_state("translated");
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });

  let provider = ProviderConfig {
    base_url: format!("http://{address}/v1"),
    model: "Gemma4".to_string(),
    api_key: "test".to_string(),
  };
  let translation = TranslationConfig {
    long_text_chars: 4000,
    timeout_seconds: 1,
    max_retries: 0,
    retry_delay_ms: 0,
  };
  let service =
    TranslationService::new(translation.clone(), provider.clone(), provider.clone()).unwrap();
  let learning_model = OpenAiLearningModel::new(&translation, provider).unwrap();
  app_router(AppState::new(service).with_learning_model(Arc::new(learning_model)))
}

#[tokio::test]
async fn health_returns_direct_status() {
  let response = app()
    .oneshot(Request::get("/health").body(Body::empty()).unwrap())
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(json(response).await, serde_json::json!({"status": "ok"}));
}

#[tokio::test]
async fn translate_returns_direct_translation() {
  let response = app_with_provider()
    .await
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":"hello","source_lang":"en","target_lang":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    json(response).await,
    serde_json::json!({"translation": "translated"})
  );
}

#[tokio::test]
async fn translate_maps_provider_failure_to_service_unavailable() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":"hello","source_lang":"en","target_lang":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "translation provider unavailable"})
  );
}

#[tokio::test]
async fn translate_rejects_invalid_input_with_direct_error() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"text":" ","source_lang":"English","target_lang":"zh_CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "text must not be blank"})
  );
}

#[tokio::test]
async fn translate_rejects_malformed_json() {
  let response = app()
    .oneshot(
      Request::post("/translate")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  assert_eq!(
    json(response).await,
    serde_json::json!({"error": "invalid JSON request"})
  );
}

#[tokio::test]
async fn removed_routes_return_not_found() {
  for path in [
    "/api/account/login",
    "/api/profile",
    "/api/transnet/history",
    "/api/transnet/favorites",
    "/api/about",
    "/api/stats",
  ] {
    let response = app()
      .oneshot(Request::get(path).body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "path: {path}");
  }
}

#[tokio::test]
async fn lookup_returns_generated_learning_card_without_canonical_ids() {
  let response = app_with_provider()
    .await
    .oneshot(
      Request::post("/v1/lookups")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"query":"caliente","source_language":"es","target_language":"en","context":"La sopa está caliente.","explanation_language":"zh-CN","english_dialect":"en-US","learner_level":"B1","detail":"full","include":["relations","word_history"],"history_mode":"incognito"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()["cache-control"], "no-store");
  assert!(response.headers().contains_key("x-request-id"));
  let body = json(response).await;
  assert_eq!(body["schema_version"], "1.0");
  assert_eq!(body["query"]["normalized"], "caliente");
  assert_eq!(body["matches"][0]["source_sense_id"], Value::Null);
  assert_eq!(body["matches"][0]["english_senses"][0]["lemma"], "hot");
  assert_eq!(
    body["matches"][0]["english_senses"][0]["part_of_speech"],
    "adjective"
  );
  assert_eq!(
    body["matches"][0]["english_senses"][0]["related_words"][0]["canonical"],
    false
  );
  assert_eq!(body["provenance"]["evidence_backed"], false);
  assert!(body["warnings"].as_array().is_some_and(|warnings| {
    warnings.iter().all(|warning| {
      warning.as_str()
        != Some(
          "Relations and word history are not included by the current canonical lookup foundation.",
        )
    })
  }));
}

#[tokio::test]
async fn lookup_rejects_non_english_target_with_problem_details() {
  let response = app()
    .oneshot(
      Request::post("/v1/lookups")
        .header("content-type", "application/json")
        .body(Body::from(
          r#"{"query":"hello","source_language":"en","target_language":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(
    response.headers()["content-type"],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["code"], "validation_error");
  assert_eq!(body["errors"][0]["field"], "target_language");
  assert_eq!(body["retryable"], false);
}

#[tokio::test]
async fn lookup_reports_unconfigured_model() {
  let response = app()
    .oneshot(
      Request::post("/v1/lookups")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"hola","source_language":"es"}"#))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(json(response).await["code"], "learning_model_unavailable");
}
