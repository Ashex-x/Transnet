//! Reviewed canonical translation identity, fingerprint, scope, and revision invariants.
//!
//! These values extend the shared canonical primitives rather than defining another lexical or
//! card model. They are transport- and storage-independent and never turn a normalized lookup key
//! into identity. A fingerprint can nominate candidates, but only an exact normalized comparison
//! with the stored source text may accept one.

use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use super::canonical::{
  CanonicalId, EvidenceId, LanguageTag, LexemeId, LexicalPartOfSpeech, ReleaseId, SenseId,
};

/// Version of the opaque canonical-identifier policy used by publishers and adapters.
pub const CANONICAL_ID_POLICY_VERSION: &str = "canonical-id-v1";
/// Version of the source normalization and hashing contract used for translation candidates.
pub const SOURCE_FINGERPRINT_VERSION: &str = "translation-source-v1";
/// Maximum source size accepted by the reusable canonical-translation model.
pub const MAX_CANONICAL_SOURCE_CHARS: usize = 4_096;
/// Maximum target size accepted by the reusable canonical-translation model.
pub const MAX_CANONICAL_TARGET_CHARS: usize = 4_096;
/// Maximum number of canonical domains that may scope one reviewed translation.
pub const MAX_TRANSLATION_DOMAINS: usize = 8;
/// Maximum number of evidence records supporting one reviewed translation.
pub const MAX_TRANSLATION_EVIDENCE: usize = 8;

/// Failure while constructing or verifying canonical translation data.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum CanonicalTranslationError {
  /// A public identity did not carry the prefix required for its entity family.
  #[error("canonical identifier does not match its required entity family")]
  InvalidIdentity,
  /// A revision number was zero or could not advance without overflow.
  #[error("canonical revision must be a positive immutable sequence number")]
  InvalidRevision,
  /// Source or target content was blank or exceeded its canonical publication bound.
  #[error("canonical translation text is outside the permitted bound")]
  InvalidText,
  /// A fingerprint was malformed or used an unsupported version.
  #[error("canonical source fingerprint is invalid")]
  InvalidFingerprint,
  /// Meaning ownership or scope was internally inconsistent.
  #[error("canonical translation meaning scope is inconsistent")]
  InvalidMeaningScope,
  /// Evidence or domain scope exceeded its closed publication bound.
  #[error("canonical translation references exceed their permitted bound")]
  TooManyReferences,
  /// The supplied fingerprint did not describe the stored canonical source.
  #[error("canonical source fingerprint does not match stored source")]
  FingerprintMismatch,
}

macro_rules! canonical_family_id {
  ($name:ident, $prefix:literal, $doc:literal) => {
    #[doc = $doc]
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct $name(CanonicalId);

    impl $name {
      /// Creates a typed canonical identity under the current versioned prefix policy.
      ///
      /// # Errors
      ///
      /// Returns [`CanonicalTranslationError::InvalidIdentity`] when the opaque ID is not in this
      /// entity family.
      pub fn new(value: impl AsRef<str>) -> Result<Self, CanonicalTranslationError> {
        let value = value.as_ref();
        let id = CanonicalId::new(value).map_err(|_| CanonicalTranslationError::InvalidIdentity)?;
        if !value.starts_with($prefix) || value.len() == $prefix.len() {
          return Err(CanonicalTranslationError::InvalidIdentity);
        }
        Ok(Self(id))
      }

      /// Returns the stable opaque identifier without deriving meaning from its spelling.
      pub fn as_str(&self) -> &str {
        self.0.as_str()
      }
    }

    impl fmt::Debug for $name {
      fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
          .debug_tuple(stringify!($name))
          .field(&self.as_str())
          .finish()
      }
    }

    impl fmt::Display for $name {
      fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
      }
    }
  };
}

canonical_family_id!(
  CanonicalTranslationId,
  "tr_",
  "Stable identity of one reviewed source-target translation choice."
);
canonical_family_id!(
  CanonicalCardId,
  "card_",
  "Stable identity of one release-addressable canonical basic card."
);
canonical_family_id!(
  ConceptRootId,
  "concept_",
  "Stable identity of a canonical concept root independent of lookup spelling."
);
canonical_family_id!(
  DomainId,
  "domain_",
  "Stable identity of one reviewed canonical domain scope."
);

/// Positive immutable revision number for one stable canonical identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalRevision(u64);

