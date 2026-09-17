//! HTTP contract tests for the unified versioned translation endpoint.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
  Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use transnet::{
  app_router, app_router_with_http_config,
  application::translation::TranslationOrchestrator,
  domain::translation_turn::{
    LexicalMeaningDraft, LexicalTurnDraft, TranslationTurn, TranslationUnit, TurnLanguage,
  },
  ports::translation_model::{
    ConnectedTextModel, ConnectedTextOutput, ConnectedTextRequest, LexicalDraftModel,
    LexicalDraftOutput, ModelOperationVersions, TranslationModelError,
  },
  AppState, HttpConfig, ProviderConfig, TranslationConfig, TranslationService,
};

#[derive(Clone)]
struct FakeConnected {
  outcome: Result<ConnectedTextOutput, TranslationModelError>,
}

#[async_trait]
impl ConnectedTextModel for FakeConnected {
  async fn translate_connected_text(
    &self,
    _request: ConnectedTextRequest<'_>,
    _source_language: TurnLanguage,
  ) -> Result<ConnectedTextOutput, TranslationModelError> {
    self.outcome.clone()
  }
}

#[derive(Clone)]
struct FakeLexical {
  outcome: Result<LexicalDraftOutput, TranslationModelError>,
}

#[async_trait]
impl LexicalDraftModel for FakeLexical {
  async fn generate_lexical_draft(
    &self,
    _turn: &TranslationTurn,
    _unit: TranslationUnit,
    _source_language: TurnLanguage,
  ) -> Result<LexicalDraftOutput, TranslationModelError> {
    self.outcome.clone()
  }
}

fn versions(model: &str, prompt: &'static str) -> ModelOperationVersions {
  ModelOperationVersions {
    model_version: model.to_string(),
    prompt_version: prompt,
  }
}

fn connected_output() -> ConnectedTextOutput {
  ConnectedTextOutput {
    translation: "那个计划仍然悬而未决。".to_string(),
    versions: versions("connected-model-v2", "connected-text-prompt-v1"),
  }
}

fn lexical_output() -> LexicalDraftOutput {
  LexicalDraftOutput {
    draft: LexicalTurnDraft {
      translations: vec![LexicalMeaningDraft {
        text: "热的".to_string(),
        meaning: "having a high temperature".to_string(),
        part_of_speech: "adjective".to_string(),
        phrase_type: String::new(),
        aliases: vec!["高温的".to_string()],
        examples: Vec::new(),
        usage_notes: vec!["Used for temperature.".to_string()],
      }],
    },
    versions: versions("lexical-model-v3", "lexical-draft-prompt-v1"),
  }
}

fn legacy_service() -> TranslationService {
  let provider = |model: &str| ProviderConfig {
    base_url: "http://127.0.0.1:9/v1".to_string(),
    model: model.to_string(),
    api_key: "unused-test-credential".into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider("legacy-short"),
    provider("legacy-long"),
  )
  .unwrap()
}

fn app(
  connected: Result<ConnectedTextOutput, TranslationModelError>,
  lexical: Result<LexicalDraftOutput, TranslationModelError>,
) -> Router {
  let orchestrator = TranslationOrchestrator::new(
    Arc::new(FakeConnected { outcome: connected }),
    Arc::new(FakeLexical { outcome: lexical }),
  );
  app_router(AppState::new(legacy_service()).with_translation_orchestrator(Arc::new(orchestrator)))
}

fn request(body: Value) -> Request<Body> {
  Request::post("/api/v1/translations")
    .header(header::CONTENT_TYPE, "application/json")
    .header("x-request-id", "frontend-contract-17")
    .body(Body::from(body.to_string()))
    .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn translation_success_uses_the_frozen_envelope_and_plural_versions() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "standard",
      "history": []
    })))
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()["x-request-id"], "frontend-contract-17");
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
  let body = body(response).await;
  assert_eq!(body["data"]["translation"]["unit"], "word");
  assert_eq!(
    body["data"]["translation"]["translations"][0]["text"],
    "热的"
  );
  assert_eq!(body["meta"]["request_id"], "frontend-contract-17");
  assert_eq!(body["meta"]["response_level"], "standard");
  assert_eq!(body["meta"]["schema_version"], "translation-result-v1");
  assert_eq!(
    body["meta"]["normalizer_version"],
    "translation-lookup-nfc-v1"
  );
  assert_eq!(
    body["meta"]["projection_version"],
    "translation-projection-v1"
  );
  assert_eq!(body["meta"]["model_versions"], json!(["lexical-model-v3"]));
  assert_eq!(
    body["meta"]["prompt_versions"],
    json!(["lexical-draft-prompt-v1"])
  );
  assert!(body["meta"].get("retrieval_version").is_none());
  assert!(body["meta"].get("content_release").is_none());
}

