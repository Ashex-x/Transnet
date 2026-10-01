//! Bounded canonical-domain inventory and request-local assessment values.

use thiserror::Error;

use super::{
  canonical::{LanguageTag, ReleaseId},
  canonical_translation::{CanonicalRevision, DomainId},
};

/// Maximum canonical domains returned for one assessment.
pub const MAX_DOMAIN_INVENTORY: usize = 32;
/// Maximum localized labels on one canonical domain.
pub const MAX_DOMAIN_LABELS: usize = 8;
/// Maximum localized aliases on one canonical domain.
pub const MAX_DOMAIN_ALIASES: usize = 16;
/// Maximum entries in one inclusion or exclusion scope.
pub const MAX_DOMAIN_SCOPE_ITEMS: usize = 16;
/// Maximum broader-domain references on one domain or proposal.
pub const MAX_BROADER_DOMAINS: usize = 8;
/// Maximum fact-family declarations in one knowledge profile.
pub const MAX_DOMAIN_FACT_FAMILIES: usize = 16;
/// Maximum language declarations in one knowledge profile.
pub const MAX_DOMAIN_LANGUAGES: usize = 8;
/// Maximum Unicode scalar count for a label or alias.
pub const MAX_DOMAIN_TERM_CHARS: usize = 128;
/// Maximum Unicode scalar count for explanatory domain prose.
pub const MAX_DOMAIN_PROSE_CHARS: usize = 1_024;
/// Maximum verified facts representable in a bounded profile.
pub const MAX_VERIFIED_DOMAIN_FACTS: u64 = 10_000_000;

/// Closed validation failure for canonical domain-assessment data.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum DomainAssessmentError {
  /// A value was blank, oversized, duplicated, or otherwise malformed.
  #[error("domain assessment value is invalid")]
  InvalidValue,
  /// A collection exceeded its fixed request or response bound.
  #[error("domain assessment collection exceeds its bound")]
  TooManyValues,
  /// Authority data contradicted the pinned release or its own references.
  #[error("domain inventory is inconsistent")]
  InconsistentInventory,
  /// A model nomination selected an identity outside the supplied allowlist.
  #[error("domain nomination is outside the supplied allowlist")]
  InvalidNomination,
}

/// One bounded multilingual canonical label or alias.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalizedDomainTerm {
  language: LanguageTag,
  text: String,
}

/// One bounded multilingual canonical domain definition.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalizedDomainDefinition {
  language: LanguageTag,
  text: String,
}

impl LocalizedDomainDefinition {
  /// Creates a nonblank definition associated with a canonical language tag.
  ///
  /// # Errors
  ///
  /// Returns [`DomainAssessmentError::InvalidValue`] for blank or oversized prose.
  pub fn new(
    language: LanguageTag,
    text: impl Into<String>,
  ) -> Result<Self, DomainAssessmentError> {
    let text = bounded_text(text.into(), MAX_DOMAIN_PROSE_CHARS)?;
    Ok(Self { language, text })
  }

  /// Returns the definition language.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the concise reviewed definition.
  pub fn text(&self) -> &str {
    &self.text
  }
}

impl LocalizedDomainTerm {
  /// Creates a nonblank NFC-preserving term associated with a canonical language tag.
  ///
  /// # Errors
  ///
  /// Returns [`DomainAssessmentError::InvalidValue`] for blank or oversized text.
  pub fn new(
    language: LanguageTag,
    text: impl Into<String>,
  ) -> Result<Self, DomainAssessmentError> {
    let text = bounded_text(text.into(), MAX_DOMAIN_TERM_CHARS)?;
    Ok(Self { language, text })
  }

  /// Returns the term language.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the reviewed label or alias.
  pub fn text(&self) -> &str {
    &self.text
  }
}

/// Closed release-pinned knowledge coverage state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainCoverageState {
  /// A small reviewed seed is available.
  Seed,
  /// Multiple useful families are present but coverage remains partial.
  Partial,
  /// The release contains a deliberately curated profile.
  Curated,
}

/// Release summary of canonical knowledge available for one domain.
#[derive(Clone, PartialEq, Eq)]
pub struct DomainKnowledgeProfile {
  available_fact_families: Vec<String>,
  languages: Vec<LanguageTag>,
  verified_fact_count: u64,
  coverage_state: DomainCoverageState,
}

