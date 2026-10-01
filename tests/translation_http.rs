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
  app_router, app_router_with_http_config, application::translation::TranslationOrchestrator,
  AppState, GenerationOutput, GenerationPort, GenerationRequest, GenerationResponse, HttpConfig,
  ModelOperationContext, ModelOperationError, ModelVersion, ProviderConfig, TranslationConfig,
  TranslationService,
};

#[derive(Clone)]
struct FakeGeneration {
  connected: Result<String, ModelOperationError>,
  lexical: Result<String, ModelOperationError>,
}

#[async_trait]
impl GenerationPort for FakeGeneration {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    let outcome = if request.input.as_str().contains("image_region_translation") {
      self.connected.clone()?;
      assert_eq!(request.images.len(), 1);
      let prompt: Value = serde_json::from_str(request.input.as_str()).unwrap();
      let key = prompt["reading_order"][0].as_str().unwrap();
      let (image_id, region_id) = key.split_once(':').unwrap();
      Ok(
        json!({"regions":[{"image_id":image_id,"region_id":region_id,
        "detected_source_language":"en","translation":"Warning"}]})
        .to_string(),
      )
    } else if request.input.as_str().contains("lexical_translation") {
      self.lexical.clone()
    } else {
      self.connected.clone()
    }?;
    Ok(GenerationResponse {
      output: GenerationOutput::new(outcome).unwrap(),
      model_version: ModelVersion::new("generation-model-v1").unwrap(),
      prompt_version: request.prompt_version,
    })
  }
}

fn connected_output() -> String {
  json!({"status":"complete", "translation":"那个计划仍然悬而未决。"}).to_string()
}

fn lexical_output() -> String {
  json!({"status":"complete", "translations":[{"text":"热的",
    "meaning":"having a high temperature", "part_of_speech":"adjective",
    "phrase_type":"", "aliases":["高温的"], "examples":[],
    "usage_notes":["Used for temperature."]}]})
  .to_string()
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
  connected: Result<String, ModelOperationError>,
  lexical: Result<String, ModelOperationError>,
) -> Router {
  let orchestrator = TranslationOrchestrator::new(Arc::new(FakeGeneration { connected, lexical }));
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
async fn composed_vlm_truthfully_activates_image_capabilities() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(
      Request::post("/api/v1/capabilities")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "image-capabilities")
        .body(Body::from("{}"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let value = body(response).await;
  assert_eq!(
    value["data"]["input_types"],
    json!(["text", "image_regions"])
  );
  assert_eq!(
    value["data"]["image_media_types"],
    json!(["image/png", "image/jpeg", "image/webp"])
  );
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
  assert_eq!(
    body["data"]["translation"]["translations"][0]["translation_id"],
    "translation_0"
  );
  assert_eq!(body["data"]["translation"]["translations"][0]["order"], 0);
  assert_eq!(
    body["data"]["translation"]["review"],
    json!({"state":"clean","issues":[]})
  );
  assert!(body["data"].get("external_sources").is_none());
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
  assert_eq!(
    body["meta"]["model_versions"],
    json!(["generation-model-v1"])
  );
  assert_eq!(
    body["meta"]["prompt_versions"],
    json!(["translation-lexical-v1"])
  );
  assert_eq!(body["meta"]["inference_profiles"], json!(["fast"]));
  assert_eq!(body["meta"]["reasoning_escalated"], false);
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
  assert_eq!(body["data"]["translation"]["review"]["state"], "clean");
  assert_eq!(body["meta"]["response_level"], "full");
  assert_eq!(
    body["meta"]["model_versions"],
    json!(["generation-model-v1"])
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
async fn target_text_guidance_executes_for_text() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input": {"type": "text", "text": "hot"},
      "source_language": "en", "target_language": "zh-CN", "response_level": "brief",
      "guidance": {"purpose": "technical", "audience": "specialist", "register": "preserve",
        "terminology": [], "max_alternatives": 0, "annotations": [], "freshness": "offline"}
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn live_freshness_remains_explicitly_unavailable() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input":{"type":"text","text":"current term"},
      "source_language":"en","target_language":"zh-CN","response_level":"brief",
      "guidance":{"freshness":"required"}
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(body(response).await["code"], "live_retrieval_unavailable");
}

#[tokio::test]
async fn oversized_generation_context_is_rejected_before_model_execution() {
  let secret = "private-generation-context-772";
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "text":"hot", "source_language":"en", "target_language":"zh-CN",
      "response_level":"brief", "history":[{
        "source_text":format!("{secret}{}", "x".repeat(8_192)),
        "translated_text":"历史", "source_language":"en", "target_language":"zh-CN"
      }]
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let problem = body(response).await;
  assert_eq!(problem["errors"][0]["field"], "generation_context");
  assert!(!problem.to_string().contains(secret));
}

#[tokio::test]
async fn unsupported_inline_image_media_type_returns_415() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input": {"type": "image_regions", "images": [{"image_id":"p1",
        "media_type":"image/gif", "data":"R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==",
        "regions":[{"region_id":"r1","x":0.0,"y":0.0,"width":1.0,"height":1.0}]}],
        "reading_order":["p1:r1"]},
      "source_language":"auto", "target_language":"en", "response_level":"brief"
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
  assert_eq!(body(response).await["code"], "unsupported_image_media_type");
}

