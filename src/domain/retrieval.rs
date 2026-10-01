//! Deterministic canonical-retrieval requests, candidates, and fusion.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use super::canonical::{
  normalize_lookup_key, ActiveContentVersion, CanonicalStatus, EvidenceFragment, EvidenceId,
  EvidenceUse, FormKind, LanguageTag, Lexeme, LexicalSource, ReleaseId, Sense, SenseId,
  VectorCollectionId,
};

/// Default maximum number of ranked candidates returned by a retrieval request.
pub const DEFAULT_RETRIEVAL_LIMIT: usize = 12;
/// Largest candidate limit accepted by the retrieval foundation.
pub const MAX_RETRIEVAL_LIMIT: usize = 50;
/// Largest valid score represented in basis points.
pub const MAX_SCORE_BASIS_POINTS: u16 = 10_000;

/// Retrieval path used to create a deterministic canonical candidate list.
///
/// This is a domain outcome rather than an infrastructure health signal. A lexical-only result
/// remains valid canonical output, while callers may surface its vector degradation distinctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalPath {
  /// Pinned vector retrieval completed; eligible vector signals were fused with lexical signals.
  Hybrid,
  /// The derived vector dependency failed, so canonical lexical retrieval continued alone.
  LexicalFallback,
}

/// Validation failure for a retrieval request or score.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RetrievalValidationError {
  /// The normalized lookup query was blank.
  #[error("retrieval query must not be blank")]
  BlankQuery,
  /// The requested candidate limit was outside the bounded retrieval range.
  #[error("retrieval limit must be between 1 and {MAX_RETRIEVAL_LIMIT}")]
  InvalidLimit,
  /// A normalized score was larger than one.
  #[error("retrieval score must be at most {MAX_SCORE_BASIS_POINTS} basis points")]
  InvalidScore,
}

/// A normalized deterministic ranking signal represented in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RetrievalScore(u16);

impl RetrievalScore {
  /// Creates a normalized score from zero through 10,000 basis points.
  ///
  /// # Errors
  ///
  /// Returns an error when `basis_points` exceeds 10,000.
  pub const fn new(basis_points: u16) -> Result<Self, RetrievalValidationError> {
    if basis_points > MAX_SCORE_BASIS_POINTS {
      return Err(RetrievalValidationError::InvalidScore);
    }
    Ok(Self(basis_points))
  }

  /// Returns a score representing an exact deterministic match.
  pub const fn exact() -> Self {
    Self(MAX_SCORE_BASIS_POINTS)
  }

  /// Returns the score in basis points.
  pub const fn basis_points(self) -> u16 {
    self.0
  }
}

/// Bounded, normalized request for canonical retrieval.
#[derive(Clone, PartialEq, Eq)]
pub struct RetrievalRequest {
  /// NFC and case-normalized query used only for retrieval, never telemetry.
  pub query: String,
  /// Language in which the query must be resolved.
  pub language: LanguageTag,
  /// Operation for which source permissions must be enforced.
  pub evidence_use: EvidenceUse,
  /// Maximum number of ranked canonical senses to return.
  pub limit: usize,
}

impl RetrievalRequest {
  /// Validates and normalizes a bounded canonical retrieval request.
  ///
  /// # Errors
  ///
  /// Returns an error for a blank query or an out-of-range candidate limit.
  pub fn new(
    query: &str,
    language: LanguageTag,
    evidence_use: EvidenceUse,
    limit: usize,
  ) -> Result<Self, RetrievalValidationError> {
    let query = normalize_lookup_key(query);
    if query.is_empty() {
      return Err(RetrievalValidationError::BlankQuery);
    }
    if !(1..=MAX_RETRIEVAL_LIMIT).contains(&limit) {
      return Err(RetrievalValidationError::InvalidLimit);
    }
    Ok(Self {
      query,
      language,
      evidence_use,
      limit,
    })
  }

