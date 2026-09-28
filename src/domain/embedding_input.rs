//! Release-pinned authoritative lexical material and canonical embedding inputs.

use std::{
  collections::{BTreeMap, BTreeSet},
  fmt,
};

use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use super::canonical::{
  CanonicalId, CanonicalStatus, EvidenceId, EvidenceUse, Lexeme, ReleaseId, Sense, WordForm,
};
use super::canonical_content::{CanonicalEvidenceLineage, LocalizedGloss};
use super::canonical_translation::{CanonicalTranslationRevision, CanonicalTranslationUnit};

/// Dense node input contract version.
pub const NODE_DENSE_INPUT_VERSION: &str = "node-dense-input-v1";
/// Deterministic lexical node input contract version.
pub const NODE_LEXICAL_INPUT_VERSION: &str = "node-lexical-input-v1";
/// Dense edge input contract version.
pub const EDGE_DENSE_INPUT_VERSION: &str = "edge-dense-input-v1";
/// Deterministic lexical edge input contract version.
pub const EDGE_LEXICAL_INPUT_VERSION: &str = "edge-lexical-input-v1";

const MAX_FORMS: usize = 24;
const MAX_GLOSSES: usize = 24;
const MAX_TRANSLATIONS: usize = 8;
const MAX_EVIDENCE_PER_ASSERTION: usize = 8;

/// Dense or lexical embedding-input family with an independent hash domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingInputFamily {
  /// Controlled dense semantic embedding.
  Dense,
  /// Versioned deterministic lexical encoding.
  Lexical,
}

/// Canonical bytes and digest supplied to the future controlled embedding boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalEmbeddingInput {
  family: EmbeddingInputFamily,
  specification_version: &'static str,
  canonical_bytes: Vec<u8>,
  input_hash: String,
}

impl CanonicalEmbeddingInput {
  /// Returns the independent input family.
  pub const fn family(&self) -> EmbeddingInputFamily {
    self.family
  }
  /// Returns the frozen canonical-input specification version.
  pub const fn specification_version(&self) -> &'static str {
    self.specification_version
  }
  /// Returns length-prefixed canonical bytes. Callers must never log these bytes.
  pub fn canonical_bytes(&self) -> &[u8] {
    &self.canonical_bytes
  }
  /// Returns the family-separated SHA-256 input digest.
  pub fn input_hash(&self) -> &str {
    &self.input_hash
  }

  pub(crate) fn from_fields(
    family: EmbeddingInputFamily,
    version: &'static str,
    fields: impl FnOnce(&mut CanonicalWriter),
  ) -> Self {
    let mut writer = CanonicalWriter::new();
    writer.field("input_spec", version);
    writer.field(
      "family",
      match family {
        EmbeddingInputFamily::Dense => "dense",
        EmbeddingInputFamily::Lexical => "lexical",
      },
    );
    fields(&mut writer);
    let canonical_bytes = writer.finish();
    let mut digest = Sha256::new();
    digest.update(b"transnet-embedding-input\0");
    digest.update(version.as_bytes());
    digest.update([0]);
    digest.update(&canonical_bytes);
    Self {
      family,
      specification_version: version,
      input_hash: format!("sha256:{:x}", digest.finalize()),
      canonical_bytes,
    }
  }
}

impl fmt::Debug for CanonicalEmbeddingInput {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CanonicalEmbeddingInput")
      .field("family", &self.family)
      .field("specification_version", &self.specification_version)
      .field("canonical_bytes", &"[redacted]")
      .field("input_hash", &self.input_hash)
      .finish()
  }
}

/// Reviewed lexical material admitted to deterministic embedding preparation.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthoritativeEmbeddingMaterial {
  lexeme: Lexeme,
  sense: Option<Sense>,
  forms: Vec<WordForm>,
  glosses: Vec<LocalizedGloss>,
  translations: Vec<CanonicalTranslationRevision>,
  evidence: Vec<CanonicalEvidenceLineage>,
}

