//! Strict target HTTP boundary for bounded verified knowledge paths.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
  extract::{rejection::JsonRejection, Extension},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::post,
  Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
  application::knowledge_paths::{BoundedKnowledgePathService, KnowledgePathSearchError},
  domain::{
    assertion::{CanonicalNodeFamily, CanonicalNodeId},
    canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
    knowledge_view::{
      KnowledgePathOutcome, KnowledgePathRequest, KnowledgePathResult, KnowledgeRoot,
      VerifiedKnowledgeStep,
    },
    model_runtime::CancellationSignal,
    request_context::RequestContext,
  },
};

use super::super::{
  problem, problem::FieldError, request_id::RequestId, AppState, SuccessEnvelope,
};

const RESULT_SCHEMA_VERSION: &str = "knowledge-path-result-v1";

#[async_trait]
pub(crate) trait KnowledgePathUseCase: Send + Sync {
  async fn find(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: KnowledgePathRequest,
  ) -> Result<KnowledgePathResult, KnowledgePathSearchError>;
}

#[async_trait]
impl KnowledgePathUseCase for BoundedKnowledgePathService {
  async fn find(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: KnowledgePathRequest,
  ) -> Result<KnowledgePathResult, KnowledgePathSearchError> {
    BoundedKnowledgePathService::find(self, context, cancellation, request).await
  }
}

/// Builds the isolated path route without changing shared runtime state composition.
pub(crate) fn router(service: Arc<dyn KnowledgePathUseCase>) -> Router<AppState> {
  Router::new()
    .route("/knowledge/paths", post(find))
    .layer(Extension(service))
    .method_not_allowed_fallback(super::method_not_allowed)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgePathHttpRequest {
  from: NodeRefDto,
  to: NodeRefDto,
  target_language: String,
  content_release: String,
  canonical_schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeRefDto {
  kind: String,
  id: String,
}

async fn find(
  Extension(service): Extension<Arc<dyn KnowledgePathUseCase>>,
  Extension(request_id): Extension<RequestId>,
  Extension(context): Extension<RequestContext>,
  payload: Result<Json<KnowledgePathHttpRequest>, JsonRejection>,
) -> Response {
  let Json(payload) = match payload {
    Ok(value) => value,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      return problem::payload_too_large(&request_id)
    }
    Err(_) => return invalid_json(&request_id),
  };
  let request = match request_from_dto(payload) {
    Ok(value) => value,
    Err(error) => return invalid_field(&request_id, error.field, error.message),
  };
  let context = match context
    .clone()
    .with_content_release(request.release.release_id.clone())
  {
    Ok(value) => value,
    Err(_) => return invalid_field(&request_id, "content_release", "is invalid."),
  };
  let cancellation = CancellationSignal::default();
  match service.find(&context, &cancellation, request).await {
    Ok(result) => match response_from_result(result) {
      Ok(data) => problem::no_store(
        (
          StatusCode::OK,
          Json(SuccessEnvelope::new(data, &context, RESULT_SCHEMA_VERSION)),
        )
          .into_response(),
      ),
      Err(error) => map_error(error, &request_id),
    },
    Err(error) => map_error(error, &request_id),
  }
}

fn request_from_dto(
  request: KnowledgePathHttpRequest,
) -> Result<KnowledgePathRequest, RequestFieldError> {
  let from = parse_node(request.from).map_err(|_| RequestFieldError {
    field: "from",
    message: "must be a supported typed canonical node.",
  })?;
  let to = parse_node(request.to).map_err(|_| RequestFieldError {
    field: "to",
    message: "must be a supported typed canonical node.",
  })?;
  if from == to {
    return Err(RequestFieldError {
      field: "to",
      message: "must differ from the starting node.",
    });
  }
  let target_language =
    LanguageTag::parse(&request.target_language).map_err(|_| RequestFieldError {
      field: "target_language",
      message: "must be a valid BCP-47 language tag.",
    })?;
  let release_id = CanonicalId::new(request.content_release).map_err(|_| RequestFieldError {
    field: "content_release",
    message: "is invalid.",
  })?;
  let release = CanonicalReleasePin::new(release_id, request.canonical_schema_version).ok_or(
    RequestFieldError {
      field: "canonical_schema_version",
      message: "is invalid.",
    },
  )?;
  Ok(KnowledgePathRequest {
    from: KnowledgeRoot { node: from },
    to: KnowledgeRoot { node: to },
    target_language,
    release,
  })
}

struct RequestFieldError {
  field: &'static str,
  message: &'static str,
}

fn parse_node(value: NodeRefDto) -> Result<CanonicalNodeId, ()> {
  let family = family_from_wire(&value.kind).ok_or(())?;
  let id = CanonicalId::new(value.id).map_err(|_| ())?;
  Ok(CanonicalNodeId::publisher_assigned(family, id))
}

fn family_from_wire(value: &str) -> Option<CanonicalNodeFamily> {
  use CanonicalNodeFamily as F;
  Some(match value {
    "sense" => F::LexicalSense,
    "phrase" => F::Phrase,
    "multilingual_term" => F::MultilingualTerm,
    "concept" => F::Concept,
    "entity" => F::Entity,
    "phenomenon" => F::Phenomenon,
    "mechanism" => F::Mechanism,
    "process" => F::Process,
    "equation" => F::Equation,
    "quantity" => F::Quantity,
    "material" => F::Material,
    "instrument" => F::Instrument,
    "method" => F::Method,
    "technology" => F::Technology,
    "application" => F::Application,
    "standard" => F::Standard,
    "organization" => F::Organization,
    "person" => F::Person,
    "place" => F::Place,
    "idiom" => F::Idiom,
    "metaphor" => F::Metaphor,
    "grammar_pattern" => F::GrammarPattern,
    "collocation" => F::Collocation,
    "misconception" => F::Misconception,
    "domain" => F::Domain,
    "semantic_scale" => F::SemanticScale,
    _ => return None,
  })
}

fn family_to_wire(value: CanonicalNodeFamily) -> Result<&'static str, KnowledgePathSearchError> {
  use CanonicalNodeFamily as F;
  Ok(match value {
    F::LexicalSense => "sense",
    F::Phrase => "phrase",
    F::MultilingualTerm => "multilingual_term",
    F::Concept => "concept",
    F::Entity => "entity",
    F::Phenomenon => "phenomenon",
    F::Mechanism => "mechanism",
    F::Process => "process",
    F::Equation => "equation",
    F::Quantity => "quantity",
    F::Material => "material",
    F::Instrument => "instrument",
    F::Method => "method",
    F::Technology => "technology",
    F::Application => "application",
    F::Standard => "standard",
    F::Organization => "organization",
    F::Person => "person",
    F::Place => "place",
    F::Idiom => "idiom",
    F::Metaphor => "metaphor",
    F::GrammarPattern => "grammar_pattern",
    F::Collocation => "collocation",
    F::Misconception => "misconception",
    F::Domain => "domain",
    F::SemanticScale => "semantic_scale",
    F::Lexeme => return Err(KnowledgePathSearchError::InconsistentProof),
  })
}