  /// Creates a public-API retrieval request with the default candidate limit.
  ///
  /// # Errors
  ///
  /// Returns an error when `query` is blank after normalization.
  pub fn for_public_api(
    query: &str,
    language: LanguageTag,
  ) -> Result<Self, RetrievalValidationError> {
    Self::new(
      query,
      language,
      EvidenceUse::ApiRedistribution,
      DEFAULT_RETRIEVAL_LIMIT,
    )
  }
}

/// Repository query pinned to one active lexical and vector content version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalSearchRequest {
  /// Caller request after validation and normalization.
  pub retrieval: RetrievalRequest,
  /// Exact release and vector version selected before retrieval begins.
  pub content: ActiveContentVersion,
}

/// Repository request that hydrates a bounded set of sense IDs into canonical candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateLoadRequest {
  /// Exact release whose canonical entities may be loaded.
  pub release_id: ReleaseId,
  /// Source permission operation that hydrated evidence must satisfy.
  pub evidence_use: EvidenceUse,
  /// Stable sense identifiers requested from vector retrieval.
  pub sense_ids: Vec<SenseId>,
}

/// Closed candidate class ordered by canonical lookup precedence.
///
/// The class is semantic, not a score: a lower-precedence candidate cannot outrank an eligible
/// higher-precedence candidate by accumulating vector or full-text signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexicalMatchKind {
  /// Query exactly matched the canonical form owned by this lexeme.
  ExactCanonical,
  /// Query exactly matched a published alias, spelling variant, or phrase form.
  ExactAlias,
  /// Query resolved through a bounded inflection or morphology rule.
  Inflection,
  /// Query resolved through a bounded language-specific spelling correction.
  SpellingCorrection,
  /// Query resolved through a bounded published transliteration rule.
  Transliteration,
  /// Query was nominated only by full-text or semantic retrieval.
  Semantic,
}

impl LexicalMatchKind {
  /// Returns the strict resolver precedence, where a larger value is preferred.
  pub const fn precedence(self) -> u8 {
    match self {
      Self::ExactCanonical => 6,
      Self::ExactAlias => 5,
      Self::Inflection => 4,
      Self::SpellingCorrection => 3,
      Self::Transliteration => 2,
      Self::Semantic => 1,
    }
  }
}

/// Vector record purpose, kept separate to prevent unrelated embeddings from being mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VectorPurpose {
  /// Canonical English sense definition embedding.
  CanonicalEnglishSense,
  /// Localized gloss embedding for one content language.
  LocalizedGloss,
  /// Independently citable evidence-fragment embedding.
  EvidenceFragment,
}

/// One entity returned by a filtered vector query.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VectorTarget {
  /// A canonical sense eligible for hybrid candidate fusion.
  Sense(SenseId),
  /// An evidence fragment reserved for future evidence-pack retrieval.
  Evidence(EvidenceId),
}

impl VectorTarget {
  /// Returns the sense ID when this target can be fused into a canonical candidate.
  pub fn sense_id(&self) -> Option<&SenseId> {
    match self {
      Self::Sense(sense_id) => Some(sense_id),
      Self::Evidence(_) => None,
    }
  }
}

/// Mandatory release and metadata filters for one vector query.
#[derive(Clone, PartialEq, Eq)]
pub struct VectorSearchRequest {
  /// Normalized query text passed to the embedding or vector adapter.
  pub query: String,
  /// Lexical release to which every returned record must belong.
  pub release_id: ReleaseId,
  /// Immutable vector collection version to search.
  pub vector_collection_id: VectorCollectionId,
  /// Logical vector record purpose.
  pub purpose: VectorPurpose,
  /// Language of the embedded record's content.
  pub content_language: LanguageTag,
  /// Maximum number of records requested from this logical collection.
  pub limit: usize,
}

/// One scored vector result with its required version metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VectorMatch {
  /// Sense or evidence entity identified by the vector record.
  pub target: VectorTarget,
  /// Vector similarity normalized by the adapter for this collection version.
  pub score: RetrievalScore,
  /// Lexical release recorded in the vector payload.
  pub release_id: ReleaseId,
  /// Immutable vector collection recorded in the vector payload.
  pub vector_collection_id: VectorCollectionId,
  /// Logical record purpose recorded in the vector payload.
  pub purpose: VectorPurpose,
  /// Language of the embedded record recorded in the vector payload.
  pub content_language: LanguageTag,
}

