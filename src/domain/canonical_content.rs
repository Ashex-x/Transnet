//! Bounded canonical lexical-detail values with factual-evidence safeguards.
//!
//! The types in this module model reviewed canonical detail only. They do not import source
//! material, create source records, choose a release, persist data, or expose an HTTP contract.
//! In particular, an unreviewed or rejected generated artifact cannot be converted into the
//! evidence lineage used by a factual assertion.

use std::collections::BTreeSet;

use thiserror::Error;

use super::canonical::{
  CanonicalId, CanonicalStatus, EvidenceFragment, EvidenceKind, EvidenceUse, LanguageTag, Lexeme,
  LexemeId, LexicalSource, ReleaseId, Sense, SenseId, SourcePermissions,
};

/// Largest number of independently citable fragments that may support one factual assertion.
pub const MAX_DETAIL_EVIDENCE: usize = 8;
/// Largest number of each detail kind retained for one canonical sense in this foundation.
pub const MAX_DETAILS_PER_KIND: usize = 24;
/// Largest number of characters retained in one canonical factual assertion.
pub const MAX_DETAIL_TEXT_LENGTH: usize = 4_096;
/// Largest number of characters retained in one controlled label or collocation term.
pub const MAX_DETAIL_LABEL_LENGTH: usize = 256;
/// Largest number of retired or successor senses represented by one evolution mapping side.
pub const MAX_SENSE_EVOLUTION_MEMBERS: usize = 16;

/// Stable identifier for a localized-gloss assertion.
pub type LocalizedGlossId = CanonicalId;
/// Stable identifier for a pronunciation assertion.
pub type PronunciationId = CanonicalId;
/// Stable identifier for a usage-label assertion.
pub type UsageLabelId = CanonicalId;
/// Stable identifier for a grammar-pattern assertion.
pub type GrammarPatternId = CanonicalId;
/// Stable identifier for a collocation assertion.
pub type CollocationId = CanonicalId;
/// Stable identifier for a canonical example assertion.
pub type CanonicalExampleId = CanonicalId;
/// Stable identifier for a learner-pitfall assertion.
pub type LearnerPitfallId = CanonicalId;
/// Stable identifier for an etymology assertion.
pub type EtymologyAssertionId = CanonicalId;
/// Stable identifier for a sense-history assertion.
pub type SenseHistoryAssertionId = CanonicalId;
/// Stable identifier for a release-scoped sense-evolution mapping.
pub type SenseEvolutionId = CanonicalId;

/// Validation failure for canonical lexical detail or its factual evidence lineage.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalContentValidationError {
  /// A lexeme and sense did not identify the same canonical ownership relationship.
  #[error("sense must belong to the lexical-content target's lexeme")]
  SenseLexemeMismatch,
  /// A lexeme and sense were not in the same immutable lexical release.
  #[error("sense and lexeme must belong to the same lexical release")]
  TargetReleaseMismatch,
  /// A detail assertion did not belong to the release of its target sense.
  #[error("detail assertion must belong to its target sense release")]
  DetailReleaseMismatch,
  /// A detail was collected under a different canonical sense target.
  #[error("canonical detail must belong to the collected target sense")]
  DetailTargetMismatch,
  /// A detail assertion did not have the lifecycle state of its target sense.
  #[error("detail assertion status must match its target sense status")]
  DetailStatusMismatch,
  /// A factual assertion used a different detail kind than the enclosing typed detail.
  #[error("factual assertion kind does not match the typed canonical detail")]
  DetailKindMismatch,
  /// A visible pronunciation or example used a language unrelated to the target lexeme.
  #[error("detail language must share the target lexeme's primary language")]
  DetailLanguageMismatch,
  /// A source-policy record did not match the source governing its evidence fragment.
  #[error("evidence fragment source must match its source-policy record")]
  EvidenceSourceMismatch,
  /// An asset-level permission claimed an operation forbidden by its source-policy record.
  #[error("evidence fragment permissions cannot exceed source permissions")]
  EvidencePermissionEscalation,
  /// Canonical storage was not licensed by both the source and the evidence fragment.
  #[error("canonical factual evidence must permit storage")]
  EvidenceStorageForbidden,
  /// A factual assertion had no independently citable source fragment.
  #[error("factual assertion must cite at least one evidence fragment")]
  EmptyEvidence,
  /// A factual assertion exceeded its bounded evidence-fragment count.
  #[error("factual assertion has too many evidence fragments")]
  TooManyEvidence,
  /// An evidence fragment appeared more than once in one factual assertion.
  #[error("factual assertion evidence identifiers must be distinct")]
  DuplicateEvidence,
  /// An evidence fragment was from a different immutable lexical release than its assertion.
  #[error("factual assertion evidence must belong to the assertion release")]
  EvidenceReleaseMismatch,
  /// A fragment kind could not support the typed canonical detail that cited it.
  #[error("evidence fragment kind is incompatible with the typed canonical detail")]
  IncompatibleEvidenceKind,
  /// A generated candidate was not explicitly reviewed and promoted before factual use.
  #[error("generated material must be reviewed and promoted before it can support a fact")]
  GeneratedMaterialNotPromoted,
  /// A source-local reference, content hash, assertion text, or controlled label was blank.
  #[error("canonical detail text must not be blank")]
  BlankDetailText,
  /// A source fragment exceeded the bounded factual-text size.
  #[error("canonical detail text exceeds its maximum length")]
  DetailTextTooLong,
  /// A bounded detail collection had more entries than its contract permits.
  #[error("canonical sense has too many entries of one detail kind")]
  TooManyDetails,
  /// A detail identifier appeared more than once in a collection of the same kind.
  #[error("canonical detail identifiers must be distinct within their kind")]
  DuplicateDetailId,
  /// A collocation did not mark its target sense on the term assigned the target role.
  #[error("collocation target role must identify the target sense and lexeme")]
  CollocationTargetMismatch,
  /// A collocation repeated the same head and dependent term.
  #[error("collocation head and dependent terms must differ")]
  DuplicateCollocationTerms,
  /// A pitfall repeated its mistaken and corrected constructions.
  #[error("learner pitfall mistake and correction must differ")]
  IdenticalPitfallForms,
  /// A historical range ended before it began.
  #[error("historical range end must not precede its start")]
  InvalidHistoricalRange,
  /// A successor mapping used the same release for retired and successor entities.
  #[error("sense evolution must move between distinct lexical releases")]
  EvolutionReleaseMismatch,
  /// A successor mapping's factual assertion did not have its successor release or lifecycle state.
  #[error("sense evolution assertion must match its successor release and lifecycle status")]
  EvolutionAssertionMismatch,
  /// A successor mapping exceeded a bounded retired or successor member count.
  #[error("sense evolution has too many retired or successor senses")]
  TooManySenseEvolutionMembers,
  /// A successor mapping repeated an identifier on one side of the evolution.
  #[error("sense evolution members must be distinct")]
  DuplicateSenseEvolutionMember,
  /// A retired sense was also declared as its own successor.
  #[error("a retired sense cannot also be a successor")]
  SelfSuccessor,
  /// The count shape did not correspond to the declared split, merge, replacement, or retirement.
  #[error("sense evolution member counts do not match its declared kind")]
  InvalidSenseEvolutionShape,
}

/// The typed factual detail supported by one evidence-backed assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalDetailKind {
  /// A gloss in a language selected for explanation or translation.
  LocalizedGloss,
  /// A phonetic, phonemic, or source-defined pronunciation assertion.
  Pronunciation,
  /// A register, domain, dialect, frequency, or similar usage-label assertion.
  UsageLabel,
  /// A structured grammar-pattern assertion.
  GrammarPattern,
  /// A collocation with explicit head and dependent roles.
  Collocation,
  /// An example that demonstrates one canonical sense.
  Example,
  /// A common learner error, false friend, or correction.
  Pitfall,
  /// An etymological assertion about a lexeme or sense scope.
  Etymology,
  /// A dated or qualified semantic-development assertion for a sense.
  SenseHistory,
  /// A reviewed canonical mapping from retired senses to successor senses.
  SenseEvolution,
}

impl CanonicalDetailKind {
  /// Returns whether an imported evidence classification may support this detail kind.
  pub const fn accepts_evidence_kind(self, evidence_kind: EvidenceKind) -> bool {
    match self {
      Self::LocalizedGloss => matches!(evidence_kind, EvidenceKind::LocalizedGloss),
      Self::Pronunciation => matches!(evidence_kind, EvidenceKind::Pronunciation),
      Self::UsageLabel | Self::GrammarPattern | Self::Collocation | Self::Pitfall => {
        matches!(evidence_kind, EvidenceKind::Usage | EvidenceKind::Other)
      }
      Self::Example => matches!(evidence_kind, EvidenceKind::Example),
      Self::Etymology | Self::SenseHistory => {
        matches!(evidence_kind, EvidenceKind::Etymology | EvidenceKind::Other)
      }
      Self::SenseEvolution => matches!(evidence_kind, EvidenceKind::Other),
    }
  }
}