#[derive(Serialize)]
struct KnowledgePathData {
  outcome: &'static str,
  from: PublicNodeRef,
  to: PublicNodeRef,
  target_language: String,
  paths: Vec<PublicPath>,
}

#[derive(Serialize)]
struct PublicNodeRef {
  kind: &'static str,
  id: String,
}

#[derive(Serialize)]
struct PublicPath {
  order: u8,
  steps: Vec<PublicStep>,
}

#[derive(Serialize)]
struct PublicStep {
  edge_id: String,
  relationship_revision: u32,
  assertion_id: String,
  assertion_revision: u32,
  traversal_id: String,
  relation: &'static str,
  relation_registry_revision: u32,
  direction: &'static str,
  source: PublicNodeRef,
  target: PublicNodeRef,
  conditions: Vec<PublicCondition>,
  evidence_ids: Vec<String>,
  content_release: String,
  canonical_schema_version: String,
}

#[derive(Serialize)]
struct PublicCondition {
  condition_id: String,
  condition_type: String,
  parameter_ids: Vec<String>,
}

fn response_from_result(
  result: KnowledgePathResult,
) -> Result<KnowledgePathData, KnowledgePathSearchError> {
  result
    .validate()
    .map_err(|_| KnowledgePathSearchError::InconsistentProof)?;
  let from = public_node(&result.request.from.node)?;
  let to = public_node(&result.request.to.node)?;
  let target_language = result.request.target_language.to_string();
  let (outcome, paths) = match result.outcome {
    KnowledgePathOutcome::NoVerifiedPath => ("no_verified_path", Vec::new()),
    KnowledgePathOutcome::Connected(paths) => (
      "connected",
      paths
        .into_iter()
        .map(|path| {
          Ok(PublicPath {
            order: path.order,
            steps: path
              .steps
              .iter()
              .map(public_step)
              .collect::<Result<_, _>>()?,
          })
        })
        .collect::<Result<_, KnowledgePathSearchError>>()?,
    ),
  };
  Ok(KnowledgePathData {
    outcome,
    from,
    to,
    target_language,
    paths,
  })
}

