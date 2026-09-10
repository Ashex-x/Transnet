//! Canonical lexical entities and provenance invariants.

use std::{fmt, str::FromStr};

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Maximum accepted length of a canonical BCP-47 language tag.
pub const MAX_LANGUAGE_TAG_LENGTH: usize = 35;

/// Validation failures for canonical lexical values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalValidationError {
  /// An opaque identifier was empty after trimming surrounding whitespace.
  #[error("canonical identifier must not be blank")]
  BlankIdentifier,
  /// A BCP-47 language tag did not have the supported conservative shape.
  #[error("language tag must be a BCP-47 language tag")]
  InvalidLanguageTag,
}

/// Opaque stable identifier for a canonical entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalId(String);

impl CanonicalId {
  /// Creates an opaque identifier after trimming surrounding whitespace.
  ///
  /// # Errors
  ///
  /// Returns an error when `value` is blank.
  pub fn new(value: impl AsRef<str>) -> Result<Self, CanonicalValidationError> {
    let value = value.as_ref().trim();
    if value.is_empty() {
      return Err(CanonicalValidationError::BlankIdentifier);
    }
    Ok(Self(value.to_string()))
  }

  /// Returns the opaque identifier as a string slice.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl AsRef<str> for CanonicalId {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl fmt::Display for CanonicalId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.as_str())
  }
}

impl FromStr for CanonicalId {
  type Err = CanonicalValidationError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    Self::new(value)
  }
}

/// Identifier of an immutable lexical-content release.
pub type ReleaseId = CanonicalId;
/// Identifier of a source-policy record.
pub type SourceId = CanonicalId;
/// Identifier of one independently citable evidence fragment.
pub type EvidenceId = CanonicalId;
/// Identifier of a language-specific lexeme.
pub type LexemeId = CanonicalId;
/// Identifier of a lexeme form or alias.
pub type FormId = CanonicalId;
/// Identifier of one lexical sense.
pub type SenseId = CanonicalId;
/// Identifier of an immutable vector collection version.
pub type VectorCollectionId = CanonicalId;

/// A normalized, validated BCP-47 language tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageTag(String);

impl LanguageTag {
  /// Parses and canonicalizes a conservative BCP-47 language tag.
  ///
  /// # Errors
  ///
  /// Returns an error when `value` is not a supported BCP-47-shaped tag.
  pub fn parse(value: &str) -> Result<Self, CanonicalValidationError> {
    if !is_language_tag(value) {
      return Err(CanonicalValidationError::InvalidLanguageTag);
    }
    Ok(Self(canonicalize_language_tag(value)))
  }

  /// Returns the canonical BCP-47 tag.
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Returns the canonical primary language subtag as a standalone tag.
  pub fn primary_language(&self) -> Self {
    let primary = self
      .0
      .split_once('-')
      .map_or(self.0.as_str(), |(primary, _)| primary);
    Self(primary.to_string())
  }
}

impl AsRef<str> for LanguageTag {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl fmt::Display for LanguageTag {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.as_str())
  }
}

impl FromStr for LanguageTag {
  type Err = CanonicalValidationError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    Self::parse(value)
  }
}

/// An enabled or known language in a canonical release.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Language {
  /// Canonical BCP-47 tag.
  pub tag: LanguageTag,
  /// Human-readable language name.
  pub display_name: String,
  /// Version of morphology data used for this language.
  pub morphology_version: String,
  /// Whether new requests may resolve to this language.
  pub enabled: bool,
}

/// Publication state of immutable lexical content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseStatus {
  /// Content is being imported, validated, or evaluated and cannot serve lookups.
  Staging,
  /// Content may be selected by the active-content pointer.
  Published,
  /// Content is retained only for rollback, provenance, or historical reads.
  Retired,
}

/// Immutable set of canonical lexical content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LexiconRelease {
  /// Stable public release identifier.
  pub id: ReleaseId,
  /// Hash of the complete source manifest used to construct the release.
  pub source_manifest_hash: String,
  /// Publication lifecycle state.
  pub status: ReleaseStatus,
  /// Prior compatible release retained for rollback, when one exists.
  pub rollback_predecessor: Option<ReleaseId>,
}

/// Exact compatible lexical and vector versions selected for a request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActiveContentVersion {
  /// Immutable lexical release used for canonical records and permissions.
  pub release_id: ReleaseId,
  /// Immutable vector collection paired with `release_id`.
  pub vector_collection_id: VectorCollectionId,
  /// Canonical-schema version shared by both stores.
  pub schema_version: String,
  /// Deterministic retrieval-ranking implementation version.
  pub ranking_version: String,
}

/// One operation that source terms may permit independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceUse {
  /// Persist a source fragment in canonical storage.
  Storage,
  /// Present a fragment in a learner-facing response.
  Display,
  /// Create a derived vector from a fragment.
  Embedding,
  /// Send a fragment to a configured model provider.
  ModelProcessing,
  /// Redistribute a fragment through a public API or export.
  ApiRedistribution,
}