/// Origin and review state of source material considered for factual canonical evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalEvidenceOrigin {
  /// Material supplied directly by a licensed source-policy record.
  LicensedSource,
  /// Material produced by a generator and tied to an auditable generation identifier.
  Generated {
    /// Opaque generation-run or candidate identifier retained by the review workflow.
    generation_id: CanonicalId,
    /// Review result required before generated material can support canonical facts.
    review: GeneratedEvidenceReview,
  },
}

/// Editorial disposition of generated material before it can be used as factual evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeneratedEvidenceReview {
  /// The generated candidate has not been reviewed and cannot support a fact.
  Unreviewed,
  /// Review rejected the generated candidate, so it cannot support a fact.
  Rejected,
  /// A reviewer promoted the generated candidate under an auditable review identifier.
  ReviewedAndPromoted {
    /// Opaque review-decision identifier retained by an editorial workflow.
    review_id: CanonicalId,
  },
}

/// Source, asset, origin, and review lineage admitted to a factual assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEvidenceLineage {
  source: LexicalSource,
  fragment: EvidenceFragment,
  origin: CanonicalEvidenceOrigin,
}

impl CanonicalEvidenceLineage {
  /// Validates one source-qualified evidence lineage eligible for canonical factual storage.
  ///
  /// This constructor rejects an unreviewed or rejected generated candidate. A generated value
  /// must first be reviewed and promoted, and its source and asset permissions must still permit
  /// canonical storage. The constructor permits inactive fragments so staging and historical
  /// records can retain provenance; [`Self::permits`] remains false until all lifecycle checks
  /// allow a concrete read operation.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when source identity, permission lineage,
  /// required factual metadata, or generated-material review state is invalid.
  pub fn new(
    source: LexicalSource,
    fragment: EvidenceFragment,
    origin: CanonicalEvidenceOrigin,
  ) -> Result<Self, CanonicalContentValidationError> {
    if source.id != fragment.source_id {
      return Err(CanonicalContentValidationError::EvidenceSourceMismatch);
    }
    if !permissions_are_subset(fragment.permissions, source.permissions) {
      return Err(CanonicalContentValidationError::EvidencePermissionEscalation);
    }
    if !source.permissions.allows(EvidenceUse::Storage)
      || !fragment.permissions.allows(EvidenceUse::Storage)
    {
      return Err(CanonicalContentValidationError::EvidenceStorageForbidden);
    }
    validate_nonblank(&fragment.source_reference)?;
    validate_nonblank(&fragment.content_hash)?;
    validate_nonblank(&fragment.text)?;
    validate_text_bound(&fragment.text)?;
    if matches!(
      origin,
      CanonicalEvidenceOrigin::Generated {
        review: GeneratedEvidenceReview::Unreviewed | GeneratedEvidenceReview::Rejected,
        ..
      }
    ) {
      return Err(CanonicalContentValidationError::GeneratedMaterialNotPromoted);
    }

    Ok(Self {
      source,
      fragment,
      origin,
    })
  }

  /// Returns the source-policy record governing this fragment.
  pub fn source(&self) -> &LexicalSource {
    &self.source
  }

  /// Returns the independently citable source fragment.
  pub fn fragment(&self) -> &EvidenceFragment {
    &self.fragment
  }

  /// Returns the source or reviewed-generated origin of this evidence.
  pub fn origin(&self) -> &CanonicalEvidenceOrigin {
    &self.origin
  }

  /// Returns whether this lineage may support the requested operation in `release_id` now.
  pub fn permits(&self, release_id: &ReleaseId, evidence_use: EvidenceUse) -> bool {
    self.fragment.permits(release_id, evidence_use) && self.source.permissions.allows(evidence_use)
  }

  fn is_storable_in(&self, release_id: &ReleaseId) -> bool {
    self.fragment.release_id == *release_id
      && self.source.permissions.allows(EvidenceUse::Storage)
      && self.fragment.permissions.allows(EvidenceUse::Storage)
  }
}

/// One immutable text assertion carrying only reviewed, source-qualified factual evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFactualAssertion {
  kind: CanonicalDetailKind,
  release_id: ReleaseId,
  status: CanonicalStatus,
  text: String,
  evidence: Vec<CanonicalEvidenceLineage>,
}

impl CanonicalFactualAssertion {
  /// Creates a bounded factual assertion with exact release and evidence lineage.
  ///
  /// The constructor never synthesizes, rewrites, or promotes text. Every supplied evidence
  /// lineage must be storage-permitted, be from `release_id`, and have a classification compatible
  /// with `kind`. Inactive assertions may remain in staging or history but
  /// [`Self::permits`] will not expose them as a current canonical fact.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when text, evidence bounds, release lineage, or
  /// evidence classifications are invalid.
  pub fn new(
    kind: CanonicalDetailKind,
    release_id: ReleaseId,
    status: CanonicalStatus,
    text: impl Into<String>,
    evidence: Vec<CanonicalEvidenceLineage>,
  ) -> Result<Self, CanonicalContentValidationError> {
    let text = text.into();
    validate_nonblank(&text)?;
    validate_text_bound(&text)?;
    if evidence.is_empty() {
      return Err(CanonicalContentValidationError::EmptyEvidence);
    }
    if evidence.len() > MAX_DETAIL_EVIDENCE {
      return Err(CanonicalContentValidationError::TooManyEvidence);
    }

    let mut evidence_ids = BTreeSet::new();
    for lineage in &evidence {
      let fragment = lineage.fragment();
      if !evidence_ids.insert(fragment.id.clone()) {
        return Err(CanonicalContentValidationError::DuplicateEvidence);
      }
      if !lineage.is_storable_in(&release_id) {
        return Err(CanonicalContentValidationError::EvidenceReleaseMismatch);
      }
      if !kind.accepts_evidence_kind(fragment.kind) {
        return Err(CanonicalContentValidationError::IncompatibleEvidenceKind);
      }
    }

    Ok(Self {
      kind,
      release_id,
      status,
      text,
      evidence,
    })
  }

  /// Returns the typed canonical detail supported by this assertion.
  pub const fn kind(&self) -> CanonicalDetailKind {
    self.kind
  }

  /// Returns the immutable lexical release that owns the assertion and its evidence.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release_id
  }

  /// Returns the editorial lifecycle state of this assertion.
  pub const fn status(&self) -> CanonicalStatus {
    self.status
  }

  /// Returns canonical factual text without applying generated simplification or presentation.
  pub fn text(&self) -> &str {
    &self.text
  }

  /// Returns the bounded evidence lineages that support this factual text.
  pub fn evidence(&self) -> &[CanonicalEvidenceLineage] {
    &self.evidence
  }

  /// Returns whether this active assertion and all its evidence can serve one operation now.
  pub fn permits(&self, evidence_use: EvidenceUse) -> bool {
    self.status.is_lookup_eligible()
      && self
        .evidence
        .iter()
        .all(|lineage| lineage.permits(&self.release_id, evidence_use))
  }
}

/// Canonical lexeme and sense identity used by every sense-scoped detail in this module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SenseContentTarget {
  lexeme_id: LexemeId,
  sense_id: SenseId,
  release_id: ReleaseId,
  language: LanguageTag,
  lexeme_status: CanonicalStatus,
  sense_status: CanonicalStatus,
}

impl SenseContentTarget {
  /// Creates a canonical sense target after validating lexeme ownership and release lineage.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when `sense` does not belong to `lexeme` or
  /// their immutable release IDs differ.
  pub fn new(lexeme: &Lexeme, sense: &Sense) -> Result<Self, CanonicalContentValidationError> {
    if lexeme.id != sense.lexeme_id {
      return Err(CanonicalContentValidationError::SenseLexemeMismatch);
    }
    if lexeme.release_id != sense.release_id {
      return Err(CanonicalContentValidationError::TargetReleaseMismatch);
    }

    Ok(Self {
      lexeme_id: lexeme.id.clone(),
      sense_id: sense.id.clone(),
      release_id: sense.release_id.clone(),
      language: lexeme.language.clone(),
      lexeme_status: lexeme.status,
      sense_status: sense.status,
    })
  }

  /// Returns the lexeme that owns the target sense.
  pub fn lexeme_id(&self) -> &LexemeId {
    &self.lexeme_id
  }