impl fmt::Debug for AuthoritativeEmbeddingMaterial {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("AuthoritativeEmbeddingMaterial")
      .field("release_id", &self.lexeme.release_id)
      .field("lexeme_id", &self.lexeme.id)
      .field("sense_id", &self.sense.as_ref().map(|sense| &sense.id))
      .field("forms", &self.forms.len())
      .field("glosses", &self.glosses.len())
      .field("translations", &self.translations.len())
      .field("evidence", &self.evidence.len())
      .finish()
  }
}

impl AuthoritativeEmbeddingMaterial {
  /// Validates one bounded, release-owned aggregate without synthesizing lexical material.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    lexeme: Lexeme,
    sense: Option<Sense>,
    mut forms: Vec<WordForm>,
    mut glosses: Vec<LocalizedGloss>,
    mut translations: Vec<CanonicalTranslationRevision>,
    mut evidence: Vec<CanonicalEvidenceLineage>,
  ) -> Result<Self, EmbeddingMaterialError> {
    if lexeme.status != CanonicalStatus::Active {
      return Err(EmbeddingMaterialError::IneligibleRecord);
    }
    if lexeme.lemma_evidence_ids.is_empty() {
      return Err(EmbeddingMaterialError::MissingLemmaEvidence);
    }
    validate_ids(&lexeme.lemma_evidence_ids)?;
    if forms.len() > MAX_FORMS
      || glosses.len() > MAX_GLOSSES
      || translations.len() > MAX_TRANSLATIONS
    {
      return Err(EmbeddingMaterialError::TooManyItems);
    }
    if let Some(item) = &sense {
      if item.lexeme_id != lexeme.id || item.release_id != lexeme.release_id {
        return Err(EmbeddingMaterialError::OwnershipMismatch);
      }
      if item.status != CanonicalStatus::Active {
        return Err(EmbeddingMaterialError::IneligibleRecord);
      }
      validate_ids(&item.definition_evidence_ids)?;
    }
    forms.sort_by(|left, right| left.id.cmp(&right.id));
    ensure_unique(forms.iter().map(|item| &item.id))?;
    for form in &forms {
      if form.lexeme_id != lexeme.id || form.release_id != lexeme.release_id {
        return Err(EmbeddingMaterialError::OwnershipMismatch);
      }
      if form.status != CanonicalStatus::Active {
        return Err(EmbeddingMaterialError::IneligibleRecord);
      }
      validate_ids(&form.evidence_ids)?;
    }
    glosses.sort_by(|left, right| left.id().cmp(right.id()));
    ensure_unique(glosses.iter().map(LocalizedGloss::id))?;
    for gloss in &glosses {
      let Some(expected_sense) = &sense else {
        return Err(EmbeddingMaterialError::OwnershipMismatch);
      };
      if gloss.target().release_id() != &lexeme.release_id
        || gloss.target().lexeme_id() != &lexeme.id
        || gloss.target().sense_id() != &expected_sense.id
      {
        return Err(EmbeddingMaterialError::OwnershipMismatch);
      }
      if !gloss.assertion().permits(EvidenceUse::Embedding) {
        return Err(EmbeddingMaterialError::EvidenceNotPermitted);
      }
    }
    translations.sort_by(|left, right| {
      left
        .id()
        .as_str()
        .cmp(right.id().as_str())
        .then(left.revision().get().cmp(&right.revision().get()))
    });
    ensure_unique(
      translations
        .iter()
        .map(|item| (item.id().as_str(), item.revision().get())),
    )?;
    for translation in &translations {
      if translation.release_id() != &lexeme.release_id
        || translation.unit() == CanonicalTranslationUnit::Passage
      {
        return Err(EmbeddingMaterialError::TranslationScopeMismatch);
      }
      let Some(scope) = translation.meaning_scope() else {
        return Err(EmbeddingMaterialError::TranslationScopeMismatch);
      };
      if scope.lexeme_id() != &lexeme.id
        || scope.part_of_speech() != lexeme.part_of_speech
        || sense
          .as_ref()
          .is_some_and(|item| scope.sense_id() != &item.id)
      {
        return Err(EmbeddingMaterialError::TranslationScopeMismatch);
      }
      validate_ids(translation.evidence_ids())?;
    }
    evidence.sort_by(|left, right| left.fragment().id.cmp(&right.fragment().id));
    ensure_unique(evidence.iter().map(|item| &item.fragment().id))?;
    let lineages = evidence
      .iter()
      .map(|item| (item.fragment().id.clone(), item))
      .collect::<BTreeMap<_, _>>();
    let mut used = BTreeSet::new();
    resolve(
      &lexeme.lemma_evidence_ids,
      &lineages,
      &lexeme.release_id,
      &mut used,
    )?;
    if let Some(item) = &sense {
      resolve(
        &item.definition_evidence_ids,
        &lineages,
        &lexeme.release_id,
        &mut used,
      )?;
    }
    for form in &forms {
      resolve(&form.evidence_ids, &lineages, &lexeme.release_id, &mut used)?;
    }
    for translation in &translations {
      resolve(
        translation.evidence_ids(),
        &lineages,
        &lexeme.release_id,
        &mut used,
      )?;
    }
    if used.len() != lineages.len() {
      return Err(EmbeddingMaterialError::DanglingEvidence);
    }
    Ok(Self {
      lexeme,
      sense,
      forms,
      glosses,
      translations,
      evidence,
    })
  }

  /// Returns the authoritative lexeme.
  pub fn lexeme(&self) -> &Lexeme {
    &self.lexeme
  }
  /// Returns the optional authoritative sense.
  pub fn sense(&self) -> Option<&Sense> {
    self.sense.as_ref()
  }
  /// Returns a canonical dense node input.
  pub fn dense_input(&self, node_id: &CanonicalId) -> CanonicalEmbeddingInput {
    self.input(
      node_id,
      EmbeddingInputFamily::Dense,
      NODE_DENSE_INPUT_VERSION,
    )
  }
  /// Returns a canonical lexical node input.
  pub fn lexical_input(&self, node_id: &CanonicalId) -> CanonicalEmbeddingInput {
    self.input(
      node_id,
      EmbeddingInputFamily::Lexical,
      NODE_LEXICAL_INPUT_VERSION,
    )
  }

  fn input(
    &self,
    node_id: &CanonicalId,
    family: EmbeddingInputFamily,
    version: &'static str,
  ) -> CanonicalEmbeddingInput {
    CanonicalEmbeddingInput::from_fields(family, version, |out| {
      out.field("release", self.lexeme.release_id.as_str());
      out.field("node_id", node_id.as_str());
      out.field("lexeme_id", self.lexeme.id.as_str());
      out.field("language", self.lexeme.language.as_str());
      out.field("part_of_speech", pos_name(self.lexeme.part_of_speech));
      out.field("lemma", &self.lexeme.lemma);
      out.field("normalized_lemma", &self.lexeme.normalized_lemma);
      out.optional("sense_id", self.sense.as_ref().map(|item| item.id.as_str()));
      out.optional(
        "sense_key",
        self.sense.as_ref().map(|item| item.sense_key.as_str()),
      );
      out.optional(
        "definition",
        self.sense.as_ref().map(|item| item.definition.as_str()),
      );
      out.list(
        "form_ids",
        self.forms.iter().map(|item| item.id.as_str().to_string()),
      );
      out.list(
        "form_kinds",
        self
          .forms
          .iter()
          .map(|item| form_kind(item.kind).to_string()),
      );
      out.list(
        "form_values",
        self.forms.iter().map(|item| item.form.clone()),
      );
      out.list(
        "form_normalized_values",
        self.forms.iter().map(|item| item.normalized_form.clone()),
      );
      out.optional_list(
        "form_morphology",
        self.forms.iter().map(|item| item.morphology.as_deref()),
      );
      out.list(
        "gloss_ids",
        self
          .glosses
          .iter()
          .map(|item| item.id().as_str().to_string()),
      );
      out.list(
        "gloss_languages",
        self
          .glosses
          .iter()
          .map(|item| item.language().as_str().to_string()),
      );
      out.list(
        "gloss_values",
        self
          .glosses
          .iter()
          .map(|item| item.assertion().text().to_string()),
      );
      out.records(
        "translations",
        self.translations.iter().map(|item| {
          let mut record = CanonicalWriter::new();
          record.field("id", item.id().as_str());
          record.field("revision", &item.revision().get().to_string());
          record.field("unit", translation_unit(item.unit()));
          record.field("source_language", item.source_language().as_str());
          record.field("source_text", item.source_text());
          record.field("target_language", item.target_language().as_str());
          record.field("target_text", item.target_text());
          if let Some(scope) = item.meaning_scope() {
            record.optional("scope_lexeme_id", Some(scope.lexeme_id().as_str()));
            record.optional("scope_sense_id", Some(scope.sense_id().as_str()));
            record.optional("scope_composition", Some(composition(scope.composition())));
            record.list(
              "scope_domain_ids",
              scope.domain_ids().iter().map(|id| id.as_str().to_string()),
            );
          }
          record.finish()
        }),
      );
      out.list(
        "evidence_ids",
        self
          .evidence
          .iter()
          .map(|item| item.fragment().id.as_str().to_string()),
      );
      out.list(
        "evidence_source_ids",
        self
          .evidence
          .iter()
          .map(|item| item.source().id.as_str().to_string()),
      );
      out.list(
        "evidence_content_hashes",
        self
          .evidence
          .iter()
          .map(|item| item.fragment().content_hash.clone()),
      );
    })
  }
}

