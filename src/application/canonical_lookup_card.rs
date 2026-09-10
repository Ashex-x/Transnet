//! Deterministic assembly of bounded canonical lookup cards.
//!
//! This foundation maps the existing canonical retrieval outcome into typed Rust values. It does
//! not call a model, expose an HTTP route, persist a snapshot, or change the current
//! `POST /v1/lookups` model-backed contract.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::{
  application::retrieval::{
    CanonicalRetrievalError, CanonicalRetrievalService, RetrievalOutcome, RetrievalPath,
  },
  domain::{
    canonical::{CanonicalStatus, EvidenceFragment, EvidenceId, EvidenceUse, WordForm},
    lookup_card::{
      CanonicalLookupAssertionKind, CanonicalLookupCard, CanonicalLookupCardAssertion,
      CanonicalLookupCardCandidate, CanonicalLookupCardCoverage, CanonicalLookupCardCoverageState,
      CanonicalLookupCardEvidence, CanonicalLookupCardEvidenceProvenance, CanonicalLookupCardForm,
      CanonicalLookupCardLexeme, CanonicalLookupCardSectionCoverage, CanonicalLookupCardSense,
      CanonicalLookupQueryAnalysis, MAX_LOOKUP_CARD_CANDIDATES,
      MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION, MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE,
    },
    retrieval::{CanonicalCandidate, RankedCandidate, RetrievalRequest},
  },
};

/// Failure while retrieving material for a canonical lookup card.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupCardError {
  /// The authoritative canonical retrieval stage could not produce a safe result.
  #[error(transparent)]
  Retrieval(#[from] CanonicalRetrievalError),
}

/// Builds a deterministic canonical lookup card from the existing retrieval service.
///
/// The service intentionally remains independent of the model-backed lookup service and HTTP
/// router. A future transport adapter may choose to call it after deciding the canonical path is
/// appropriate, but this module does not wire that policy itself.
#[derive(Clone)]
pub struct CanonicalLookupCardService {
  retrieval: CanonicalRetrievalService,
}

impl CanonicalLookupCardService {
  /// Creates a card service around the supplied canonical retrieval service.
  pub fn new(retrieval: CanonicalRetrievalService) -> Self {
    Self { retrieval }
  }

  /// Retrieves and assembles one bounded evidence-backed canonical lookup card.
  ///
  /// No model is invoked. Vector unavailability yields a lexical-only card whose retrieval
  /// coverage is `VectorDegraded`; an authoritative canonical-repository failure remains an
  /// error.
  ///
  /// # Errors
  ///
  /// Returns an error when the canonical retrieval service cannot safely read authoritative
  /// canonical content.
  pub async fn lookup(
    &self,
    request: RetrievalRequest,
  ) -> Result<CanonicalLookupCard, CanonicalLookupCardError> {
    let outcome = self.retrieval.retrieve(request.clone()).await?;
    Ok(CanonicalLookupCardMapper::assemble(&request, outcome))
  }
}

/// Pure mapper from a retrieval outcome to a bounded canonical lookup card.
///
/// It preserves the incoming ranked-candidate order and rank values. The mapper is public so
/// non-HTTP callers can assemble a card from an already retrieved immutable outcome without
/// reinvoking retrieval.
pub struct CanonicalLookupCardMapper;