  /// Returns the one specific canonical sense to which the details attach.
  pub fn sense_id(&self) -> &SenseId {
    &self.sense_id
  }

  /// Returns the immutable lexical release for this target.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release_id
  }

  /// Returns the language of the target lexeme.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the lifecycle state of the owning lexeme.
  pub const fn lexeme_status(&self) -> CanonicalStatus {
    self.lexeme_status
  }

  /// Returns the lifecycle state of the target sense.
  pub const fn sense_status(&self) -> CanonicalStatus {
    self.sense_status
  }

  /// Returns whether both the lexeme and sense are eligible for default canonical reads.
  pub const fn is_lookup_eligible(&self) -> bool {
    self.lexeme_status.is_lookup_eligible() && self.sense_status.is_lookup_eligible()
  }
}

fn permissions_are_subset(
  asset_permissions: SourcePermissions,
  source_permissions: SourcePermissions,
) -> bool {
  [
    EvidenceUse::Storage,
    EvidenceUse::Display,
    EvidenceUse::Embedding,
    EvidenceUse::ModelProcessing,
    EvidenceUse::ApiRedistribution,
  ]
  .into_iter()
  .all(|evidence_use| {
    !asset_permissions.allows(evidence_use) || source_permissions.allows(evidence_use)
  })
}

fn validate_nonblank(value: &str) -> Result<(), CanonicalContentValidationError> {
  if value.trim().is_empty() {
    return Err(CanonicalContentValidationError::BlankDetailText);
  }
  Ok(())
}

fn validate_text_bound(value: &str) -> Result<(), CanonicalContentValidationError> {
  if value.chars().count() > MAX_DETAIL_TEXT_LENGTH {
    return Err(CanonicalContentValidationError::DetailTextTooLong);
  }
  Ok(())
}

fn validate_label(value: &str) -> Result<String, CanonicalContentValidationError> {
  validate_nonblank(value)?;
  if value.chars().count() > MAX_DETAIL_LABEL_LENGTH {
    return Err(CanonicalContentValidationError::DetailTextTooLong);
  }
  Ok(value.to_string())
}

fn validate_detail_assertion(
  target: &SenseContentTarget,
  assertion: &CanonicalFactualAssertion,
  expected_kind: CanonicalDetailKind,
) -> Result<(), CanonicalContentValidationError> {
  if assertion.kind() != expected_kind {
    return Err(CanonicalContentValidationError::DetailKindMismatch);
  }
  if assertion.release_id() != target.release_id() {
    return Err(CanonicalContentValidationError::DetailReleaseMismatch);
  }
  if assertion.status() != target.sense_status() {
    return Err(CanonicalContentValidationError::DetailStatusMismatch);
  }
  Ok(())
}

/// A language-specific gloss for one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedGloss {
  id: LocalizedGlossId,
  target: SenseContentTarget,
  language: LanguageTag,
  assertion: CanonicalFactualAssertion,
}

impl LocalizedGloss {
  /// Creates a reviewed localized gloss for `target`.
  ///
  /// The gloss language is deliberately independent of the lexical language, so a canonical
  /// English sense can carry a carefully sourced Simplified-Chinese explanation without treating
  /// that explanation as an English definition.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when the assertion kind, release, or status does
  /// not match `target`.
  pub fn new(
    id: LocalizedGlossId,
    target: SenseContentTarget,
    language: LanguageTag,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::LocalizedGloss)?;
    Ok(Self {
      id,
      target,
      language,
      assertion,
    })
  }

  /// Returns the stable localized-gloss identifier.
  pub fn id(&self) -> &LocalizedGlossId {
    &self.id
  }

  /// Returns the exact canonical sense to which this gloss applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the language used by the localized gloss.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the factual text and evidence lineage for this gloss.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Scope of a pronunciation assertion relative to its target sense and owning lexeme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PronunciationScope {
  /// The pronunciation applies to all active senses of the owning lexeme.
  LexemeWide,
  /// The pronunciation is known to apply only to the target sense.
  SenseSpecific,
}

/// Notation used to render a source-backed pronunciation assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PronunciationNotation {
  /// International Phonetic Alphabet notation.
  Ipa,
  /// A source-defined phonemic notation with its own documented key.
  Phonemic,
  /// A source-defined respelling or other documented pronunciation representation.
  SourceDefined,
}

/// A dialect-qualified pronunciation for one canonical sense or its owning lexeme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalPronunciation {
  id: PronunciationId,
  target: SenseContentTarget,
  scope: PronunciationScope,
  dialect: LanguageTag,
  notation: PronunciationNotation,
  assertion: CanonicalFactualAssertion,
}

impl CanonicalPronunciation {
  /// Creates a reviewed dialect-qualified pronunciation.
  ///
  /// The pronunciation dialect must share a primary language with the target lexeme. This permits
  /// `en-GB` alternatives on an `en-US` lexeme while rejecting a pronunciation that belongs to an
  /// unrelated language.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when language, assertion kind, release, or
  /// status lineage is invalid.
  pub fn new(
    id: PronunciationId,
    target: SenseContentTarget,
    scope: PronunciationScope,
    dialect: LanguageTag,
    notation: PronunciationNotation,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::Pronunciation)?;
    if dialect.primary_language() != target.language().primary_language() {
      return Err(CanonicalContentValidationError::DetailLanguageMismatch);
    }
    Ok(Self {
      id,
      target,
      scope,
      dialect,
      notation,
      assertion,
    })
  }

  /// Returns the stable pronunciation identifier.
  pub fn id(&self) -> &PronunciationId {
    &self.id
  }

  /// Returns the target sense and owning lexeme for this pronunciation.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns whether the pronunciation applies lexeme-wide or only to this sense.
  pub const fn scope(&self) -> PronunciationScope {
    self.scope
  }

  /// Returns the documented dialect of the pronunciation assertion.
  pub fn dialect(&self) -> &LanguageTag {
    &self.dialect
  }

  /// Returns the notation used by the factual pronunciation text.
  pub const fn notation(&self) -> PronunciationNotation {
    self.notation
  }

  /// Returns the factual pronunciation text and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Taxonomy of a canonical usage label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageLabelKind {
  /// Formality, informality, slang, or another register distinction.
  Register,
  /// Subject area or professional domain.
  Domain,
  /// Geographic or dialect-specific distribution.
  Dialect,
  /// Positive, negative, humorous, taboo, or another connotative qualification.
  Connotation,
  /// Politeness, honorific, or interpersonal force.
  Politeness,
  /// Dated, archaic, historical, or current-use qualification.
  Datedness,
  /// Sensitivity, mature-content, or other learner-safety qualification.
  Sensitivity,
  /// Source-qualified frequency or commonness label.
  Frequency,
  /// Source-qualified CEFR or other pedagogical-level label.
  Level,
}

/// A controlled usage label with its own factual evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLabel {
  id: UsageLabelId,
  target: SenseContentTarget,
  kind: UsageLabelKind,
  code: String,
  assertion: CanonicalFactualAssertion,
}

impl UsageLabel {
  /// Creates a reviewed controlled usage label for one sense.
  ///
  /// `code` is an adapter-neutral controlled value, while `assertion` preserves the learner-facing
  /// source-backed wording and its evidence rather than treating the code as self-evidencing.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when `code` is blank or oversized, or assertion
  /// lineage does not match the target and usage-label kind.
  pub fn new(
    id: UsageLabelId,
    target: SenseContentTarget,
    kind: UsageLabelKind,
    code: impl AsRef<str>,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::UsageLabel)?;
    Ok(Self {
      id,
      target,
      kind,
      code: validate_label(code.as_ref())?,
      assertion,
    })
  }

  /// Returns the stable usage-label assertion identifier.
  pub fn id(&self) -> &UsageLabelId {
    &self.id
  }

  /// Returns the exact canonical sense to which this label applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the usage-label taxonomy.
  pub const fn kind(&self) -> UsageLabelKind {
    self.kind
  }

  /// Returns the bounded controlled code, not a source-generated explanation.
  pub fn code(&self) -> &str {
    &self.code
  }

  /// Returns the factual wording and evidence that qualify this label.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Family of grammar pattern represented by a canonical assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrammarPatternKind {
  /// A transitivity, complement, or argument-structure pattern.
  Valency,
  /// A clause, phrase, or constituent construction.
  Construction,
  /// A governed preposition, particle, or case-marking pattern.
  Government,
  /// A morphology or inflection pattern relevant to this sense.
  Morphology,
  /// A source-defined grammar pattern that does not fit another stable family.
  SourceDefined,
}