/// Source-license permissions captured on every evidence fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePermissions {
  /// Whether the source may be stored in canonical infrastructure.
  pub storage: bool,
  /// Whether the source may be displayed to a learner.
  pub display: bool,
  /// Whether a derived vector may be created from the source.
  pub embedding: bool,
  /// Whether the source may be sent to a model provider.
  pub model_processing: bool,
  /// Whether the source may be returned through a public API or export.
  pub api_redistribution: bool,
}

impl SourcePermissions {
  /// Returns whether the requested operation is permitted.
  pub const fn allows(self, evidence_use: EvidenceUse) -> bool {
    match evidence_use {
      EvidenceUse::Storage => self.storage,
      EvidenceUse::Display => self.display,
      EvidenceUse::Embedding => self.embedding,
      EvidenceUse::ModelProcessing => self.model_processing,
      EvidenceUse::ApiRedistribution => self.api_redistribution,
    }
  }
}

/// Licensed source and policy metadata for canonical evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LexicalSource {
  /// Stable source-policy identifier.
  pub id: SourceId,
  /// Stable source name owned by the provider or licensor.
  pub name: String,
  /// Imported source version or retrieval date.
  pub version: String,
  /// SPDX identifier or another stable license reference.
  pub license: String,
  /// Required attribution text or URL, when any.
  pub attribution: Option<String>,
  /// Machine-readable permissions for this source version.
  pub permissions: SourcePermissions,
}

/// Lifecycle state of a factual fragment or lexical record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalStatus {
  /// Record is eligible for the published release that contains it.
  Active,
  /// Record requires editorial review and is not eligible for default reads.
  Draft,
  /// Record is temporarily excluded because of policy, safety, or licensing concerns.
  Quarantined,
  /// Record is no longer active but remains available for historical provenance.
  Retired,
}

impl CanonicalStatus {
  /// Returns whether this record may participate in a default canonical lookup.
  pub const fn is_lookup_eligible(self) -> bool {
    matches!(self, Self::Active)
  }
}

/// Type of factual material carried by an evidence fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
  /// Dictionary definition or sense description.
  Definition,
  /// Localized gloss or translation.
  LocalizedGloss,
  /// Corpus or dictionary example.
  Example,
  /// Pronunciation, spelling, or phonetic assertion.
  Pronunciation,
  /// Usage, grammar, register, frequency, or level assertion.
  Usage,
  /// Etymology or dated historical assertion.
  Etymology,
  /// Other citable canonical detail.
  Other,
}

/// Source-qualified confidence in one evidence-backed assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceConfidence {
  /// Multiple or directly authoritative sources support the assertion.
  High,
  /// The assertion is useful but has a material source or scope qualification.
  Medium,
  /// The assertion is retained for review or explicitly cautious presentation.
  Low,
}

/// Independently citable, source-qualified evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceFragment {
  /// Stable evidence identifier.
  pub id: EvidenceId,
  /// Source-policy record that governs this fragment.
  pub source_id: SourceId,
  /// Source-local identifier retained for correction and removal.
  pub source_reference: String,
  /// Immutable content release containing the fragment.
  pub release_id: ReleaseId,
  /// Language of the evidence text.
  pub language: LanguageTag,
  /// Classification of the supported factual assertion.
  pub kind: EvidenceKind,
  /// Source-qualified confidence retained separately from retrieval rank.
  pub confidence: EvidenceConfidence,
  /// Licensed source text retained as the independently citable fragment.
  pub text: String,
  /// Content hash for idempotent imports and vector reconciliation.
  pub content_hash: String,
  /// Per-fragment permission snapshot, including asset-level restrictions.
  pub permissions: SourcePermissions,
  /// Editorial lifecycle state.
  pub status: CanonicalStatus,
}

impl EvidenceFragment {
  /// Returns whether this evidence can support the requested operation in `release_id`.
  pub fn permits(&self, release_id: &ReleaseId, evidence_use: EvidenceUse) -> bool {
    self.release_id == *release_id
      && self.status.is_lookup_eligible()
      && self.permissions.allows(evidence_use)
  }
}

/// Canonical part of speech attached to a language-specific lexeme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexicalPartOfSpeech {
  /// Noun.
  Noun,
  /// Verb.
  Verb,
  /// Adjective.
  Adjective,
  /// Adverb.
  Adverb,
  /// Pronoun.
  Pronoun,
  /// Preposition.
  Preposition,
  /// Conjunction.
  Conjunction,
  /// Determiner.
  Determiner,
  /// Interjection.
  Interjection,
  /// Numeral.
  Numeral,
  /// Multi-word expression or source-specific lexical class.
  Other,
}