#[tokio::test]
async fn structured_segments_return_ordered_ids_and_translations() {
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input": {"type": "segments", "segments": [{"segment_id":"seg_title",
        "text":"That plan is still up in the air.", "role":"title", "format":"plain",
        "protected_ranges":[]}]},
      "source_language":"en", "target_language":"zh-CN", "response_level":"standard",
      "history":[]
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let result = body(response).await;
  assert_eq!(result["data"]["translation"]["unit"], "segment");
  assert_eq!(
    result["data"]["translation"]["segments"][0]["segment_id"],
    "seg_title"
  );
  assert_eq!(result["data"]["translation"]["segments"][0]["order"], 0);
  assert_eq!(
    result["data"]["translation"]["segments"][0]["translations"][0]["translation_id"],
    "translation_0"
  );
  assert_eq!(
    result["meta"]["prompt_versions"],
    json!(["translation-segment-v1"])
  );
}

#[tokio::test]
async fn segment_postcondition_failure_returns_redacted_bad_gateway() {
  let secret = "private-protected-442";
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input": {"type": "segments", "segments": [{"segment_id":"seg_title",
        "text":format!("Launch {secret}"), "role":"title", "format":"plain",
        "protected_ranges":[{"start":7,"end":7 + secret.chars().count()}]}]},
      "source_language":"en", "target_language":"zh-CN", "response_level":"standard",
      "history":[]
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
  let problem = body(response).await;
  assert_eq!(problem["code"], "invalid_model_output");
  assert!(!problem.to_string().contains(secret));
}

#[tokio::test]
async fn validated_image_regions_return_ordered_translation_without_image_bytes() {
  let image_id = "private-image-991";
  let region_id = "private-region-992";
  let encoded = "iVBORw0KGgoAAAAAAAAAAAAAAAEAAAAB";
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input":{"type":"image_regions","images":[{"image_id":image_id,
        "media_type":"image/png","data":encoded,"regions":[{"region_id":region_id,
        "x":0.0,"y":0.0,"width":1.0,"height":1.0}]}],
        "reading_order":[format!("{image_id}:{region_id}")]},
      "source_language":"auto","target_language":"en","response_level":"brief"
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let result = body(response).await;
  assert_eq!(result["data"]["translation"]["unit"], "image_region");
  assert_eq!(
    result["data"]["translation"]["regions"][0]["image_id"],
    image_id
  );
  assert_eq!(
    result["data"]["translation"]["regions"][0]["region_id"],
    region_id
  );
  assert_eq!(result["data"]["translation"]["regions"][0]["order"], 0);
  assert_eq!(
    result["data"]["translation"]["regions"][0]["translations"][0]["text"],
    "Warning"
  );
  assert!(!result.to_string().contains(encoded));
}

#[tokio::test]
async fn image_dependency_failure_never_echoes_image_or_region_content() {
  let encoded = "iVBORw0KGgoAAAAAAAAAAAAAAAEAAAAB";
  let response = app(Err(ModelOperationError::Unavailable), Ok(lexical_output()))
    .oneshot(request(json!({
      "input":{"type":"image_regions","images":[{"image_id":"private-image-771",
        "media_type":"image/png","data":encoded,"regions":[{"region_id":"private-region-772",
        "x":0.0,"y":0.0,"width":1.0,"height":1.0}]}],
        "reading_order":["private-image-771:private-region-772"]},
      "source_language":"auto","target_language":"en","response_level":"brief"
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  let rendered = body(response).await.to_string();
  assert!(!rendered.contains("private-image-771"));
  assert!(!rendered.contains("private-region-772"));
  assert!(!rendered.contains(encoded));
}

#[tokio::test]
async fn contradictory_guidance_returns_redacted_constraint_problem() {
  let secret = "private-source-term-193";
  let response = app(Ok(connected_output()), Ok(lexical_output()))
    .oneshot(request(json!({
      "input": {"type":"text", "text":"ordinary text"},
      "source_language":"en", "target_language":"zh-CN", "response_level":"brief",
      "guidance":{"terminology":[
        {"source":secret,"target":"甲","policy":"required"},
        {"source":secret,"target":"乙","policy":"required"}
      ]}
    })))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let problem = body(response).await;
  assert_eq!(problem["code"], "constraint_conflict");
  assert!(!problem.to_string().contains(secret));
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
  let unavailable = app(Err(ModelOperationError::Unavailable), Ok(lexical_output()))
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
    Err(ModelOperationError::InvalidOutput),
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
  let orchestrator = TranslationOrchestrator::new(Arc::new(FakeGeneration {
    connected: Ok(connected_output()),
    lexical: Ok(lexical_output()),
  }));
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