impl CanonicalRevision {
  /// Creates a positive revision number.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalTranslationError::InvalidRevision`] for zero.
  pub const fn new(value: u64) -> Result<Self, CanonicalTranslationError> {
    if value == 0 {
      Err(CanonicalTranslationError::InvalidRevision)
    } else {
      Ok(Self(value))
    }
  }

  /// Returns the stored positive sequence number.
  pub const fn get(self) -> u64 {
    self.0
  }

  /// Returns the next immutable revision for a correction.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalTranslationError::InvalidRevision`] when the sequence is exhausted.
  pub fn next(self) -> Result<Self, CanonicalTranslationError> {
    self
      .0
      .checked_add(1)
      .map(Self)
      .ok_or(CanonicalTranslationError::InvalidRevision)
  }
}

/// Whether a reviewed translation belongs to a word, established phrase, or reusable passage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalTranslationUnit {
  /// One lexical word or technical term.
  Word,
  /// An established multi-token expression with its own meaning identity.
  Phrase,
  /// A bounded reusable reference passage approved for canonical publication.
  Passage,
}

/// How a lexical meaning relates to the surface expression that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeaningComposition {
  /// The meaning follows from the independently selectable component senses.
  Compositional,
  /// The expression owns a distinct phrase-level sense.
  PhraseLevel,
}

/// Stable lexical and domain scope that prevents unrelated meanings from sharing an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMeaningScope {
  lexeme_id: LexemeId,
  sense_id: SenseId,
  part_of_speech: LexicalPartOfSpeech,
  composition: MeaningComposition,
  domain_ids: Vec<DomainId>,
}

impl CanonicalMeaningScope {
  /// Creates an explicit lexical meaning scope with bounded, unique domain IDs.
  ///
  /// # Errors
  ///
  /// Returns an error for duplicate or excessive domain references, or when phrase-level meaning
  /// is requested for a non-phrase translation unit.
  pub fn new(
    lexeme_id: LexemeId,
    sense_id: SenseId,
    part_of_speech: LexicalPartOfSpeech,
    composition: MeaningComposition,
    domain_ids: Vec<DomainId>,
    unit: CanonicalTranslationUnit,
  ) -> Result<Self, CanonicalTranslationError> {
    if domain_ids.len() > MAX_TRANSLATION_DOMAINS
      || domain_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
      return Err(CanonicalTranslationError::InvalidMeaningScope);
    }
    if composition == MeaningComposition::PhraseLevel && unit != CanonicalTranslationUnit::Phrase {
      return Err(CanonicalTranslationError::InvalidMeaningScope);
    }
    Ok(Self {
      lexeme_id,
      sense_id,
      part_of_speech,
      composition,
      domain_ids,
    })
  }

  /// Returns the stable owning lexeme identity.
  pub fn lexeme_id(&self) -> &LexemeId {
    &self.lexeme_id
  }

  /// Returns the independently selectable sense identity.
  pub fn sense_id(&self) -> &SenseId {
    &self.sense_id
  }

  /// Returns the part of speech retained in identity comparison.
  pub const fn part_of_speech(&self) -> LexicalPartOfSpeech {
    self.part_of_speech
  }

  /// Returns whether the sense is compositional or belongs to the phrase as a whole.
  pub const fn composition(&self) -> MeaningComposition {
    self.composition
  }

  /// Returns the ordered canonical domain scope.
  pub fn domain_ids(&self) -> &[DomainId] {
    &self.domain_ids
  }
}

/// SHA-256 candidate key computed under an explicit source-normalization version.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
  /// Computes a version-bound candidate fingerprint without retaining source text.
  ///
  /// The digest includes the normalization version and source language so equal text under a
  /// different contract cannot silently share the same candidate key.
  pub fn compute(source: &str, language: &LanguageTag) -> Self {
    let normalized = normalize_canonical_source(source);
    let mut digest = Sha256::new();
    digest.update(SOURCE_FINGERPRINT_VERSION.as_bytes());
    digest.update([0]);
    digest.update(language.as_str().as_bytes());
    digest.update([0]);
    digest.update(normalized.as_bytes());
    Self(digest.finalize().into())
  }

  /// Parses the canonical `sha256:` representation used by the storage adapter contract.
  ///
  /// # Errors
  ///
  /// Returns [`CanonicalTranslationError::InvalidFingerprint`] for malformed values.
  pub fn parse(value: &str) -> Result<Self, CanonicalTranslationError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
      return Err(CanonicalTranslationError::InvalidFingerprint);
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
      return Err(CanonicalTranslationError::InvalidFingerprint);
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
      let offset = index * 2;
      let pair = &hex[offset..offset + 2];
      *byte =
        u8::from_str_radix(pair, 16).map_err(|_| CanonicalTranslationError::InvalidFingerprint)?;
    }
    Ok(Self(bytes))
  }

  /// Returns the storage-adapter representation without exposing source text.
  pub fn to_storage_key(&self) -> String {
    let mut value = String::with_capacity(71);
    value.push_str("sha256:");
    for byte in self.0 {
      use std::fmt::Write;
      let _ = write!(value, "{byte:02x}");
    }
    value
  }

  /// Verifies a nominated candidate by comparing its stored source after fingerprint selection.
  ///
  /// `self` is the fingerprint returned by candidate lookup. Matching it to the requested digest
  /// is necessary but not sufficient: the normalized stored source must also equal the normalized
  /// requested source. This remains safe if two different sources ever share one digest.
  pub fn verifies_candidate(
    &self,
    requested_source: &str,
    stored_source: &str,
    language: &LanguageTag,
  ) -> bool {
    *self == Self::compute(requested_source, language)
      && normalize_canonical_source(requested_source) == normalize_canonical_source(stored_source)
  }
}