/// A structured grammar pattern connected to one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarPattern {
  id: GrammarPatternId,
  target: SenseContentTarget,
  kind: GrammarPatternKind,
  assertion: CanonicalFactualAssertion,
}

impl GrammarPattern {
  /// Creates a reviewed grammar-pattern assertion for one sense.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when assertion kind, release, or lifecycle state
  /// does not match the target.
  pub fn new(
    id: GrammarPatternId,
    target: SenseContentTarget,
    kind: GrammarPatternKind,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::GrammarPattern)?;
    Ok(Self {
      id,
      target,
      kind,
      assertion,
    })
  }

  /// Returns the stable grammar-pattern identifier.
  pub fn id(&self) -> &GrammarPatternId {
    &self.id
  }

  /// Returns the exact canonical sense to which this pattern applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the stable grammar-pattern family.
  pub const fn kind(&self) -> GrammarPatternKind {
    self.kind
  }

  /// Returns the factual pattern text and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// The grammatical role that the target sense occupies in a collocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollocationRole {
  /// The target sense supplies the collocation head.
  Head,
  /// The target sense supplies the collocation dependent.
  Dependent,
}

/// Coarse construction metadata retained beside a collocation's explicit term roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollocationConstruction {
  /// A verb and its object or complement.
  VerbObject,
  /// An adjective and its noun.
  AdjectiveNoun,
  /// An adverb and its modified adjective or verb.
  AdverbModifier,
  /// A noun compound or fixed noun phrase.
  NounCompound,
  /// A governed preposition, particle, or case-marked complement.
  GovernedComplement,
  /// A source-defined construction whose finer grammar remains in the factual assertion.
  SourceDefined,
}

/// One head or dependent term in a canonical collocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollocationTerm {
  surface: String,
  lexeme_id: Option<LexemeId>,
  sense_id: Option<SenseId>,
}

impl CollocationTerm {
  /// Creates a bounded collocation term with optional canonical entity references.
  ///
  /// A term that identifies a sense must also identify its owning lexeme. The constructor cannot
  /// resolve those identifiers without a repository; [`Collocation::new`] validates the target
  /// term against its supplied canonical sense target.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when `surface` is blank or oversized, or when a
  /// sense reference omits its lexeme reference.
  pub fn new(
    surface: impl AsRef<str>,
    lexeme_id: Option<LexemeId>,
    sense_id: Option<SenseId>,
  ) -> Result<Self, CanonicalContentValidationError> {
    if sense_id.is_some() && lexeme_id.is_none() {
      return Err(CanonicalContentValidationError::CollocationTargetMismatch);
    }
    Ok(Self {
      surface: validate_label(surface.as_ref())?,
      lexeme_id,
      sense_id,
    })
  }

  /// Returns the source-preserving surface term used in the collocation.
  pub fn surface(&self) -> &str {
    &self.surface
  }

  /// Returns the canonical lexeme reference, when the term is aligned.
  pub fn lexeme_id(&self) -> Option<&LexemeId> {
    self.lexeme_id.as_ref()
  }

  /// Returns the canonical sense reference, when the term is sense-specific.
  pub fn sense_id(&self) -> Option<&SenseId> {
    self.sense_id.as_ref()
  }
}

/// A collocation with explicit head/dependent roles and factual evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collocation {
  id: CollocationId,
  target: SenseContentTarget,
  target_role: CollocationRole,
  construction: CollocationConstruction,
  head: CollocationTerm,
  dependent: CollocationTerm,
  assertion: CanonicalFactualAssertion,
}

impl Collocation {
  /// Creates a reviewed collocation for one sense.
  ///
  /// The term selected by `target_role` must identify the target sense and its owning lexeme. This
  /// prevents an importer from preserving a surface phrase while losing which grammatical role
  /// the learner should associate with the canonical sense.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when the target role, term identities, or
  /// assertion lineage is invalid.
  pub fn new(
    id: CollocationId,
    target: SenseContentTarget,
    target_role: CollocationRole,
    construction: CollocationConstruction,
    head: CollocationTerm,
    dependent: CollocationTerm,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::Collocation)?;
    if head == dependent {
      return Err(CanonicalContentValidationError::DuplicateCollocationTerms);
    }
    let target_term = match target_role {
      CollocationRole::Head => &head,
      CollocationRole::Dependent => &dependent,
    };
    if target_term.lexeme_id() != Some(target.lexeme_id())
      || target_term.sense_id() != Some(target.sense_id())
    {
      return Err(CanonicalContentValidationError::CollocationTargetMismatch);
    }

    Ok(Self {
      id,
      target,
      target_role,
      construction,
      head,
      dependent,
      assertion,
    })
  }

  /// Returns the stable collocation identifier.
  pub fn id(&self) -> &CollocationId {
    &self.id
  }

  /// Returns the exact canonical sense to which this collocation applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns whether the target sense is the head or dependent in this collocation.
  pub const fn target_role(&self) -> CollocationRole {
    self.target_role
  }

  /// Returns construction metadata retained separately from surface phrasing.
  pub const fn construction(&self) -> CollocationConstruction {
    self.construction
  }

  /// Returns the collocation head term.
  pub fn head(&self) -> &CollocationTerm {
    &self.head
  }

  /// Returns the collocation dependent term.
  pub fn dependent(&self) -> &CollocationTerm {
    &self.dependent
  }

  /// Returns the factual collocation assertion and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// A source-backed example demonstrating one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExample {
  id: CanonicalExampleId,
  target: SenseContentTarget,
  language: LanguageTag,
  assertion: CanonicalFactualAssertion,
}

impl CanonicalExample {
  /// Creates a reviewed example for one sense.
  ///
  /// The example language must share its primary language with the target lexeme. A translated
  /// explanation belongs in a separate localized-gloss assertion rather than being mislabeled as
  /// an example of the target-language sense.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when language, assertion kind, release, or
  /// status lineage is invalid.
  pub fn new(
    id: CanonicalExampleId,
    target: SenseContentTarget,
    language: LanguageTag,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::Example)?;
    if language.primary_language() != target.language().primary_language() {
      return Err(CanonicalContentValidationError::DetailLanguageMismatch);
    }
    Ok(Self {
      id,
      target,
      language,
      assertion,
    })
  }

  /// Returns the stable canonical-example identifier.
  pub fn id(&self) -> &CanonicalExampleId {
    &self.id
  }

  /// Returns the exact canonical sense demonstrated by this example.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the language in which the example is written.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the factual example text and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Category of an evidence-backed learner pitfall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LearnerPitfallKind {
  /// A deceptive similarity between the learner's language and the target sense.
  FalseFriend,
  /// A sense that learners commonly confuse with a different canonical sense.
  Confusable,
  /// An unnatural word-for-word construction transferred from another language.
  LiteralTranslation,
  /// A source-observed common learner error not covered by another stable category.
  CommonError,
}

/// A source-language-qualified mistake and correction for one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnerPitfall {
  id: LearnerPitfallId,
  target: SenseContentTarget,
  learner_language: LanguageTag,
  kind: LearnerPitfallKind,
  mistake: CanonicalFactualAssertion,
  correction: CanonicalFactualAssertion,
}

impl LearnerPitfall {
  /// Creates a reviewed learner pitfall with distinct mistake and correction assertions.
  ///
  /// Both assertions retain independent evidence because a correction must not be inferred from a
  /// source that only documents the error. The assertions can share permitted evidence when that
  /// source explicitly supports both facts.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when assertion lineage is invalid or the source
  /// text presents identical mistake and correction forms.
  pub fn new(
    id: LearnerPitfallId,
    target: SenseContentTarget,
    learner_language: LanguageTag,
    kind: LearnerPitfallKind,
    mistake: CanonicalFactualAssertion,
    correction: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &mistake, CanonicalDetailKind::Pitfall)?;
    validate_detail_assertion(&target, &correction, CanonicalDetailKind::Pitfall)?;
    if mistake.text() == correction.text() {
      return Err(CanonicalContentValidationError::IdenticalPitfallForms);
    }
    Ok(Self {
      id,
      target,
      learner_language,
      kind,
      mistake,
      correction,
    })
  }

  /// Returns the stable learner-pitfall identifier.
  pub fn id(&self) -> &LearnerPitfallId {
    &self.id
  }

  /// Returns the exact canonical sense to which this pitfall applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the learner language that qualifies the likely error.
  pub fn learner_language(&self) -> &LanguageTag {
    &self.learner_language
  }

  /// Returns the stable learner-pitfall category.
  pub const fn kind(&self) -> LearnerPitfallKind {
    self.kind
  }

  /// Returns the source-backed mistaken construction.
  pub fn mistake(&self) -> &CanonicalFactualAssertion {
    &self.mistake
  }

  /// Returns the source-backed corrected construction.
  pub fn correction(&self) -> &CanonicalFactualAssertion {
    &self.correction
  }
}