/// A language-specific lemma and part of speech.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lexeme {
  /// Stable lexeme identifier.
  pub id: LexemeId,
  /// Immutable release containing the lexeme.
  pub release_id: ReleaseId,
  /// Language of the lemma.
  pub language: LanguageTag,
  /// User-visible lemma, preserving source spelling.
  pub lemma: String,
  /// Explicit normalized lookup key rather than an implicit database collation.
  pub normalized_lemma: String,
  /// Canonical part of speech.
  pub part_of_speech: LexicalPartOfSpeech,
  /// Editorial lifecycle state.
  pub status: CanonicalStatus,
}

/// Retrieval role of a stored surface form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FormKind {
  /// Canonical lemma form.
  Lemma,
  /// Exact spelling variant that does not change the lexical meaning.
  SpellingVariant,
  /// Inflected or morphologically derived surface form.
  Inflection,
  /// Multi-token expression attached to a lexeme.
  Phrase,
  /// Romanization, common misspelling, or explicitly marked alias.
  Alias,
}

/// One inflection, spelling variant, phrase, or alias belonging to a lexeme.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WordForm {
  /// Stable form identifier.
  pub id: FormId,
  /// Lexeme that owns this form.
  pub lexeme_id: LexemeId,
  /// Immutable release containing the form.
  pub release_id: ReleaseId,
  /// User-visible surface form.
  pub form: String,
  /// Explicit normalized lookup key.
  pub normalized_form: String,
  /// Retrieval and display role for the form.
  pub kind: FormKind,
  /// Source-qualified morphology metadata version or feature summary.
  pub morphology: Option<String>,
  /// Evidence supporting the form assertion.
  pub evidence_ids: Vec<EvidenceId>,
  /// Editorial lifecycle state.
  pub status: CanonicalStatus,
}

/// One meaning of a lexeme, kept distinct from its spelling and other senses.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sense {
  /// Stable sense identifier.
  pub id: SenseId,
  /// Lexeme to which this meaning belongs.
  pub lexeme_id: LexemeId,
  /// Immutable release containing the sense.
  pub release_id: ReleaseId,
  /// Source-stable key that distinguishes senses within a lexeme.
  pub sense_key: String,
  /// Concise canonical definition; evidence remains authoritative for this text.
  pub definition: String,
  /// Evidence fragments that support the definition.
  pub definition_evidence_ids: Vec<EvidenceId>,
  /// Editorial lifecycle state.
  pub status: CanonicalStatus,
}

/// Produces a baseline NFC and case-normalized lookup key.
///
/// Adapters may apply additional language-aware segmentation or morphology rules, but they must
/// retain this explicit key rather than depend on a database collation.
pub fn normalize_lookup_key(value: &str) -> String {
  value.trim().nfc().flat_map(char::to_lowercase).collect()
}

fn is_language_tag(value: &str) -> bool {
  if value.is_empty() || value.len() > MAX_LANGUAGE_TAG_LENGTH || value.trim() != value {
    return false;
  }

  let mut subtags = value.split('-');
  let Some(language) = subtags.next() else {
    return false;
  };
  if !(2..=8).contains(&language.len()) || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
  {
    return false;
  }

  subtags.all(|subtag| {
    !subtag.is_empty()
      && subtag.len() <= 8
      && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
  })
}

fn canonicalize_language_tag(value: &str) -> String {
  value
    .split('-')
    .enumerate()
    .map(|(index, subtag)| {
      if index == 0 {
        subtag.to_ascii_lowercase()
      } else if subtag.len() == 2 {
        subtag.to_ascii_uppercase()
      } else if subtag.len() == 4 {
        let mut characters = subtag.chars();
        let first = characters
          .next()
          .map(|character| character.to_ascii_uppercase())
          .into_iter();
        first
          .chain(characters.map(|character| character.to_ascii_lowercase()))
          .collect()
      } else {
        subtag.to_ascii_lowercase()
      }
    })
    .collect::<Vec<_>>()
    .join("-")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn language_tags_are_validated_and_canonicalized() {
    let language = LanguageTag::parse("zh-cn").unwrap();

    assert_eq!(language.as_str(), "zh-CN");
    assert_eq!(
      LanguageTag::parse("en-US")
        .unwrap()
        .primary_language()
        .as_str(),
      "en"
    );
    assert!(LanguageTag::parse("zh_CN").is_err());
    assert!(LanguageTag::parse(" en ").is_err());
  }

  #[test]
  fn permissions_are_checked_per_evidence_operation() {
    let permissions = SourcePermissions {
      storage: true,
      display: true,
      embedding: false,
      model_processing: false,
      api_redistribution: true,
    };

    assert!(permissions.allows(EvidenceUse::Display));
    assert!(!permissions.allows(EvidenceUse::Embedding));
  }

  #[test]
  fn lookup_keys_normalize_unicode_and_case() {
    assert_eq!(normalize_lookup_key("  CAFE\u{301}  "), "café");
  }
}