impl DomainKnowledgeProfile {
  /// Creates a sorted, unique bounded knowledge profile.
  ///
  /// # Errors
  ///
  /// Returns a closed validation error for invalid, duplicate, or oversized values.
  pub fn new(
    available_fact_families: Vec<String>,
    languages: Vec<LanguageTag>,
    verified_fact_count: u64,
    coverage_state: DomainCoverageState,
  ) -> Result<Self, DomainAssessmentError> {
    if available_fact_families.len() > MAX_DOMAIN_FACT_FAMILIES
      || languages.len() > MAX_DOMAIN_LANGUAGES
    {
      return Err(DomainAssessmentError::TooManyValues);
    }
    let available_fact_families = validate_sorted_texts(available_fact_families, 64)?;
    ensure_sorted_unique(&languages)?;
    if verified_fact_count > MAX_VERIFIED_DOMAIN_FACTS {
      return Err(DomainAssessmentError::InvalidValue);
    }
    Ok(Self {
      available_fact_families,
      languages,
      verified_fact_count,
      coverage_state,
    })
  }

  /// Returns closed fact-family names available in the pinned release.
  pub fn available_fact_families(&self) -> &[String] {
    &self.available_fact_families
  }

  /// Returns languages represented by eligible canonical facts.
  pub fn languages(&self) -> &[LanguageTag] {
    &self.languages
  }

  /// Returns the number of verified facts represented by the profile.
  pub const fn verified_fact_count(&self) -> u64 {
    self.verified_fact_count
  }

  /// Returns the declared coverage state without implying completeness.
  pub const fn coverage_state(&self) -> DomainCoverageState {
    self.coverage_state
  }
}

/// One immutable canonical domain revision in a release-pinned allowlist.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalDomain {
  release_id: ReleaseId,
  domain_id: DomainId,
  revision: CanonicalRevision,
  labels: Vec<LocalizedDomainTerm>,
  aliases: Vec<LocalizedDomainTerm>,
  definitions: Vec<LocalizedDomainDefinition>,
  inclusion_scope: Vec<String>,
  exclusion_scope: Vec<String>,
  broader_domain_ids: Vec<DomainId>,
  knowledge_profile: DomainKnowledgeProfile,
}

impl CanonicalDomain {
  /// Creates a complete bounded domain record owned by one immutable release.
  ///
  /// # Errors
  ///
  /// Returns a closed validation error for missing labels, duplicate values, invalid prose, or
  /// out-of-bound collections.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    release_id: ReleaseId,
    domain_id: DomainId,
    revision: CanonicalRevision,
    labels: Vec<LocalizedDomainTerm>,
    aliases: Vec<LocalizedDomainTerm>,
    definitions: Vec<LocalizedDomainDefinition>,
    inclusion_scope: Vec<String>,
    exclusion_scope: Vec<String>,
    broader_domain_ids: Vec<DomainId>,
    knowledge_profile: DomainKnowledgeProfile,
  ) -> Result<Self, DomainAssessmentError> {
    if labels.is_empty()
      || labels.len() > MAX_DOMAIN_LABELS
      || aliases.len() > MAX_DOMAIN_ALIASES
      || definitions.is_empty()
      || definitions.len() > MAX_DOMAIN_LABELS
      || inclusion_scope.len() > MAX_DOMAIN_SCOPE_ITEMS
      || exclusion_scope.len() > MAX_DOMAIN_SCOPE_ITEMS
      || broader_domain_ids.len() > MAX_BROADER_DOMAINS
    {
      return Err(DomainAssessmentError::TooManyValues);
    }
    ensure_terms_sorted_unique(&labels)?;
    ensure_terms_sorted_unique(&aliases)?;
    ensure_definitions_sorted_unique(&definitions)?;
    let inclusion_scope = validate_sorted_texts(inclusion_scope, MAX_DOMAIN_TERM_CHARS)?;
    let exclusion_scope = validate_sorted_texts(exclusion_scope, MAX_DOMAIN_TERM_CHARS)?;
    ensure_sorted_unique(&broader_domain_ids)?;
    if broader_domain_ids.contains(&domain_id) {
      return Err(DomainAssessmentError::InconsistentInventory);
    }
    Ok(Self {
      release_id,
      domain_id,
      revision,
      labels,
      aliases,
      definitions,
      inclusion_scope,
      exclusion_scope,
      broader_domain_ids,
      knowledge_profile,
    })
  }

  /// Returns the immutable release owning this revision.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release_id
  }

  /// Returns the stable canonical domain identity.
  pub fn domain_id(&self) -> &DomainId {
    &self.domain_id
  }

  /// Returns the immutable revision sequence.
  pub const fn revision(&self) -> CanonicalRevision {
    self.revision
  }

  /// Returns multilingual reviewed labels.
  pub fn labels(&self) -> &[LocalizedDomainTerm] {
    &self.labels
  }

  /// Returns multilingual reviewed aliases.
  pub fn aliases(&self) -> &[LocalizedDomainTerm] {
    &self.aliases
  }

  /// Returns concise multilingual reviewed definitions.
  pub fn definitions(&self) -> &[LocalizedDomainDefinition] {
    &self.definitions
  }

  /// Returns the bounded inclusion-scope phrases.
  pub fn inclusion_scope(&self) -> &[String] {
    &self.inclusion_scope
  }

  /// Returns the bounded exclusion-scope phrases.
  pub fn exclusion_scope(&self) -> &[String] {
    &self.exclusion_scope
  }

  /// Returns canonical broader-domain candidates in the same release.
  pub fn broader_domain_ids(&self) -> &[DomainId] {
    &self.broader_domain_ids
  }

  /// Returns release-pinned RAG coverage metadata.
  pub const fn knowledge_profile(&self) -> &DomainKnowledgeProfile {
    &self.knowledge_profile
  }
}