/// Closed validation failures for authoritative embedding material.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum EmbeddingMaterialError {
  /// The canonical lemma has no dedicated evidence.
  #[error("canonical lemma evidence is missing")]
  MissingLemmaEvidence,
  /// A bounded list is oversized, empty where required, duplicated, or unordered.
  #[error("embedding material references are invalid")]
  InvalidReferences,
  /// Too much lexical material was supplied.
  #[error("embedding material exceeds its bound")]
  TooManyItems,
  /// A record is not active and publication eligible.
  #[error("embedding material is not publication eligible")]
  IneligibleRecord,
  /// A record belongs to another release, lexeme, or sense.
  #[error("embedding material ownership is inconsistent")]
  OwnershipMismatch,
  /// An evidence reference is absent or unreferenced.
  #[error("embedding evidence references are incomplete")]
  DanglingEvidence,
  /// Evidence does not permit embedding.
  #[error("embedding evidence is not permitted")]
  EvidenceNotPermitted,
  /// A translation is passage-scoped or belongs to another meaning.
  #[error("canonical translation scope is incompatible")]
  TranslationScopeMismatch,
}

fn validate_ids(ids: &[EvidenceId]) -> Result<(), EmbeddingMaterialError> {
  if ids.is_empty()
    || ids.len() > MAX_EVIDENCE_PER_ASSERTION
    || ids.windows(2).any(|pair| pair[0] >= pair[1])
  {
    Err(EmbeddingMaterialError::InvalidReferences)
  } else {
    Ok(())
  }
}
fn ensure_unique<T: Ord>(items: impl IntoIterator<Item = T>) -> Result<(), EmbeddingMaterialError> {
  let mut seen = BTreeSet::new();
  if items.into_iter().all(|item| seen.insert(item)) {
    Ok(())
  } else {
    Err(EmbeddingMaterialError::InvalidReferences)
  }
}
fn resolve(
  ids: &[EvidenceId],
  lineages: &BTreeMap<EvidenceId, &CanonicalEvidenceLineage>,
  release: &ReleaseId,
  used: &mut BTreeSet<EvidenceId>,
) -> Result<(), EmbeddingMaterialError> {
  for id in ids {
    let lineage = lineages
      .get(id)
      .ok_or(EmbeddingMaterialError::DanglingEvidence)?;
    if !lineage.permits(release, EvidenceUse::Embedding) {
      return Err(EmbeddingMaterialError::EvidenceNotPermitted);
    }
    used.insert(id.clone());
  }
  Ok(())
}