/// Bounded year range retained when a source qualifies an etymological or semantic-history claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HistoricalRange {
  first_year: Option<i32>,
  last_year: Option<i32>,
}

impl HistoricalRange {
  /// Creates an optional inclusive year range without inventing an absent date.
  ///
  /// Negative years may represent a source's BCE convention. The values are source qualifiers,
  /// not server-generated historical claims, and callers can leave either endpoint absent when a
  /// source does not establish it.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError::InvalidHistoricalRange`] when `last_year` is
  /// earlier than `first_year`.
  pub const fn new(
    first_year: Option<i32>,
    last_year: Option<i32>,
  ) -> Result<Self, CanonicalContentValidationError> {
    if matches!((first_year, last_year), (Some(first), Some(last)) if last < first) {
      return Err(CanonicalContentValidationError::InvalidHistoricalRange);
    }
    Ok(Self {
      first_year,
      last_year,
    })
  }

  /// Returns the inclusive earliest source-qualified year, when known.
  pub const fn first_year(self) -> Option<i32> {
    self.first_year
  }

  /// Returns the inclusive latest source-qualified year, when known.
  pub const fn last_year(self) -> Option<i32> {
    self.last_year
  }
}

/// Scope of an etymology assertion relative to a target sense and its lexeme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EtymologyScope {
  /// The assertion applies to the owning lexeme across all of its senses.
  LexemeWide,
  /// The assertion applies only to the target sense's historical development.
  SenseSpecific,
}

/// Relationship stated by a source-backed etymology assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EtymologyKind {
  /// The target was borrowed from a source language or lexical form.
  BorrowedFrom,
  /// The target was derived from another form or construction.
  DerivedFrom,
  /// The target is a cognate of another form without asserting direct descent.
  CognateWith,
  /// A qualified origin summary that does not claim a more specific relation.
  OriginSummary,
}

/// A source-backed etymology assertion for one target sense or its owning lexeme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtymologyAssertion {
  id: EtymologyAssertionId,
  target: SenseContentTarget,
  scope: EtymologyScope,
  kind: EtymologyKind,
  source_language: LanguageTag,
  period: HistoricalRange,
  assertion: CanonicalFactualAssertion,
}

impl EtymologyAssertion {
  /// Creates a reviewed etymology assertion with its source-language and time qualification.
  ///
  /// The optional time endpoints remain source-qualified metadata. The constructor does not infer
  /// a date, language, borrowing path, or cognate relationship from lexical similarity.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when assertion kind, release, or lifecycle state
  /// does not match the target.
  pub fn new(
    id: EtymologyAssertionId,
    target: SenseContentTarget,
    scope: EtymologyScope,
    kind: EtymologyKind,
    source_language: LanguageTag,
    period: HistoricalRange,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::Etymology)?;
    Ok(Self {
      id,
      target,
      scope,
      kind,
      source_language,
      period,
      assertion,
    })
  }

  /// Returns the stable etymology-assertion identifier.
  pub fn id(&self) -> &EtymologyAssertionId {
    &self.id
  }

  /// Returns the target sense and owning lexeme for this assertion.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns whether this etymology applies lexeme-wide or only to this sense.
  pub const fn scope(&self) -> EtymologyScope {
    self.scope
  }

  /// Returns the source-qualified etymology relationship.
  pub const fn kind(&self) -> EtymologyKind {
    self.kind
  }

  /// Returns the source language named by the etymology evidence.
  pub fn source_language(&self) -> &LanguageTag {
    &self.source_language
  }

  /// Returns the optional inclusive historical range qualified by the source.
  pub const fn period(&self) -> HistoricalRange {
    self.period
  }

  /// Returns the factual etymology text and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Type of source-backed semantic-development event for a canonical sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SenseHistoryEventKind {
  /// The sense is documented as attested in the qualified period.
  Attestation,
  /// The sense acquired, narrowed, broadened, or otherwise shifted a documented meaning.
  SemanticShift,
  /// The sense acquired a documented dialect, register, or domain restriction.
  ScopeChange,
  /// The sense was documented as obsolete, retired, or superseded.
  Retirement,
}

/// A source-backed semantic-development assertion for one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenseHistoryAssertion {
  id: SenseHistoryAssertionId,
  target: SenseContentTarget,
  kind: SenseHistoryEventKind,
  period: HistoricalRange,
  assertion: CanonicalFactualAssertion,
}

impl SenseHistoryAssertion {
  /// Creates a reviewed semantic-history assertion for one sense.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when assertion kind, release, or lifecycle state
  /// does not match the target.
  pub fn new(
    id: SenseHistoryAssertionId,
    target: SenseContentTarget,
    kind: SenseHistoryEventKind,
    period: HistoricalRange,
    assertion: CanonicalFactualAssertion,
  ) -> Result<Self, CanonicalContentValidationError> {
    validate_detail_assertion(&target, &assertion, CanonicalDetailKind::SenseHistory)?;
    Ok(Self {
      id,
      target,
      kind,
      period,
      assertion,
    })
  }

  /// Returns the stable sense-history assertion identifier.
  pub fn id(&self) -> &SenseHistoryAssertionId {
    &self.id
  }

  /// Returns the exact canonical sense to which this history event applies.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns the stable semantic-history event classification.
  pub const fn kind(&self) -> SenseHistoryEventKind {
    self.kind
  }

  /// Returns the optional inclusive historical range qualified by the source.
  pub const fn period(&self) -> HistoricalRange {
    self.period
  }

  /// Returns the factual history text and evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

/// Bounded reviewed details attached to one canonical sense.
///
/// This aggregate preserves separate collections for meanings, usage, grammar, examples, and
/// history. It intentionally does not flatten them into one generated card or infer missing
/// sections from neighboring details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSenseDetailsInput {
  /// Canonical sense to which every collected detail must attach.
  pub target: SenseContentTarget,
  /// Localized glosses, retained separately from definitions and examples.
  pub localized_glosses: Vec<LocalizedGloss>,
  /// Dialect-qualified pronunciations.
  pub pronunciations: Vec<CanonicalPronunciation>,
  /// Controlled usage-label assertions.
  pub usage_labels: Vec<UsageLabel>,
  /// Structured grammar-pattern assertions.
  pub grammar_patterns: Vec<GrammarPattern>,
  /// Explicit-role collocation assertions.
  pub collocations: Vec<Collocation>,
  /// Source-backed examples.
  pub examples: Vec<CanonicalExample>,
  /// Source-language-qualified learner pitfalls.
  pub pitfalls: Vec<LearnerPitfall>,
  /// Etymology assertions scoped to the target sense or its lexeme.
  pub etymologies: Vec<EtymologyAssertion>,
  /// Source-backed semantic-development assertions.
  pub history: Vec<SenseHistoryAssertion>,
}

impl CanonicalSenseDetailsInput {
  /// Creates an empty explicit-detail input for `target` without inventing missing content.
  pub fn empty(target: SenseContentTarget) -> Self {
    Self {
      target,
      localized_glosses: Vec::new(),
      pronunciations: Vec::new(),
      usage_labels: Vec::new(),
      grammar_patterns: Vec::new(),
      collocations: Vec::new(),
      examples: Vec::new(),
      pitfalls: Vec::new(),
      etymologies: Vec::new(),
      history: Vec::new(),
    }
  }
}

/// Bounded reviewed details attached to one canonical sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSenseDetails {
  target: SenseContentTarget,
  localized_glosses: Vec<LocalizedGloss>,
  pronunciations: Vec<CanonicalPronunciation>,
  usage_labels: Vec<UsageLabel>,
  grammar_patterns: Vec<GrammarPattern>,
  collocations: Vec<Collocation>,
  examples: Vec<CanonicalExample>,
  pitfalls: Vec<LearnerPitfall>,
  etymologies: Vec<EtymologyAssertion>,
  history: Vec<SenseHistoryAssertion>,
}