impl CanonicalLookupCardMapper {
  /// Assembles one card from the normalized request and an existing deterministic outcome.
  ///
  /// This method defensively reapplies release, lifecycle, ownership, and source-permission
  /// checks before exposing an assertion. It never creates, rewrites, or ranks factual text.
  pub fn assemble(request: &RetrievalRequest, outcome: RetrievalOutcome) -> CanonicalLookupCard {
    let candidate_limit = request.limit.min(MAX_LOOKUP_CARD_CANDIDATES);
    let source_candidate_count = outcome.candidates.len();
    let mut counts = CardCounts::default();
    let mut candidates = Vec::with_capacity(candidate_limit);

    for ranked in outcome.candidates.into_iter().take(candidate_limit) {
      if let Some(candidate) = map_candidate(&ranked, request, &outcome.content, &mut counts) {
        candidates.push(candidate);
      } else {
        counts.record_filtered_candidate();
      }
    }
    counts.retrieval.truncated += source_candidate_count.saturating_sub(candidate_limit);
    counts.lexemes.truncated += source_candidate_count.saturating_sub(candidate_limit);
    counts.parts_of_speech.truncated += source_candidate_count.saturating_sub(candidate_limit);
    counts.senses.truncated += source_candidate_count.saturating_sub(candidate_limit);

    let candidate_coverage = counts.candidate_section();
    let retrieval = match outcome.path {
      RetrievalPath::Hybrid => candidate_coverage.clone(),
      RetrievalPath::LexicalFallback => CanonicalLookupCardSectionCoverage {
        state: CanonicalLookupCardCoverageState::VectorDegraded,
        ..candidate_coverage
      },
    };

    CanonicalLookupCard {
      query: CanonicalLookupQueryAnalysis {
        normalized_query: request.query.clone(),
        language: request.language.clone(),
        evidence_use: request.evidence_use,
      },
      content: outcome.content,
      candidates,
      coverage: CanonicalLookupCardCoverage {
        retrieval,
        lexemes: counts.lexemes.to_coverage(),
        parts_of_speech: counts.parts_of_speech.to_coverage(),
        senses: counts.senses.to_coverage(),
        definitions: counts.definitions.to_coverage(),
        forms: counts.forms.to_coverage(),
        evidence: counts.evidence.to_coverage(),
      },
    }
  }
}

fn map_candidate(
  ranked: &RankedCandidate,
  request: &RetrievalRequest,
  content: &crate::domain::canonical::ActiveContentVersion,
  counts: &mut CardCounts,
) -> Option<CanonicalLookupCardCandidate> {
  let candidate = &ranked.candidate;
  if !candidate_is_visible(candidate, request, content) {
    return None;
  }

  counts.record_visible_candidate();
  let definition = map_assertion(
    CanonicalLookupAssertionKind::Definition,
    &candidate.sense.definition,
    &candidate.sense.definition_evidence_ids,
    candidate,
    &content.release_id,
    request.evidence_use,
    &mut counts.evidence,
  );
  counts.definitions.record(definition.state);

  let forms = map_forms(candidate, content, request.evidence_use, counts);
  Some(CanonicalLookupCardCandidate {
    rank: ranked.rank,
    fusion_score: ranked.fusion_score,
    lexeme: CanonicalLookupCardLexeme {
      id: candidate.lexeme.id.clone(),
      lemma: candidate.lexeme.lemma.clone(),
      language: candidate.lexeme.language.clone(),
      part_of_speech: candidate.lexeme.part_of_speech,
    },
    sense: CanonicalLookupCardSense {
      id: candidate.sense.id.clone(),
      sense_key: candidate.sense.sense_key.clone(),
      definition: definition.assertion,
    },
    forms,
  })
}

fn candidate_is_visible(
  candidate: &CanonicalCandidate,
  request: &RetrievalRequest,
  content: &crate::domain::canonical::ActiveContentVersion,
) -> bool {
  candidate.lexeme.release_id == content.release_id
    && candidate.sense.release_id == content.release_id
    && candidate.lexeme.id == candidate.sense.lexeme_id
    && candidate.lexeme.language == request.language
    && candidate.lexeme.status == CanonicalStatus::Active
    && candidate.sense.status == CanonicalStatus::Active
}

fn map_forms(
  candidate: &CanonicalCandidate,
  content: &crate::domain::canonical::ActiveContentVersion,
  evidence_use: EvidenceUse,
  counts: &mut CardCounts,
) -> Vec<CanonicalLookupCardForm> {
  if candidate.forms.is_empty() {
    counts.forms.missing += 1;
    return Vec::new();
  }

  let mut forms = candidate.forms.iter().collect::<Vec<_>>();
  forms.sort_by(|left, right| left.id.cmp(&right.id));
  counts.forms.truncated += forms
    .len()
    .saturating_sub(MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE);

  let mut card_forms = Vec::with_capacity(forms.len().min(MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE));
  for form in forms.into_iter().take(MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE) {
    if !form_is_visible(form, candidate, content) {
      counts.forms.filtered += 1;
      continue;
    }
    let assertion = map_assertion(
      CanonicalLookupAssertionKind::Form,
      &form.form,
      &form.evidence_ids,
      candidate,
      &content.release_id,
      evidence_use,
      &mut counts.evidence,
    );
    match assertion.assertion {
      Some(assertion) => {
        counts.forms.available += 1;
        card_forms.push(CanonicalLookupCardForm {
          id: form.id.clone(),
          kind: form.kind,
          morphology: form.morphology.clone(),
          assertion,
        });
      }
      None => counts.forms.record(assertion.state),
    }
  }
  card_forms
}

