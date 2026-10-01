//! Deterministic assembly of bounded canonical lookup cards.
//!
//! This mapper converts release-pinned canonical candidates into typed target card values. It
//! does not call a model, expose a route, or persist a snapshot.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
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
};

/// Pure mapper from release-pinned candidates to a bounded canonical lookup card.
///
/// It preserves the incoming ranked-candidate order and rank values. The mapper is public so
/// non-HTTP callers can assemble a card from an already retrieved immutable outcome without
/// reinvoking retrieval.
pub struct CanonicalLookupCardMapper;

impl CanonicalLookupCardMapper {
  /// Assembles the same bounded card from a canonical-only pin and ranked lexical candidates.
  pub fn assemble_canonical(
    request: &RetrievalRequest,
    pin: crate::domain::canonical::CanonicalReleasePin,
    candidates: Vec<RankedCandidate>,
  ) -> CanonicalLookupCard<crate::domain::canonical::CanonicalReleasePin> {
    let release_id = pin.release_id.clone();
    Self::assemble_with_content(request, candidates, pin, &release_id)
  }

  fn assemble_with_content<C>(
    request: &RetrievalRequest,
    ranked_candidates: Vec<RankedCandidate>,
    content: C,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> CanonicalLookupCard<C> {
    let candidate_limit = request.limit.min(MAX_LOOKUP_CARD_CANDIDATES);
    let source_candidate_count = ranked_candidates.len();
    let mut counts = CardCounts::default();
    let mut candidates = Vec::with_capacity(candidate_limit);

    for ranked in ranked_candidates.into_iter().take(candidate_limit) {
      if let Some(candidate) = map_candidate(&ranked, request, release_id, &mut counts) {
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
    let retrieval = candidate_coverage.clone();

    CanonicalLookupCard {
      query: CanonicalLookupQueryAnalysis {
        normalized_query: request.query.clone(),
        language: request.language.clone(),
        evidence_use: request.evidence_use,
      },
      content,
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
  release_id: &crate::domain::canonical::ReleaseId,
  counts: &mut CardCounts,
) -> Option<CanonicalLookupCardCandidate> {
  let candidate = &ranked.candidate;
  if !candidate_is_visible(candidate, request, release_id) {
    return None;
  }

  counts.record_visible_candidate();
  let definition = map_assertion(
    CanonicalLookupAssertionKind::Definition,
    &candidate.sense.definition,
    &candidate.sense.definition_evidence_ids,
    candidate,
    release_id,
    request.evidence_use,
    &mut counts.evidence,
  );
  counts.definitions.record(definition.state);

  let forms = map_forms(candidate, release_id, request.evidence_use, counts);
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
  release_id: &crate::domain::canonical::ReleaseId,
) -> bool {
  candidate.lexeme.release_id == *release_id
    && candidate.sense.release_id == *release_id
    && candidate.lexeme.id == candidate.sense.lexeme_id
    && candidate.lexeme.language == request.language
    && candidate.lexeme.status == CanonicalStatus::Active
    && candidate.sense.status == CanonicalStatus::Active
}

fn map_forms(
  candidate: &CanonicalCandidate,
  release_id: &crate::domain::canonical::ReleaseId,
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
    if !form_is_visible(form, candidate, release_id) {
      counts.forms.filtered += 1;
      continue;
    }
    let assertion = map_assertion(
      CanonicalLookupAssertionKind::Form,
      &form.form,
      &form.evidence_ids,
      candidate,
      release_id,
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
  release_id: &crate::domain::canonical::ReleaseId,
) -> bool {
  form.lexeme_id == candidate.lexeme.id
    && form.release_id == *release_id
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
  permitted.truncate(MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION);
  let evidence = permitted
    .into_iter()
    .map(|fragment| card_evidence(fragment, candidate, evidence_use))
    .collect::<Option<Vec<_>>>();
  let Some(evidence) = evidence else {
    evidence_counts.filtered += permitted_count;
    return MappedAssertion::filtered();
  };
  evidence_counts.available += evidence.len();
  evidence_counts.truncated += truncated;

  MappedAssertion::available(CanonicalLookupCardAssertion {
    kind,
    text: text.to_string(),
    evidence,
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

fn card_evidence(
  fragment: EvidenceFragment,
  candidate: &CanonicalCandidate,
  evidence_use: EvidenceUse,
) -> Option<CanonicalLookupCardEvidence> {
  let source = candidate.source_for(&fragment)?;
  let attribution = source.attribution.as_ref()?;
  if !source.permissions.allows(evidence_use) || attribution.trim().is_empty() {
    return None;
  }
  Some(CanonicalLookupCardEvidence {
    id: fragment.id,
    kind: fragment.kind,
    confidence: fragment.confidence,
    text: fragment.text,
    provenance: CanonicalLookupCardEvidenceProvenance {
      source_id: fragment.source_id,
      attribution: attribution.clone(),
      source_reference: fragment.source_reference,
      release_id: fragment.release_id,
      language: fragment.language,
      content_hash: fragment.content_hash,
    },
  })
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
      LexicalPartOfSpeech, LexicalSource, Sense, SourcePermissions,
    },
    retrieval::CandidateFeatures,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language() -> LanguageTag {
    LanguageTag::parse("en").unwrap()
  }

  fn pin() -> crate::domain::canonical::CanonicalReleasePin {
    crate::domain::canonical::CanonicalReleasePin::new(
      id("release-1"),
      "canonical-data-v1".to_string(),
    )
    .unwrap()
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
        lemma_evidence_ids: vec![evidence_id.clone()],
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
      sources: vec![LexicalSource {
        id: id("source-1"),
        name: "Test dictionary".into(),
        version: "v1".into(),
        license: "test".into(),
        attribution: Some("Test dictionary".into()),
        permissions: permissions(),
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

  fn assemble(
    candidates: Vec<RankedCandidate>,
  ) -> CanonicalLookupCard<crate::domain::canonical::CanonicalReleasePin> {
    CanonicalLookupCardMapper::assemble_canonical(&request(), pin(), candidates)
  }

  #[test]
  fn mapper_preserves_ranked_candidate_order_and_separates_card_entities() {
    let first = candidate("sense-first", "first");
    let second = candidate("sense-second", "second");
    let card = assemble(vec![ranked(second, 2), ranked(first, 1)]);

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
    let card = assemble(vec![ranked(blocked, 1)]);

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
  fn mapper_marks_missing_forms_without_dropping_definition() {
    let mut without_forms = candidate("sense-hot", "hot");
    without_forms.forms.clear();
    let card = assemble(vec![ranked(without_forms, 1)]);

    assert!(card.candidates[0].sense.definition.is_some());
    assert_eq!(
      card.coverage.forms.state,
      CanonicalLookupCardCoverageState::Missing
    );
    assert_eq!(
      card.coverage.retrieval.state,
      CanonicalLookupCardCoverageState::Available
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

    let card = assemble(vec![ranked(bounded, 1)]);

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
