//! `POST /v1/lookups` transport contract.

use axum::{
  extract::{rejection::JsonRejection, Extension, State},
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
  domain::observability::{LookupStage, MetricEvent, MetricOutcome, ModelValidationOutcome},
  domain::{
    canonical::{
      EvidenceConfidence, EvidenceKind, EvidenceUse, FormKind, LanguageTag, LexicalPartOfSpeech,
    },
    canonical_lookup_cache::{
      CanonicalCardPolicyVersions, CanonicalLookupCacheEligibility, PublicCanonicalLookupRequest,
    },
    lookup_card::{
      CanonicalLookupAssertionKind, CanonicalLookupCard, CanonicalLookupCardAssertion,
      CanonicalLookupCardCandidate, CanonicalLookupCardCoverage, CanonicalLookupCardCoverageState,
      CanonicalLookupCardEvidence, CanonicalLookupCardEvidenceProvenance, CanonicalLookupCardForm,
      CanonicalLookupCardLexeme, CanonicalLookupCardSectionCoverage, CanonicalLookupCardSense,
    },
    retrieval::DEFAULT_RETRIEVAL_LIMIT,
    translation::{
      CefrLevel, Confidence, EnglishDialect, EnglishEntry, PartOfSpeech, RelationKind,
      TranslationInput, TranslationValidationError, UsageNoteKind,
    },
  },
  ports::learning_model::LearningModelError,
};

const CANONICAL_RETRIEVAL_POLICY_VERSION: &str = "canonical-retrieval-v1";
const CANONICAL_PRESENTATION_POLICY_VERSION: &str = "canonical-card-v1";

/// Public request for a model-backed learning lookup.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LookupRequest {
  query: String,
  #[serde(default = "default_source_language")]
  source_language: String,
  #[serde(default = "default_target_language")]
  target_language: String,
  context: Option<String>,
  #[serde(default = "default_explanation_language")]
  explanation_language: String,
  #[serde(default)]
  english_dialect: ApiEnglishDialect,
  learner_level: Option<ApiCefrLevel>,
  #[serde(default)]
  detail: Detail,
  include: Option<Vec<IncludeSection>>,
  #[serde(default)]
  history_mode: HistoryMode,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