fn public_step(step: &VerifiedKnowledgeStep) -> Result<PublicStep, KnowledgePathSearchError> {
  let projection = step.projection();
  let assertion = projection.assertion();
  let traversal = projection.traversal();
  let relation = traversal
    .relation_type
    .rule()
    .require_qdrant_wire_name()
    .map_err(|_| KnowledgePathSearchError::InconsistentProof)?;
  Ok(PublicStep {
    edge_id: traversal.edge_id.to_string(),
    relationship_revision: traversal.relationship_revision,
    assertion_id: assertion.assertion_id.to_string(),
    assertion_revision: assertion.assertion_revision,
    traversal_id: traversal.traversal_id.to_string(),
    relation,
    relation_registry_revision: traversal.relation_registry_revision,
    direction: "forward",
    source: public_node(&traversal.source)?,
    target: public_node(&traversal.target)?,
    conditions: assertion
      .conditions
      .iter()
      .map(|condition| PublicCondition {
        condition_id: condition.condition_id.to_string(),
        condition_type: condition.condition_type.to_string(),
        parameter_ids: condition
          .parameter_ids
          .iter()
          .map(ToString::to_string)
          .collect(),
      })
      .collect(),
    evidence_ids: assertion
      .evidence_ids
      .iter()
      .map(ToString::to_string)
      .collect(),
    content_release: step.release().release_id.to_string(),
    canonical_schema_version: step.release().canonical_schema_version.clone(),
  })
}

fn public_node(node: &CanonicalNodeId) -> Result<PublicNodeRef, KnowledgePathSearchError> {
  Ok(PublicNodeRef {
    kind: family_to_wire(node.family())?,
    id: node.id().to_string(),
  })
}

fn invalid_json(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_json",
    "Invalid JSON request",
    "The request body is not valid knowledge-path JSON.",
    request_id,
    false,
    Vec::new(),
  )
}

fn invalid_field(request_id: &RequestId, field: &'static str, message: &'static str) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_knowledge_path_request",
    "Invalid knowledge-path request",
    "One or more knowledge-path request fields are invalid.",
    request_id,
    false,
    vec![FieldError::new(field, message)],
  )
}

fn map_error(error: KnowledgePathSearchError, request_id: &RequestId) -> Response {
  let (status, code, title, detail, retryable) = match error {
    KnowledgePathSearchError::InvalidRequest => (
      StatusCode::UNPROCESSABLE_ENTITY,
      "invalid_knowledge_path_request",
      "Invalid knowledge-path request",
      "The knowledge-path request is inconsistent.",
      false,
    ),
    KnowledgePathSearchError::ContentReleaseUnavailable => (
      StatusCode::CONFLICT,
      "content_release_unavailable",
      "Content release unavailable",
      "The requested immutable content release is no longer available.",
      false,
    ),
    KnowledgePathSearchError::DependencyUnavailable => (
      StatusCode::SERVICE_UNAVAILABLE,
      "knowledge_dependency_unavailable",
      "Knowledge dependency unavailable",
      "A required knowledge dependency is temporarily unavailable.",
      true,
    ),
    KnowledgePathSearchError::DeadlineExceeded => (
      StatusCode::GATEWAY_TIMEOUT,
      "deadline_exceeded",
      "Request deadline exceeded",
      "The knowledge-path request did not complete before its deadline.",
      true,
    ),
    KnowledgePathSearchError::InconsistentProof => (
      StatusCode::BAD_GATEWAY,
      "invalid_knowledge_proof",
      "Invalid knowledge proof",
      "A knowledge dependency returned an inconsistent immutable proof.",
      false,
    ),
    KnowledgePathSearchError::IncompleteSearch => (
      StatusCode::SERVICE_UNAVAILABLE,
      "knowledge_path_incomplete",
      "Knowledge-path search incomplete",
      "The bounded eligible search could not be completed.",
      true,
    ),
  };
  problem::response(
    status,
    code,
    title,
    detail,
    request_id,
    retryable,
    Vec::new(),
  )
}

