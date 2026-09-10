//! Typed, evidence-backed canonical lookup-card presentation values.
//!
//! These values are an internal Rust contract, not an HTTP DTO. They deliberately model only
//! deterministic canonical material and carry no generated explanation, personalization, or UI
//! layout instructions.

use crate::domain::{
  canonical::{
    ActiveContentVersion, EvidenceConfidence, EvidenceId, EvidenceKind, EvidenceUse, FormId,
    FormKind, LanguageTag, LexemeId, LexicalPartOfSpeech, ReleaseId, SenseId, SourceId,
  },
  retrieval::MAX_RETRIEVAL_LIMIT,
};

/// Largest number of canonical forms presented for one lookup-card candidate.
pub const MAX_LOOKUP_CARD_FORMS_PER_CANDIDATE: usize = 24;
/// Largest number of evidence fragments presented for one factual assertion.
pub const MAX_LOOKUP_CARD_EVIDENCE_PER_ASSERTION: usize = 8;
/// Largest number of ranked canonical candidates presented by one lookup card.
pub const MAX_LOOKUP_CARD_CANDIDATES: usize = MAX_RETRIEVAL_LIMIT;

/// One normalized request property retained in a canonical lookup card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupQueryAnalysis {
  /// NFC and case-normalized lookup key used by the deterministic retriever.
  pub normalized_query: String,
  /// Language in which the normalized key was resolved.
  pub language: LanguageTag,
  /// Source-permission operation applied before any evidence reached this card.
  pub evidence_use: EvidenceUse,
}

/// Bounded deterministic canonical result ready for a future transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCard {
  /// Normalized query analysis used to produce this result.
  pub query: CanonicalLookupQueryAnalysis,
  /// Immutable lexical, vector, schema, and ranking versions selected for the read.
  pub content: ActiveContentVersion,
  /// Canonical candidates in the exact deterministic order supplied by retrieval.
  pub candidates: Vec<CanonicalLookupCardCandidate>,
  /// Coverage for every material section without conflating absence with policy filtering.
  pub coverage: CanonicalLookupCardCoverage,
}

/// One ranked lexeme-and-sense result in a canonical lookup card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardCandidate {
  /// Stable rank supplied by deterministic retrieval; card assembly never recalculates it.
  pub rank: usize,
  /// Deterministic fusion value retained for explainability, not a calibrated confidence.
  pub fusion_score: u64,
  /// Lemma and part-of-speech identity, separate from forms and meanings.
  pub lexeme: CanonicalLookupCardLexeme,
  /// One meaning of `lexeme`, separate from spelling and morphology variants.
  pub sense: CanonicalLookupCardSense,
  /// Evidence-backed surface forms that belong to `lexeme`.
  pub forms: Vec<CanonicalLookupCardForm>,
}

/// Stable identity and display data for one canonical lemma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardLexeme {
  /// Stable canonical lexeme identifier.
  pub id: LexemeId,
  /// User-visible lemma preserving canonical source spelling.
  pub lemma: String,
  /// Language of the lexeme.
  pub language: LanguageTag,
  /// Canonical grammatical category of the lexeme.
  pub part_of_speech: LexicalPartOfSpeech,
}

/// Stable identity and factual content for one meaning of a lexeme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardSense {
  /// Stable canonical sense identifier.
  pub id: SenseId,
  /// Source-stable discriminator for meanings within the lexeme.
  pub sense_key: String,
  /// Definition assertion when at least one permitted evidence fragment supports it.
  pub definition: Option<CanonicalLookupCardAssertion>,
}

/// One lexeme-owned surface form with its independently evidenced assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardForm {
  /// Stable canonical form identifier.
  pub id: FormId,
  /// Retrieval and presentation role of the surface form.
  pub kind: FormKind,
  /// Source-qualified morphology summary, when the canonical record provides one.
  pub morphology: Option<String>,
  /// Surface-form assertion and the evidence permitted for this card operation.
  pub assertion: CanonicalLookupCardAssertion,
}

/// Kind of factual assertion carried by a canonical lookup card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalLookupAssertionKind {
  /// A canonical sense definition.
  Definition,
  /// A lexeme-owned surface-form assertion.
  Form,
}

/// One factual display value with the evidence that permits presenting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardAssertion {
  /// Classification that prevents definition and form evidence from being conflated.
  pub kind: CanonicalLookupAssertionKind,
  /// Canonical factual text; it is never synthesized by this foundation.
  pub text: String,
  /// Independently citable evidence fragments permitted for this operation.
  pub evidence: Vec<CanonicalLookupCardEvidence>,
}

/// One permitted evidence fragment attached directly to a factual assertion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalLookupCardEvidence {
  /// Stable canonical evidence identifier.
  pub id: EvidenceId,
  /// Classification of the assertion material supplied by the source.
  pub kind: EvidenceKind,
  /// Source-qualified confidence; this is distinct from retrieval ranking.
  pub confidence: EvidenceConfidence,
  /// Licensed evidence text permitted for the current operation.
  pub text: String,
  /// Stable source and release metadata needed to audit the assertion.
  pub provenance: CanonicalLookupCardEvidenceProvenance,
}

/// Per-fragment provenance retained beside the assertion that cites it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalLookupCardEvidenceProvenance {
  /// Source-policy identifier governing this evidence version.
  pub source_id: SourceId,
  /// Source-local identifier used for correction, removal, and attribution lookup.
  pub source_reference: String,
  /// Immutable lexical release containing the evidence fragment.
  pub release_id: ReleaseId,
  /// Language of the evidence text.
  pub language: LanguageTag,
  /// Content hash used for import idempotency and vector reconciliation.
  pub content_hash: String,
}

/// Summary state for one lookup-card section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalLookupCardCoverageState {
  /// At least one usable item is present in the section.
  Available,
  /// No usable or filtered item is known for the section.
  Missing,
  /// Known material was excluded by release, lifecycle, or source-permission checks.
  Filtered,
  /// Vector retrieval failed, while lexical retrieval continued deterministically.
  VectorDegraded,
}

/// Counts and state for one lookup-card section.
///
/// `state` summarizes the most useful outcome. The counters retain mixed outcomes, such as a
/// section with usable assertions alongside other assertions that source policy filtered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardSectionCoverage {
  /// Summary state for this section.
  pub state: CanonicalLookupCardCoverageState,
  /// Number of items included in the card.
  pub available_items: usize,
  /// Number of expected items with no supporting canonical material.
  pub missing_items: usize,
  /// Number of known items withheld by safety, lifecycle, or source-permission checks.
  pub filtered_items: usize,
  /// Number of otherwise permitted items omitted by a card presentation bound.
  pub truncated_items: usize,
}

/// Coverage for the bounded canonical sections represented in a lookup card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLookupCardCoverage {
  /// Hybrid retrieval availability; `VectorDegraded` preserves a lexical-only result.
  pub retrieval: CanonicalLookupCardSectionCoverage,
  /// Lexeme identities returned by canonical retrieval.
  pub lexemes: CanonicalLookupCardSectionCoverage,
  /// Part-of-speech values attached to returned lexemes.
  pub parts_of_speech: CanonicalLookupCardSectionCoverage,
  /// Canonical senses returned by retrieval.
  pub senses: CanonicalLookupCardSectionCoverage,
  /// Evidence-backed definitions shown for returned senses.
  pub definitions: CanonicalLookupCardSectionCoverage,
  /// Evidence-backed surface forms shown for returned lexemes.
  pub forms: CanonicalLookupCardSectionCoverage,
  /// Assertion-level permitted evidence shown in definitions and forms.
  pub evidence: CanonicalLookupCardSectionCoverage,
}