fn form_is_visible(
  form: &WordForm,
  candidate: &CanonicalCandidate,
  content: &crate::domain::canonical::ActiveContentVersion,
) -> bool {
  form.lexeme_id == candidate.lexeme.id
    && form.release_id == content.release_id
    && form.status == CanonicalStatus::Active
}

fn map_assertion(
  kind: CanonicalLookupAssertionKind,
  text: &str,
  evidence_ids: &[EvidenceId],
  candidate: &CanonicalCandidate,
  release_id: &crate::domain::canonical::ReleaseId,
  evidence_use: EvidenceUse,
  evidence_counts: &mut SectionCounts,
) -> MappedAssertion {
  let requested = evidence_ids.iter().cloned().collect::<BTreeSet<_>>();
  if requested.is_empty() {
    evidence_counts.missing += 1;
    return MappedAssertion::missing();
  }

  let evidence_by_id = indexed_evidence(candidate, &requested);
  let mut permitted = Vec::new();
  for evidence_id in requested {
    let Some(fragment) = evidence_by_id.get(&evidence_id) else {
      evidence_counts.filtered += 1;
      continue;
    };
    if !fragment.permits(release_id, evidence_use) {
      evidence_counts.filtered += 1;
      continue;
    }
    permitted.push((*fragment).clone());
  }

  if permitted.is_empty() {
    return MappedAssertion::filtered();
  }

  let permitted_count = permitted.len();
  let truncated = permitted_count.saturating_sub(MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION);
  evidence_counts.available += permitted_count.min(MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION);
  evidence_counts.truncated += truncated;
  permitted.truncate(MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION);

  MappedAssertion::available(CanonicalLookupCardAssertion {
    kind,
    text: text.to_string(),
    evidence: permitted.into_iter().map(card_evidence).collect(),
  })
}

fn indexed_evidence<'a>(
  candidate: &'a CanonicalCandidate,
  requested: &BTreeSet<EvidenceId>,
) -> BTreeMap<EvidenceId, &'a EvidenceFragment> {
  let mut indexed = BTreeMap::new();
  for fragment in &candidate.evidence {
    if !requested.contains(&fragment.id) {
      continue;
    }
    match indexed.entry(fragment.id.clone()) {
      std::collections::btree_map::Entry::Occupied(mut entry) => {
        if fragment < *entry.get() {
          entry.insert(fragment);
        }
      }
      std::collections::btree_map::Entry::Vacant(entry) => {
        entry.insert(fragment);
      }
    }
  }
  indexed
}