impl VectorMatch {
  /// Returns whether this result exactly satisfies all required vector-query filters.
  pub fn matches_request(&self, request: &VectorSearchRequest) -> bool {
    self.release_id == request.release_id
      && self.vector_collection_id == request.vector_collection_id
      && self.purpose == request.purpose
      && self.content_language == request.content_language
  }
}

/// Canonical data required to render or further enrich one sense candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalCandidate {
  /// Lexeme that owns the candidate sense.
  pub lexeme: Lexeme,
  /// One specific meaning of the lexeme.
  pub sense: Sense,
  /// Stored forms that contributed to the retrieval decision.
  pub forms: Vec<super::canonical::WordForm>,
  /// Permitted evidence available to support the sense definition.
  pub evidence: Vec<EvidenceFragment>,
  /// Authoritative source-policy records for evidence carried by this candidate.
  pub sources: Vec<LexicalSource>,
}

impl CanonicalCandidate {
  /// Finds the authoritative source-policy record for one evidence fragment.
  pub fn source_for(&self, fragment: &EvidenceFragment) -> Option<&LexicalSource> {
    self
      .sources
      .iter()
      .find(|source| source.id == fragment.source_id)
  }

  /// Returns whether this candidate is active, internally consistent, and fully permitted.
  pub fn is_eligible_for(&self, release_id: &ReleaseId, evidence_use: EvidenceUse) -> bool {
    self.lexeme.release_id == *release_id
      && self.sense.release_id == *release_id
      && self.lexeme.id == self.sense.lexeme_id
      && self.lexeme.status.is_lookup_eligible()
      && self.sense.status.is_lookup_eligible()
      && !self.lexeme.lemma_evidence_ids.is_empty()
      && self.lexeme.lemma_evidence_ids.len() <= 8
      && self
        .lexeme
        .lemma_evidence_ids
        .windows(2)
        .all(|pair| pair[0] < pair[1])
      && self.lexeme.lemma_evidence_ids.iter().all(|evidence_id| {
        self
          .evidence
          .iter()
          .find(|fragment| fragment.id == *evidence_id)
          .is_some_and(|fragment| fragment.permits(release_id, evidence_use))
      })
      && self.forms.iter().all(|form| {
        form.lexeme_id == self.lexeme.id
          && form.release_id == *release_id
          && form.status.is_lookup_eligible()
          && form.evidence_ids.iter().all(|evidence_id| {
            self
              .evidence
              .iter()
              .find(|fragment| fragment.id == *evidence_id)
              .is_some_and(|fragment| fragment.permits(release_id, evidence_use))
          })
      })
      && self
        .sense
        .definition_evidence_ids
        .iter()
        .all(|evidence_id| {
          self
            .evidence
            .iter()
            .find(|fragment| fragment.id == *evidence_id)
            .is_some_and(|fragment| fragment.permits(release_id, evidence_use))
        })
  }

  /// Returns the strongest simple in-memory lexical signal for `query`.
  pub fn in_memory_match(&self, query: &str) -> Option<(LexicalMatchKind, RetrievalScore)> {
    if self.lexeme.normalized_lemma == query {
      return Some((LexicalMatchKind::ExactCanonical, RetrievalScore::exact()));
    }

    self
      .forms
      .iter()
      .filter(|form| form.status == CanonicalStatus::Active && form.normalized_form == query)
      .map(|form| (form_match_kind(form.kind), form_score(form.kind)))
      .max_by(|left, right| compare_lexical_signals(*left, *right))
      .or_else(|| {
        normalize_lookup_key(&self.sense.definition)
          .contains(query)
          .then_some((LexicalMatchKind::Semantic, RetrievalScore(2_500)))
      })
  }
}

/// One repository-produced canonical candidate and its lexical signal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryMatch {
  /// Fully hydrated canonical candidate.
  pub candidate: CanonicalCandidate,
  /// Retrieval signal produced by the repository.
  pub kind: LexicalMatchKind,
  /// Normalized rank or match confidence for this signal.
  pub score: RetrievalScore,
}

