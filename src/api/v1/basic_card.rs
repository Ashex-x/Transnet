//! Frozen MySQL-only BasicCard lookup and release-pinned sense HTTP contracts.

use std::collections::BTreeMap;

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
  application::canonical_read::{CanonicalReadOutcome, CanonicalReadService},
  domain::{
    canonical::{
      CanonicalId, CanonicalReleasePin, EvidenceConfidence, EvidenceKind, EvidenceUse, FormKind,
      LanguageTag, LexicalPartOfSpeech,
    },
    canonical_translation::CanonicalTranslationRevision,
    lookup_card::{
      CanonicalLookupCardAssertion, CanonicalLookupCardCandidate, CanonicalLookupCardCoverage,
      CanonicalLookupCardCoverageState, CanonicalLookupCardEvidence, CanonicalLookupCardForm,
      CanonicalLookupCardSectionCoverage,
    },
    observability::{LookupStage, MetricEvent, MetricOutcome},
    retrieval::RetrievalRequest,
  },
  ports::canonical_read::{CanonicalReadContext, CanonicalReadError},
};

use super::{
  super::{problem, problem::FieldError, request_id::RequestId, AppState},
  sense::CanonicalSenseDetailsResponse,
};

const MAX_QUERY_CHARS: usize = 100;
const MAX_PUBLIC_RESPONSE_BYTES: usize = 1_048_576;
const MAX_TRANSLATIONS_PER_MATCH: usize = 8;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BasicCardLookupRequest {
  query: String,
  source_language: String,
  target_language: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PinnedSenseRequest {
  sense_id: String,
  content_release: String,
  canonical_schema_version: String,
  target_language: String,
}

pub(super) async fn lookup(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  payload: Result<Json<BasicCardLookupRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(value) => value,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      record_lookup_validation(&state, MetricOutcome::Rejected);
      return problem::payload_too_large(&request_id);
    }
    Err(_) => {
      record_lookup_validation(&state, MetricOutcome::Rejected);
      return invalid_json(&request_id, "BasicCard lookup");
    }
  };
  if request.query.trim().is_empty() || request.query.chars().count() > MAX_QUERY_CHARS {
    record_lookup_validation(&state, MetricOutcome::Rejected);
    return invalid_field(
      &request_id,
      "query",
      "must be nonblank and at most 100 Unicode characters.",
    );
  }
  let source_language = match LanguageTag::parse(&request.source_language) {
    Ok(value) => value,
    Err(_) => {
      record_lookup_validation(&state, MetricOutcome::Rejected);
      return invalid_field(
        &request_id,
        "source_language",
        "must be a valid BCP-47 language tag.",
      );
    }
  };
  let target_language = match LanguageTag::parse(&request.target_language) {
    Ok(value) => value,
    Err(_) => {
      record_lookup_validation(&state, MetricOutcome::Rejected);
      return invalid_field(
        &request_id,
        "target_language",
        "must be a valid BCP-47 language tag.",
      );
    }
  };
  let retrieval = match RetrievalRequest::for_public_api(&request.query, source_language) {
    Ok(value) => value,
    Err(_) => {
      record_lookup_validation(&state, MetricOutcome::Rejected);
      return invalid_field(
        &request_id,
        "query",
        "must contain a valid canonical lookup value.",
      );
    }
  };
  record_lookup_validation(&state, MetricOutcome::Succeeded);
  let Some((service, context)) = canonical_dependency(&state, &request_id) else {
    return dependency_unavailable(&request_id);
  };
  match service
    .resolve(&context, retrieval, &request.query, target_language)
    .await
  {
    Ok(outcome) => bounded_json(
      BasicCardResponse::from_outcome(outcome, &request_id),
      &request_id,
    ),
    Err(error) => map_read_error(error, &request_id, false),
  }
}