enum ApiEnglishDialect {
  #[default]
  #[serde(rename = "en-US")]
  American,
  #[serde(rename = "en-GB")]
  British,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum ApiCefrLevel {
  A1,
  A2,
  B1,
  B2,
  C1,
  C2,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Detail {
  Brief,
  #[default]
  Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IncludeSection {
  Relations,
  WordHistory,
  PracticePreview,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HistoryMode {
  Save,
  #[default]
  Incognito,
}

#[derive(Debug, Serialize)]
struct LookupResponse {
  schema_version: &'static str,
  query: QueryAnalysis,
  matches: Vec<GeneratedMatch>,
  coverage: Coverage,
  warnings: Vec<String>,
  provenance: LookupProvenance,
}

#[derive(Debug, Serialize)]
struct CanonicalLookupResponse {
  schema_version: &'static str,
  query: CanonicalQueryAnalysis,
  matches: Vec<CanonicalMatch>,
  coverage: CanonicalCoverage,
  warnings: Vec<String>,
  provenance: CanonicalLookupProvenance,
}

#[derive(Debug, Serialize)]
struct QueryAnalysis {
  original: String,
  normalized: String,
  language: String,
  language_confidence: &'static str,
}

#[derive(Debug, Serialize)]
struct CanonicalQueryAnalysis {
  original: String,
  normalized: String,
  language: String,
  language_confidence: &'static str,
  evidence_use: &'static str,
}

#[derive(Debug, Serialize)]
struct CanonicalMatch {
  rank: usize,
  fusion_score: u64,
  lexeme: CanonicalLexemeResponse,
  part_of_speech: &'static str,
  sense: CanonicalSenseResponse,
  forms: Vec<CanonicalFormResponse>,
}

#[derive(Debug, Serialize)]
struct CanonicalLexemeResponse {
  id: String,
  lemma: String,
  language: String,
}

#[derive(Debug, Serialize)]
struct CanonicalSenseResponse {
  id: String,
  sense_key: String,
  definition: Option<CanonicalAssertionResponse>,
}

#[derive(Debug, Serialize)]
struct CanonicalFormResponse {
  id: String,
  kind: &'static str,
  morphology: Option<String>,
  assertion: CanonicalAssertionResponse,
}

#[derive(Debug, Serialize)]
struct CanonicalAssertionResponse {
  kind: &'static str,
  text: String,
  evidence: Vec<CanonicalEvidenceResponse>,
}

#[derive(Debug, Serialize)]
struct CanonicalEvidenceResponse {
  id: String,
  kind: &'static str,
  confidence: &'static str,
  text: String,
  provenance: CanonicalEvidenceProvenanceResponse,
}

#[derive(Debug, Serialize)]
struct CanonicalEvidenceProvenanceResponse {
  source_id: String,
  source_reference: String,
  release_id: String,
  language: String,
  content_hash: String,
}

#[derive(Debug, Serialize)]
struct CanonicalCoverage {
  retrieval: CanonicalSectionCoverage,
  lexemes: CanonicalSectionCoverage,
  parts_of_speech: CanonicalSectionCoverage,
  senses: CanonicalSectionCoverage,
  definitions: CanonicalSectionCoverage,
  forms: CanonicalSectionCoverage,
  evidence: CanonicalSectionCoverage,
}

#[derive(Debug, Serialize)]
struct CanonicalSectionCoverage {
  state: &'static str,
  available_items: usize,
  missing_items: usize,
  filtered_items: usize,
  truncated_items: usize,
}

#[derive(Debug, Serialize)]
struct CanonicalLookupProvenance {
  lexicon_release: String,
  index_version: String,
  schema_version: String,
  ranking_version: String,
  retrieval_policy_version: &'static str,
  presentation_policy_version: &'static str,
  retrieval_path: &'static str,
  generation_contract: &'static str,
  evidence_backed: bool,
}

#[derive(Debug, Serialize)]
struct GeneratedMatch {
  source_sense_id: Option<String>,
  english_senses: Vec<GeneratedSense>,
  context_relevance: Option<f32>,
}

#[derive(Debug, Serialize)]
struct GeneratedSense {
  sense_id: Option<String>,
  lemma: String,
  part_of_speech: &'static str,
  definition: GeneratedText,
  localized_gloss: Option<LocalizedGeneratedText>,
  confidence: &'static str,
  rank: u16,
  pronunciations: Vec<PronunciationResponse>,
  forms: Vec<WordFormResponse>,
  usage_notes: Vec<UsageNoteResponse>,
  examples: Vec<ExampleResponse>,
  etymology: Option<GeneratedText>,
  related_words: Vec<RelatedWordResponse>,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct GeneratedText {
  text: String,
  evidence_ids: Vec<String>,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct LocalizedGeneratedText {
  text: String,
  language: String,
  evidence_ids: Vec<String>,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct PronunciationResponse {
  value: String,
  notation: String,
  dialect: Option<String>,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct WordFormResponse {
  form: String,
  label: String,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct UsageNoteResponse {
  kind: &'static str,
  text: String,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct ExampleResponse {
  english: String,
  localized: Option<String>,
  evidence_ids: Vec<String>,
  generated: bool,
}

#[derive(Debug, Serialize)]
struct RelatedWordResponse {
  relation_id: Option<String>,
  lemma: String,
  relation: &'static str,
  note: Option<String>,
  generated: bool,
  canonical: bool,
}

#[derive(Debug, Serialize)]
struct Coverage {
  parts_of_speech: &'static str,
  examples: &'static str,
  word_history: &'static str,
  relations: &'static str,
  canonical_evidence: &'static str,
}

#[derive(Debug, Serialize)]
struct LookupProvenance {
  lexicon_release: Option<String>,
  index_version: Option<String>,
  ranking_version: &'static str,
  generation_contract: &'static str,
  evidence_backed: bool,
}

pub(crate) async fn lookup(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  payload: Result<Json<LookupRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(request) => request,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      record_lookup_stage(
        &state,
        LookupStage::RequestValidation,
        MetricOutcome::Rejected,
      );
      return problem::payload_too_large(&request_id);
    }
    Err(_) => {
      record_lookup_stage(
        &state,
        LookupStage::RequestValidation,
        MetricOutcome::Rejected,
      );
      return problem::response(
        StatusCode::BAD_REQUEST,
        "invalid_json",
        "Invalid JSON request",
        "The request body is not valid lookup JSON.",
        &request_id,
        false,
        Vec::new(),
      );
    }
  };

  if !request.target_language.eq_ignore_ascii_case("en") {
    record_lookup_stage(
      &state,
      LookupStage::RequestValidation,
      MetricOutcome::Rejected,
    );
    return validation_problem(
      TranslationValidationError {
        field: "target_language",
        message: "must be `en` in basic core".to_string(),
      },
      &request_id,
    );
  }

  let input = match TranslationInput::new(
    &request.query,
    &request.source_language,
    request.context.as_deref(),
    &request.explanation_language,
    request.english_dialect.into(),
    request.learner_level.map(Into::into),
  ) {
    Ok(input) => input,
    Err(error) => {
      record_lookup_stage(
        &state,
        LookupStage::RequestValidation,
        MetricOutcome::Rejected,
      );
      return validation_problem(error, &request_id);
    }
  };

  if let Some(service) = state.canonical_lookup_service() {
    if let Some(canonical_request) = canonical_request(&request, &input) {
      let eligibility = canonical_cache_eligibility(&request);
      return match service.lookup(canonical_request, eligibility).await {
        Ok(card) => {
          let response = build_canonical_response(&request, card);
          problem::no_store((StatusCode::OK, Json(response)).into_response())
        }
        Err(error) => problem::response(
          StatusCode::SERVICE_UNAVAILABLE,
          "canonical_lookup_unavailable",
          "Canonical lookup unavailable",
          "The canonical lookup service could not produce an evidence-backed result.",
          &request_id,
          error.is_retryable(),
          Vec::new(),
        ),
      };
    }
  }

  record_lookup_stage(
    &state,
    LookupStage::RequestValidation,
    MetricOutcome::Succeeded,
  );

  let Some(service) = &state.lookup else {
    return problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "learning_model_unavailable",
      "Learning model unavailable",
      "The structured learning model is not configured.",
      &request_id,
      true,
      Vec::new(),
    );
  };

  match service.lookup(&input).await {
    Ok(result) => {
      let response = build_response(request, input, result);
      record_lookup_stage(
        &state,
        LookupStage::ResponseAssembly,
        MetricOutcome::Succeeded,
      );
      problem::no_store((StatusCode::OK, Json(response)).into_response())
    }
    Err(LearningModelError::Unavailable) => problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "learning_model_unavailable",
      "Learning model unavailable",
      "The learning model did not return a result.",
      &request_id,
      true,
      Vec::new(),
    ),
    Err(LearningModelError::InvalidOutput) => {
      record_lookup_event(
        &state,
        MetricEvent::ModelValidation {
          outcome: ModelValidationOutcome::Rejected,
        },
      );
      problem::response(
        StatusCode::BAD_GATEWAY,
        "invalid_model_output",
        "Invalid model output",
        "The learning model could not satisfy the structured output contract.",
        &request_id,
        true,
        Vec::new(),
      )
    }
  }
}

fn record_lookup_stage(state: &AppState, stage: LookupStage, outcome: MetricOutcome) {
  record_lookup_event(state, MetricEvent::LookupStage { stage, outcome });
}

fn record_lookup_event(state: &AppState, event: MetricEvent) {
  state.dispatch_lookup_metric(event);
}

fn build_response(
  request: LookupRequest,
  input: TranslationInput,
  result: crate::domain::translation::TranslationResult,
) -> LookupResponse {
  let include_relations = requested(&request, IncludeSection::Relations);
  let include_history = requested(&request, IncludeSection::WordHistory);
  let mut warnings = result.warnings;
  warnings.push(
    "This result is model-generated and is not backed by the canonical lexicon yet.".to_string(),
  );
  if matches!(request.history_mode, HistoryMode::Save) {
    warnings.push("History is not stored by the current anonymous basic-core slice.".to_string());
  }
  if request
    .include
    .as_ref()
    .is_some_and(|sections| sections.contains(&IncludeSection::PracticePreview))
  {
    warnings.push("Practice preview is not implemented yet.".to_string());
  }

  let entry_limit = if matches!(request.detail, Detail::Brief) {
    3
  } else {
    usize::MAX
  };
  let entries = result
    .entries
    .into_iter()
    .take(entry_limit)
    .map(|entry| GeneratedMatch {
      source_sense_id: None,
      english_senses: vec![sense_response(
        entry,
        &input.explanation_language,
        request.detail,
        include_relations,
        include_history,
      )],
      context_relevance: None,
    })
    .collect::<Vec<_>>();
  let examples = coverage(entries.iter().any(|value| {
    value
      .english_senses
      .iter()
      .any(|sense| !sense.examples.is_empty())
  }));
  let word_history = coverage(entries.iter().any(|value| {
    value
      .english_senses
      .iter()
      .any(|sense| sense.etymology.is_some())
  }));
  let relations = coverage(entries.iter().any(|value| {
    value
      .english_senses
      .iter()
      .any(|sense| !sense.related_words.is_empty())
  }));

  LookupResponse {
    schema_version: "1.0",
    query: QueryAnalysis {
      original: request.query,
      normalized: input.query,
      language: result.source_language,
      language_confidence: confidence(result.language_confidence),
    },
    matches: entries,
    coverage: Coverage {
      parts_of_speech: "available",
      examples,
      word_history,
      relations,
      canonical_evidence: "unavailable",
    },
    warnings,
    provenance: LookupProvenance {
      lexicon_release: None,
      index_version: None,
      ranking_version: "model-ranking-v1",
      generation_contract: "learning-card-v1",
      evidence_backed: false,
    },
  }
}

fn canonical_request(
  request: &LookupRequest,
  input: &TranslationInput,
) -> Option<PublicCanonicalLookupRequest> {
  if input.source_language == "auto" || input.context.is_some() {
    return None;
  }

  let language = LanguageTag::parse(&input.source_language).ok()?;
  let policy_versions = CanonicalCardPolicyVersions::new(
    CANONICAL_RETRIEVAL_POLICY_VERSION,
    CANONICAL_PRESENTATION_POLICY_VERSION,
  )
  .ok()?;
  PublicCanonicalLookupRequest::new(
    &input.query,
    language,
    canonical_candidate_limit(request.detail),
    policy_versions,
  )
  .ok()
}

fn canonical_candidate_limit(detail: Detail) -> usize {
  match detail {
    Detail::Brief => 3,
    Detail::Full => DEFAULT_RETRIEVAL_LIMIT,
  }
}

fn canonical_cache_eligibility(request: &LookupRequest) -> CanonicalLookupCacheEligibility {
  match request.history_mode {
    HistoryMode::Save => CanonicalLookupCacheEligibility::with_history(),
    HistoryMode::Incognito => CanonicalLookupCacheEligibility::incognito(),
  }
}

fn build_canonical_response(
  request: &LookupRequest,
  card: CanonicalLookupCard,
) -> CanonicalLookupResponse {
  let CanonicalLookupCard {
    query,
    content,
    candidates,
    coverage,
  } = card;
  let retrieval_path = canonical_retrieval_path(coverage.retrieval.state);

  CanonicalLookupResponse {
    schema_version: "1.0",
    query: CanonicalQueryAnalysis {
      original: request.query.clone(),
      normalized: query.normalized_query,
      language: query.language.to_string(),
      language_confidence: "explicit",
      evidence_use: evidence_use(query.evidence_use),
    },
    matches: candidates
      .into_iter()
      .map(canonical_match_response)
      .collect(),
    coverage: canonical_coverage_response(coverage),
    warnings: canonical_warnings(request),
    provenance: CanonicalLookupProvenance {
      lexicon_release: content.release_id.to_string(),
      index_version: content.vector_collection_id.to_string(),
      schema_version: content.schema_version,
      ranking_version: content.ranking_version,
      retrieval_policy_version: CANONICAL_RETRIEVAL_POLICY_VERSION,
      presentation_policy_version: CANONICAL_PRESENTATION_POLICY_VERSION,
      retrieval_path,
      generation_contract: "not_generated",
      evidence_backed: true,
    },
  }
}

fn canonical_warnings(request: &LookupRequest) -> Vec<String> {
  let mut warnings = vec![
    "This result contains deterministic canonical lexical content and no generated explanation."
      .to_string(),
  ];
  if matches!(request.history_mode, HistoryMode::Save) {
    warnings.push("History is not stored by the current anonymous basic-core slice.".to_string());
  }
  if request.include.as_ref().is_some_and(|sections| {
    sections.contains(&IncludeSection::Relations) || sections.contains(&IncludeSection::WordHistory)
  }) {
    warnings.push(
      "Relations and word history are not included by the current canonical lookup foundation."
        .to_string(),
    );
  }
  if request
    .include
    .as_ref()
    .is_some_and(|sections| sections.contains(&IncludeSection::PracticePreview))
  {
    warnings.push("Practice preview is not implemented yet.".to_string());
  }
  warnings
}

fn canonical_match_response(candidate: CanonicalLookupCardCandidate) -> CanonicalMatch {
  let CanonicalLookupCardCandidate {
    rank,
    fusion_score,
    lexeme,
    sense,
    forms,
  } = candidate;
  CanonicalMatch {
    rank,
    fusion_score,
    part_of_speech: lexical_part_of_speech(lexeme.part_of_speech),
    lexeme: canonical_lexeme_response(lexeme),
    sense: canonical_sense_response(sense),
    forms: forms.into_iter().map(canonical_form_response).collect(),
  }
}

fn canonical_lexeme_response(lexeme: CanonicalLookupCardLexeme) -> CanonicalLexemeResponse {
  CanonicalLexemeResponse {
    id: lexeme.id.to_string(),
    lemma: lexeme.lemma,
    language: lexeme.language.to_string(),
  }
}

fn canonical_sense_response(sense: CanonicalLookupCardSense) -> CanonicalSenseResponse {
  CanonicalSenseResponse {
    id: sense.id.to_string(),
    sense_key: sense.sense_key,
    definition: sense.definition.map(canonical_assertion_response),
  }
}

fn canonical_form_response(form: CanonicalLookupCardForm) -> CanonicalFormResponse {
  CanonicalFormResponse {
    id: form.id.to_string(),
    kind: form_kind(form.kind),
    morphology: form.morphology,
    assertion: canonical_assertion_response(form.assertion),
  }
}

fn canonical_assertion_response(
  assertion: CanonicalLookupCardAssertion,
) -> CanonicalAssertionResponse {
  CanonicalAssertionResponse {
    kind: canonical_assertion_kind(assertion.kind),
    text: assertion.text,
    evidence: assertion
      .evidence
      .into_iter()
      .map(canonical_evidence_response)
      .collect(),
  }
}

fn canonical_evidence_response(evidence: CanonicalLookupCardEvidence) -> CanonicalEvidenceResponse {
  CanonicalEvidenceResponse {
    id: evidence.id.to_string(),
    kind: evidence_kind(evidence.kind),
    confidence: evidence_confidence(evidence.confidence),
    text: evidence.text,
    provenance: canonical_evidence_provenance_response(evidence.provenance),
  }
}

fn canonical_evidence_provenance_response(
  provenance: CanonicalLookupCardEvidenceProvenance,
) -> CanonicalEvidenceProvenanceResponse {
  CanonicalEvidenceProvenanceResponse {
    source_id: provenance.source_id.to_string(),
    source_reference: provenance.source_reference,
    release_id: provenance.release_id.to_string(),
    language: provenance.language.to_string(),
    content_hash: provenance.content_hash,
  }
}

fn canonical_coverage_response(coverage: CanonicalLookupCardCoverage) -> CanonicalCoverage {
  CanonicalCoverage {
    retrieval: canonical_section_coverage(coverage.retrieval),
    lexemes: canonical_section_coverage(coverage.lexemes),
    parts_of_speech: canonical_section_coverage(coverage.parts_of_speech),
    senses: canonical_section_coverage(coverage.senses),
    definitions: canonical_section_coverage(coverage.definitions),
    forms: canonical_section_coverage(coverage.forms),
    evidence: canonical_section_coverage(coverage.evidence),
  }
}

fn canonical_section_coverage(
  coverage: CanonicalLookupCardSectionCoverage,
) -> CanonicalSectionCoverage {
  CanonicalSectionCoverage {
    state: canonical_coverage_state(coverage.state),
    available_items: coverage.available_items,
    missing_items: coverage.missing_items,
    filtered_items: coverage.filtered_items,
    truncated_items: coverage.truncated_items,
  }
}

fn canonical_retrieval_path(state: CanonicalLookupCardCoverageState) -> &'static str {
  match state {
    CanonicalLookupCardCoverageState::VectorDegraded => "lexical_fallback",
    CanonicalLookupCardCoverageState::Available
    | CanonicalLookupCardCoverageState::Missing
    | CanonicalLookupCardCoverageState::Filtered => "hybrid",
  }
}

fn canonical_coverage_state(value: CanonicalLookupCardCoverageState) -> &'static str {
  match value {
    CanonicalLookupCardCoverageState::Available => "available",
    CanonicalLookupCardCoverageState::Missing => "missing",
    CanonicalLookupCardCoverageState::Filtered => "filtered",
    CanonicalLookupCardCoverageState::VectorDegraded => "vector_degraded",
  }
}

fn lexical_part_of_speech(value: LexicalPartOfSpeech) -> &'static str {
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

fn canonical_assertion_kind(value: CanonicalLookupAssertionKind) -> &'static str {
  match value {
    CanonicalLookupAssertionKind::Definition => "definition",
    CanonicalLookupAssertionKind::Form => "form",
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

fn evidence_use(value: EvidenceUse) -> &'static str {
  match value {
    EvidenceUse::Storage => "storage",
    EvidenceUse::Display => "display",
    EvidenceUse::Embedding => "embedding",
    EvidenceUse::ModelProcessing => "model_processing",
    EvidenceUse::ApiRedistribution => "api_redistribution",
  }
}

fn sense_response(
  mut entry: EnglishEntry,
  explanation_language: &str,
  detail: Detail,
  include_relations: bool,
  include_history: bool,
) -> GeneratedSense {
  if matches!(detail, Detail::Brief) {
    entry.usage_notes.truncate(2);
    entry.examples.truncate(1);
    entry.related_words.truncate(4);
  }
  if !include_relations {
    entry.related_words.clear();
  }
  if !include_history {
    entry.etymology = None;
  }

  GeneratedSense {
    sense_id: None,
    lemma: entry.lemma,
    part_of_speech: part_of_speech(entry.part_of_speech),
    definition: generated_text(entry.definition),
    localized_gloss: entry.localized_gloss.map(|text| LocalizedGeneratedText {
      text,
      language: explanation_language.to_string(),
      evidence_ids: Vec::new(),
      generated: true,
    }),
    confidence: confidence(entry.confidence),
    rank: entry.rank,
    pronunciations: entry
      .pronunciations
      .into_iter()
      .map(|value| PronunciationResponse {
        value: value.value,
        notation: value.notation,
        dialect: value.dialect,
        generated: true,
      })
      .collect(),
    forms: entry
      .forms
      .into_iter()
      .map(|value| WordFormResponse {
        form: value.form,
        label: value.label,
        generated: true,
      })
      .collect(),
    usage_notes: entry
      .usage_notes
      .into_iter()
      .map(|value| UsageNoteResponse {
        kind: usage_note_kind(value.kind),
        text: value.text,
        generated: true,
      })
      .collect(),
    examples: entry
      .examples
      .into_iter()
      .map(|value| ExampleResponse {
        english: value.english,
        localized: value.localized,
        evidence_ids: Vec::new(),
        generated: true,
      })
      .collect(),
    etymology: entry.etymology.map(generated_text),
    related_words: entry
      .related_words
      .into_iter()
      .map(|value| RelatedWordResponse {
        relation_id: None,
        lemma: value.lemma,
        relation: relation_kind(value.relation),
        note: value.note,
        generated: true,
        canonical: false,
      })
      .collect(),
    generated: true,
  }
}

fn requested(request: &LookupRequest, section: IncludeSection) -> bool {
  request
    .include
    .as_ref()
    .is_none_or(|sections| sections.contains(&section))
}

fn generated_text(text: String) -> GeneratedText {
  GeneratedText {
    text,
    evidence_ids: Vec::new(),
    generated: true,
  }
}

fn coverage(available: bool) -> &'static str {
  if available {
    "available"
  } else {
    "unavailable"
  }
}

fn confidence(value: Confidence) -> &'static str {
  match value {
    Confidence::High => "high",
    Confidence::Medium => "medium",
    Confidence::Low => "low",
  }
}

fn part_of_speech(value: PartOfSpeech) -> &'static str {
  match value {
    PartOfSpeech::Noun => "noun",
    PartOfSpeech::Verb => "verb",
    PartOfSpeech::Adjective => "adjective",
    PartOfSpeech::Adverb => "adverb",
    PartOfSpeech::Pronoun => "pronoun",
    PartOfSpeech::Preposition => "preposition",
    PartOfSpeech::Conjunction => "conjunction",
    PartOfSpeech::Determiner => "determiner",
    PartOfSpeech::Interjection => "interjection",
    PartOfSpeech::Numeral => "numeral",
    PartOfSpeech::Other => "other",
  }
}

fn usage_note_kind(value: UsageNoteKind) -> &'static str {
  match value {
    UsageNoteKind::Register => "register",
    UsageNoteKind::Dialect => "dialect",
    UsageNoteKind::Grammar => "grammar",
    UsageNoteKind::Collocation => "collocation",
    UsageNoteKind::Pitfall => "pitfall",
    UsageNoteKind::Habit => "habit",
  }
}