/// Hydrated canonical data associated with one filtered vector result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HydratedVectorMatch {
  /// Canonical candidate loaded through the repository port.
  pub candidate: CanonicalCandidate,
  /// Filtered vector result that selected the candidate.
  pub vector: VectorMatch,
}

/// Independent ranking features retained after hybrid fusion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CandidateFeatures {
  /// Strongest exact-canonical signal.
  pub exact_canonical: Option<RetrievalScore>,
  /// Strongest exact-alias signal.
  pub exact_alias: Option<RetrievalScore>,
  /// Strongest bounded inflection signal.
  pub inflection: Option<RetrievalScore>,
  /// Strongest bounded spelling-correction signal.
  pub spelling_correction: Option<RetrievalScore>,
  /// Strongest bounded transliteration signal.
  pub transliteration: Option<RetrievalScore>,
  /// Strongest semantic or full-text signal.
  pub semantic: Option<RetrievalScore>,
  /// Strongest vector similarity from the pinned collection.
  pub vector_similarity: Option<RetrievalScore>,
  /// Logical vector purposes that corroborated the candidate.
  pub vector_purposes: BTreeSet<VectorPurpose>,
}

impl CandidateFeatures {
  /// Calculates the deterministic fusion value used only for ordering.
  ///
  /// The value is not a probability or a semantic confidence score. Exact lexical evidence has
  /// the highest weight, while full-text and vector signals add corroborating evidence.
  pub fn fusion_score(&self) -> u64 {
    const EXACT_CANONICAL_WEIGHT: u64 = 1_000;
    const EXACT_ALIAS_WEIGHT: u64 = 900;
    const INFLECTION_WEIGHT: u64 = 700;
    const SPELLING_WEIGHT: u64 = 500;
    const TRANSLITERATION_WEIGHT: u64 = 400;
    const SEMANTIC_WEIGHT: u64 = 300;
    const VECTOR_WEIGHT: u64 = 300;

    weighted(self.exact_canonical, EXACT_CANONICAL_WEIGHT)
      + weighted(self.exact_alias, EXACT_ALIAS_WEIGHT)
      + weighted(self.inflection, INFLECTION_WEIGHT)
      + weighted(self.spelling_correction, SPELLING_WEIGHT)
      + weighted(self.transliteration, TRANSLITERATION_WEIGHT)
      + weighted(self.semantic, SEMANTIC_WEIGHT)
      + weighted(self.vector_similarity, VECTOR_WEIGHT)
  }

  /// Returns the best deterministic match class carried by this candidate.
  ///
  /// Vector-only candidates are semantic nominations and therefore never outrank an exact or
  /// bounded lexical match.
  pub fn best_match_kind(&self) -> Option<LexicalMatchKind> {
    [
      (LexicalMatchKind::ExactCanonical, self.exact_canonical),
      (LexicalMatchKind::ExactAlias, self.exact_alias),
      (LexicalMatchKind::Inflection, self.inflection),
      (
        LexicalMatchKind::SpellingCorrection,
        self.spelling_correction,
      ),
      (LexicalMatchKind::Transliteration, self.transliteration),
      (LexicalMatchKind::Semantic, self.semantic),
    ]
    .into_iter()
    .find_map(|(kind, score)| score.map(|_| kind))
    .or_else(|| self.vector_similarity.map(|_| LexicalMatchKind::Semantic))
  }

  fn record_lexical(&mut self, kind: LexicalMatchKind, score: RetrievalScore) {
    let feature = match kind {
      LexicalMatchKind::ExactCanonical => &mut self.exact_canonical,
      LexicalMatchKind::ExactAlias => &mut self.exact_alias,
      LexicalMatchKind::Inflection => &mut self.inflection,
      LexicalMatchKind::SpellingCorrection => &mut self.spelling_correction,
      LexicalMatchKind::Transliteration => &mut self.transliteration,
      LexicalMatchKind::Semantic => &mut self.semantic,
    };
    update_highest(feature, score);
  }

