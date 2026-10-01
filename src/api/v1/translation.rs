//! `POST /api/v1/translations` unified translation transport contract.

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
  application::translation::TranslationOrchestrationError,
  domain::request_context::RequestContext,
  domain::translation_turn::{
    ProjectedTranslationResult, ResponseLevel, TranslationInputKind, TranslationTurn,
    TranslationTurnRequest, TranslationTurnResult, TurnValidationError,
  },
};

use super::super::{problem, problem::FieldError, request_id::RequestId, AppState};
use super::knowledge_paths::RequestCancellationFactory;

pub(crate) async fn translate(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  Extension(context): Extension<RequestContext>,
  Extension(cancellations): Extension<RequestCancellationFactory>,
  payload: Result<Json<TranslationTurnRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(request) => request,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      return problem::payload_too_large(&request_id)
    }
    Err(_) => return invalid_json(&request_id),
  };
  let turn = match TranslationTurn::new(request) {
    Ok(turn) => turn,
    Err(TurnValidationError::TooLarge) => return problem::payload_too_large(&request_id),
    Err(TurnValidationError::Field(field)) => return invalid_translation_field(field, &request_id),
    Err(TurnValidationError::ConstraintConflict(field)) => {
      return constraint_conflict(field, &request_id)
    }
    Err(TurnValidationError::Unsupported(capability)) => {
      return unsupported_capability(capability, &request_id)
    }
    Err(TurnValidationError::UnsupportedImageMediaType) => {
      return problem::response(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "unsupported_image_media_type",
        "Unsupported image media type",
        "The declared inline-image media type is not supported.",
        &request_id,
        false,
        Vec::new(),
      )
    }
  };
  match turn.input_kind() {
    TranslationInputKind::Text
    | TranslationInputKind::Segments
    | TranslationInputKind::ImageRegions => {}
  }
  let Some(orchestrator) = state.translation_orchestrator() else {
    return translation_model_unavailable(&request_id);
  };
  if turn.guidance().max_alternatives != 0 && state.relationship_page_runtime().is_none() {
    return unsupported_capability("guidance.max_alternatives", &request_id);
  }

  let cancellation = cancellations.start();
  match orchestrator
    .translate(&context, cancellation.signal_arc(), &turn)
    .await
  {
    Ok(result) => {
      let result = if let Some(runtime) = state.relationship_page_runtime() {
        let signal = cancellation.signal_arc();
        match runtime.enrich(&context, &signal, &turn, result).await {
          Ok(result) => result,
          Err(_) => return relationship_page_unavailable(&request_id),
        }
      } else {
        result
      };
      if turn.guidance().max_alternatives != 0 && result.relationship_page.is_none() {
        return unsupported_capability("guidance.max_alternatives", &request_id);
      }
      success(result, &turn, &request_id)
    }
    Err(TranslationOrchestrationError::UnsupportedSourceLanguage) => {
      invalid_translation_field("source_language", &request_id)
    }
    Err(TranslationOrchestrationError::ChunkPlanLimit) => {
      invalid_translation_field("text", &request_id)
    }
    Err(TranslationOrchestrationError::UnsupportedInput) => {
      unsupported_capability("input", &request_id)
    }
    Err(TranslationOrchestrationError::UnsupportedImageGuidance) => {
      unsupported_capability("guidance", &request_id)
    }
    Err(TranslationOrchestrationError::UnsupportedStructuredFreshness) => {
      unsupported_capability("guidance.freshness", &request_id)
    }
    Err(TranslationOrchestrationError::InvalidImageData) => {
      invalid_translation_field("input.images.data", &request_id)
    }
    Err(TranslationOrchestrationError::ModelUnavailable) => {
      translation_model_unavailable(&request_id)
    }
    Err(TranslationOrchestrationError::InvalidModelOutput) => problem::response(
      StatusCode::BAD_GATEWAY,
      "invalid_model_output",
      "Invalid model output",
      "The translation model could not satisfy the bounded output contract.",
      &request_id,
      true,
      Vec::new(),
    ),
    Err(TranslationOrchestrationError::GuidanceViolation) => problem::response(
      StatusCode::BAD_GATEWAY,
      "guidance_postcondition_failed",
      "Guidance postcondition failed",
      "The model could not satisfy the deterministic translation guidance.",
      &request_id,
      false,
      Vec::new(),
    ),
    Err(TranslationOrchestrationError::LiveRetrievalUnavailable) => problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "live_retrieval_unavailable",
      "Live retrieval unavailable",
      "Required live retrieval is not configured for this service.",
      &request_id,
      true,
      Vec::new(),
    ),
    Err(TranslationOrchestrationError::DeadlineExceeded) => problem::response(
      StatusCode::GATEWAY_TIMEOUT,
      "deadline_exceeded",
      "Request deadline exceeded",
      "The request deadline was exhausted during translation.",
      &request_id,
      true,
      Vec::new(),
    ),
    Err(TranslationOrchestrationError::Cancelled) => problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "request_cancelled",
      "Request cancelled",
      "The translation request was cancelled before completion.",
      &request_id,
      true,
      Vec::new(),
    ),
  }
}

