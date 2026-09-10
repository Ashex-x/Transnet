//! `GET /v1/senses/{sense_id}` active-release-pinned canonical detail contract.
//!
//! This conditional route exposes one complete, bounded canonical sense-detail aggregate only
//! after its injected active-content reader selects a safe release and every assertion permits
//! public API redistribution. It contains no cache, persistence, generator invocation,
//! authentication, or source-import behavior.

use axum::{
  extract::{rejection::PathRejection, Extension, Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde::{Deserialize, Serialize};

use crate::{
  api::{
    problem::{self, FieldError},
    request_id::RequestId,
    AppState,
  },
  application::canonical_sense_details::{
    ActiveCanonicalSenseDetails, ActiveCanonicalSenseDetailsError, CanonicalSenseDetailsReadError,
  },
  domain::{
    canonical::{CanonicalId, EvidenceConfidence, EvidenceKind},
    canonical_content::{
      CanonicalDetailKind, CanonicalFactualAssertion, CanonicalSenseDetails,
      CollocationConstruction, CollocationRole, EtymologyKind, EtymologyScope, GrammarPatternKind,
      HistoricalRange, LearnerPitfallKind, PronunciationNotation, PronunciationScope,
      SenseHistoryEventKind, UsageLabelKind,
    },
  },
  ports::{
    active_content_reader::ActiveContentReaderError,
    canonical_sense_details_repository::CanonicalSenseDetailsRepositoryError,
  },
};

const MAX_SENSE_ID_LENGTH: usize = 256;

/// Reads one public canonical sense-details aggregate when the conditional service is present.
pub(super) async fn read(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  path: Result<Path<SenseDetailsPath>, PathRejection>,
) -> Response {
  let Path(path) = match path {
    Ok(path) => path,
    Err(_) => return malformed_path_problem(&request_id),
  };
  let sense_id = match parse_sense_id(&path.sense_id) {
    Ok(sense_id) => sense_id,
    Err(error) => return invalid_sense_request(error, &request_id),
  };
  let Some(service) = state.canonical_sense_details_service() else {
    return sense_details_unavailable(&request_id, false);
  };

  match service.read(sense_id).await {
    Ok(Some(outcome)) => {
      let response = CanonicalSenseDetailsResponse::from_outcome(outcome);
      problem::no_store((StatusCode::OK, Json(response)).into_response())
    }
    Ok(None) => sense_not_found(&request_id),
    Err(error) => {
      tracing::warn!(
        error_category = sense_details_error_category(&error),
        "canonical sense-details dependency could not serve a request"
      );
      sense_details_unavailable(&request_id, error.is_retryable())
    }
  }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SenseDetailsPath {
  sense_id: String,
}

#[derive(Debug)]
struct SenseRequestValidation {
  field: &'static str,
  message: &'static str,
}

impl SenseRequestValidation {
  const fn new(field: &'static str, message: &'static str) -> Self {
    Self { field, message }
  }
}

fn parse_sense_id(value: &str) -> Result<CanonicalId, SenseRequestValidation> {
  if value.chars().count() > MAX_SENSE_ID_LENGTH {
    return Err(SenseRequestValidation::new(
      "sense_id",
      "must not exceed the canonical sense identifier length limit.",
    ));
  }
  CanonicalId::new(value).map_err(|_| {
    SenseRequestValidation::new("sense_id", "must be a nonblank canonical sense identifier.")
  })
}

fn malformed_path_problem(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_sense_request",
    "Invalid canonical sense request",
    "The canonical sense path is malformed.",
    request_id,
    false,
    Vec::new(),
  )
}

fn invalid_sense_request(error: SenseRequestValidation, request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_sense_request",
    "Invalid canonical sense request",
    "The canonical sense identifier is invalid.",
    request_id,
    false,
    vec![FieldError::new(error.field, error.message)],
  )
}

fn sense_not_found(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::NOT_FOUND,
    "canonical_sense_not_found",
    "Canonical sense not found",
    "The requested canonical sense is not available in the active content.",
    request_id,
    false,
    Vec::new(),
  )
}