impl CanonicalSenseDetails {
  /// Creates a bounded collection of typed canonical details for one sense.
  ///
  /// Each collection is independently bounded and must contain distinct IDs all tied to the same
  /// target. Callers retain section absence explicitly through [`CanonicalSenseDetailsInput::empty`]
  /// or empty fields; the foundation never generates a substitute definition, example,
  /// pronunciation, or history field.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when a collection is oversized, contains a
  /// duplicate ID, or holds an item for a different target sense.
  pub fn new(input: CanonicalSenseDetailsInput) -> Result<Self, CanonicalContentValidationError> {
    let CanonicalSenseDetailsInput {
      target,
      localized_glosses,
      pronunciations,
      usage_labels,
      grammar_patterns,
      collocations,
      examples,
      pitfalls,
      etymologies,
      history,
    } = input;
    validate_detail_collection(&target, &localized_glosses)?;
    validate_detail_collection(&target, &pronunciations)?;
    validate_detail_collection(&target, &usage_labels)?;
    validate_detail_collection(&target, &grammar_patterns)?;
    validate_detail_collection(&target, &collocations)?;
    validate_detail_collection(&target, &examples)?;
    validate_detail_collection(&target, &pitfalls)?;
    validate_detail_collection(&target, &etymologies)?;
    validate_detail_collection(&target, &history)?;

    Ok(Self {
      target,
      localized_glosses,
      pronunciations,
      usage_labels,
      grammar_patterns,
      collocations,
      examples,
      pitfalls,
      etymologies,
      history,
    })
  }

  /// Returns the canonical sense to which all contained details attach.
  pub fn target(&self) -> &SenseContentTarget {
    &self.target
  }

  /// Returns bounded localized-gloss assertions.
  pub fn localized_glosses(&self) -> &[LocalizedGloss] {
    &self.localized_glosses
  }

  /// Returns bounded pronunciation assertions.
  pub fn pronunciations(&self) -> &[CanonicalPronunciation] {
    &self.pronunciations
  }

  /// Returns bounded usage-label assertions.
  pub fn usage_labels(&self) -> &[UsageLabel] {
    &self.usage_labels
  }

  /// Returns bounded grammar-pattern assertions.
  pub fn grammar_patterns(&self) -> &[GrammarPattern] {
    &self.grammar_patterns
  }

  /// Returns bounded collocation assertions.
  pub fn collocations(&self) -> &[Collocation] {
    &self.collocations
  }

  /// Returns bounded canonical example assertions.
  pub fn examples(&self) -> &[CanonicalExample] {
    &self.examples
  }

  /// Returns bounded learner-pitfall assertions.
  pub fn pitfalls(&self) -> &[LearnerPitfall] {
    &self.pitfalls
  }

  /// Returns bounded etymology assertions.
  pub fn etymologies(&self) -> &[EtymologyAssertion] {
    &self.etymologies
  }

  /// Returns bounded semantic-history assertions.
  pub fn history(&self) -> &[SenseHistoryAssertion] {
    &self.history
  }

  /// Returns whether every detail is safe to serve for one pinned canonical sense read.
  ///
  /// The release and sense identifiers must match the aggregate target. Both the owning lexeme
  /// and target sense must be active, and every returned factual assertion must permit
  /// `evidence_use` through its complete source and asset lineage. This method deliberately
  /// treats one inaccessible assertion as making the aggregate ineligible rather than silently
  /// omitting a section and making a policy exclusion indistinguishable from absent content.
  pub fn is_eligible_for(
    &self,
    release_id: &ReleaseId,
    sense_id: &SenseId,
    evidence_use: EvidenceUse,
  ) -> bool {
    self.target.release_id() == release_id
      && self.target.sense_id() == sense_id
      && self.target.is_lookup_eligible()
      && self
        .localized_glosses
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .pronunciations
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .usage_labels
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .grammar_patterns
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .collocations
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .examples
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self.pitfalls.iter().all(|detail| {
        detail.mistake().permits(evidence_use) && detail.correction().permits(evidence_use)
      })
      && self
        .etymologies
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
      && self
        .history
        .iter()
        .all(|detail| detail.assertion().permits(evidence_use))
  }
}

trait SenseScopedDetail {
  fn id(&self) -> &CanonicalId;
  fn target(&self) -> &SenseContentTarget;
}

macro_rules! impl_sense_scoped_detail {
  ($detail:ty) => {
    impl SenseScopedDetail for $detail {
      fn id(&self) -> &CanonicalId {
        &self.id
      }

      fn target(&self) -> &SenseContentTarget {
        &self.target
      }
    }
  };
}

impl_sense_scoped_detail!(LocalizedGloss);
impl_sense_scoped_detail!(CanonicalPronunciation);
impl_sense_scoped_detail!(UsageLabel);
impl_sense_scoped_detail!(GrammarPattern);
impl_sense_scoped_detail!(Collocation);
impl_sense_scoped_detail!(CanonicalExample);
impl_sense_scoped_detail!(LearnerPitfall);
impl_sense_scoped_detail!(EtymologyAssertion);
impl_sense_scoped_detail!(SenseHistoryAssertion);

fn validate_detail_collection<T: SenseScopedDetail>(
  target: &SenseContentTarget,
  details: &[T],
) -> Result<(), CanonicalContentValidationError> {
  if details.len() > MAX_DETAILS_PER_KIND {
    return Err(CanonicalContentValidationError::TooManyDetails);
  }

  let mut ids = BTreeSet::new();
  for detail in details {
    if detail.target() != target {
      return Err(CanonicalContentValidationError::DetailTargetMismatch);
    }
    if !ids.insert(detail.id().clone()) {
      return Err(CanonicalContentValidationError::DuplicateDetailId);
    }
  }
  Ok(())
}

/// Shape of a release-scoped canonical sense evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SenseEvolutionKind {
  /// Exactly one retired sense is replaced by one successor sense.
  Replacement,
  /// Exactly one retired sense has several successor senses after a split.
  Split,
  /// Several retired senses are consolidated into exactly one successor sense.
  Merge,
  /// One or more retired senses have no compatible successor in the new release.
  Retirement,
}

/// Input fields for validating one release-scoped canonical sense-evolution mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenseEvolutionMappingInput {
  /// Stable identifier for this canonical evolution record.
  pub id: SenseEvolutionId,
  /// Immutable release containing the retired sense identities.
  pub from_release_id: ReleaseId,
  /// Immutable later release containing successor sense identities.
  pub to_release_id: ReleaseId,
  /// Structural split, merge, replacement, or retirement classification.
  pub kind: SenseEvolutionKind,
  /// Editorial lifecycle state of the mapping in the successor release.
  pub status: CanonicalStatus,
  /// Retired sense IDs from `from_release_id`.
  pub retired_sense_ids: Vec<SenseId>,
  /// Replacement sense IDs from `to_release_id`.
  pub successor_sense_ids: Vec<SenseId>,
  /// Reviewed factual explanation and evidence for this canonical transition.
  pub assertion: CanonicalFactualAssertion,
}

/// Reviewed mapping from retired canonical senses in one release to successors in another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenseEvolutionMapping {
  id: SenseEvolutionId,
  from_release_id: ReleaseId,
  to_release_id: ReleaseId,
  kind: SenseEvolutionKind,
  status: CanonicalStatus,
  retired_sense_ids: Vec<SenseId>,
  successor_sense_ids: Vec<SenseId>,
  assertion: CanonicalFactualAssertion,
}

impl SenseEvolutionMapping {
  /// Creates a bounded release-scoped replacement, split, merge, or retirement mapping.
  ///
  /// The mapping is canonical evidence for a content transition, not a private learner-state
  /// mutation. Private saved vocabulary can later consume an eligible one-to-one mapping or pause
  /// safely for an ambiguous split, without silently attaching an old judgment to a new sense.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalContentValidationError`] when release lineage, member uniqueness, mapping
  /// shape, or factual assertion lineage is invalid.
  pub fn new(input: SenseEvolutionMappingInput) -> Result<Self, CanonicalContentValidationError> {
    let SenseEvolutionMappingInput {
      id,
      from_release_id,
      to_release_id,
      kind,
      status,
      retired_sense_ids,
      successor_sense_ids,
      assertion,
    } = input;
    if from_release_id == to_release_id {
      return Err(CanonicalContentValidationError::EvolutionReleaseMismatch);
    }
    if assertion.kind() != CanonicalDetailKind::SenseEvolution
      || assertion.release_id() != &to_release_id
      || assertion.status() != status
    {
      return Err(CanonicalContentValidationError::EvolutionAssertionMismatch);
    }
    validate_evolution_members(&retired_sense_ids)?;
    validate_evolution_members(&successor_sense_ids)?;
    if retired_sense_ids.iter().any(|retired| {
      successor_sense_ids
        .iter()
        .any(|successor| successor == retired)
    }) {
      return Err(CanonicalContentValidationError::SelfSuccessor);
    }
    if !evolution_shape_is_valid(kind, &retired_sense_ids, &successor_sense_ids) {
      return Err(CanonicalContentValidationError::InvalidSenseEvolutionShape);
    }

    Ok(Self {
      id,
      from_release_id,
      to_release_id,
      kind,
      status,
      retired_sense_ids,
      successor_sense_ids,
      assertion,
    })
  }