fn relationship_page_unavailable(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "relationship_page_unavailable",
    "Relationship page unavailable",
    "The release-pinned relationship-page authority could not complete this request.",
    request_id,
    true,
    Vec::new(),
  )
}

fn success(
  result: ProjectedTranslationResult,
  turn: &TranslationTurn,
  request_id: &RequestId,
) -> Response {
  if result.validate_for_turn(turn).is_err() {
    return problem::response(
      StatusCode::BAD_GATEWAY,
      "invalid_model_output",
      "Invalid model output",
      "The translation result did not satisfy the bounded output contract.",
      request_id,
      true,
      Vec::new(),
    );
  }
  let ProjectedTranslationResult {
    translation,
    metadata,
    external_sources,
    relationship_page,
    relationship_page_canonical_only,
  } = result;
  problem::no_store(
    (
      StatusCode::OK,
      Json(TranslationResponse {
        data: TranslationData {
          translation,
          external_sources,
          relationship_page: relationship_page
            .as_ref()
            .map(|page| relationship_page_wire(page, relationship_page_canonical_only)),
        },
        meta: TranslationMeta {
          request_id: request_id.as_str().to_string(),
          response_level: metadata.response_level,
          schema_version: metadata.schema_version,
          normalizer_version: metadata.normalizer_version,
          projection_version: metadata.projection_version,
          model_versions: metadata.model_versions,
          prompt_versions: metadata.prompt_versions,
          inference_profiles: metadata.inference_profiles,
          reasoning_escalated: metadata.reasoning_escalated,
          retrieval_version: metadata.retrieval_version,
          content_release: metadata.content_release,
        },
      }),
    )
      .into_response(),
  )
}

fn invalid_json(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_json",
    "Invalid JSON request",
    "The request body is not valid translation JSON.",
    request_id,
    false,
    Vec::new(),
  )
}

fn invalid_translation_field(field: &'static str, request_id: &RequestId) -> Response {
  let message = match field {
    "text" => "must be nonblank and processable within the bounded translation limits.",
    "source_language" => "must be one of `auto`, `en`, or `zh-CN` and detectable when automatic.",
    "target_language" => "must be one of `en` or `zh-CN`.",
    "response_level" => "must be one of `brief`, `standard`, or `full`.",
    "history" => "must contain only valid chronological minimal translation turns.",
    "input" => "must contain exactly one legacy text or tagged input value.",
    "input.text" => "must be nonblank and contain at most 131072 Unicode scalars.",
    "guidance.max_alternatives" => {
      "must be between zero and two; only zero is currently available."
    }
    "generation_context" => "must fit the bounded request-local model context after JSON encoding.",
    _ => "is invalid.",
  };
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_translation_request",
    "Invalid translation request",
    "One or more translation fields are invalid.",
    request_id,
    false,
    vec![FieldError::new(field, message)],
  )
}

fn constraint_conflict(field: &'static str, request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "constraint_conflict",
    "Translation constraints conflict",
    "The request contains translation constraints that cannot all be satisfied.",
    request_id,
    false,
    vec![FieldError::new(
      field,
      "contains contradictory constraints.",
    )],
  )
}

fn unsupported_capability(capability: &'static str, request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::NOT_IMPLEMENTED,
    "translation_capability_unavailable",
    "Translation capability unavailable",
    "The input is valid, but this translation capability is not available in the current runtime.",
    request_id,
    false,
    vec![FieldError::new(
      capability,
      "is not implemented by the current runtime.",
    )],
  )
}