fn sense_details_unavailable(request_id: &RequestId, retryable: bool) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "canonical_sense_details_unavailable",
    "Canonical sense details unavailable",
    "Canonical sense details are temporarily unavailable.",
    request_id,
    retryable,
    Vec::new(),
  )
}

fn sense_details_error_category(error: &ActiveCanonicalSenseDetailsError) -> &'static str {
  match error {
    ActiveCanonicalSenseDetailsError::ActiveContent(ActiveContentReaderError::Unavailable) => {
      "active_content_unavailable"
    }
    ActiveCanonicalSenseDetailsError::ActiveContent(ActiveContentReaderError::InconsistentData) => {
      "active_content_inconsistent"
    }
    ActiveCanonicalSenseDetailsError::Details(CanonicalSenseDetailsReadError::Repository(
      CanonicalSenseDetailsRepositoryError::Unavailable,
    )) => "canonical_sense_details_repository_unavailable",
    ActiveCanonicalSenseDetailsError::Details(CanonicalSenseDetailsReadError::Repository(
      CanonicalSenseDetailsRepositoryError::InconsistentData,
    )) => "canonical_sense_details_repository_inconsistent",
  }
}

#[derive(Debug, Serialize)]
struct CanonicalSenseDetailsResponse {
  schema_version: &'static str,
  target: CanonicalSenseTargetResponse,
  localized_glosses: Vec<LocalizedGlossResponse>,
  pronunciations: Vec<PronunciationResponse>,
  usage_labels: Vec<UsageLabelResponse>,
  grammar_patterns: Vec<GrammarPatternResponse>,
  collocations: Vec<CollocationResponse>,
  examples: Vec<ExampleResponse>,
  pitfalls: Vec<PitfallResponse>,
  etymologies: Vec<EtymologyResponse>,
  history: Vec<SenseHistoryResponse>,
  provenance: CanonicalSenseDetailsProvenanceResponse,
}

impl CanonicalSenseDetailsResponse {
  fn from_outcome(outcome: ActiveCanonicalSenseDetails) -> Self {
    let content = outcome.content();
    let details = outcome.details();
    Self {
      schema_version: "1.0",
      target: CanonicalSenseTargetResponse::from(details),
      localized_glosses: details
        .localized_glosses()
        .iter()
        .map(LocalizedGlossResponse::from)
        .collect(),
      pronunciations: details
        .pronunciations()
        .iter()
        .map(PronunciationResponse::from)
        .collect(),
      usage_labels: details
        .usage_labels()
        .iter()
        .map(UsageLabelResponse::from)
        .collect(),
      grammar_patterns: details
        .grammar_patterns()
        .iter()
        .map(GrammarPatternResponse::from)
        .collect(),
      collocations: details
        .collocations()
        .iter()
        .map(CollocationResponse::from)
        .collect(),
      examples: details
        .examples()
        .iter()
        .map(ExampleResponse::from)
        .collect(),
      pitfalls: details
        .pitfalls()
        .iter()
        .map(PitfallResponse::from)
        .collect(),
      etymologies: details
        .etymologies()
        .iter()
        .map(EtymologyResponse::from)
        .collect(),
      history: details
        .history()
        .iter()
        .map(SenseHistoryResponse::from)
        .collect(),
      provenance: CanonicalSenseDetailsProvenanceResponse {
        release_id: content.release_id.to_string(),
        evidence_use: "api_redistribution",
        evidence_backed: true,
      },
    }
  }
}

#[derive(Debug, Serialize)]
struct CanonicalSenseTargetResponse {
  lexeme_id: String,
  sense_id: String,
  release_id: String,
  language: String,
}