/// Successful bounded authority result for one domain query.
#[derive(Clone, PartialEq, Eq)]
pub struct DomainInventory {
  domains: Vec<CanonicalDomain>,
  catalog_complete: bool,
}

impl DomainInventory {
  /// Creates an ordered inventory and validates stable identity uniqueness.
  ///
  /// # Errors
  ///
  /// Returns a closed error for an oversized inventory or duplicate domain identity.
  pub fn new(
    domains: Vec<CanonicalDomain>,
    catalog_complete: bool,
  ) -> Result<Self, DomainAssessmentError> {
    if domains.len() > MAX_DOMAIN_INVENTORY {
      return Err(DomainAssessmentError::TooManyValues);
    }
    if domains
      .windows(2)
      .any(|pair| pair[0].domain_id() >= pair[1].domain_id())
    {
      return Err(DomainAssessmentError::InconsistentInventory);
    }
    Ok(Self {
      domains,
      catalog_complete,
    })
  }

  /// Returns ordered canonical candidates supplied to assessment.
  pub fn domains(&self) -> &[CanonicalDomain] {
    &self.domains
  }

  /// Reports whether the authority examined the complete eligible catalog for this query.
  pub const fn catalog_complete(&self) -> bool {
    self.catalog_complete
  }
}

/// Structured request-local proposal emitted only when no complete-catalog scope fits.
#[derive(Clone, PartialEq, Eq)]
pub struct ProposedDomain {
  label: LocalizedDomainTerm,
  definition: String,
  broader_domain_ids: Vec<DomainId>,
  reason_existing_scopes_do_not_fit: String,
}

impl ProposedDomain {
  /// Creates a bounded proposal without assigning a stable identity.
  ///
  /// # Errors
  ///
  /// Returns a closed validation error for invalid prose or broader-domain references.
  pub fn new(
    label: LocalizedDomainTerm,
    definition: impl Into<String>,
    broader_domain_ids: Vec<DomainId>,
    reason_existing_scopes_do_not_fit: impl Into<String>,
  ) -> Result<Self, DomainAssessmentError> {
    if broader_domain_ids.len() > MAX_BROADER_DOMAINS {
      return Err(DomainAssessmentError::TooManyValues);
    }
    ensure_sorted_unique(&broader_domain_ids)?;
    Ok(Self {
      label,
      definition: bounded_text(definition.into(), MAX_DOMAIN_PROSE_CHARS)?,
      broader_domain_ids,
      reason_existing_scopes_do_not_fit: bounded_text(
        reason_existing_scopes_do_not_fit.into(),
        MAX_DOMAIN_PROSE_CHARS,
      )?,
    })
  }

  /// Returns the request-local proposed label.
  pub const fn label(&self) -> &LocalizedDomainTerm {
    &self.label
  }

  /// Returns the concise proposed definition.
  pub fn definition(&self) -> &str {
    &self.definition
  }

  /// Returns broader-domain candidates selected only from the supplied allowlist.
  pub fn broader_domain_ids(&self) -> &[DomainId] {
    &self.broader_domain_ids
  }

  /// Returns why none of the supplied canonical scopes fit.
  pub fn reason_existing_scopes_do_not_fit(&self) -> &str {
    &self.reason_existing_scopes_do_not_fit
  }
}

/// Closed request-local model nomination awaiting deterministic allowlist validation.
#[derive(Clone, PartialEq, Eq)]
pub enum DomainNomination {
  /// Selects one or more existing canonical IDs.
  Existing {
    /// Stable IDs selected from the supplied allowlist.
    domain_ids: Vec<DomainId>,
    /// Concise request-local reason.
    reason: String,
  },
  /// Proposes a request-local scope only when the complete catalog has no fit.
  ProposedNew(ProposedDomain),
  /// Classifies the resolved meaning as general rather than specialist.
  General {
    /// Concise request-local reason.
    reason: String,
  },
  /// Declares that available evidence does not support a confident classification.
  Uncertain {
    /// Concise request-local reason.
    reason: String,
  },
}