fn translation_model_unavailable(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "translation_model_unavailable",
    "Translation model unavailable",
    "The translation model is temporarily unavailable.",
    request_id,
    true,
    Vec::new(),
  )
}

#[derive(Serialize)]
struct TranslationResponse {
  data: TranslationData,
  meta: TranslationMeta,
}

#[derive(Serialize)]
struct TranslationData {
  translation: TranslationTurnResult,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  external_sources: Vec<crate::domain::translation_turn::ExternalSourceReference>,
  #[serde(skip_serializing_if = "Option::is_none")]
  relationship_page: Option<Value>,
}

#[derive(Serialize)]
struct TranslationMeta {
  request_id: String,
  response_level: ResponseLevel,
  schema_version: &'static str,
  normalizer_version: &'static str,
  projection_version: &'static str,
  model_versions: Vec<String>,
  prompt_versions: Vec<String>,
  inference_profiles: Vec<crate::domain::model_runtime::GenerationProfile>,
  reasoning_escalated: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  retrieval_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  content_release: Option<String>,
}

fn relationship_page_wire(
  page: &crate::domain::relationship_page::ProjectedRelationshipPage,
  canonical_only: bool,
) -> Value {
  use crate::domain::relationship_page::RelationshipPageSummaryRef;
  let summary = match &page.summary {
    RelationshipPageSummaryRef::BasicCard { sense_id, release } => json!({
      "type":"basic_card", "sense_id":sense_id.as_str(), "release":release_wire(release)
    }),
    RelationshipPageSummaryRef::Concept { concept, release } => json!({
      "type":"concept", "concept":node_wire(concept), "release":release_wire(release)
    }),
  };
  json!({
    "status": if canonical_only { "canonical_only" } else { "complete" },
    "summary": summary,
    "domain": page.domain.as_ref().map(|domain| json!({
      "assessment": domain.assessment,
      "profiles": domain.profiles.iter().map(|profile| json!({
        "domain_id":profile.domain_id.as_str(),
        "available_fact_families":profile.available_fact_families.iter().map(fact_family_wire).collect::<Vec<_>>(),
        "languages":profile.languages.iter().map(|value| value.as_str()).collect::<Vec<_>>(),
        "verified_fact_count":profile.verified_fact_count,
        "coverage":profile.coverage,
        "release":release_wire(&profile.release)
      })).collect::<Vec<_>>()
    })),
    "groups":page.groups.iter().map(|group| json!({
      "kind":group_wire(group.kind),
      "relationships":group.relationships.iter().map(|relationship| json!({
        "node":node_wire(&relationship.node),
        "assertion_id":relationship.assertion_id.as_str(),
        "path_to_root":path_wire(&relationship.path_to_root),
        "usefulness":relationship.usefulness
      })).collect::<Vec<_>>()
    })).collect::<Vec<_>>(),
    "paths":page.paths.iter().map(|path| json!({"label":path.label,"path":path_wire(&path.path)})).collect::<Vec<_>>(),
    "scales":page.scales.iter().map(|scale| json!({
      "scale_id":scale.scale.scale_id.as_str(), "revision":scale.scale.revision,
      "dimension":scale.scale.dimension, "direction":scale_direction_wire(scale.scale.direction),
      "domain_ids":scale.scale.domain_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
      "conditions":scale.scale.conditions.iter().map(condition_wire).collect::<Vec<_>>(),
      "members":scale.scale.members.iter().map(|member| json!({"node_id":member.node_id.as_str(),"position":member.position})).collect::<Vec<_>>(),
      "evidence_ids":scale.scale.evidence_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
      "group":group_wire(scale.group), "release":release_wire(&scale.release)
    })).collect::<Vec<_>>(),
    "generated_examples":page.generated_examples.iter().map(|item| json!({"source_text":item.source_text,"translated_text":item.translated_text,"generated":item.generated})).collect::<Vec<_>>(),
    "inferred_explanations":page.inferred_explanations.iter().map(|item| json!({"text":item.text,"supporting_assertion_ids":item.supporting_assertion_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
    "exploratory_items":page.exploratory_items.iter().map(|item| json!({"node":node_wire(&item.node),"reason":item.reason})).collect::<Vec<_>>(),
    "alternatives":page.alternatives.iter().map(|item| json!({"translation_id":item.translation_id,"translation_order":item.translation_order,"text":item.text,"dimension":alternative_wire(item.dimension),"usefulness":item.usefulness,"consequence":item.consequence,"usefulness_reason":item.usefulness_reason})).collect::<Vec<_>>(),
    "release":release_wire(&page.release),
    "versions": {"schema_version":page.versions.schema_version,"policy_version":page.versions.policy_version,"projection_version":page.versions.projection_version,"router_version":page.versions.router_version,"resolver_version":page.versions.resolver_version,"domain_assessor_version":page.versions.domain_assessor_version,"ranker_version":page.versions.ranker_version,"composer_version":page.versions.composer_version,"prompt_version":page.versions.prompt_version,"repair_policy_version":page.versions.repair_policy_version}
  })
}

fn release_wire(release: &crate::domain::canonical::CanonicalReleasePin) -> Value {
  json!({"release_id":release.release_id.as_str(),"canonical_schema_version":release.canonical_schema_version})
}

fn node_wire(node: &crate::domain::assertion::CanonicalNodeId) -> Value {
  json!({"kind":node.family(),"id":node.id().as_str()})
}

fn fact_family_wire(value: &crate::domain::relationship_page::PageFactFamily) -> &'static str {
  use crate::domain::relationship_page::PageFactFamily::*;
  match value {
    Definition => "definition",
    Relationship => "relationship",
    Usage => "usage",
    SemanticScale => "semantic_scale",
  }
}

fn group_wire(value: crate::domain::relationship_page::RelationshipGroupKind) -> &'static str {
  use crate::domain::relationship_page::RelationshipGroupKind::*;
  match value {
    Meaning => "meaning",
    Terminology => "terminology",
    TaxonomyOrDegree => "taxonomy_or_degree",
    Contrast => "contrast",
    Valency => "valency",
    Collocation => "collocation",
    Suitability => "suitability",
    Morphology => "morphology",
    CulturalExtension => "cultural_extension",
    Mechanism => "mechanism",
    Phenomenon => "phenomenon",
    Application => "application",
    Measurement => "measurement",
    Standard => "standard",
    UsageConvention => "usage_convention",
  }
}

fn alternative_wire(value: crate::domain::relationship_page::AlternativeDimension) -> &'static str {
  use crate::domain::relationship_page::AlternativeDimension::*;
  match value {
    Degree => "degree",
    Formality => "formality",
    Approval => "approval",
    Danger => "danger",
    Domain => "domain",
    Syntax => "syntax",
    Register => "register",
    Meaning => "meaning",
  }
}