  fn record_vector(&mut self, score: RetrievalScore, purpose: VectorPurpose) {
    update_highest(&mut self.vector_similarity, score);
    self.vector_purposes.insert(purpose);
  }
}

/// One deduplicated, deterministically ordered canonical retrieval result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCandidate {
  /// One canonical lexeme and sense.
  pub candidate: CanonicalCandidate,
  /// Independent lexical and vector features kept for explainable ranking.
  pub features: CandidateFeatures,
  /// Deterministic zero-free rank after fusion and tie-breaking.
  pub rank: usize,
  /// Deterministic weighted ordering value, not a calibrated confidence.
  pub fusion_score: u64,
}

/// Fuses lexical and vector candidates by sense ID, retaining each strongest independent signal.
///
/// Candidates are filtered again for release, lifecycle, lexeme/sense ownership, and source
/// permissions as defense in depth. Only candidates in the best available match class survive;
/// equal-class candidates use scores and complete lexical IDs as stable tie-breaks, so reordering
/// adapter results does not change the ranked output or collapse genuine ambiguity.
pub fn fuse_candidates(
  content: &ActiveContentVersion,
  evidence_use: EvidenceUse,
  lexical_matches: impl IntoIterator<Item = RepositoryMatch>,
  vector_matches: impl IntoIterator<Item = HydratedVectorMatch>,
) -> Vec<RankedCandidate> {
  let mut candidates = BTreeMap::<SenseId, CandidateAccumulator>::new();

  for lexical in lexical_matches {
    if !lexical
      .candidate
      .is_eligible_for(&content.release_id, evidence_use)
    {
      continue;
    }
    merge_candidate(
      &mut candidates,
      lexical.candidate,
      CandidateSignal::Lexical(lexical.kind, lexical.score),
    );
  }

  for vector in vector_matches {
    if vector.vector.target.sense_id() != Some(&vector.candidate.sense.id)
      || !matches!(
        vector.vector.purpose,
        VectorPurpose::CanonicalEnglishSense | VectorPurpose::LocalizedGloss
      )
      || vector.vector.release_id != content.release_id
      || vector.vector.vector_collection_id != content.vector_collection_id
      || !vector
        .candidate
        .is_eligible_for(&content.release_id, evidence_use)
    {
      continue;
    }
    merge_candidate(
      &mut candidates,
      vector.candidate,
      CandidateSignal::Vector(vector.vector.score, vector.vector.purpose),
    );
  }

  rank_accumulated(candidates)
}

/// Deterministically resolves, deduplicates, and ranks canonical-only lexical matches.
///
/// This shares the exact class precedence and ordering used by hybrid fusion without requiring
/// a vector collection or a fabricated ranking-version identifier.
pub fn rank_lexical_candidates(
  release_id: &ReleaseId,
  evidence_use: EvidenceUse,
  lexical_matches: impl IntoIterator<Item = RepositoryMatch>,
) -> Vec<RankedCandidate> {
  let mut candidates = BTreeMap::<SenseId, CandidateAccumulator>::new();
  for lexical in lexical_matches {
    if lexical.candidate.is_eligible_for(release_id, evidence_use) {
      merge_candidate(
        &mut candidates,
        lexical.candidate,
        CandidateSignal::Lexical(lexical.kind, lexical.score),
      );
    }
  }
  rank_accumulated(candidates)
}