impl fmt::Debug for SourceFingerprint {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("SourceFingerprint([redacted])")
  }
}

/// Immutable reviewed source-target content for one canonical translation revision.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalTranslationRevision {
  id: CanonicalTranslationId,
  revision: CanonicalRevision,
  release_id: ReleaseId,
  unit: CanonicalTranslationUnit,
  source_language: LanguageTag,
  target_language: LanguageTag,
  source_text: String,
  target_text: String,
  source_fingerprint: SourceFingerprint,
  meaning_scope: Option<CanonicalMeaningScope>,
  evidence_ids: Vec<EvidenceId>,
}

impl CanonicalTranslationRevision {
  /// Creates an immutable reviewed revision and verifies its supplied candidate fingerprint.
  ///
  /// Domain references and evidence IDs must be sorted and unique. Words and phrases require an
  /// explicit meaning scope; passages must not claim a lexical sense.
  ///
  /// # Errors
  ///
  /// Returns an error when text, fingerprint, scope, or evidence invariants are not satisfied.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    id: CanonicalTranslationId,
    revision: CanonicalRevision,
    release_id: ReleaseId,
    unit: CanonicalTranslationUnit,
    source_language: LanguageTag,
    target_language: LanguageTag,
    source_text: impl Into<String>,
    target_text: impl Into<String>,
    source_fingerprint: SourceFingerprint,
    meaning_scope: Option<CanonicalMeaningScope>,
    evidence_ids: Vec<EvidenceId>,
  ) -> Result<Self, CanonicalTranslationError> {
    let source_text = source_text.into();
    let target_text = target_text.into();
    if !bounded_text(&source_text, MAX_CANONICAL_SOURCE_CHARS)
      || !bounded_text(&target_text, MAX_CANONICAL_TARGET_CHARS)
    {
      return Err(CanonicalTranslationError::InvalidText);
    }
    if source_fingerprint != SourceFingerprint::compute(&source_text, &source_language) {
      return Err(CanonicalTranslationError::FingerprintMismatch);
    }
    let lexical = unit != CanonicalTranslationUnit::Passage;
    if lexical != meaning_scope.is_some() {
      return Err(CanonicalTranslationError::InvalidMeaningScope);
    }
    if evidence_ids.len() > MAX_TRANSLATION_EVIDENCE
      || evidence_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
      return Err(CanonicalTranslationError::TooManyReferences);
    }
    Ok(Self {
      id,
      revision,
      release_id,
      unit,
      source_language,
      target_language,
      source_text,
      target_text,
      source_fingerprint,
      meaning_scope,
      evidence_ids,
    })
  }

  /// Returns the stable identity shared by all corrections of this translation.
  pub fn id(&self) -> &CanonicalTranslationId {
    &self.id
  }

  /// Returns this immutable content revision number.
  pub const fn revision(&self) -> CanonicalRevision {
    self.revision
  }

  /// Returns the immutable release containing this revision.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release_id
  }

  /// Returns the translation unit governed by this revision.
  pub const fn unit(&self) -> CanonicalTranslationUnit {
    self.unit
  }

  /// Returns the source language.
  pub fn source_language(&self) -> &LanguageTag {
    &self.source_language
  }

  /// Returns the target language.
  pub fn target_language(&self) -> &LanguageTag {
    &self.target_language
  }

  /// Returns the reviewed source text for exact application-layer verification.
  pub fn source_text(&self) -> &str {
    &self.source_text
  }

  /// Returns the reviewed target text.
  pub fn target_text(&self) -> &str {
    &self.target_text
  }

  /// Returns the candidate fingerprint, which is never sufficient by itself for acceptance.
  pub fn source_fingerprint(&self) -> &SourceFingerprint {
    &self.source_fingerprint
  }

  /// Returns the lexical meaning scope, absent only for reusable passages.
  pub fn meaning_scope(&self) -> Option<&CanonicalMeaningScope> {
    self.meaning_scope.as_ref()
  }

  /// Returns the bounded evidence references supporting publication.
  pub fn evidence_ids(&self) -> &[EvidenceId] {
    &self.evidence_ids
  }

  /// Verifies both the candidate fingerprint and the stored source text under the same contract.
  ///
  /// This second comparison is mandatory because a fingerprint match only nominates a candidate.
  pub fn verifies_source(&self, requested_source: &str) -> bool {
    self.source_fingerprint.verifies_candidate(
      requested_source,
      &self.source_text,
      &self.source_language,
    )
  }

  /// Creates the next immutable correction while retaining the stable translation identity.
  ///
  /// The current value is not mutated. Callers must stage and publish the returned revision in a
  /// later release; this method does not activate or persist it.
  ///
  /// # Errors
  ///
  /// Returns an error when the next revision or replacement content violates an invariant.
  pub fn corrected(
    &self,
    release_id: ReleaseId,
    target_text: impl Into<String>,
    evidence_ids: Vec<EvidenceId>,
  ) -> Result<Self, CanonicalTranslationError> {
    Self::new(
      self.id.clone(),
      self.revision.next()?,
      release_id,
      self.unit,
      self.source_language.clone(),
      self.target_language.clone(),
      self.source_text.clone(),
      target_text,
      self.source_fingerprint.clone(),
      self.meaning_scope.clone(),
      evidence_ids,
    )
  }
}

