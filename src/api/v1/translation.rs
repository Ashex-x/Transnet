//! `POST /api/v1/translations` unified translation transport contract.

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;

use crate::{
  application::translation::TranslationOrchestrationError,
  domain::translation_turn::{
    ProjectedTranslationResult, ResponseLevel, TranslationInputKind, TranslationTurn,
    TranslationTurnRequest, TranslationTurnResult, TurnValidationError,
  },
  domain::{model_runtime::CancellationSignal, request_context::RequestContext},
};

use super::super::{problem, problem::FieldError, request_id::RequestId, AppState};

pub(crate) async fn translate(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  Extension(context): Extension<RequestContext>,
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
    TranslationInputKind::Segments => return unsupported_capability("segments", &request_id),
    TranslationInputKind::ImageRegions => {
      return unsupported_capability("image_regions", &request_id)
    }
    TranslationInputKind::Text => {}
  }
  if turn.requires_guidance_execution() {
    return unsupported_capability("guidance", &request_id);
  }
  let Some(orchestrator) = state.translation_orchestrator() else {
    return translation_model_unavailable(&request_id);
  };

  match orchestrator
    .translate(
      &context,
      std::sync::Arc::new(CancellationSignal::default()),
      &turn,
    )
    .await
  {
    Ok(result) => success(result, &turn, &request_id),
    Err(TranslationOrchestrationError::UnsupportedSourceLanguage) => {
      invalid_translation_field("source_language", &request_id)
    }
    Err(TranslationOrchestrationError::ChunkPlanLimit) => {
      invalid_translation_field("text", &request_id)
    }
    Err(TranslationOrchestrationError::UnsupportedInput) => {
      unsupported_capability("input", &request_id)
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
  } = result;
  problem::no_store(
    (
      StatusCode::OK,
      Json(TranslationResponse {
        data: TranslationData {
          translation,
          external_sources,
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
