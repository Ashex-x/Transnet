//! Contract tests that keep the published default-runtime OpenAPI document aligned with HTTP.

use std::{collections::BTreeSet, sync::Arc};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Method, Request, StatusCode},
  routing::post,
  Json, Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use transnet::{
  app_router, app_router_with_http_config, types::is_language_code, AppState, HttpConfig,
  OpenAiLearningModel, ProviderConfig, Readiness, TranslationConfig, TranslationService,
};

const OPENAPI: &str = include_str!("../docs/reference/transnet-openapi.json");

fn openapi() -> Value {
  serde_json::from_str(OPENAPI).unwrap()
}

fn provider_config(base_url: String) -> ProviderConfig {
  ProviderConfig {
    base_url,
    model: "test-model".to_string(),
    api_key: "test-key".to_string().into(),
  }
}

fn translation_config() -> TranslationConfig {
  TranslationConfig {
    long_text_chars: 4_000,
    timeout_seconds: 1,
    max_retries: 0,
    retry_delay_ms: 0,
  }
}

fn unavailable_service() -> TranslationService {
  let provider = provider_config("http://127.0.0.1:1/v1".to_string());
  TranslationService::new(translation_config(), provider.clone(), provider).unwrap()
}

async fn provider_completion(Json(body): Json<Value>) -> Json<Value> {
  if body.get("response_format").is_some() {
    return Json(json!({
      "choices": [{"message": {"content": json!({
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

  Json(json!({
    "choices": [{"message": {"content": "translated"}}]
  }))
}

async fn configured_app() -> Router {
  let provider = Router::new().route("/v1/chat/completions", post(provider_completion));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move {
    axum::serve(listener, provider).await.unwrap();
  });

  let config = provider_config(format!("http://{address}/v1"));
  let translation = translation_config();
  let service =
    TranslationService::new(translation.clone(), config.clone(), config.clone()).unwrap();
  let learning_model = OpenAiLearningModel::new(&translation, config).unwrap();
  app_router(AppState::new(service).with_learning_model(Arc::new(learning_model)))
}

async fn json_body(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn response_has_header(spec: &Value, path: &str, status: &str, header_name: &str) -> bool {
  spec["paths"][path]["post"]["responses"][status]["headers"]
    .as_object()
    .is_some_and(|headers| headers.contains_key(header_name))
    || spec["paths"][path]["get"]["responses"][status]["headers"]
      .as_object()
      .is_some_and(|headers| headers.contains_key(header_name))
}

fn safe_request_id(value: &str) -> bool {
  (1..=128).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn source_language_branch_matches(branch: &Value, value: &str) -> bool {
  if branch.get("const").is_some() {
    return branch["const"] == value;
  }

  let all_of = branch["allOf"].as_array().unwrap();
  assert_eq!(all_of.len(), 2);
  assert_eq!(all_of[0]["$ref"], "#/components/schemas/Bcp47LanguageTag");
  is_language_code(value) && all_of[1]["not"]["const"] != value
}

#[derive(Debug)]
struct FixedReadiness(bool);

#[async_trait]
impl Readiness for FixedReadiness {
  async fn is_ready(&self) -> bool {
    self.0
  }
}

#[test]
fn openapi_is_parseable_and_describes_the_complete_no_auth_contract() {
  let spec = openapi();

  assert_eq!(spec["openapi"], "3.1.0");
  assert_eq!(spec["info"]["title"], "Transnet HTTP API");
  assert_eq!(spec["security"], json!([]));
  assert!(spec["components"].get("securitySchemes").is_none());
  assert_eq!(spec["x-transnet-boundary"]["authentication"], "none");

  let paths = spec["paths"].as_object().unwrap();
  let actual_paths = paths.keys().cloned().collect::<BTreeSet<_>>();
  let expected_paths = [
    "/health",
    "/livez",
    "/readyz",
    "/translate",
    "/v1/lookups",
    "/v1/lookup-jobs/{job_id}",
    "/v1/senses/{sense_id}",
    "/v1/graph",
    "/v1/graph/nodes/{kind}/{id}/neighbors",
    "/v1/graph-edges/{edge_id}/feedback",
    "/v1/history",
    "/v1/history/{lookup_id}",
    "/v1/saved-senses",
    "/v1/saved-senses/{sense_id}",
    "/v1/practice/sessions",
    "/v1/practice/sessions/{session_id}/next",
    "/v1/practice/sessions/{session_id}/current",
    "/v1/practice/sessions/{session_id}/attempts",
    "/v1/progress",
    "/v1/graph-views",
    "/v1/graph-views/{view_id}",
    "/v1/me",
    "/v1/me/preferences",
    "/v1/me/export",
    "/v1/privacy-requests/{request_id}",
    "/v1/privacy-requests/{request_id}/result",
  ]
  .into_iter()
  .map(str::to_string)
  .collect::<BTreeSet<_>>();
  assert_eq!(actual_paths, expected_paths);
  assert!(paths["/health"].get("get").is_some());
  assert!(paths["/livez"].get("get").is_some());
  assert!(paths["/readyz"].get("get").is_some());
  assert!(paths["/translate"].get("post").is_some());
  assert!(paths["/v1/lookups"].get("post").is_some());

  assert_eq!(spec["x-transnet-cors"]["default"], "disabled");
  assert_eq!(
    spec["x-transnet-cache-policy"]["description"],
    "Successful /v1/lookups responses and all versioned problem responses use Cache-Control: no-store. The health and legacy translation routes do not set this directive."
  );
  assert_eq!(
    spec["components"]["schemas"]["Problem"]["required"],
    json!([
      "type",
      "title",
      "status",
      "code",
      "detail",
      "request_id",
      "retryable",
      "errors"
    ])
  );
  assert!(response_has_header(&spec, "/health", "200", "X-Request-Id"));
  assert!(response_has_header(
    &spec,
    "/translate",
    "422",
    "X-Request-Id"
  ));
  assert!(response_has_header(
    &spec,
    "/v1/lookups",
    "200",
    "Cache-Control"
  ));
  assert!(response_has_header(
    &spec,
    "/v1/lookups",
    "503",
    "Cache-Control"
  ));
}

#[test]
fn source_language_schema_accepts_auto_once_and_rejects_invalid_language_tags() {
  let spec = openapi();
  let source_language =
    &spec["components"]["schemas"]["LookupRequest"]["properties"]["source_language"];
  let alternatives = source_language["oneOf"].as_array().unwrap();

  assert_eq!(alternatives.len(), 2);
  assert_eq!(alternatives[0]["const"], "auto");
  assert_eq!(
    spec["components"]["schemas"]["Bcp47LanguageTag"]["pattern"],
    "^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8})*$"
  );
  for value in ["auto", "es", "zh-CN"] {
    let matches = alternatives
      .iter()
      .filter(|branch| source_language_branch_matches(branch, value))
      .count();
    assert_eq!(
      matches, 1,
      "expected `{value}` to match exactly one alternative"
    );
  }
  for value in ["e", "auto-", "zh_CN"] {
    let matches = alternatives
      .iter()
      .filter(|branch| source_language_branch_matches(branch, value))
      .count();
    assert_eq!(matches, 0, "expected `{value}` not to match the schema");
  }
}

#[tokio::test]
async fn health_liveness_and_readiness_match_the_published_status_contract() {
  let spec = openapi();
  let app = configured_app().await;

  for path in ["/health", "/livez", "/readyz"] {
    let response = app
      .clone()
      .oneshot(
        Request::get(path)
          .header("x-request-id", "contract-health-42")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "path: {path}");
    assert_eq!(response.headers()["x-request-id"], "contract-health-42");
    assert_eq!(json_body(response).await, json!({"status": "ok"}));
    assert!(spec["paths"][path]["get"]["responses"].get("200").is_some());
  }

  let response = app_router(
    AppState::new(unavailable_service()).with_readiness(Arc::new(FixedReadiness(false))),
  )
  .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
  .await
  .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert!(response.headers().contains_key("x-request-id"));
  assert_eq!(json_body(response).await, json!({"status": "unavailable"}));
  assert!(spec["paths"]["/readyz"]["get"]["responses"]
    .get("503")
    .is_some());
}

#[tokio::test]
async fn direct_translation_preserves_its_published_legacy_success_and_error_envelopes() {
  let spec = openapi();
  let response = configured_app()
    .await
    .oneshot(
      Request::post("/translate")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "contract-translate-42")
        .body(Body::from(
          r#"{"text":"hello","source_lang":"en","target_lang":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()["x-request-id"], "contract-translate-42");
  assert!(!response.headers().contains_key(header::CACHE_CONTROL));
  assert_eq!(
    json_body(response).await,
    json!({"translation": "translated"})
  );
  assert!(spec["paths"]["/translate"]["post"]["responses"]
    .get("200")
    .is_some());

  let response = configured_app()
    .await
    .oneshot(
      Request::post("/translate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{"))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  assert_eq!(
    json_body(response).await,
    json!({"error": "invalid JSON request"})
  );
  assert!(spec["paths"]["/translate"]["post"]["responses"]
    .get("400")
    .is_some());
}

#[tokio::test]
async fn model_lookup_matches_published_no_store_and_generated_response_contract() {
  let spec = openapi();
  let response = configured_app()
    .await
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "contract-lookup-42")
        .body(Body::from(
          r#"{"query":"caliente","source_language":"es","target_language":"en","include":["relations","word_history"]}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()["x-request-id"], "contract-lookup-42");
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json_body(response).await;
  assert_eq!(body["schema_version"], "1.0");
  assert_eq!(body["matches"][0]["source_sense_id"], Value::Null);
  assert_eq!(body["matches"][0]["context_relevance"], Value::Null);
  assert_eq!(
    body["matches"][0]["english_senses"][0]["sense_id"],
    Value::Null
  );
  assert_eq!(body["matches"][0]["english_senses"][0]["generated"], true);
  assert_eq!(
    body["matches"][0]["english_senses"][0]["definition"]["evidence_ids"],
    json!([])
  );
  assert_eq!(
    body["matches"][0]["english_senses"][0]["related_words"][0]["canonical"],
    false
  );
  assert_eq!(body["coverage"]["canonical_evidence"], "unavailable");
  assert_eq!(body["provenance"]["lexicon_release"], Value::Null);
  assert_eq!(body["provenance"]["index_version"], Value::Null);
  assert_eq!(body["provenance"]["evidence_backed"], false);
  assert!(body["warnings"]
    .as_array()
    .is_some_and(|warnings| warnings.iter().any(|warning| {
      warning
        .as_str()
        .is_some_and(|value| value.contains("model-generated"))
    })));
  assert!(spec["paths"]["/v1/lookups"]["post"]["responses"]
    .get("200")
    .is_some());
}

#[tokio::test]
async fn versioned_problems_share_the_published_envelope_and_cache_policy() {
  let spec = openapi();
  let app = app_router(AppState::new(unavailable_service()));

  let response = app
    .clone()
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"query":"hola","source_language":"es"}"#))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json_body(response).await;
  assert_problem_shape(
    &body,
    StatusCode::SERVICE_UNAVAILABLE,
    "learning_model_unavailable",
  );
  assert_eq!(body["retryable"], true);
  assert!(spec["paths"]["/v1/lookups"]["post"]["responses"]
    .get("503")
    .is_some());

  let response = app
    .clone()
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "contract-problem-42")
        .body(Body::from(
          r#"{"query":"hola","source_language":"es","target_language":"zh-CN"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(response.headers()["x-request-id"], "contract-problem-42");
  let body = json_body(response).await;
  assert_problem_shape(&body, StatusCode::UNPROCESSABLE_ENTITY, "validation_error");
  assert_eq!(body["request_id"], "contract-problem-42");
  assert_eq!(body["errors"][0]["field"], "target_language");
  assert!(spec["paths"]["/v1/lookups"]["post"]["responses"]
    .get("422")
    .is_some());

  let response = app
    .oneshot(Request::get("/v1/not-a-route").body(Body::empty()).unwrap())
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json_body(response).await;
  assert_problem_shape(&body, StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn configured_cors_matches_the_published_exact_origin_policy() {
  let spec = openapi();
  let config = HttpConfig {
    max_request_body_bytes: 1_024,
    allowed_origins: vec!["https://app.example.test".to_string()],
    allow_credentials: true,
  };
  let app = app_router_with_http_config(AppState::new(unavailable_service()), &config).unwrap();

  let response = app
    .oneshot(
      Request::builder()
        .method(Method::OPTIONS)
        .uri("/translate")
        .header(header::ORIGIN, "https://app.example.test")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
          header::ACCESS_CONTROL_REQUEST_HEADERS,
          "content-type, x-request-id",
        )
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert!(response.status().is_success());
  assert_eq!(
    response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
    "https://app.example.test"
  );
  assert_eq!(
    response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
    "true"
  );
  assert!(response.headers().contains_key("x-request-id"));
  assert!(safe_request_id(
    response.headers()["x-request-id"].to_str().unwrap()
  ));
  assert_eq!(spec["x-transnet-cors"]["exactOriginsOnly"], true);
  assert_eq!(
    spec["x-transnet-cors"]["allowedMethods"],
    json!(["GET", "POST", "OPTIONS"])
  );
}

fn assert_problem_shape(body: &Value, status: StatusCode, code: &str) {
  assert_eq!(body["type"], "about:blank");
  assert_eq!(body["status"], status.as_u16());
  assert_eq!(body["code"], code);
  assert!(body["title"].is_string());
  assert!(body["detail"].is_string());
  assert!(body["retryable"].is_boolean());
  assert!(body["errors"].is_array());
  assert!(body["request_id"].as_str().is_some_and(safe_request_id));
}