fn relation_kind(value: RelationKind) -> &'static str {
  match value {
    RelationKind::Synonym => "synonym",
    RelationKind::Antonym => "antonym",
    RelationKind::Broader => "broader",
    RelationKind::Narrower => "narrower",
    RelationKind::WordFamily => "word_family",
    RelationKind::LowerDegree => "lower_degree",
    RelationKind::HigherDegree => "higher_degree",
    RelationKind::Confusable => "confusable",
    RelationKind::Related => "related",
  }
}

fn validation_problem(error: TranslationValidationError, request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "validation_error",
    "Invalid lookup request",
    "One or more lookup fields are invalid.",
    request_id,
    false,
    vec![FieldError::new(error.field, error.message)],
  )
}

fn default_source_language() -> String {
  "auto".to_string()
}

fn default_target_language() -> String {
  "en".to_string()
}

fn default_explanation_language() -> String {
  "en".to_string()
}

impl From<ApiEnglishDialect> for EnglishDialect {
  fn from(value: ApiEnglishDialect) -> Self {
    match value {
      ApiEnglishDialect::American => Self::American,
      ApiEnglishDialect::British => Self::British,
    }
  }
}

impl From<ApiCefrLevel> for CefrLevel {
  fn from(value: ApiCefrLevel) -> Self {
    match value {
      ApiCefrLevel::A1 => Self::A1,
      ApiCefrLevel::A2 => Self::A2,
      ApiCefrLevel::B1 => Self::B1,
      ApiCefrLevel::B2 => Self::B2,
      ApiCefrLevel::C1 => Self::C1,
      ApiCefrLevel::C2 => Self::C2,
    }
  }
}