impl fmt::Debug for CanonicalTranslationRevision {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CanonicalTranslationRevision")
      .field("id", &self.id)
      .field("revision", &self.revision)
      .field("release_id", &self.release_id)
      .field("unit", &self.unit)
      .field("source_language", &self.source_language)
      .field("target_language", &self.target_language)
      .field("source_text", &"[redacted]")
      .field("target_text", &"[redacted]")
      .field("source_fingerprint", &self.source_fingerprint)
      .field("meaning_scope", &self.meaning_scope)
      .field("evidence_count", &self.evidence_ids.len())
      .finish()
  }
}

fn normalize_canonical_source(value: &str) -> String {
  value.trim().nfc().collect()
}

fn bounded_text(value: &str, max_chars: usize) -> bool {
  !value.trim().is_empty() && value.chars().count() <= max_chars
}

#[cfg(test)]
mod tests {
  use super::*;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language(value: &str) -> LanguageTag {
    LanguageTag::parse(value).unwrap()
  }

  fn scope(
    sense: &str,
    part_of_speech: LexicalPartOfSpeech,
    composition: MeaningComposition,
    domains: &[&str],
    unit: CanonicalTranslationUnit,
  ) -> CanonicalMeaningScope {
    CanonicalMeaningScope::new(
      id(&format!("lexeme-{sense}")),
      id(sense),
      part_of_speech,
      composition,
      domains
        .iter()
        .map(|domain| DomainId::new(domain).unwrap())
        .collect(),
      unit,
    )
    .unwrap()
  }

  fn revision(source: &str, meaning_scope: CanonicalMeaningScope) -> CanonicalTranslationRevision {
    let source_language = language("en");
    CanonicalTranslationRevision::new(
      CanonicalTranslationId::new("tr_test_01").unwrap(),
      CanonicalRevision::new(1).unwrap(),
      id("release-1"),
      CanonicalTranslationUnit::Word,
      source_language.clone(),
      language("zh-CN"),
      source,
      "目标译文",
      SourceFingerprint::compute(source, &source_language),
      Some(meaning_scope),
      vec![id("evidence-1")],
    )
    .unwrap()
  }

  #[test]
  fn typed_ids_are_stable_and_reject_wrong_families() {
    let id = CanonicalTranslationId::new("tr_fixed_01").unwrap();
    assert_eq!(id.as_str(), "tr_fixed_01");
    assert_eq!(CANONICAL_ID_POLICY_VERSION, "canonical-id-v1");
    assert_eq!(
      CanonicalTranslationId::new("sense_fixed_01"),
      Err(CanonicalTranslationError::InvalidIdentity)
    );
    assert_eq!(
      DomainId::new("domain_"),
      Err(CanonicalTranslationError::InvalidIdentity)
    );
  }