#[cfg(test)]
mod tests {
  use std::sync::Mutex;

  use axum::{
    body::{to_bytes, Body},
    http::{header, Request},
    middleware, Router,
  };
  use serde_json::Value;
  use tower::ServiceExt;

  use super::*;
  use crate::{
    config::{ProviderApiKey, ProviderConfig, TranslationConfig},
    domain::{
      knowledge_hydration::HydratedAssertionProjection, knowledge_view::VerifiedKnowledgePath,
    },
    provider::TranslationService,
  };

  #[derive(Clone, Copy)]
  enum Mode {
    Connected,
    Empty,
    Error(KnowledgePathSearchError),
  }

  struct FakeUseCase {
    mode: Mode,
    observed_schema: Mutex<Option<String>>,
  }

  #[async_trait]
  impl KnowledgePathUseCase for FakeUseCase {
    async fn find(
      &self,
      context: &RequestContext,
      _cancellation: &CancellationSignal,
      request: KnowledgePathRequest,
    ) -> Result<KnowledgePathResult, KnowledgePathSearchError> {
      assert_eq!(context.content_release(), Some(&request.release.release_id));
      *self.observed_schema.lock().unwrap() = Some(context.schema_version().to_string());
      match self.mode {
        Mode::Error(error) => Err(error),
        Mode::Empty => Ok(KnowledgePathResult {
          request,
          outcome: KnowledgePathOutcome::NoVerifiedPath,
        }),
        Mode::Connected => {
          let projection = HydratedAssertionProjection::topology_fixture(
            request.from.node.clone(),
            request.to.node.clone(),
            id("edge-weather"),
            id("assertion-weather"),
            request.release.release_id.clone(),
          );
          let step =
            VerifiedKnowledgeStep::from_hydrated(projection, request.release.clone()).unwrap();
          Ok(KnowledgePathResult {
            request,
            outcome: KnowledgePathOutcome::Connected(vec![VerifiedKnowledgePath {
              order: 1,
              steps: vec![step],
            }]),
          })
        }
      }
    }
  }

  fn app(mode: Mode) -> Router {
    let use_case: Arc<dyn KnowledgePathUseCase> = Arc::new(FakeUseCase {
      mode,
      observed_schema: Mutex::new(None),
    });
    Router::new()
      .nest("/api/v1", router(use_case))
      .with_state(state())
      .layer(middleware::from_fn(
        super::super::super::request_context::establish,
      ))
      .layer(middleware::from_fn(
        super::super::super::request_id::propagate_request_id,
      ))
  }

  fn state() -> AppState {
    let provider = ProviderConfig {
      base_url: "http://127.0.0.1:1/v1".into(),
      model: "unused".into(),
      api_key: ProviderApiKey::new("unused"),
    };
    AppState::new(
      TranslationService::new(
        TranslationConfig {
          long_text_chars: 4_000,
          timeout_seconds: 1,
          max_retries: 0,
          retry_delay_ms: 0,
        },
        provider.clone(),
        provider,
      )
      .unwrap(),
    )
  }

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn request(body: impl Into<Body>) -> Request<Body> {
    Request::post("/api/v1/knowledge/paths")
      .header(header::CONTENT_TYPE, "application/json")
      .header("x-request-id", "knowledge-path-http-1")
      .body(body.into())
      .unwrap()
  }