impl From<&CanonicalSenseDetails> for CanonicalSenseTargetResponse {
  fn from(details: &CanonicalSenseDetails) -> Self {
    let target = details.target();
    Self {
      lexeme_id: target.lexeme_id().to_string(),
      sense_id: target.sense_id().to_string(),
      release_id: target.release_id().to_string(),
      language: target.language().to_string(),
    }
  }
}

#[derive(Debug, Serialize)]
struct CanonicalSenseDetailsProvenanceResponse {
  release_id: String,
  evidence_use: &'static str,
  evidence_backed: bool,
}

#[derive(Debug, Serialize)]
struct LocalizedGlossResponse {
  id: String,
  language: String,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::LocalizedGloss> for LocalizedGlossResponse {
  fn from(value: &crate::domain::canonical_content::LocalizedGloss) -> Self {
    Self {
      id: value.id().to_string(),
      language: value.language().to_string(),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct PronunciationResponse {
  id: String,
  scope: &'static str,
  dialect: String,
  notation: &'static str,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::CanonicalPronunciation> for PronunciationResponse {
  fn from(value: &crate::domain::canonical_content::CanonicalPronunciation) -> Self {
    Self {
      id: value.id().to_string(),
      scope: pronunciation_scope(value.scope()),
      dialect: value.dialect().to_string(),
      notation: pronunciation_notation(value.notation()),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct UsageLabelResponse {
  id: String,
  kind: &'static str,
  code: String,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::UsageLabel> for UsageLabelResponse {
  fn from(value: &crate::domain::canonical_content::UsageLabel) -> Self {
    Self {
      id: value.id().to_string(),
      kind: usage_label_kind(value.kind()),
      code: value.code().to_string(),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct GrammarPatternResponse {
  id: String,
  kind: &'static str,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::GrammarPattern> for GrammarPatternResponse {
  fn from(value: &crate::domain::canonical_content::GrammarPattern) -> Self {
    Self {
      id: value.id().to_string(),
      kind: grammar_pattern_kind(value.kind()),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct CollocationResponse {
  id: String,
  target_role: &'static str,
  construction: &'static str,
  head: CollocationTermResponse,
  dependent: CollocationTermResponse,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::Collocation> for CollocationResponse {
  fn from(value: &crate::domain::canonical_content::Collocation) -> Self {
    Self {
      id: value.id().to_string(),
      target_role: collocation_role(value.target_role()),
      construction: collocation_construction(value.construction()),
      head: CollocationTermResponse::from(value.head()),
      dependent: CollocationTermResponse::from(value.dependent()),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct CollocationTermResponse {
  surface: String,
  lexeme_id: Option<String>,
  sense_id: Option<String>,
}

impl From<&crate::domain::canonical_content::CollocationTerm> for CollocationTermResponse {
  fn from(value: &crate::domain::canonical_content::CollocationTerm) -> Self {
    Self {
      surface: value.surface().to_string(),
      lexeme_id: value.lexeme_id().map(ToString::to_string),
      sense_id: value.sense_id().map(ToString::to_string),
    }
  }
}

#[derive(Debug, Serialize)]
struct ExampleResponse {
  id: String,
  language: String,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::CanonicalExample> for ExampleResponse {
  fn from(value: &crate::domain::canonical_content::CanonicalExample) -> Self {
    Self {
      id: value.id().to_string(),
      language: value.language().to_string(),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct PitfallResponse {
  id: String,
  learner_language: String,
  kind: &'static str,
  mistake: FactualAssertionResponse,
  correction: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::LearnerPitfall> for PitfallResponse {
  fn from(value: &crate::domain::canonical_content::LearnerPitfall) -> Self {
    Self {
      id: value.id().to_string(),
      learner_language: value.learner_language().to_string(),
      kind: learner_pitfall_kind(value.kind()),
      mistake: FactualAssertionResponse::from(value.mistake()),
      correction: FactualAssertionResponse::from(value.correction()),
    }
  }
}

#[derive(Debug, Serialize)]
struct EtymologyResponse {
  id: String,
  scope: &'static str,
  kind: &'static str,
  source_language: String,
  period: HistoricalRangeResponse,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::EtymologyAssertion> for EtymologyResponse {
  fn from(value: &crate::domain::canonical_content::EtymologyAssertion) -> Self {
    Self {
      id: value.id().to_string(),
      scope: etymology_scope(value.scope()),
      kind: etymology_kind(value.kind()),
      source_language: value.source_language().to_string(),
      period: HistoricalRangeResponse::from(value.period()),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct SenseHistoryResponse {
  id: String,
  kind: &'static str,
  period: HistoricalRangeResponse,
  assertion: FactualAssertionResponse,
}

impl From<&crate::domain::canonical_content::SenseHistoryAssertion> for SenseHistoryResponse {
  fn from(value: &crate::domain::canonical_content::SenseHistoryAssertion) -> Self {
    Self {
      id: value.id().to_string(),
      kind: sense_history_event_kind(value.kind()),
      period: HistoricalRangeResponse::from(value.period()),
      assertion: FactualAssertionResponse::from(value.assertion()),
    }
  }
}

#[derive(Debug, Serialize)]
struct HistoricalRangeResponse {
  first_year: Option<i32>,
  last_year: Option<i32>,
}

impl From<HistoricalRange> for HistoricalRangeResponse {
  fn from(value: HistoricalRange) -> Self {
    Self {
      first_year: value.first_year(),
      last_year: value.last_year(),
    }
  }
}

#[derive(Debug, Serialize)]
struct FactualAssertionResponse {
  kind: &'static str,
  text: String,
  evidence: Vec<FactualEvidenceResponse>,
}

impl From<&CanonicalFactualAssertion> for FactualAssertionResponse {
  fn from(value: &CanonicalFactualAssertion) -> Self {
    Self {
      kind: canonical_detail_kind(value.kind()),
      text: value.text().to_string(),
      evidence: value
        .evidence()
        .iter()
        .map(FactualEvidenceResponse::from)
        .collect(),
    }
  }
}

#[derive(Debug, Serialize)]
struct FactualEvidenceResponse {
  id: String,
  kind: &'static str,
  confidence: &'static str,
  text: String,
  provenance: FactualEvidenceProvenanceResponse,
}

impl From<&crate::domain::canonical_content::CanonicalEvidenceLineage> for FactualEvidenceResponse {
  fn from(value: &crate::domain::canonical_content::CanonicalEvidenceLineage) -> Self {
    let fragment = value.fragment();
    Self {
      id: fragment.id.to_string(),
      kind: evidence_kind(fragment.kind),
      confidence: evidence_confidence(fragment.confidence),
      text: fragment.text.clone(),
      provenance: FactualEvidenceProvenanceResponse {
        source_id: value.source().id.to_string(),
        attribution: value.source().attribution.clone(),
        source_reference: fragment.source_reference.clone(),
        release_id: fragment.release_id.to_string(),
        language: fragment.language.to_string(),
        content_hash: fragment.content_hash.clone(),
        origin: evidence_origin(value.origin()),
      },
    }
  }
}

#[derive(Debug, Serialize)]
struct FactualEvidenceProvenanceResponse {
  source_id: String,
  attribution: Option<String>,
  source_reference: String,
  release_id: String,
  language: String,
  content_hash: String,
  origin: &'static str,
}

fn evidence_origin(
  value: &crate::domain::canonical_content::CanonicalEvidenceOrigin,
) -> &'static str {
  match value {
    crate::domain::canonical_content::CanonicalEvidenceOrigin::LicensedSource => "licensed_source",
    crate::domain::canonical_content::CanonicalEvidenceOrigin::Generated { .. } => {
      "reviewed_generated"
    }
  }
}

fn canonical_detail_kind(value: CanonicalDetailKind) -> &'static str {
  match value {
    CanonicalDetailKind::LocalizedGloss => "localized_gloss",
    CanonicalDetailKind::Pronunciation => "pronunciation",
    CanonicalDetailKind::UsageLabel => "usage_label",
    CanonicalDetailKind::GrammarPattern => "grammar_pattern",
    CanonicalDetailKind::Collocation => "collocation",
    CanonicalDetailKind::Example => "example",
    CanonicalDetailKind::Pitfall => "pitfall",
    CanonicalDetailKind::Etymology => "etymology",
    CanonicalDetailKind::SenseHistory => "sense_history",
    CanonicalDetailKind::SenseEvolution => "sense_evolution",
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

fn pronunciation_scope(value: PronunciationScope) -> &'static str {
  match value {
    PronunciationScope::LexemeWide => "lexeme_wide",
    PronunciationScope::SenseSpecific => "sense_specific",
  }
}

fn pronunciation_notation(value: PronunciationNotation) -> &'static str {
  match value {
    PronunciationNotation::Ipa => "ipa",
    PronunciationNotation::Phonemic => "phonemic",
    PronunciationNotation::SourceDefined => "source_defined",
  }
}

fn usage_label_kind(value: UsageLabelKind) -> &'static str {
  match value {
    UsageLabelKind::Register => "register",
    UsageLabelKind::Domain => "domain",
    UsageLabelKind::Dialect => "dialect",
    UsageLabelKind::Connotation => "connotation",
    UsageLabelKind::Politeness => "politeness",
    UsageLabelKind::Datedness => "datedness",
    UsageLabelKind::Sensitivity => "sensitivity",
    UsageLabelKind::Frequency => "frequency",
    UsageLabelKind::Level => "level",
  }
}

fn grammar_pattern_kind(value: GrammarPatternKind) -> &'static str {
  match value {
    GrammarPatternKind::Valency => "valency",
    GrammarPatternKind::Construction => "construction",
    GrammarPatternKind::Government => "government",
    GrammarPatternKind::Morphology => "morphology",
    GrammarPatternKind::SourceDefined => "source_defined",
  }
}

fn collocation_role(value: CollocationRole) -> &'static str {
  match value {
    CollocationRole::Head => "head",
    CollocationRole::Dependent => "dependent",
  }
}

fn collocation_construction(value: CollocationConstruction) -> &'static str {
  match value {
    CollocationConstruction::VerbObject => "verb_object",
    CollocationConstruction::AdjectiveNoun => "adjective_noun",
    CollocationConstruction::AdverbModifier => "adverb_modifier",
    CollocationConstruction::NounCompound => "noun_compound",
    CollocationConstruction::GovernedComplement => "governed_complement",
    CollocationConstruction::SourceDefined => "source_defined",
  }
}

fn learner_pitfall_kind(value: LearnerPitfallKind) -> &'static str {
  match value {
    LearnerPitfallKind::FalseFriend => "false_friend",
    LearnerPitfallKind::Confusable => "confusable",
    LearnerPitfallKind::LiteralTranslation => "literal_translation",
    LearnerPitfallKind::CommonError => "common_error",
  }
}

fn etymology_scope(value: EtymologyScope) -> &'static str {
  match value {
    EtymologyScope::LexemeWide => "lexeme_wide",
    EtymologyScope::SenseSpecific => "sense_specific",
  }
}

fn etymology_kind(value: EtymologyKind) -> &'static str {
  match value {
    EtymologyKind::BorrowedFrom => "borrowed_from",
    EtymologyKind::DerivedFrom => "derived_from",
    EtymologyKind::CognateWith => "cognate_with",
    EtymologyKind::OriginSummary => "origin_summary",
  }
}

fn sense_history_event_kind(value: SenseHistoryEventKind) -> &'static str {
  match value {
    SenseHistoryEventKind::Attestation => "attestation",
    SenseHistoryEventKind::SemanticShift => "semantic_shift",
    SenseHistoryEventKind::ScopeChange => "scope_change",
    SenseHistoryEventKind::Retirement => "retirement",
  }
}