fn record_lookup_validation(state: &AppState, outcome: MetricOutcome) {
  if let Some(metrics) = state.canonical_lookup_metrics() {
    metrics.dispatch(MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome,
    });
  }
}

pub(super) async fn sense(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  payload: Result<Json<PinnedSenseRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(value) => value,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      return problem::payload_too_large(&request_id)
    }
    Err(_) => return invalid_json(&request_id, "pinned sense"),
  };
  let sense_id = match CanonicalId::new(&request.sense_id) {
    Ok(value) if request.sense_id.chars().count() <= 256 => value,
    _ => {
      return invalid_field(
        &request_id,
        "sense_id",
        "must be a valid canonical sense identifier.",
      )
    }
  };
  let release_id = match CanonicalId::new(&request.content_release) {
    Ok(value) => value,
    Err(_) => {
      return invalid_field(
        &request_id,
        "content_release",
        "must be a valid immutable release identifier.",
      )
    }
  };
  let Some(pin) = CanonicalReleasePin::new(release_id, request.canonical_schema_version) else {
    return invalid_field(
      &request_id,
      "canonical_schema_version",
      "must identify the canonical schema returned by lookup.",
    );
  };
  let target_language = match LanguageTag::parse(&request.target_language) {
    Ok(value) => value,
    Err(_) => {
      return invalid_field(
        &request_id,
        "target_language",
        "must be a valid BCP-47 language tag.",
      )
    }
  };
  let Some((service, context)) = canonical_dependency(&state, &request_id) else {
    return dependency_unavailable(&request_id);
  };
  match service
    .read_pinned_sense(
      &context,
      &pin,
      sense_id,
      target_language,
      EvidenceUse::ApiRedistribution,
    )
    .await
  {
    Ok(details) => bounded_json(
      PinnedSenseResponse {
        data: PinnedSenseData {
          sense: CanonicalSenseDetailsResponse::pinned_value(&details),
        },
        meta: CanonicalMeta::new(&request_id, &pin),
      },
      &request_id,
    ),
    Err(error) => map_read_error(error, &request_id, true),
  }
}

fn canonical_dependency(
  state: &AppState,
  request_id: &RequestId,
) -> Option<(std::sync::Arc<CanonicalReadService>, CanonicalReadContext)> {
  let service = state.canonical_read_service()?.clone();
  let timeout = state.canonical_read_timeout()?;
  let deadline = OffsetDateTime::now_utc() + time::Duration::try_from(timeout).ok()?;
  let deadline_at = deadline.format(&Rfc3339).ok()?;
  Some((
    service,
    CanonicalReadContext {
      request_id: request_id.as_str().to_string(),
      deadline_at,
      timeout,
    },
  ))
}

fn bounded_json<T: Serialize>(value: T, request_id: &RequestId) -> Response {
  if serde_json::to_vec(&value).map_or(true, |body| body.len() > MAX_PUBLIC_RESPONSE_BYTES) {
    return problem::response(
      StatusCode::BAD_GATEWAY,
      "canonical_response_too_large",
      "Canonical response too large",
      "The canonical authority response exceeds the public response bound.",
      request_id,
      false,
      Vec::new(),
    );
  }
  problem::no_store((StatusCode::OK, Json(value)).into_response())
}

fn invalid_json(request_id: &RequestId, operation: &'static str) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_json",
    "Invalid JSON request",
    format!("The request body is not valid {operation} JSON."),
    request_id,
    false,
    Vec::new(),
  )
}

fn invalid_field(request_id: &RequestId, field: &'static str, message: &'static str) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_canonical_request",
    "Invalid canonical request",
    "One or more canonical request fields are invalid.",
    request_id,
    false,
    vec![FieldError::new(field, message)],
  )
}

fn dependency_unavailable(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "canonical_dependency_unavailable",
    "Canonical dependency unavailable",
    "The canonical content capability is not available.",
    request_id,
    true,
    Vec::new(),
  )
}