  fn valid_body() -> &'static str {
    r#"{
      "from":{"kind":"concept","id":"concept-coriolis"},
      "to":{"kind":"concept","id":"concept-weather"},
      "target_language":"en",
      "content_release":"release-1",
      "canonical_schema_version":"canonical-v1"
    }"#
  }

  async fn body(response: Response) -> (String, Value) {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let raw = String::from_utf8(bytes.to_vec()).unwrap();
    let value = serde_json::from_str(&raw).unwrap();
    (raw, value)
  }

  #[tokio::test]
  async fn connected_result_uses_strict_envelope_and_hydrated_step_fields() {
    let response = app(Mode::Connected)
      .oneshot(request(valid_body()))
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()["x-request-id"], "knowledge-path-http-1");
    let (_, json) = body(response).await;
    assert_eq!(json["data"]["outcome"], "connected");
    assert_eq!(json["data"]["paths"][0]["order"], 1);
    assert_eq!(json["data"]["paths"][0]["steps"][0]["direction"], "forward");
    assert_eq!(
      json["data"]["paths"][0]["steps"][0]["relation"],
      "has_subtype"
    );
    assert_eq!(
      json["data"]["paths"][0]["steps"][0]["evidence_ids"],
      serde_json::json!(["evidence-fixture"])
    );
    assert_eq!(json["meta"]["request_id"], "knowledge-path-http-1");
    assert_eq!(json["meta"]["schema_version"], RESULT_SCHEMA_VERSION);
    assert_eq!(json["meta"]["content_release"], "release-1");
  }

  #[tokio::test]
  async fn complete_empty_result_is_a_successful_no_verified_path() {
    let response = app(Mode::Empty)
      .oneshot(request(valid_body()))
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let (_, json) = body(response).await;
    assert_eq!(json["data"]["outcome"], "no_verified_path");
    assert_eq!(json["data"]["paths"], serde_json::json!([]));
  }

  #[tokio::test]
  async fn rejects_unknown_fields_invalid_content_type_and_wrong_method() {
    for payload in [
      r#"{"from":{"kind":"concept","id":"a"},"to":{"kind":"concept","id":"b"},"target_language":"en","content_release":"r","canonical_schema_version":"v","extra":true}"#,
      r#"{"from":{"kind":"concept","id":"a","extra":true},"to":{"kind":"concept","id":"b"},"target_language":"en","content_release":"r","canonical_schema_version":"v"}"#,
    ] {
      let response = app(Mode::Empty).oneshot(request(payload)).await.unwrap();
      assert_eq!(response.status(), StatusCode::BAD_REQUEST);
      assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
      assert_eq!(body(response).await.1["code"], "invalid_json");
    }

    let no_content_type = app(Mode::Empty)
      .oneshot(
        Request::post("/api/v1/knowledge/paths")
          .body(Body::from(valid_body()))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(no_content_type.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(no_content_type).await.1["code"], "invalid_json");

    let wrong_method = app(Mode::Empty)
      .oneshot(
        Request::get("/api/v1/knowledge/paths")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(wrong_method.headers()[header::CACHE_CONTROL], "no-store");
  }

  #[tokio::test]
  async fn rejects_oversized_and_non_utf8_bodies_without_echoing_them() {
    let oversized = format!(r#"{{"padding":"{}"}}"#, "x".repeat(2_100_000));
    let response = app(Mode::Empty).oneshot(request(oversized)).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(body(response).await.1["code"], "payload_too_large");

    let response = app(Mode::Empty)
      .oneshot(request(Body::from(vec![0xff, 0xfe, 0xfd])))
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let (raw, json) = body(response).await;
    assert_eq!(json["code"], "invalid_json");
    assert!(!raw.contains("xff"));
  }

  #[tokio::test]
  async fn maps_closed_service_failures_without_echoing_request_content() {
    let cases = [
      (
        KnowledgePathSearchError::InvalidRequest,
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_knowledge_path_request",
      ),
      (
        KnowledgePathSearchError::ContentReleaseUnavailable,
        StatusCode::CONFLICT,
        "content_release_unavailable",
      ),
      (
        KnowledgePathSearchError::DependencyUnavailable,
        StatusCode::SERVICE_UNAVAILABLE,
        "knowledge_dependency_unavailable",
      ),
      (
        KnowledgePathSearchError::DeadlineExceeded,
        StatusCode::GATEWAY_TIMEOUT,
        "deadline_exceeded",
      ),
      (
        KnowledgePathSearchError::InconsistentProof,
        StatusCode::BAD_GATEWAY,
        "invalid_knowledge_proof",
      ),
      (
        KnowledgePathSearchError::IncompleteSearch,
        StatusCode::SERVICE_UNAVAILABLE,
        "knowledge_path_incomplete",
      ),
    ];
    for (error, status, code) in cases {
      let response = app(Mode::Error(error))
        .oneshot(request(valid_body()))
        .await
        .unwrap();
      assert_eq!(response.status(), status);
      assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
      let (raw, json) = body(response).await;
      assert_eq!(json["code"], code);
      assert!(!raw.contains("concept-coriolis"));
      assert!(!raw.contains("concept-weather"));
    }
  }
}