fn rank_accumulated(candidates: BTreeMap<SenseId, CandidateAccumulator>) -> Vec<RankedCandidate> {
  let mut ranked = candidates
    .into_values()
    .map(|accumulator| RankedCandidate {
      fusion_score: accumulator.features.fusion_score(),
      candidate: accumulator.candidate,
      features: accumulator.features,
      rank: 0,
    })
    .collect::<Vec<_>>();
  ranked.sort_by(|left, right| {
    right
      .features
      .best_match_kind()
      .map(LexicalMatchKind::precedence)
      .cmp(
        &left
          .features
          .best_match_kind()
          .map(LexicalMatchKind::precedence),
      )
      .then_with(|| right.fusion_score.cmp(&left.fusion_score))
      .then_with(|| {
        right
          .features
          .vector_similarity
          .cmp(&left.features.vector_similarity)
      })
      .then_with(|| left.candidate.sense.id.cmp(&right.candidate.sense.id))
      .then_with(|| left.candidate.lexeme.id.cmp(&right.candidate.lexeme.id))
  });
  if let Some(best_kind) = ranked
    .first()
    .and_then(|candidate| candidate.features.best_match_kind())
  {
    ranked.retain(|candidate| candidate.features.best_match_kind() == Some(best_kind));
  }
  for (index, candidate) in ranked.iter_mut().enumerate() {
    candidate.rank = index + 1;
  }
  ranked
}

#[derive(Debug)]
struct CandidateAccumulator {
  candidate: CanonicalCandidate,
  features: CandidateFeatures,
}

#[derive(Debug, Clone, Copy)]
enum CandidateSignal {
  Lexical(LexicalMatchKind, RetrievalScore),
  Vector(RetrievalScore, VectorPurpose),
}

fn merge_candidate(
  candidates: &mut BTreeMap<SenseId, CandidateAccumulator>,
  candidate: CanonicalCandidate,
  signal: CandidateSignal,
) {
  let sense_id = candidate.sense.id.clone();
  match candidates.entry(sense_id) {
    std::collections::btree_map::Entry::Occupied(mut entry) => {
      let accumulator = entry.get_mut();
      if candidate < accumulator.candidate {
        accumulator.candidate = candidate;
      }
      record_signal(&mut accumulator.features, signal);
    }
    std::collections::btree_map::Entry::Vacant(entry) => {
      let mut features = CandidateFeatures::default();
      record_signal(&mut features, signal);
      entry.insert(CandidateAccumulator {
        candidate,
        features,
      });
    }
  }
}

fn record_signal(features: &mut CandidateFeatures, signal: CandidateSignal) {
  match signal {
    CandidateSignal::Lexical(kind, score) => features.record_lexical(kind, score),
    CandidateSignal::Vector(score, purpose) => features.record_vector(score, purpose),
  }
}

fn update_highest(feature: &mut Option<RetrievalScore>, score: RetrievalScore) {
  if feature.is_none_or(|current| score > current) {
    *feature = Some(score);
  }
}

fn weighted(score: Option<RetrievalScore>, weight: u64) -> u64 {
  score.map_or(0, |value| u64::from(value.basis_points()) * weight)
}

fn form_match_kind(kind: FormKind) -> LexicalMatchKind {
  match kind {
    FormKind::Lemma => LexicalMatchKind::ExactCanonical,
    FormKind::SpellingVariant | FormKind::Alias | FormKind::Phrase => LexicalMatchKind::ExactAlias,
    FormKind::Inflection => LexicalMatchKind::Inflection,
  }
}

fn form_score(kind: FormKind) -> RetrievalScore {
  match kind {
    FormKind::Lemma | FormKind::SpellingVariant | FormKind::Alias | FormKind::Phrase => {
      RetrievalScore::exact()
    }
    FormKind::Inflection => RetrievalScore(7_500),
  }
}

fn compare_lexical_signals(
  left: (LexicalMatchKind, RetrievalScore),
  right: (LexicalMatchKind, RetrievalScore),
) -> std::cmp::Ordering {
  left
    .0
    .precedence()
    .cmp(&right.0.precedence())
    .then_with(|| left.1.cmp(&right.1))
}

impl std::fmt::Debug for RetrievalRequest {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str("RetrievalRequest(REDACTED)")
  }
}