  /// Returns the stable evolution-mapping identifier.
  pub fn id(&self) -> &SenseEvolutionId {
    &self.id
  }

  /// Returns the immutable release containing the retired senses.
  pub fn from_release_id(&self) -> &ReleaseId {
    &self.from_release_id
  }

  /// Returns the immutable successor release selected by this mapping.
  pub fn to_release_id(&self) -> &ReleaseId {
    &self.to_release_id
  }

  /// Returns the split, merge, replacement, or retirement shape.
  pub const fn kind(&self) -> SenseEvolutionKind {
    self.kind
  }

  /// Returns the lifecycle state of this canonical evolution assertion.
  pub const fn status(&self) -> CanonicalStatus {
    self.status
  }

  /// Returns bounded retired sense identifiers from the prior release.
  pub fn retired_sense_ids(&self) -> &[SenseId] {
    &self.retired_sense_ids
  }

  /// Returns bounded successor sense identifiers in the later release.
  pub fn successor_sense_ids(&self) -> &[SenseId] {
    &self.successor_sense_ids
  }

  /// Returns the factual mapping explanation and its evidence lineage.
  pub fn assertion(&self) -> &CanonicalFactualAssertion {
    &self.assertion
  }
}

fn validate_evolution_members(members: &[SenseId]) -> Result<(), CanonicalContentValidationError> {
  if members.len() > MAX_SENSE_EVOLUTION_MEMBERS {
    return Err(CanonicalContentValidationError::TooManySenseEvolutionMembers);
  }
  let mut distinct = BTreeSet::new();
  if !members.iter().all(|member| distinct.insert(member)) {
    return Err(CanonicalContentValidationError::DuplicateSenseEvolutionMember);
  }
  Ok(())
}