fn card_evidence(fragment: EvidenceFragment) -> CanonicalLookupCardEvidence {
  CanonicalLookupCardEvidence {
    id: fragment.id,
    kind: fragment.kind,
    confidence: fragment.confidence,
    text: fragment.text,
    provenance: CanonicalLookupCardEvidenceProvenance {
      source_id: fragment.source_id,
      source_reference: fragment.source_reference,
      release_id: fragment.release_id,
      language: fragment.language,
      content_hash: fragment.content_hash,
    },
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssertionState {
  Available,
  Missing,
  Filtered,
}

struct MappedAssertion {
  assertion: Option<CanonicalLookupCardAssertion>,
  state: AssertionState,
}

impl MappedAssertion {
  fn available(assertion: CanonicalLookupCardAssertion) -> Self {
    Self {
      assertion: Some(assertion),
      state: AssertionState::Available,
    }
  }

  fn missing() -> Self {
    Self {
      assertion: None,
      state: AssertionState::Missing,
    }
  }

  fn filtered() -> Self {
    Self {
      assertion: None,
      state: AssertionState::Filtered,
    }
  }
}

#[derive(Default)]
struct CardCounts {
  retrieval: SectionCounts,
  lexemes: SectionCounts,
  parts_of_speech: SectionCounts,
  senses: SectionCounts,
  definitions: SectionCounts,
  forms: SectionCounts,
  evidence: SectionCounts,
}

impl CardCounts {
  fn record_visible_candidate(&mut self) {
    self.retrieval.available += 1;
    self.lexemes.available += 1;
    self.parts_of_speech.available += 1;
    self.senses.available += 1;
  }

  fn record_filtered_candidate(&mut self) {
    self.retrieval.filtered += 1;
    self.lexemes.filtered += 1;
    self.parts_of_speech.filtered += 1;
    self.senses.filtered += 1;
    self.definitions.filtered += 1;
    self.forms.filtered += 1;
    self.evidence.filtered += 1;
  }

  fn candidate_section(&self) -> CanonicalLookupCardSectionCoverage {
    self.retrieval.to_coverage()
  }
}

#[derive(Default)]
struct SectionCounts {
  available: usize,
  missing: usize,
  filtered: usize,
  truncated: usize,
}

impl SectionCounts {
  fn record(&mut self, state: AssertionState) {
    match state {
      AssertionState::Available => self.available += 1,
      AssertionState::Missing => self.missing += 1,
      AssertionState::Filtered => self.filtered += 1,
    }
  }

  fn to_coverage(&self) -> CanonicalLookupCardSectionCoverage {
    let mut missing = self.missing;
    if self.available == 0 && self.filtered == 0 && missing == 0 {
      missing = 1;
    }
    let state = if self.available > 0 {
      CanonicalLookupCardCoverageState::Available
    } else if self.filtered > 0 {
      CanonicalLookupCardCoverageState::Filtered
    } else {
      CanonicalLookupCardCoverageState::Missing
    };
    CanonicalLookupCardSectionCoverage {
      state,
      available_items: self.available,
      missing_items: missing,
      filtered_items: self.filtered,
      truncated_items: self.truncated,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{
    canonical::{
      CanonicalId, EvidenceConfidence, EvidenceKind, FormKind, LanguageTag, Lexeme,
      LexicalPartOfSpeech, Sense, SourcePermissions,
    },
    retrieval::CandidateFeatures,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language() -> LanguageTag {
    LanguageTag::parse("en").unwrap()
  }

  fn content() -> crate::domain::canonical::ActiveContentVersion {
    crate::domain::canonical::ActiveContentVersion {
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "rank-v1".to_string(),
    }
  }

  fn request() -> RetrievalRequest {
    RetrievalRequest::new(" HOT ", language(), EvidenceUse::ApiRedistribution, 4).unwrap()
  }

  fn permissions() -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding: true,
      model_processing: true,
      api_redistribution: true,
    }
  }

  fn candidate(sense_id: &str, lemma: &str) -> CanonicalCandidate {
    let lexeme_id = id(&format!("lexeme-{sense_id}"));
    let evidence_id = id(&format!("evidence-{sense_id}"));
    CanonicalCandidate {
      lexeme: Lexeme {
        id: lexeme_id.clone(),
        release_id: id("release-1"),
        language: language(),
        lemma: lemma.to_string(),
        normalized_lemma: lemma.to_string(),
        part_of_speech: LexicalPartOfSpeech::Adjective,
        status: CanonicalStatus::Active,
      },
      sense: Sense {
        id: id(sense_id),
        lexeme_id: lexeme_id.clone(),
        release_id: id("release-1"),
        sense_key: format!("{lemma}-sense"),
        definition: format!("definition for {lemma}"),
        definition_evidence_ids: vec![evidence_id.clone()],
        status: CanonicalStatus::Active,
      },
      forms: vec![WordForm {
        id: id(&format!("form-{sense_id}")),
        lexeme_id,
        release_id: id("release-1"),
        form: format!("{lemma}er"),
        normalized_form: format!("{lemma}er"),
        kind: FormKind::Inflection,
        morphology: Some("comparative".to_string()),
        evidence_ids: vec![evidence_id.clone()],
        status: CanonicalStatus::Active,
      }],
      evidence: vec![EvidenceFragment {
        id: evidence_id,
        source_id: id("source-1"),
        source_reference: "entry-1".to_string(),
        release_id: id("release-1"),
        language: language(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: format!("evidence for {lemma}"),
        content_hash: format!("hash-{sense_id}"),
        permissions: permissions(),
        status: CanonicalStatus::Active,
      }],
    }
  }

  fn ranked(candidate: CanonicalCandidate, rank: usize) -> RankedCandidate {
    RankedCandidate {
      candidate,
      features: CandidateFeatures::default(),
      rank,
      fusion_score: 1_000 - rank as u64,
    }
  }

  #[test]
  fn mapper_preserves_ranked_candidate_order_and_separates_card_entities() {
    let first = candidate("sense-first", "first");
    let second = candidate("sense-second", "second");
    let card = CanonicalLookupCardMapper::assemble(
      &request(),
      RetrievalOutcome {
        content: content(),
        path: RetrievalPath::Hybrid,
        candidates: vec![ranked(second, 2), ranked(first, 1)],
      },
    );

    assert_eq!(card.query.normalized_query, "hot");
    assert_eq!(card.candidates[0].sense.id.as_str(), "sense-second");
    assert_eq!(card.candidates[0].rank, 2);
    assert_eq!(card.candidates[0].lexeme.lemma, "second");
    assert_eq!(
      card.candidates[0].lexeme.part_of_speech,
      LexicalPartOfSpeech::Adjective
    );
    assert_eq!(card.candidates[0].forms[0].assertion.text, "seconder");
    assert_eq!(
      card.candidates[0]
        .sense
        .definition
        .as_ref()
        .unwrap()
        .evidence[0]
        .provenance
        .source_reference,
      "entry-1"
    );
  }

  #[test]
  fn mapper_hides_unpermitted_assertions_and_reports_filtered_coverage() {
    let mut blocked = candidate("sense-blocked", "blocked");
    blocked.evidence[0].permissions.api_redistribution = false;
    let card = CanonicalLookupCardMapper::assemble(
      &request(),
      RetrievalOutcome {
        content: content(),
        path: RetrievalPath::Hybrid,
        candidates: vec![ranked(blocked, 1)],
      },
    );

    assert!(card.candidates[0].sense.definition.is_none());
    assert!(card.candidates[0].forms.is_empty());
    assert_eq!(
      card.coverage.definitions.state,
      CanonicalLookupCardCoverageState::Filtered
    );
    assert_eq!(
      card.coverage.forms.state,
      CanonicalLookupCardCoverageState::Filtered
    );
    assert_eq!(card.coverage.evidence.filtered_items, 2);
  }

  #[test]
  fn mapper_marks_missing_forms_and_vector_fallback_without_dropping_definition() {
    let mut without_forms = candidate("sense-hot", "hot");
    without_forms.forms.clear();
    let card = CanonicalLookupCardMapper::assemble(
      &request(),
      RetrievalOutcome {
        content: content(),
        path: RetrievalPath::LexicalFallback,
        candidates: vec![ranked(without_forms, 1)],
      },
    );

    assert!(card.candidates[0].sense.definition.is_some());
    assert_eq!(
      card.coverage.forms.state,
      CanonicalLookupCardCoverageState::Missing
    );
    assert_eq!(
      card.coverage.retrieval.state,
      CanonicalLookupCardCoverageState::VectorDegraded
    );
  }

  #[test]
  fn mapper_applies_form_and_evidence_presentation_bounds() {
    let mut bounded = candidate("sense-bounded", "bounded");
    let template_evidence = bounded.evidence[0].clone();
    let evidence_ids = (0..=MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION)
      .map(|index| {
        let evidence_id = id(&format!("evidence-bounded-{index}"));
        let mut evidence = template_evidence.clone();
        evidence.id = evidence_id.clone();
        evidence.source_reference = format!("entry-{index}");
        bounded.evidence.push(evidence);
        evidence_id
      })
      .collect::<Vec<_>>();
    bounded.evidence.remove(0);
    bounded.sense.definition_evidence_ids = evidence_ids.clone();
    bounded.forms = (0..=MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE)
      .map(|index| WordForm {
        id: id(&format!("form-bounded-{index}")),
        lexeme_id: bounded.lexeme.id.clone(),
        release_id: id("release-1"),
        form: format!("bounded-{index}"),
        normalized_form: format!("bounded-{index}"),
        kind: FormKind::Inflection,
        morphology: None,
        evidence_ids: vec![evidence_ids[0].clone()],
        status: CanonicalStatus::Active,
      })
      .collect();

    let card = CanonicalLookupCardMapper::assemble(
      &request(),
      RetrievalOutcome {
        content: content(),
        path: RetrievalPath::Hybrid,
        candidates: vec![ranked(bounded, 1)],
      },
    );

    assert_eq!(
      card.candidates[0].forms.len(),
      MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE
    );
    assert_eq!(
      card.candidates[0]
        .sense
        .definition
        .as_ref()
        .unwrap()
        .evidence
        .len(),
      MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION
    );
    assert_eq!(card.coverage.forms.truncated_items, 1);
    assert_eq!(card.coverage.evidence.truncated_items, 1);
  }
}