  #[test]
  fn meaning_scope_keeps_sense_pos_domain_and_composition_distinct() {
    let noun = scope(
      "sense-run-noun",
      LexicalPartOfSpeech::Noun,
      MeaningComposition::Compositional,
      &["domain_general"],
      CanonicalTranslationUnit::Word,
    );
    let verb = scope(
      "sense-run-verb",
      LexicalPartOfSpeech::Verb,
      MeaningComposition::Compositional,
      &["domain_computing"],
      CanonicalTranslationUnit::Word,
    );
    let phrase = scope(
      "sense-run-out",
      LexicalPartOfSpeech::Verb,
      MeaningComposition::PhraseLevel,
      &["domain_general"],
      CanonicalTranslationUnit::Phrase,
    );

    assert_ne!(noun, verb);
    assert_ne!(verb, phrase);
    assert_eq!(phrase.composition(), MeaningComposition::PhraseLevel);
    assert_eq!(phrase.domain_ids()[0].as_str(), "domain_general");
  }

  #[test]
  fn technical_symbols_remain_distinct_in_source_fingerprints() {
    let language = language("en");
    let c = SourceFingerprint::compute("C", &language);
    let cpp = SourceFingerprint::compute("C++", &language);
    let csharp = SourceFingerprint::compute("C#", &language);

    assert_ne!(c, cpp);
    assert_ne!(cpp, csharp);
    assert_ne!(c, csharp);
  }

  #[test]
  fn fingerprint_candidate_requires_stored_source_verification() {
    let scope = scope(
      "sense-cpp",
      LexicalPartOfSpeech::Noun,
      MeaningComposition::Compositional,
      &["domain_computing"],
      CanonicalTranslationUnit::Word,
    );
    let canonical = revision("C++", scope);

    assert!(canonical.verifies_source(" C++ "));
    assert!(!canonical.verifies_source("C#"));

    let colliding_candidate_key = SourceFingerprint::compute("C++", &language("en"));
    assert_eq!(colliding_candidate_key, *canonical.source_fingerprint());
    assert!(!colliding_candidate_key.verifies_candidate("C++", "C#", &language("en")));
  }

  #[test]
  fn corrections_create_new_revisions_without_mutating_published_content() {
    let original = revision(
      "run",
      scope(
        "sense-run-verb",
        LexicalPartOfSpeech::Verb,
        MeaningComposition::Compositional,
        &["domain_general"],
        CanonicalTranslationUnit::Word,
      ),
    );
    let corrected = original
      .corrected(id("release-2"), "运行", vec![id("evidence-2")])
      .unwrap();

    assert_eq!(original.revision().get(), 1);
    assert_eq!(original.target_text(), "目标译文");
    assert_eq!(corrected.revision().get(), 2);
    assert_eq!(corrected.target_text(), "运行");
    assert_eq!(original.id(), corrected.id());
  }

  #[test]
  fn normalized_lookup_text_is_not_used_as_canonical_identity() {
    let normalized = super::super::canonical::normalize_lookup_key(" C++ ");

    assert_eq!(normalized, "c++");
    assert_eq!(
      CanonicalTranslationId::new(&normalized),
      Err(CanonicalTranslationError::InvalidIdentity)
    );
  }

  #[test]
  fn debug_and_errors_redact_canonical_text_and_fingerprint() {
    let source = "canonical-source-secret-9137";
    let target = "canonical-target-secret-2458";
    let source_language = language("en");
    let value = CanonicalTranslationRevision::new(
      CanonicalTranslationId::new("tr_secret_01").unwrap(),
      CanonicalRevision::new(1).unwrap(),
      id("release-1"),
      CanonicalTranslationUnit::Word,
      source_language.clone(),
      language("zh-CN"),
      source,
      target,
      SourceFingerprint::compute(source, &source_language),
      Some(scope(
        "sense-secret",
        LexicalPartOfSpeech::Noun,
        MeaningComposition::Compositional,
        &["domain_general"],
        CanonicalTranslationUnit::Word,
      )),
      vec![id("evidence-1")],
    )
    .unwrap();

    let rendered = format!("{value:?} {:?}", value.source_fingerprint());
    assert!(!rendered.contains(source));
    assert!(!rendered.contains(target));
    assert!(!rendered.contains(&value.source_fingerprint().to_storage_key()));
    assert!(!CanonicalTranslationError::InvalidText
      .to_string()
      .contains(source));
  }
}