fn evolution_shape_is_valid(
  kind: SenseEvolutionKind,
  retired_sense_ids: &[SenseId],
  successor_sense_ids: &[SenseId],
) -> bool {
  match kind {
    SenseEvolutionKind::Replacement => {
      retired_sense_ids.len() == 1 && successor_sense_ids.len() == 1
    }
    SenseEvolutionKind::Split => retired_sense_ids.len() == 1 && successor_sense_ids.len() > 1,
    SenseEvolutionKind::Merge => retired_sense_ids.len() > 1 && successor_sense_ids.len() == 1,
    SenseEvolutionKind::Retirement => {
      !retired_sense_ids.is_empty() && successor_sense_ids.is_empty()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::{EvidenceConfidence, LexicalPartOfSpeech, SourceId};

  fn canonical_id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn release(value: &str) -> ReleaseId {
    canonical_id(value)
  }

  fn language(value: &str) -> LanguageTag {
    LanguageTag::parse(value).unwrap()
  }

  fn permissions(api_redistribution: bool) -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding: false,
      model_processing: false,
      api_redistribution,
    }
  }

  fn source(source_id: SourceId, source_permissions: SourcePermissions) -> LexicalSource {
    LexicalSource {
      id: source_id,
      name: "Licensed source".to_string(),
      version: "2026.09".to_string(),
      license: "LicenseRef-Test".to_string(),
      attribution: None,
      permissions: source_permissions,
    }
  }

  fn fragment(
    id: &str,
    release_id: ReleaseId,
    kind: EvidenceKind,
    status: CanonicalStatus,
    source_id: SourceId,
    source_permissions: SourcePermissions,
  ) -> EvidenceFragment {
    EvidenceFragment {
      id: canonical_id(id),
      source_id,
      source_reference: format!("reference:{id}"),
      release_id,
      language: language("en-US"),
      kind,
      confidence: EvidenceConfidence::High,
      text: "Source-backed text".to_string(),
      content_hash: format!("hash:{id}"),
      permissions: source_permissions,
      status,
    }
  }

  fn evidence_kind(kind: CanonicalDetailKind) -> EvidenceKind {
    match kind {
      CanonicalDetailKind::LocalizedGloss => EvidenceKind::LocalizedGloss,
      CanonicalDetailKind::Pronunciation => EvidenceKind::Pronunciation,
      CanonicalDetailKind::UsageLabel
      | CanonicalDetailKind::GrammarPattern
      | CanonicalDetailKind::Collocation
      | CanonicalDetailKind::Pitfall => EvidenceKind::Usage,
      CanonicalDetailKind::Example => EvidenceKind::Example,
      CanonicalDetailKind::Etymology | CanonicalDetailKind::SenseHistory => EvidenceKind::Etymology,
      CanonicalDetailKind::SenseEvolution => EvidenceKind::Other,
    }
  }

  fn licensed_lineage(
    id: &str,
    release_id: ReleaseId,
    kind: EvidenceKind,
    status: CanonicalStatus,
    api_redistribution: bool,
  ) -> CanonicalEvidenceLineage {
    let source_id = canonical_id("source-a");
    let source_permissions = permissions(api_redistribution);
    CanonicalEvidenceLineage::new(
      source(source_id.clone(), source_permissions),
      fragment(id, release_id, kind, status, source_id, source_permissions),
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap()
  }

  fn assertion(
    kind: CanonicalDetailKind,
    release_id: ReleaseId,
    status: CanonicalStatus,
  ) -> CanonicalFactualAssertion {
    CanonicalFactualAssertion::new(
      kind,
      release_id.clone(),
      status,
      "Reviewed factual text",
      vec![licensed_lineage(
        "evidence-a",
        release_id,
        evidence_kind(kind),
        status,
        true,
      )],
    )
    .unwrap()
  }

  fn target(status: CanonicalStatus) -> SenseContentTarget {
    let release_id = release("release-a");
    let lexeme = Lexeme {
      id: canonical_id("lexeme-a"),
      release_id: release_id.clone(),
      language: language("en-US"),
      lemma: "run".to_string(),
      normalized_lemma: "run".to_string(),
      part_of_speech: LexicalPartOfSpeech::Verb,
      status,
    };
    let sense = Sense {
      id: canonical_id("sense-a"),
      lexeme_id: lexeme.id.clone(),
      release_id,
      sense_key: "run-1".to_string(),
      definition: "move quickly".to_string(),
      definition_evidence_ids: vec![canonical_id("definition-evidence")],
      status,
    };
    SenseContentTarget::new(&lexeme, &sense).unwrap()
  }

  #[test]
  fn unreviewed_generated_material_cannot_become_factual_evidence() {
    let release_id = release("release-a");
    let source_id = canonical_id("source-a");
    let source_permissions = permissions(true);
    let result = CanonicalEvidenceLineage::new(
      source(source_id.clone(), source_permissions),
      fragment(
        "generated-evidence",
        release_id,
        EvidenceKind::Example,
        CanonicalStatus::Draft,
        source_id,
        source_permissions,
      ),
      CanonicalEvidenceOrigin::Generated {
        generation_id: canonical_id("generation-a"),
        review: GeneratedEvidenceReview::Unreviewed,
      },
    );

    assert_eq!(
      result,
      Err(CanonicalContentValidationError::GeneratedMaterialNotPromoted)
    );
  }

  #[test]
  fn promoted_generated_material_still_requires_source_permissions() {
    let release_id = release("release-a");
    let source_id = canonical_id("source-a");
    let source_permissions = permissions(false);
    let lineage = CanonicalEvidenceLineage::new(
      source(source_id.clone(), source_permissions),
      fragment(
        "promoted-evidence",
        release_id.clone(),
        EvidenceKind::Example,
        CanonicalStatus::Active,
        source_id,
        source_permissions,
      ),
      CanonicalEvidenceOrigin::Generated {
        generation_id: canonical_id("generation-a"),
        review: GeneratedEvidenceReview::ReviewedAndPromoted {
          review_id: canonical_id("review-a"),
        },
      },
    )
    .unwrap();
    let factual = CanonicalFactualAssertion::new(
      CanonicalDetailKind::Example,
      release_id,
      CanonicalStatus::Active,
      "A reviewed generated example.",
      vec![lineage],
    )
    .unwrap();

    assert!(factual.permits(EvidenceUse::Display));
    assert!(!factual.permits(EvidenceUse::ApiRedistribution));
  }

  #[test]
  fn source_permission_escalation_is_rejected() {
    let release_id = release("release-a");
    let source_id = canonical_id("source-a");
    let source_permissions = permissions(false);
    let result = CanonicalEvidenceLineage::new(
      source(source_id.clone(), source_permissions),
      fragment(
        "escalated-evidence",
        release_id,
        EvidenceKind::Usage,
        CanonicalStatus::Active,
        source_id,
        permissions(true),
      ),
      CanonicalEvidenceOrigin::LicensedSource,
    );

    assert_eq!(
      result,
      Err(CanonicalContentValidationError::EvidencePermissionEscalation)
    );
  }

  #[test]
  fn typed_details_require_matching_release_status_and_evidence_kind() {
    let target = target(CanonicalStatus::Active);
    let wrong_release = LocalizedGloss::new(
      canonical_id("gloss-a"),
      target.clone(),
      language("zh-CN"),
      assertion(
        CanonicalDetailKind::LocalizedGloss,
        release("release-other"),
        CanonicalStatus::Active,
      ),
    );
    assert_eq!(
      wrong_release,
      Err(CanonicalContentValidationError::DetailReleaseMismatch)
    );

    let wrong_kind = LocalizedGloss::new(
      canonical_id("gloss-a"),
      target.clone(),
      language("zh-CN"),
      assertion(
        CanonicalDetailKind::Pronunciation,
        target.release_id().clone(),
        CanonicalStatus::Active,
      ),
    );
    assert_eq!(
      wrong_kind,
      Err(CanonicalContentValidationError::DetailKindMismatch)
    );

    let unsupported = CanonicalFactualAssertion::new(
      CanonicalDetailKind::LocalizedGloss,
      target.release_id().clone(),
      CanonicalStatus::Active,
      "跑",
      vec![licensed_lineage(
        "wrong-kind-evidence",
        target.release_id().clone(),
        EvidenceKind::Example,
        CanonicalStatus::Active,
        true,
      )],
    );
    assert_eq!(
      unsupported,
      Err(CanonicalContentValidationError::IncompatibleEvidenceKind)
    );
  }

  #[test]
  fn collocations_and_pitfalls_keep_explicit_roles_and_corrections() {
    let target = target(CanonicalStatus::Active);
    let head = CollocationTerm::new(
      "run",
      Some(target.lexeme_id().clone()),
      Some(target.sense_id().clone()),
    )
    .unwrap();
    let dependent = CollocationTerm::new("quickly", None, None).unwrap();
    let collocation = Collocation::new(
      canonical_id("collocation-a"),
      target.clone(),
      CollocationRole::Head,
      CollocationConstruction::AdverbModifier,
      head,
      dependent,
      assertion(
        CanonicalDetailKind::Collocation,
        target.release_id().clone(),
        CanonicalStatus::Active,
      ),
    );
    assert!(collocation.is_ok());

    let same = assertion(
      CanonicalDetailKind::Pitfall,
      target.release_id().clone(),
      CanonicalStatus::Active,
    );
    let pitfall = LearnerPitfall::new(
      canonical_id("pitfall-a"),
      target,
      language("zh-CN"),
      LearnerPitfallKind::LiteralTranslation,
      same.clone(),
      same,
    );
    assert_eq!(
      pitfall,
      Err(CanonicalContentValidationError::IdenticalPitfallForms)
    );
  }

  #[test]
  fn detail_collection_is_bounded_and_preserves_explicit_absence() {
    let target = target(CanonicalStatus::Active);
    let gloss = LocalizedGloss::new(
      canonical_id("gloss-a"),
      target.clone(),
      language("zh-CN"),
      assertion(
        CanonicalDetailKind::LocalizedGloss,
        target.release_id().clone(),
        CanonicalStatus::Active,
      ),
    )
    .unwrap();
    let empty =
      CanonicalSenseDetails::new(CanonicalSenseDetailsInput::empty(target.clone())).unwrap();
    assert!(empty.examples().is_empty());

    let oversized = CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
      target,
      localized_glosses: vec![gloss; MAX_DETAILS_PER_KIND + 1],
      pronunciations: Vec::new(),
      usage_labels: Vec::new(),
      grammar_patterns: Vec::new(),
      collocations: Vec::new(),
      examples: Vec::new(),
      pitfalls: Vec::new(),
      etymologies: Vec::new(),
      history: Vec::new(),
    });
    assert_eq!(
      oversized,
      Err(CanonicalContentValidationError::TooManyDetails)
    );
  }

  #[test]
  fn detail_aggregate_requires_its_pinned_target_and_every_evidence_permission() {
    let target = target(CanonicalStatus::Active);
    let assertion = CanonicalFactualAssertion::new(
      CanonicalDetailKind::LocalizedGloss,
      target.release_id().clone(),
      CanonicalStatus::Active,
      "跑",
      vec![licensed_lineage(
        "gloss-evidence",
        target.release_id().clone(),
        EvidenceKind::LocalizedGloss,
        CanonicalStatus::Active,
        false,
      )],
    )
    .unwrap();
    let details = CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
      target: target.clone(),
      localized_glosses: vec![LocalizedGloss::new(
        canonical_id("gloss-a"),
        target.clone(),
        language("zh-CN"),
        assertion,
      )
      .unwrap()],
      pronunciations: Vec::new(),
      usage_labels: Vec::new(),
      grammar_patterns: Vec::new(),
      collocations: Vec::new(),
      examples: Vec::new(),
      pitfalls: Vec::new(),
      etymologies: Vec::new(),
      history: Vec::new(),
    })
    .unwrap();

    assert!(details.is_eligible_for(target.release_id(), target.sense_id(), EvidenceUse::Display));
    assert!(!details.is_eligible_for(
      target.release_id(),
      target.sense_id(),
      EvidenceUse::ApiRedistribution
    ));
    assert!(!details.is_eligible_for(
      &release("other-release"),
      target.sense_id(),
      EvidenceUse::Display
    ));
  }

  #[test]
  fn evolution_mappings_enforce_release_scoped_split_and_merge_shapes() {
    let evolution_assertion = assertion(
      CanonicalDetailKind::SenseEvolution,
      release("release-b"),
      CanonicalStatus::Active,
    );
    let split = SenseEvolutionMapping::new(SenseEvolutionMappingInput {
      id: canonical_id("evolution-a"),
      from_release_id: release("release-a"),
      to_release_id: release("release-b"),
      kind: SenseEvolutionKind::Split,
      status: CanonicalStatus::Active,
      retired_sense_ids: vec![canonical_id("old-sense")],
      successor_sense_ids: vec![canonical_id("new-sense-a"), canonical_id("new-sense-b")],
      assertion: evolution_assertion,
    });
    assert!(split.is_ok());

    let invalid_merge = SenseEvolutionMapping::new(SenseEvolutionMappingInput {
      id: canonical_id("evolution-b"),
      from_release_id: release("release-a"),
      to_release_id: release("release-b"),
      kind: SenseEvolutionKind::Merge,
      status: CanonicalStatus::Active,
      retired_sense_ids: vec![canonical_id("old-sense")],
      successor_sense_ids: vec![canonical_id("new-sense")],
      assertion: assertion(
        CanonicalDetailKind::SenseEvolution,
        release("release-b"),
        CanonicalStatus::Active,
      ),
    });
    assert_eq!(
      invalid_merge,
      Err(CanonicalContentValidationError::InvalidSenseEvolutionShape)
    );
  }

  #[test]
  fn inactive_details_are_not_servable_even_with_valid_lineage() {
    let release_id = release("release-a");
    let factual = CanonicalFactualAssertion::new(
      CanonicalDetailKind::Example,
      release_id.clone(),
      CanonicalStatus::Draft,
      "A staging example.",
      vec![licensed_lineage(
        "draft-example",
        release_id,
        EvidenceKind::Example,
        CanonicalStatus::Draft,
        true,
      )],
    )
    .unwrap();

    assert!(!factual.permits(EvidenceUse::Display));
  }

  #[test]
  fn sense_targets_reject_unrelated_ownership_and_release_lineage() {
    let lexeme = Lexeme {
      id: canonical_id("lexeme-a"),
      release_id: release("release-a"),
      language: language("en-US"),
      lemma: "run".to_string(),
      normalized_lemma: "run".to_string(),
      part_of_speech: LexicalPartOfSpeech::Verb,
      status: CanonicalStatus::Active,
    };
    let unrelated_sense = Sense {
      id: canonical_id("sense-a"),
      lexeme_id: canonical_id("other-lexeme"),
      release_id: release("release-a"),
      sense_key: "run-1".to_string(),
      definition: "move quickly".to_string(),
      definition_evidence_ids: vec![],
      status: CanonicalStatus::Active,
    };
    assert_eq!(
      SenseContentTarget::new(&lexeme, &unrelated_sense),
      Err(CanonicalContentValidationError::SenseLexemeMismatch)
    );
  }
}