pub(crate) struct CanonicalWriter(Vec<u8>);
impl CanonicalWriter {
  pub(crate) fn new() -> Self {
    Self(Vec::new())
  }
  pub(crate) fn field(&mut self, tag: &str, value: &str) {
    self.bytes(tag.as_bytes());
    self.bytes(value.nfc().collect::<String>().as_bytes());
  }
  pub(crate) fn optional(&mut self, tag: &str, value: Option<&str>) {
    self.bytes(tag.as_bytes());
    match value {
      Some(value) => {
        self.0.push(1);
        self.bytes(value.nfc().collect::<String>().as_bytes());
      }
      None => self.0.push(0),
    }
  }
  pub(crate) fn list(&mut self, tag: &str, values: impl IntoIterator<Item = String>) {
    let values = values.into_iter().collect::<Vec<_>>();
    self.bytes(tag.as_bytes());
    self
      .0
      .extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
      self.bytes(value.nfc().collect::<String>().as_bytes());
    }
  }
  pub(crate) fn optional_list<'a>(
    &mut self,
    tag: &str,
    values: impl IntoIterator<Item = Option<&'a str>>,
  ) {
    let values = values.into_iter().collect::<Vec<_>>();
    self.bytes(tag.as_bytes());
    self
      .0
      .extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
      match value {
        Some(value) => {
          self.0.push(1);
          self.bytes(value.nfc().collect::<String>().as_bytes());
        }
        None => self.0.push(0),
      }
    }
  }
  pub(crate) fn records(&mut self, tag: &str, values: impl IntoIterator<Item = Vec<u8>>) {
    let values = values.into_iter().collect::<Vec<_>>();
    self.bytes(tag.as_bytes());
    self
      .0
      .extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
      self.bytes(&value);
    }
  }
  fn bytes(&mut self, value: &[u8]) {
    self
      .0
      .extend_from_slice(&(value.len() as u64).to_be_bytes());
    self.0.extend_from_slice(value);
  }
  pub(crate) fn finish(self) -> Vec<u8> {
    self.0
  }
}