fn scale_direction_wire(
  value: crate::domain::knowledge_hydration::SemanticScaleDirection,
) -> &'static str {
  match value {
    crate::domain::knowledge_hydration::SemanticScaleDirection::Increasing => "increasing",
    crate::domain::knowledge_hydration::SemanticScaleDirection::Decreasing => "decreasing",
  }
}

fn condition_wire(condition: &crate::domain::knowledge_hydration::KnowledgeCondition) -> Value {
  json!({"condition_id":condition.condition_id.as_str(),"condition_type":condition.condition_type,"parameter_ids":condition.parameter_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()})
}

fn assertion_condition_wire(condition: &crate::domain::assertion::AssertionCondition) -> Value {
  json!({"condition_id":condition.condition_id.as_str(),"condition_type":condition.condition_type,"parameter_ids":condition.parameter_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()})
}

fn path_wire(path: &crate::domain::knowledge_view::UsefulRootPath) -> Value {
  json!({"root":node_wire(&path.root),"item":node_wire(&path.item),"steps":path.steps.iter().map(|step| {
    let assertion=step.projection().assertion(); let traversal=step.projection().traversal();
    json!({"edge_id":traversal.edge_id.as_str(),"relationship_revision":traversal.relationship_revision,"assertion_id":assertion.assertion_id.as_str(),"assertion_revision":assertion.assertion_revision,"traversal_id":traversal.traversal_id.as_str(),"relation":traversal.relation_type.rule().require_qdrant_wire_name().ok(),"relation_registry_revision":traversal.relation_registry_revision,"source":node_wire(&traversal.source),"target":node_wire(&traversal.target),"conditions":assertion.conditions.iter().map(assertion_condition_wire).collect::<Vec<_>>(),"evidence_ids":assertion.evidence_ids.iter().map(|id|id.as_str()).collect::<Vec<_>>(),"release":release_wire(step.release())})
  }).collect::<Vec<_>>()})
}