#[tokio::test]
async fn passage_and_request_local_history_use_the_same_wire_contract() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "text": "That plan is still up in the air.",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "full",
      "history": [{
        "source_text": "We discussed the proposal.",
        "translated_text": "我们讨论了这个提案。",
        "source_language": "en",
        "target_language": "zh-CN"
      }]
    })))
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let body = body(response).await;
  assert_eq!(body["data"]["translation"]["unit"], "passage");
  assert_eq!(
    body["data"]["translation"]["translations"][0]["text"],
    "那个计划仍然悬而未决。"
  );
  assert!(body["data"]["translation"]["translations"][0]
    .get("details")
    .is_none());
  assert_eq!(body["meta"]["response_level"], "full");
  assert_eq!(
    body["meta"]["model_versions"],
    json!(["connected-model-v2"])
  );
  assert!(!body.to_string().contains("We discussed the proposal"));
}

#[tokio::test]
async fn malformed_unknown_and_semantically_invalid_requests_use_safe_problems() {
  let service = app(Ok(connected_output()), Ok(lexical_output()));
  let unknown = service
    .clone()
    .oneshot(request(json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "provider": "forbidden"
    })))
    .await
    .unwrap();
  assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
  assert_eq!(unknown.headers()["x-request-id"], "frontend-contract-17");
  assert_eq!(
    unknown.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let unknown = body(unknown).await;
  assert_eq!(unknown["code"], "invalid_json");
  assert_eq!(unknown["request_id"], "frontend-contract-17");

  let unknown_history = service
    .clone()
    .oneshot(request(json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "history": [{
        "source_text": "private-history-771",
        "translated_text": "历史",
        "source_language": "en",
        "target_language": "zh-CN",
        "timestamp": "forbidden"
      }]
    })))
    .await
    .unwrap();
  assert_eq!(unknown_history.status(), StatusCode::BAD_REQUEST);
  let unknown_history = body(unknown_history).await;
  assert_eq!(unknown_history["code"], "invalid_json");
  assert!(!unknown_history.to_string().contains("private-history-771"));

  let invalid = service
    .oneshot(request(json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "fr",
      "response_level": "brief"
    })))
    .await
    .unwrap();
  assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let invalid = body(invalid).await;
  assert_eq!(invalid["code"], "invalid_translation_request");
  assert_eq!(invalid["errors"][0]["field"], "target_language");
  assert!(!invalid.to_string().contains("hot"));
}

#[tokio::test]
async fn target_route_rejects_other_methods_and_unknown_paths_with_shared_problems() {
  let router = app(Ok(connected_output()), Ok(lexical_output()));
  let method = router
    .clone()
    .oneshot(
      Request::get("/api/v1/translations")
        .header("x-request-id", "frontend-contract-18")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(method.status(), StatusCode::METHOD_NOT_ALLOWED);
  assert_eq!(body(method).await["code"], "method_not_allowed");

  let missing = router
    .oneshot(
      Request::post("/api/v1/translation")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "frontend-contract-19")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(missing.status(), StatusCode::NOT_FOUND);
  assert_eq!(body(missing).await["code"], "not_found");
}

#[tokio::test]
async fn model_failures_map_to_stable_redacted_problem_statuses() {
  let unavailable = app(
    Err(TranslationModelError::Unavailable),
    Ok(lexical_output()),
  )
  .oneshot(request(json!({
    "text": "This contains private-source-991.",
    "source_language": "en",
    "target_language": "zh-CN",
    "response_level": "brief"
  })))
  .await
  .unwrap();
  assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    unavailable.headers()["x-request-id"],
    "frontend-contract-17"
  );
  let unavailable = body(unavailable).await;
  assert_eq!(unavailable["code"], "translation_model_unavailable");
  assert_eq!(unavailable["retryable"], true);
  assert!(!unavailable.to_string().contains("private-source-991"));

  let invalid = app(
    Ok(connected_output()),
    Err(TranslationModelError::InvalidOutput),
  )
  .oneshot(request(json!({
    "text": "hot",
    "source_language": "en",
    "target_language": "zh-CN",
    "response_level": "full"
  })))
  .await
  .unwrap();
  assert_eq!(invalid.status(), StatusCode::BAD_GATEWAY);
  assert_eq!(body(invalid).await["code"], "invalid_model_output");
}

#[tokio::test]
async fn target_payload_limit_uses_the_shared_problem_contract() {
  let orchestrator = TranslationOrchestrator::new(
    Arc::new(FakeConnected {
      outcome: Ok(connected_output()),
    }),
    Arc::new(FakeLexical {
      outcome: Ok(lexical_output()),
    }),
  );
  let router = app_router_with_http_config(
    AppState::new(legacy_service()).with_translation_orchestrator(Arc::new(orchestrator)),
    &HttpConfig {
      max_request_body_bytes: 32,
      allowed_origins: Vec::new(),
      allow_credentials: false,
    },
  )
  .unwrap();
  let response = router
    .oneshot(request(json!({
      "text": "This payload exceeds the configured test limit.",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief"
    })))
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
  assert_eq!(body(response).await["code"], "payload_too_large");
}

#[tokio::test]
async fn missing_orchestrator_fails_closed_without_affecting_the_legacy_route() {
  let router = app_router(AppState::new(legacy_service()));
  let response = router
    .oneshot(request(json!({
      "text": "hot",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief"
    })))
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    body(response).await["code"],
    "translation_model_unavailable"
  );
}