fn pos_name(value: super::canonical::LexicalPartOfSpeech) -> &'static str {
  use super::canonical::LexicalPartOfSpeech::*;
  match value {
    Noun => "noun",
    Verb => "verb",
    Adjective => "adjective",
    Adverb => "adverb",
    Pronoun => "pronoun",
    Preposition => "preposition",
    Conjunction => "conjunction",
    Determiner => "determiner",
    Interjection => "interjection",
    Numeral => "numeral",
    Other => "other",
  }
}
fn form_kind(value: super::canonical::FormKind) -> &'static str {
  use super::canonical::FormKind::*;
  match value {
    Lemma => "lemma",
    SpellingVariant => "spelling_variant",
    Inflection => "inflection",
    Phrase => "phrase",
    Alias => "alias",
  }
}
fn translation_unit(value: CanonicalTranslationUnit) -> &'static str {
  match value {
    CanonicalTranslationUnit::Word => "word",
    CanonicalTranslationUnit::Phrase => "phrase",
    CanonicalTranslationUnit::Passage => "passage",
  }
}
fn composition(value: super::canonical_translation::MeaningComposition) -> &'static str {
  match value {
    super::canonical_translation::MeaningComposition::Compositional => "compositional",
    super::canonical_translation::MeaningComposition::PhraseLevel => "phrase_level",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::{
    EvidenceConfidence, EvidenceFragment, EvidenceKind, LanguageTag, LexicalPartOfSpeech,
    LexicalSource, SourcePermissions,
  };
  use crate::domain::canonical_content::CanonicalEvidenceOrigin;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }
  fn permissions(embedding: bool) -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding,
      model_processing: false,
      api_redistribution: true,
    }
  }
  fn lexeme(lemma: &str, evidence: Vec<EvidenceId>) -> Lexeme {
    Lexeme {
      id: id("lexeme-technical"),
      release_id: id("release-1"),
      language: LanguageTag::parse("en").unwrap(),
      lemma: lemma.into(),
      lemma_evidence_ids: evidence,
      normalized_lemma: lemma.to_lowercase(),
      part_of_speech: LexicalPartOfSpeech::Noun,
      status: CanonicalStatus::Active,
    }
  }
  fn lineage(evidence: &str, embedding: bool) -> CanonicalEvidenceLineage {
    let source_id = id("source-1");
    CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed".into(),
        version: "1".into(),
        license: "reviewed".into(),
        attribution: Some("Reviewed".into()),
        permissions: permissions(embedding),
      },
      EvidenceFragment {
        id: id(evidence),
        source_id,
        source_reference: "entry".into(),
        release_id: id("release-1"),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Other,
        confidence: EvidenceConfidence::High,
        text: "C++".into(),
        content_hash: "sha256:reviewed".into(),
        permissions: permissions(embedding),
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap()
  }

  #[test]
  fn lemma_evidence_is_required_sorted_resolved_and_embedding_permitted() {
    assert_eq!(
      AuthoritativeEmbeddingMaterial::new(
        lexeme("C++", vec![]),
        None,
        vec![],
        vec![],
        vec![],
        vec![]
      ),
      Err(EmbeddingMaterialError::MissingLemmaEvidence)
    );
    assert_eq!(
      AuthoritativeEmbeddingMaterial::new(
        lexeme("C++", vec![id("evidence-1")]),
        None,
        vec![],
        vec![],
        vec![],
        vec![]
      ),
      Err(EmbeddingMaterialError::DanglingEvidence)
    );
    assert_eq!(
      AuthoritativeEmbeddingMaterial::new(
        lexeme("C++", vec![id("evidence-1")]),
        None,
        vec![],
        vec![],
        vec![],
        vec![lineage("evidence-1", false)]
      ),
      Err(EmbeddingMaterialError::EvidenceNotPermitted)
    );
  }

  #[test]
  fn node_inputs_are_nfc_length_prefixed_family_separated_and_symbol_preserving() {
    let material = AuthoritativeEmbeddingMaterial::new(
      lexeme("C++", vec![id("evidence-1")]),
      None,
      vec![],
      vec![],
      vec![],
      vec![lineage("evidence-1", true)],
    )
    .unwrap();
    let dense = material.dense_input(&id("lexeme-technical"));
    let lexical = material.lexical_input(&id("lexeme-technical"));
    assert_eq!(dense.specification_version(), NODE_DENSE_INPUT_VERSION);
    assert_eq!(lexical.specification_version(), NODE_LEXICAL_INPUT_VERSION);
    assert_ne!(dense.input_hash(), lexical.input_hash());
    assert!(dense
      .canonical_bytes()
      .windows(3)
      .any(|window| window == b"C++"));
    assert!(!format!("{dense:?}").contains("C++"));
  }

  #[test]
  fn evidence_changes_do_not_change_lexeme_identity_but_change_input_hash() {
    let first = AuthoritativeEmbeddingMaterial::new(
      lexeme("C#", vec![id("evidence-1")]),
      None,
      vec![],
      vec![],
      vec![],
      vec![lineage("evidence-1", true)],
    )
    .unwrap();
    let second = AuthoritativeEmbeddingMaterial::new(
      lexeme("C#", vec![id("evidence-2")]),
      None,
      vec![],
      vec![],
      vec![],
      vec![lineage("evidence-2", true)],
    )
    .unwrap();
    assert_eq!(first.lexeme().id, second.lexeme().id);
    assert_ne!(
      first.dense_input(&first.lexeme().id).input_hash(),
      second.dense_input(&second.lexeme().id).input_hash()
    );
  }
}