fn map_read_error(error: CanonicalReadError, request_id: &RequestId, sense_read: bool) -> Response {
  let (status, code, title, detail, retryable) = match error {
    CanonicalReadError::InvalidRequest => (
      StatusCode::UNPROCESSABLE_ENTITY,
      "invalid_canonical_request",
      "Invalid canonical request",
      "The canonical request is invalid.",
      false,
    ),
    CanonicalReadError::NotFound if sense_read => (
      StatusCode::NOT_FOUND,
      "canonical_sense_not_found",
      "Canonical sense not found",
      "The requested sense is not present in the pinned release.",
      false,
    ),
    CanonicalReadError::NotFound => (
      StatusCode::SERVICE_UNAVAILABLE,
      "canonical_dependency_unavailable",
      "Canonical dependency unavailable",
      "No active canonical release is available.",
      true,
    ),
    CanonicalReadError::ContentReleaseUnavailable => (
      StatusCode::CONFLICT,
      "content_release_unavailable",
      "Content release unavailable",
      "The requested immutable content release is no longer available.",
      false,
    ),
    CanonicalReadError::SchemaIncompatible => (
      StatusCode::BAD_GATEWAY,
      "canonical_schema_incompatible",
      "Canonical schema incompatible",
      "The canonical dependency uses an incompatible schema.",
      false,
    ),
    CanonicalReadError::Unavailable => (
      StatusCode::SERVICE_UNAVAILABLE,
      "canonical_dependency_unavailable",
      "Canonical dependency unavailable",
      "The canonical dependency is temporarily unavailable.",
      true,
    ),
    CanonicalReadError::Timeout => (
      StatusCode::SERVICE_UNAVAILABLE,
      "canonical_dependency_timeout",
      "Canonical dependency timeout",
      "The canonical dependency did not respond before the request deadline.",
      true,
    ),
    CanonicalReadError::InconsistentData => (
      StatusCode::BAD_GATEWAY,
      "invalid_canonical_response",
      "Invalid canonical response",
      "The canonical dependency returned an incompatible response.",
      false,
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

#[derive(Serialize)]
struct BasicCardResponse {
  data: BasicCardData,
  meta: CanonicalMeta,
}
#[derive(Serialize)]
struct BasicCardData {
  resolution: &'static str,
  matches: Vec<BasicCardMatch>,
  coverage: CoverageResponse,
}
#[derive(Serialize)]
struct CanonicalMeta {
  request_id: String,
  content_release: String,
  canonical_schema_version: String,
}
impl CanonicalMeta {
  fn new(request_id: &RequestId, pin: &CanonicalReleasePin) -> Self {
    Self {
      request_id: request_id.as_str().to_string(),
      content_release: pin.release_id.to_string(),
      canonical_schema_version: pin.canonical_schema_version.clone(),
    }
  }
}

impl BasicCardResponse {
  fn from_outcome(outcome: CanonicalReadOutcome, request_id: &RequestId) -> Self {
    let coverage = CoverageResponse::from(&outcome.card.coverage);
    let matches = outcome
      .card
      .candidates
      .iter()
      .filter_map(|candidate| {
        let translations = outcome
          .translations
          .iter()
          .filter(|translation| {
            translation
              .meaning_scope()
              .is_some_and(|scope| scope.sense_id() == &candidate.sense.id)
          })
          .take(MAX_TRANSLATIONS_PER_MATCH)
          .map(TranslationResponse::from)
          .collect::<Vec<_>>();
        (candidate.sense.definition.is_some() || !translations.is_empty())
          .then(|| BasicCardMatch::new(candidate, translations))
      })
      .collect::<Vec<_>>();
    let resolution = match matches.len() {
      0 => "not_found",
      1 => "resolved",
      _ => "clarification_required",
    };
    Self {
      data: BasicCardData {
        resolution,
        matches,
        coverage,
      },
      meta: CanonicalMeta::new(request_id, &outcome.pin),
    }
  }
}

#[derive(Serialize)]
struct BasicCardMatch {
  rank: usize,
  lexeme: LexemeResponse,
  sense: SenseResponse,
  translations: Vec<TranslationResponse>,
  forms: Vec<FormResponse>,
  evidence: Vec<EvidenceResponse>,
}
impl BasicCardMatch {
  fn new(value: &CanonicalLookupCardCandidate, translations: Vec<TranslationResponse>) -> Self {
    let evidence = value
      .sense
      .definition
      .iter()
      .flat_map(|assertion| &assertion.evidence)
      .chain(value.forms.iter().flat_map(|form| &form.assertion.evidence))
      .map(|evidence| (evidence.id.to_string(), EvidenceResponse::from(evidence)))
      .collect::<BTreeMap<_, _>>()
      .into_values()
      .collect();
    Self {
      rank: value.rank,
      lexeme: LexemeResponse {
        id: value.lexeme.id.to_string(),
        lemma: value.lexeme.lemma.clone(),
        language: value.lexeme.language.to_string(),
        part_of_speech: part_of_speech(value.lexeme.part_of_speech),
      },
      sense: SenseResponse {
        id: value.sense.id.to_string(),
        sense_key: value.sense.sense_key.clone(),
        definition: value.sense.definition.as_ref().map(AssertionResponse::from),
      },
      translations,
      forms: value.forms.iter().map(FormResponse::from).collect(),
      evidence,
    }
  }
}

#[derive(Serialize)]
struct LexemeResponse {
  id: String,
  lemma: String,
  language: String,
  part_of_speech: &'static str,
}
#[derive(Serialize)]
struct SenseResponse {
  id: String,
  sense_key: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  definition: Option<AssertionResponse>,
}
#[derive(Serialize)]
struct TranslationResponse {
  id: String,
  revision: u64,
  text: String,
}
impl From<&CanonicalTranslationRevision> for TranslationResponse {
  fn from(value: &CanonicalTranslationRevision) -> Self {
    Self {
      id: value.id().to_string(),
      revision: value.revision().get(),
      text: value.target_text().to_string(),
    }
  }
}
#[derive(Serialize)]
struct FormResponse {
  id: String,
  kind: &'static str,
  #[serde(skip_serializing_if = "Option::is_none")]
  morphology: Option<String>,
  assertion: AssertionResponse,
}
impl From<&CanonicalLookupCardForm> for FormResponse {
  fn from(value: &CanonicalLookupCardForm) -> Self {
    Self {
      id: value.id.to_string(),
      kind: form_kind(value.kind),
      morphology: value.morphology.clone(),
      assertion: AssertionResponse::from(&value.assertion),
    }
  }
}
#[derive(Serialize)]
struct AssertionResponse {
  text: String,
  evidence: Vec<EvidenceResponse>,
}
impl From<&CanonicalLookupCardAssertion> for AssertionResponse {
  fn from(value: &CanonicalLookupCardAssertion) -> Self {
    Self {
      text: value.text.clone(),
      evidence: value.evidence.iter().map(EvidenceResponse::from).collect(),
    }
  }
}
#[derive(Serialize)]
struct EvidenceResponse {
  id: String,
  kind: &'static str,
  confidence: &'static str,
  text: String,
  source: SourceResponse,
}
impl From<&CanonicalLookupCardEvidence> for EvidenceResponse {
  fn from(value: &CanonicalLookupCardEvidence) -> Self {
    Self {
      id: value.id.to_string(),
      kind: evidence_kind(value.kind),
      confidence: evidence_confidence(value.confidence),
      text: value.text.clone(),
      source: SourceResponse {
        source_id: value.provenance.source_id.to_string(),
        attribution: value.provenance.attribution.clone(),
        source_reference: value.provenance.source_reference.clone(),
        language: value.provenance.language.to_string(),
      },
    }
  }
}
#[derive(Serialize)]
struct SourceResponse {
  source_id: String,
  attribution: String,
  source_reference: String,
  language: String,
}

#[derive(Serialize)]
struct CoverageResponse {
  retrieval: SectionCoverage,
  lexemes: SectionCoverage,
  parts_of_speech: SectionCoverage,
  senses: SectionCoverage,
  definitions: SectionCoverage,
  forms: SectionCoverage,
  evidence: SectionCoverage,
}
impl From<&CanonicalLookupCardCoverage> for CoverageResponse {
  fn from(value: &CanonicalLookupCardCoverage) -> Self {
    Self {
      retrieval: (&value.retrieval).into(),
      lexemes: (&value.lexemes).into(),
      parts_of_speech: (&value.parts_of_speech).into(),
      senses: (&value.senses).into(),
      definitions: (&value.definitions).into(),
      forms: (&value.forms).into(),
      evidence: (&value.evidence).into(),
    }
  }
}
#[derive(Serialize)]
struct SectionCoverage {
  state: &'static str,
  available_items: usize,
  missing_items: usize,
  filtered_items: usize,
  truncated_items: usize,
}
impl From<&CanonicalLookupCardSectionCoverage> for SectionCoverage {
  fn from(value: &CanonicalLookupCardSectionCoverage) -> Self {
    Self {
      state: coverage_state(value.state),
      available_items: value.available_items,
      missing_items: value.missing_items,
      filtered_items: value.filtered_items,
      truncated_items: value.truncated_items,
    }
  }
}

#[derive(Serialize)]
struct PinnedSenseResponse {
  data: PinnedSenseData,
  meta: CanonicalMeta,
}
#[derive(Serialize)]
struct PinnedSenseData {
  sense: serde_json::Value,
}

fn part_of_speech(value: LexicalPartOfSpeech) -> &'static str {
  match value {
    LexicalPartOfSpeech::Noun => "noun",
    LexicalPartOfSpeech::Verb => "verb",
    LexicalPartOfSpeech::Adjective => "adjective",
    LexicalPartOfSpeech::Adverb => "adverb",
    LexicalPartOfSpeech::Pronoun => "pronoun",
    LexicalPartOfSpeech::Preposition => "preposition",
    LexicalPartOfSpeech::Conjunction => "conjunction",
    LexicalPartOfSpeech::Determiner => "determiner",
    LexicalPartOfSpeech::Interjection => "interjection",
    LexicalPartOfSpeech::Numeral => "numeral",
    LexicalPartOfSpeech::Other => "other",
  }
}
fn form_kind(value: FormKind) -> &'static str {
  match value {
    FormKind::Lemma => "lemma",
    FormKind::SpellingVariant => "spelling_variant",
    FormKind::Inflection => "inflection",
    FormKind::Phrase => "phrase",
    FormKind::Alias => "alias",
  }
}
fn evidence_kind(value: EvidenceKind) -> &'static str {
  match value {
    EvidenceKind::Definition => "definition",
    EvidenceKind::LocalizedGloss => "localized_gloss",
    EvidenceKind::Example => "example",
    EvidenceKind::Pronunciation => "pronunciation",
    EvidenceKind::Usage => "usage",
    EvidenceKind::Etymology => "etymology",
    EvidenceKind::Other => "other",
  }
}
fn evidence_confidence(value: EvidenceConfidence) -> &'static str {
  match value {
    EvidenceConfidence::High => "high",
    EvidenceConfidence::Medium => "medium",
    EvidenceConfidence::Low => "low",
  }
}
fn coverage_state(value: CanonicalLookupCardCoverageState) -> &'static str {
  match value {
    CanonicalLookupCardCoverageState::Available => "available",
    CanonicalLookupCardCoverageState::Missing => "missing",
    CanonicalLookupCardCoverageState::Filtered => "filtered",
    CanonicalLookupCardCoverageState::VectorDegraded => "vector_degraded",
  }
}