impl std::fmt::Debug for VectorSearchRequest {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str("VectorSearchRequest(REDACTED)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::{
    CanonicalId, EvidenceConfidence, EvidenceKind, LexicalPartOfSpeech, SourcePermissions, WordForm,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language() -> LanguageTag {
    LanguageTag::parse("en").unwrap()
  }

  fn content() -> ActiveContentVersion {
    ActiveContentVersion {
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "lookup-rank-v1".to_string(),
    }
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
    let release_id = id("release-1");
    let lexeme_id = id(&format!("lexeme-{sense_id}"));
    let evidence_id = id(&format!("evidence-{sense_id}"));
    CanonicalCandidate {
      lexeme: Lexeme {
        id: lexeme_id.clone(),
        release_id: release_id.clone(),
        language: language(),
        lemma: lemma.to_string(),
        lemma_evidence_ids: vec![evidence_id.clone()],
        normalized_lemma: normalize_lookup_key(lemma),
        part_of_speech: LexicalPartOfSpeech::Adjective,
        status: CanonicalStatus::Active,
      },
      sense: Sense {
        id: id(sense_id),
        lexeme_id,
        release_id: release_id.clone(),
        sense_key: format!("{lemma}-1"),
        definition: format!("definition for {lemma}"),
        definition_evidence_ids: vec![evidence_id.clone()],
        status: CanonicalStatus::Active,
      },
      forms: vec![WordForm {
        id: id(&format!("form-{sense_id}")),
        lexeme_id: id(&format!("lexeme-{sense_id}")),
        release_id: release_id.clone(),
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
        source_reference: "item-1".to_string(),
        release_id,
        language: language(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: format!("definition evidence for {lemma}"),
        content_hash: "hash-1".to_string(),
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

  fn vector_match(candidate: CanonicalCandidate, score: u16) -> HydratedVectorMatch {
    HydratedVectorMatch {
      vector: VectorMatch {
        target: VectorTarget::Sense(candidate.sense.id.clone()),
        score: RetrievalScore::new(score).unwrap(),
        release_id: id("release-1"),
        vector_collection_id: id("vectors-1"),
        purpose: VectorPurpose::LocalizedGloss,
        content_language: language(),
      },
      candidate,
    }
  }

  #[test]
  fn fusion_deduplicates_by_sense_and_retains_signals() {
    let hot = candidate("sense-hot", "hot");
    let warm = candidate("sense-warm", "warm");
    let ranked = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [
        RepositoryMatch {
          candidate: hot.clone(),
          kind: LexicalMatchKind::Inflection,
          score: RetrievalScore::new(7_500).unwrap(),
        },
        RepositoryMatch {
          candidate: hot.clone(),
          kind: LexicalMatchKind::Semantic,
          score: RetrievalScore::new(2_000).unwrap(),
        },
        RepositoryMatch {
          candidate: warm,
          kind: LexicalMatchKind::Semantic,
          score: RetrievalScore::new(9_000).unwrap(),
        },
      ],
      [vector_match(hot, 9_200)],
    );

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].candidate.sense.id.as_str(), "sense-hot");
    assert_eq!(ranked[0].features.inflection.unwrap().basis_points(), 7_500);
    assert_eq!(
      ranked[0].features.vector_similarity.unwrap().basis_points(),
      9_200
    );
    assert_eq!(ranked[0].rank, 1);
  }

  #[test]
  fn fusion_is_stable_when_adapter_order_changes() {
    let first = candidate("sense-a", "a");
    let second = candidate("sense-b", "b");
    let match_for = |candidate: CanonicalCandidate| RepositoryMatch {
      candidate,
      kind: LexicalMatchKind::Semantic,
      score: RetrievalScore::new(5_000).unwrap(),
    };
    let forward = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [match_for(first.clone()), match_for(second.clone())],
      [],
    );
    let reverse = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [match_for(second), match_for(first)],
      [],
    );

    assert_eq!(forward, reverse);
    assert_eq!(forward[0].candidate.sense.id.as_str(), "sense-a");
  }

  #[test]
  fn exact_matches_exclude_lower_precedence_candidates_regardless_of_score() {
    let exact = candidate("sense-exact", "exact");
    let semantic = candidate("sense-semantic", "semantic");
    let ranked = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [
        RepositoryMatch {
          candidate: semantic,
          kind: LexicalMatchKind::Semantic,
          score: RetrievalScore::exact(),
        },
        RepositoryMatch {
          candidate: exact,
          kind: LexicalMatchKind::ExactAlias,
          score: RetrievalScore::new(8_000).unwrap(),
        },
      ],
      [],
    );

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].candidate.sense.id.as_str(), "sense-exact");
    assert_eq!(
      ranked[0].features.best_match_kind(),
      Some(LexicalMatchKind::ExactAlias)
    );
  }

  #[test]
  fn collisions_at_the_best_match_class_preserve_distinct_senses() {
    let noun = candidate("sense-run-noun", "run-noun");
    let verb = candidate("sense-run-verb", "run-verb");
    let ranked = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [
        RepositoryMatch {
          candidate: verb,
          kind: LexicalMatchKind::ExactCanonical,
          score: RetrievalScore::exact(),
        },
        RepositoryMatch {
          candidate: noun,
          kind: LexicalMatchKind::ExactCanonical,
          score: RetrievalScore::exact(),
        },
      ],
      [],
    );

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].candidate.sense.id.as_str(), "sense-run-noun");
    assert_eq!(ranked[1].candidate.sense.id.as_str(), "sense-run-verb");
  }

  #[test]
  fn fusion_excludes_evidence_that_cannot_be_redistributed() {
    let mut blocked = candidate("sense-blocked", "blocked");
    blocked.evidence[0].permissions.api_redistribution = false;
    let ranked = fuse_candidates(
      &content(),
      EvidenceUse::ApiRedistribution,
      [RepositoryMatch {
        candidate: blocked,
        kind: LexicalMatchKind::ExactCanonical,
        score: RetrievalScore::exact(),
      }],
      [],
    );

    assert!(ranked.is_empty());
  }

  #[test]
  fn vector_match_requires_exact_release_and_collection_filters() {
    let candidate = candidate("sense-hot", "hot");
    let request = VectorSearchRequest {
      query: "hot".to_string(),
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      purpose: VectorPurpose::LocalizedGloss,
      content_language: language(),
      limit: 4,
    };
    let mut matched = vector_match(candidate, 9_000).vector;

    assert!(matched.matches_request(&request));
    matched.vector_collection_id = id("vectors-2");
    assert!(!matched.matches_request(&request));
  }

  #[test]
  fn fusion_does_not_treat_evidence_vectors_as_sense_candidates() {
    let candidate = candidate("sense-hot", "hot");
    let mut vector = vector_match(candidate, 9_000);
    vector.vector.target = VectorTarget::Evidence(id("evidence-other"));
    vector.vector.purpose = VectorPurpose::EvidenceFragment;

    let ranked = fuse_candidates(&content(), EvidenceUse::ApiRedistribution, [], [vector]);

    assert!(ranked.is_empty());
  }

  #[test]
  fn in_memory_matching_exposes_a_bounded_full_text_signal() {
    let matched = candidate("sense-hot", "hot")
      .in_memory_match("definition")
      .unwrap();

    assert_eq!(matched.0, LexicalMatchKind::Semantic);
    assert_eq!(matched.1.basis_points(), 2_500);
  }

  #[test]
  fn match_classes_have_the_documented_closed_precedence() {
    let ordered = [
      LexicalMatchKind::ExactCanonical,
      LexicalMatchKind::ExactAlias,
      LexicalMatchKind::Inflection,
      LexicalMatchKind::SpellingCorrection,
      LexicalMatchKind::Transliteration,
      LexicalMatchKind::Semantic,
    ];

    assert!(ordered
      .windows(2)
      .all(|pair| { pair[0].precedence() > pair[1].precedence() }));
  }

  #[test]
  fn exact_alias_beats_an_inflection_for_the_same_query() {
    let mut value = candidate("sense-bound", "bound");
    let mut alias = value.forms[0].clone();
    alias.id = id("form-bound-alias");
    alias.form = "bounder".to_string();
    alias.normalized_form = "bounder".to_string();
    alias.kind = FormKind::Alias;
    value.forms.push(alias);

    let matched = value.in_memory_match("bounder").unwrap();

    assert_eq!(matched.0, LexicalMatchKind::ExactAlias);
    assert_eq!(matched.1, RetrievalScore::exact());
  }
}