/// Validated closed outcome of one domain assessment.
#[derive(Clone, PartialEq, Eq)]
pub enum DomainAssessment {
  /// One or more supplied canonical domains apply.
  Existing {
    /// Stable canonical IDs in deterministic order.
    domain_ids: Vec<DomainId>,
    /// Concise request-local reason.
    reason: String,
  },
  /// A complete catalog had no fit; the proposal remains request-local.
  ProposedNew(ProposedDomain),
  /// The resolved meaning is general rather than specialist.
  General {
    /// Concise request-local reason.
    reason: String,
  },
  /// Dependencies or evidence did not support another outcome.
  Uncertain,
}

fn bounded_text(value: String, max_chars: usize) -> Result<String, DomainAssessmentError> {
  if value.trim() != value || value.is_empty() || value.chars().count() > max_chars {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(value)
}

fn validate_sorted_texts(
  values: Vec<String>,
  max_chars: usize,
) -> Result<Vec<String>, DomainAssessmentError> {
  let values = values
    .into_iter()
    .map(|value| bounded_text(value, max_chars))
    .collect::<Result<Vec<_>, _>>()?;
  ensure_sorted_unique(&values)?;
  Ok(values)
}

fn ensure_sorted_unique<T: Ord>(values: &[T]) -> Result<(), DomainAssessmentError> {
  if values.windows(2).any(|pair| pair[0] >= pair[1]) {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

fn ensure_terms_sorted_unique(values: &[LocalizedDomainTerm]) -> Result<(), DomainAssessmentError> {
  if values
    .windows(2)
    .any(|pair| (pair[0].language(), pair[0].text()) >= (pair[1].language(), pair[1].text()))
  {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

fn ensure_definitions_sorted_unique(
  values: &[LocalizedDomainDefinition],
) -> Result<(), DomainAssessmentError> {
  if values
    .windows(2)
    .any(|pair| (pair[0].language(), pair[0].text()) >= (pair[1].language(), pair[1].text()))
  {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::CanonicalId;

  fn language(value: &str) -> LanguageTag {
    LanguageTag::parse(value).unwrap()
  }

  fn term(language_tag: &str, text: &str) -> LocalizedDomainTerm {
    LocalizedDomainTerm::new(language(language_tag), text).unwrap()
  }

  fn definition(language_tag: &str, text: &str) -> LocalizedDomainDefinition {
    LocalizedDomainDefinition::new(language(language_tag), text).unwrap()
  }

  fn profile() -> DomainKnowledgeProfile {
    DomainKnowledgeProfile::new(
      vec!["definition".to_string()],
      vec![language("en")],
      1,
      DomainCoverageState::Seed,
    )
    .unwrap()
  }

  fn domain_with_lists(
    id: &str,
    labels: Vec<LocalizedDomainTerm>,
    aliases: Vec<LocalizedDomainTerm>,
    definitions: Vec<LocalizedDomainDefinition>,
  ) -> Result<CanonicalDomain, DomainAssessmentError> {
    CanonicalDomain::new(
      CanonicalId::new("release-1").unwrap(),
      DomainId::new(id).unwrap(),
      CanonicalRevision::new(1).unwrap(),
      labels,
      aliases,
      definitions,
      vec![],
      vec![],
      vec![],
      profile(),
    )
  }

  fn domain(id: &str) -> CanonicalDomain {
    domain_with_lists(
      id,
      vec![term("en", id)],
      vec![],
      vec![definition("en", "A definition.")],
    )
    .unwrap()
  }

  #[test]
  fn localized_catalog_fields_require_strict_language_then_text_order() {
    let cases = [
      domain_with_lists(
        "domain_test",
        vec![term("en", "zeta"), term("en", "alpha")],
        vec![],
        vec![definition("en", "A definition.")],
      ),
      domain_with_lists(
        "domain_test",
        vec![term("en", "label")],
        vec![term("zh-CN", "别名"), term("en", "alias")],
        vec![definition("en", "A definition.")],
      ),
      domain_with_lists(
        "domain_test",
        vec![term("en", "label")],
        vec![],
        vec![
          definition("zh-CN", "定义。"),
          definition("en", "A definition."),
        ],
      ),
      domain_with_lists(
        "domain_test",
        vec![term("en", "label"), term("en", "label")],
        vec![],
        vec![definition("en", "A definition.")],
      ),
    ];
    for result in cases {
      assert!(matches!(result, Err(DomainAssessmentError::InvalidValue)));
    }
  }

  #[test]
  fn inventory_candidates_require_strict_domain_id_order() {
    let reversed = DomainInventory::new(vec![domain("domain_zeta"), domain("domain_alpha")], true);
    assert!(matches!(
      reversed,
      Err(DomainAssessmentError::InconsistentInventory)
    ));

    let duplicated =
      DomainInventory::new(vec![domain("domain_alpha"), domain("domain_alpha")], true);
    assert!(matches!(
      duplicated,
      Err(DomainAssessmentError::InconsistentInventory)
    ));
  }
}
