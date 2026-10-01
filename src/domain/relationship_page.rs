//! Validated relationship-page supersets and deterministic progressive disclosure.

use std::collections::BTreeSet;

use thiserror::Error;

use super::{
  assertion::{CanonicalNodeFamily, CanonicalNodeId},
  canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
  canonical_translation::DomainId,
  domain_assessment::{DomainAssessment, DomainCoverageState},
  knowledge_hydration::{HydratedAssertionProjection, HydratedSemanticScale},
  knowledge_view::UsefulRootPath,
  translation_turn::ResponseLevel,
};

/// Relationship-page schema version.
pub const RELATIONSHIP_PAGE_SCHEMA_VERSION: &str = "relationship-page-v1";
/// Deterministic grouping and ranking policy version.
pub const RELATIONSHIP_PAGE_POLICY_VERSION: &str = "relationship-page-policy-v1";
/// Progressive-disclosure projector version.
pub const RELATIONSHIP_PAGE_PROJECTION_VERSION: &str = "relationship-page-projection-v1";
/// Lexical router contract version.
pub const RELATIONSHIP_PAGE_ROUTER_VERSION: &str = "relationship-page-router-v1";
/// Canonical root resolver contract version.
pub const RELATIONSHIP_PAGE_RESOLVER_VERSION: &str = "relationship-page-resolver-v1";
/// Domain assessor contract version.
pub const RELATIONSHIP_PAGE_DOMAIN_ASSESSOR_VERSION: &str = "relationship-page-domain-v1";
/// Usefulness ranker contract version.
pub const RELATIONSHIP_PAGE_RANKER_VERSION: &str = "relationship-page-ranker-v1";
/// Structured composer contract version.
pub const RELATIONSHIP_PAGE_COMPOSER_VERSION: &str = "relationship-page-composer-v1";
/// Composer prompt contract version.
pub const RELATIONSHIP_PAGE_PROMPT_VERSION: &str = "relationship-page-prompt-v1";
/// Bounded structured-output repair policy version.
pub const RELATIONSHIP_PAGE_REPAIR_POLICY_VERSION: &str = "relationship-page-repair-v1";
/// Maximum relationship groups in one validated superset.
pub const MAX_PAGE_GROUPS: usize = 16;
/// Maximum relationships in one group.
pub const MAX_GROUP_RELATIONSHIPS: usize = 16;
/// Maximum request-local generated examples.
pub const MAX_GENERATED_EXAMPLES: usize = 4;
/// Maximum labeled alternatives requested for one lexical unit.
pub const MAX_LABELED_ALTERNATIVES: usize = 2;

/// Request for a relationship page embedded in one lexical translation result.
#[derive(Clone, Eq, PartialEq)]
pub struct RelationshipPageRequest {
  /// Resolved word or established-phrase root.
  pub root: CanonicalNodeId,
  /// Language used for reviewed and generated presentation.
  pub target_language: LanguageTag,
  /// Requested progressive-disclosure breadth.
  pub response_level: ResponseLevel,
  /// Exact immutable canonical release and schema.
  pub release: CanonicalReleasePin,
  /// Explicit caller limit for useful labeled alternatives.
  pub max_alternatives: u8,
}

/// Stable reference to the authoritative leading summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationshipPageSummaryRef {
  /// Existing canonical BasicCard rooted at one lexical sense.
  BasicCard { sense_id: CanonicalId },
  /// Existing canonical multilingual concept summary.
  Concept { concept: CanonicalNodeId },
}

/// One release-pinned domain profile selected for the page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageDomainProfile {
  /// Stable canonical domain identity.
  pub domain_id: DomainId,
  /// Closed fact families present in the release.
  pub available_fact_families: Vec<String>,
  /// Languages represented by eligible reviewed facts.
  pub languages: Vec<LanguageTag>,
  /// Count of verified facts in the release profile.
  pub verified_fact_count: u64,
  /// Coverage state that does not imply completeness.
  pub coverage: DomainCoverageState,
}

/// Request-local assessment and the exact profiles used by page composition.
#[derive(Clone, PartialEq, Eq)]
pub struct PageDomainContext {
  /// Closed assessment result.
  pub assessment: DomainAssessment,
  /// Strictly ordered profiles for selected canonical domains.
  pub profiles: Vec<PageDomainProfile>,
}

/// One exact authoritative atomic fact retained for page references.
#[derive(Clone, Eq, PartialEq)]
pub struct PageFact {
  /// Exact hydrated assertion, registry, and traversal proof.
  pub projection: HydratedAssertionProjection,
  /// Full release pin under which the fact was hydrated.
  pub release: CanonicalReleasePin,
}

/// One complete semantic scale preserved without pairwise reconstruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageSemanticScale {
  /// Exact authoritative scale.
  pub scale: HydratedSemanticScale,
  /// Full release pin under which the scale was hydrated.
  pub release: CanonicalReleasePin,
}

/// Closed relationship-page group catalog in deterministic usefulness order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationshipGroupKind {
  /// Definition, equivalence, and core meaning.
  Meaning,
  /// Preferred terms, translations, aliases, symbols, and abbreviations.
  Terminology,
  /// Taxonomy and complete semantic degree scales.
  TaxonomyOrDegree,
  /// Explicit contrasts and reviewed confusions.
  Contrast,
  /// Argument structure and construction behavior.
  Valency,
  /// Reviewed collocations.
  Collocation,
  /// Register, region, period, scene, and domain suitability.
  Suitability,
  /// Inflection and derivation.
  Morphology,
  /// Idiom, metaphor, and cultural extension.
  CulturalExtension,
  /// Explicit technical mechanism and dependency.
  Mechanism,
  /// Neighboring evidence-backed phenomena.
  Phenomenon,
  /// Reviewed applications and implementations.
  Application,
  /// Quantities, equations, instruments, and measurements.
  Measurement,
  /// Standards and conventions.
  Standard,
  /// Professional and disciplinary usage.
  UsageConvention,
}

impl RelationshipGroupKind {
  /// Returns the frozen usefulness rank; lower values appear first.
  pub const fn rank(self) -> u8 {
    match self {
      Self::Meaning => 1,
      Self::Terminology => 2,
      Self::TaxonomyOrDegree => 3,
      Self::Contrast => 4,
      Self::Valency => 5,
      Self::Collocation => 6,
      Self::Suitability => 7,
      Self::Morphology => 8,
      Self::CulturalExtension => 9,
      Self::Mechanism => 10,
      Self::Phenomenon => 11,
      Self::Application => 12,
      Self::Measurement => 13,
      Self::Standard => 14,
      Self::UsageConvention => 15,
    }
  }
}

/// One verified relationship displayed through an exact fact and useful root path.
#[derive(Clone, Eq, PartialEq)]
pub struct PageRelationship {
  /// Related canonical node.
  pub node: CanonicalNodeId,
  /// Exact assertion identity from [`RelationshipPageSuperset::facts`].
  pub assertion_id: CanonicalId,
  /// Explicit evidence-backed path to the page root.
  pub path_to_root: UsefulRootPath,
  /// Deterministic bounded usefulness score; larger values rank first.
  pub usefulness: u16,
}

/// One non-empty deterministic relationship group.
#[derive(Clone, Eq, PartialEq)]
pub struct PageRelationshipGroup {
  /// Closed group kind.
  pub kind: RelationshipGroupKind,
  /// Relationships ordered by usefulness, then canonical identity.
  pub relationships: Vec<PageRelationship>,
}

/// One optional named verified short path used to explain a non-obvious connection.
#[derive(Clone, Eq, PartialEq)]
pub struct PageNamedPath {
  /// Concise display label for the connection.
  pub label: String,
  /// Exact evidence-backed root path.
  pub path: UsefulRootPath,
}

/// One explicitly model-generated request-local example.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageGeneratedExample {
  /// Source-language example.
  pub source_text: String,
  /// Target-language rendering.
  pub translated_text: String,
  /// Always true in the online projection.
  pub generated: bool,
}

/// Evidence-grounded synthesis that remains distinct from a canonical fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageInferredExplanation {
  /// Concise request-local explanation.
  pub text: String,
  /// Strictly ordered canonical assertions supporting the synthesis.
  pub supporting_assertion_ids: Vec<CanonicalId>,
}

/// Similarity or model nomination kept outside factual sections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageExploratoryItem {
  /// Nominated canonical node when one hydrated identity exists.
  pub node: CanonicalNodeId,
  /// Concise reason it may be useful, never a factual assertion.
  pub reason: String,
}

/// Closed dimension changed by a labeled alternative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlternativeDimension {
  /// Strength or degree.
  Degree,
  /// Formality.
  Formality,
  /// Approval or stance.
  Approval,
  /// Risk or danger implication.
  Danger,
  /// Specialist domain.
  Domain,
  /// Syntax or construction.
  Syntax,
  /// Register or scene.
  Register,
  /// Material lexical meaning.
  Meaning,
}

/// One explicitly requested useful alternative, never a canonical alias claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabeledAlternative {
  /// Zero-based lexical translation unit index.
  pub unit_index: u16,
  /// Alternative target-language expression.
  pub text: String,
  /// Dimension changed relative to the primary translation.
  pub dimension: AlternativeDimension,
  /// Practical consequence of choosing this expression.
  pub consequence: String,
  /// Deterministic reason the alternative is materially useful.
  pub usefulness_reason: String,
}

/// Request-local missing-relationship nomination reserved for offline review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipGapProposal {
  /// Proposed source endpoint.
  pub source: CanonicalNodeId,
  /// Proposed target endpoint.
  pub target: CanonicalNodeId,
  /// Existing registry relation identity; no edge identity is assigned.
  pub relation_type_id: CanonicalId,
  /// Concise rationale for offline reviewers.
  pub rationale: String,
  /// Strictly ordered candidate evidence identities.
  pub candidate_evidence_ids: Vec<CanonicalId>,
}

/// Versions of every deterministic relationship-page contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipPageVersionMetadata {
  /// Result schema.
  pub schema_version: &'static str,
  /// Grouping/ranking policy.
  pub policy_version: &'static str,
  /// Progressive-disclosure projection.
  pub projection_version: &'static str,
  /// Lexical word/phrase router policy.
  pub router_version: &'static str,
  /// Canonical root resolver policy.
  pub resolver_version: &'static str,
  /// Domain assessment policy.
  pub domain_assessor_version: &'static str,
  /// Usefulness ranker policy.
  pub ranker_version: &'static str,
  /// Structured page composer policy.
  pub composer_version: &'static str,
  /// Model prompt contract.
  pub prompt_version: &'static str,
  /// Invalid-output repair policy.
  pub repair_policy_version: &'static str,
}

impl Default for RelationshipPageVersionMetadata {
  fn default() -> Self {
    Self {
      schema_version: RELATIONSHIP_PAGE_SCHEMA_VERSION,
      policy_version: RELATIONSHIP_PAGE_POLICY_VERSION,
      projection_version: RELATIONSHIP_PAGE_PROJECTION_VERSION,
      router_version: RELATIONSHIP_PAGE_ROUTER_VERSION,
      resolver_version: RELATIONSHIP_PAGE_RESOLVER_VERSION,
      domain_assessor_version: RELATIONSHIP_PAGE_DOMAIN_ASSESSOR_VERSION,
      ranker_version: RELATIONSHIP_PAGE_RANKER_VERSION,
      composer_version: RELATIONSHIP_PAGE_COMPOSER_VERSION,
      prompt_version: RELATIONSHIP_PAGE_PROMPT_VERSION,
      repair_policy_version: RELATIONSHIP_PAGE_REPAIR_POLICY_VERSION,
    }
  }
}

/// Complete validated request-local relationship-page material before response projection.
#[derive(Clone)]
pub struct RelationshipPageSuperset {
  /// Owning lexical request and immutable pin.
  pub request: RelationshipPageRequest,
  /// Leading authoritative summary reference.
  pub summary: RelationshipPageSummaryRef,
  /// Domain assessment and release coverage.
  pub domain: PageDomainContext,
  /// Exact authoritative atomic facts.
  pub facts: Vec<PageFact>,
  /// Complete eligible semantic scales.
  pub scales: Vec<PageSemanticScale>,
  /// Supported direct groups.
  pub groups: Vec<PageRelationshipGroup>,
  /// Explicit optional short paths not already used as direct-item paths.
  pub paths: Vec<PageNamedPath>,
  /// Clearly labeled generated examples.
  pub generated_examples: Vec<PageGeneratedExample>,
  /// Clearly separated inferred explanations.
  pub inferred_explanations: Vec<PageInferredExplanation>,
  /// Clearly separated exploratory nominations.
  pub exploratory_items: Vec<PageExploratoryItem>,
  /// Explicitly requested labeled alternatives.
  pub alternatives: Vec<LabeledAlternative>,
  /// Offline-only proposals, deliberately excluded from online projection.
  pub gap_proposals: Vec<RelationshipGapProposal>,
  /// Version metadata.
  pub versions: RelationshipPageVersionMetadata,
}

/// Online projection; it deliberately has no gap-proposal field.
#[derive(Clone)]
pub struct ProjectedRelationshipPage {
  /// Authoritative leading summary reference.
  pub summary: RelationshipPageSummaryRef,
  /// Domain context admitted at this level.
  pub domain: Option<PageDomainContext>,
  /// Deterministically selected supported groups.
  pub groups: Vec<PageRelationshipGroup>,
  /// Optional independently useful named paths.
  pub paths: Vec<PageNamedPath>,
  /// Complete scales referenced by admitted groups.
  pub scales: Vec<PageSemanticScale>,
  /// Labeled request-local generated examples.
  pub generated_examples: Vec<PageGeneratedExample>,
  /// Labeled evidence-grounded synthesis.
  pub inferred_explanations: Vec<PageInferredExplanation>,
  /// Separate exploratory section.
  pub exploratory_items: Vec<PageExploratoryItem>,
  /// Useful explicitly requested alternatives.
  pub alternatives: Vec<LabeledAlternative>,
  /// Exact release pin.
  pub release: CanonicalReleasePin,
  /// Version metadata.
  pub versions: RelationshipPageVersionMetadata,
}

/// Closed validation failure without request content.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RelationshipPageValidationError {
  /// A bounded collection or prose field is invalid.
  #[error("relationship page value is invalid")]
  InvalidValue,
  /// A canonical identity is duplicated, missing, or unresolved.
  #[error("relationship page reference is inconsistent")]
  InconsistentReference,
  /// Material crosses the immutable request release.
  #[error("relationship page release is inconsistent")]
  ReleaseMismatch,
  /// Deterministic ordering is invalid.
  #[error("relationship page ordering is invalid")]
  InvalidOrdering,
}

impl RelationshipPageSuperset {
  /// Validates exact facts, scale preservation, grouping, labels, bounds, and ordering.
  pub fn validate(&self) -> Result<(), RelationshipPageValidationError> {
    if self.request.max_alternatives as usize > MAX_LABELED_ALTERNATIVES
      || self.groups.len() > MAX_PAGE_GROUPS
      || self.generated_examples.len() > MAX_GENERATED_EXAMPLES
      || self.alternatives.len() > 16
    {
      return Err(RelationshipPageValidationError::InvalidValue);
    }
    validate_summary(&self.request, &self.summary)?;
    validate_domain(&self.domain)?;
    let fact_ids = validate_facts(&self.request.release, &self.facts)?;
    validate_scales(&self.request, &self.scales)?;
    validate_groups(&self.request.root, &self.groups, &fact_ids)?;
    validate_paths(&self.request, &self.paths)?;
    validate_generated(&self.generated_examples)?;
    validate_inferred(&self.inferred_explanations, &fact_ids)?;
    validate_exploratory(&self.exploratory_items)?;
    validate_alternatives(&self.alternatives, self.request.max_alternatives as usize)?;
    validate_gaps(&self.gap_proposals)?;
    if self.versions != RelationshipPageVersionMetadata::default() {
      return Err(RelationshipPageValidationError::InvalidValue);
    }
    Ok(())
  }

  /// Projects progressive breadth from the same validated superset.
  pub fn project(&self) -> Result<ProjectedRelationshipPage, RelationshipPageValidationError> {
    self.validate()?;
    let (group_limit, path_limit, example_limit, inferred_limit, exploratory_limit, alt_limit) =
      match self.request.response_level {
        ResponseLevel::Brief => (2, 0, 0, 0, 0, 0),
        ResponseLevel::Standard => (6, 1, 1, 1, 0, 1),
        ResponseLevel::Full => (
          MAX_PAGE_GROUPS,
          3,
          MAX_GENERATED_EXAMPLES,
          4,
          8,
          MAX_LABELED_ALTERNATIVES,
        ),
      };
    Ok(ProjectedRelationshipPage {
      summary: self.summary.clone(),
      domain: if self.request.response_level == ResponseLevel::Full
        || !self.domain.assessment.is_proposed_new()
      {
        Some(self.domain.clone())
      } else {
        None
      },
      groups: self.groups.iter().take(group_limit).cloned().collect(),
      paths: self.paths.iter().take(path_limit).cloned().collect(),
      scales: if self.request.response_level == ResponseLevel::Brief {
        Vec::new()
      } else {
        self.scales.clone()
      },
      generated_examples: self
        .generated_examples
        .iter()
        .take(example_limit)
        .cloned()
        .collect(),
      inferred_explanations: self
        .inferred_explanations
        .iter()
        .take(inferred_limit)
        .cloned()
        .collect(),
      exploratory_items: self
        .exploratory_items
        .iter()
        .take(exploratory_limit)
        .cloned()
        .collect(),
      alternatives: project_alternatives(&self.alternatives, alt_limit),
      release: self.request.release.clone(),
      versions: self.versions.clone(),
    })
  }
}

fn validate_summary(
  request: &RelationshipPageRequest,
  summary: &RelationshipPageSummaryRef,
) -> Result<(), RelationshipPageValidationError> {
  match summary {
    RelationshipPageSummaryRef::BasicCard { sense_id } => {
      if request.root.family() != CanonicalNodeFamily::LexicalSense || request.root.id() != sense_id
      {
        return Err(RelationshipPageValidationError::InconsistentReference);
      }
    }
    RelationshipPageSummaryRef::Concept { concept } => {
      if concept.family() != CanonicalNodeFamily::Concept || &request.root != concept {
        return Err(RelationshipPageValidationError::InconsistentReference);
      }
    }
  }
  Ok(())
}

fn validate_domain(value: &PageDomainContext) -> Result<(), RelationshipPageValidationError> {
  let values = &value.profiles;
  if values.len() > 32
    || values
      .windows(2)
      .any(|pair| pair[0].domain_id >= pair[1].domain_id)
    || values.iter().any(|profile| {
      profile.available_fact_families.len() > 16
        || profile.languages.len() > 8
        || profile
          .available_fact_families
          .windows(2)
          .any(|pair| pair[0] >= pair[1])
        || profile.languages.windows(2).any(|pair| pair[0] >= pair[1])
    })
  {
    return Err(RelationshipPageValidationError::InvalidOrdering);
  }
  match value.assessment.existing_domain_ids() {
    Some(ids)
      if ids.len() == values.len()
        && ids
          .iter()
          .zip(values)
          .all(|(id, profile)| id == &profile.domain_id) => {}
    Some(_) => return Err(RelationshipPageValidationError::InconsistentReference),
    None if !values.is_empty() => {
      return Err(RelationshipPageValidationError::InconsistentReference);
    }
    None => {}
  }
  Ok(())
}

fn validate_facts(
  release: &CanonicalReleasePin,
  values: &[PageFact],
) -> Result<BTreeSet<CanonicalId>, RelationshipPageValidationError> {
  let mut ids = BTreeSet::new();
  for fact in values {
    if &fact.release != release || fact.projection.assertion().release_id != release.release_id {
      return Err(RelationshipPageValidationError::ReleaseMismatch);
    }
    if !ids.insert(fact.projection.assertion().assertion_id.clone()) {
      return Err(RelationshipPageValidationError::InconsistentReference);
    }
  }
  Ok(ids)
}

fn validate_scales(
  request: &RelationshipPageRequest,
  values: &[PageSemanticScale],
) -> Result<(), RelationshipPageValidationError> {
  let mut ids = BTreeSet::new();
  for value in values {
    if value.release != request.release {
      return Err(RelationshipPageValidationError::ReleaseMismatch);
    }
    value
      .scale
      .validate()
      .map_err(|_| RelationshipPageValidationError::InvalidValue)?;
    if !value
      .scale
      .members
      .iter()
      .any(|member| &member.node_id == request.root.id())
      || !ids.insert(value.scale.scale_id.clone())
    {
      return Err(RelationshipPageValidationError::InconsistentReference);
    }
  }
  Ok(())
}

fn validate_groups(
  root: &CanonicalNodeId,
  values: &[PageRelationshipGroup],
  facts: &BTreeSet<CanonicalId>,
) -> Result<(), RelationshipPageValidationError> {
  if values.windows(2).any(|pair| pair[0].kind >= pair[1].kind) {
    return Err(RelationshipPageValidationError::InvalidOrdering);
  }
  for group in values {
    if group.relationships.is_empty() || group.relationships.len() > MAX_GROUP_RELATIONSHIPS {
      return Err(RelationshipPageValidationError::InvalidValue);
    }
    let mut nodes = BTreeSet::new();
    for (index, relationship) in group.relationships.iter().enumerate() {
      if !facts.contains(&relationship.assertion_id)
        || !nodes.insert(relationship.node.clone())
        || relationship.path_to_root.item != relationship.node
        || &relationship.path_to_root.root != root
        || relationship.path_to_root.validate().is_err()
      {
        return Err(RelationshipPageValidationError::InconsistentReference);
      }
      if index > 0 {
        let previous = &group.relationships[index - 1];
        if previous.usefulness < relationship.usefulness
          || (previous.usefulness == relationship.usefulness && previous.node >= relationship.node)
        {
          return Err(RelationshipPageValidationError::InvalidOrdering);
        }
      }
    }
  }
  Ok(())
}

fn validate_paths(
  request: &RelationshipPageRequest,
  values: &[PageNamedPath],
) -> Result<(), RelationshipPageValidationError> {
  if values.len() > 3
    || values.iter().any(|value| {
      !valid_text(&value.label, 128)
        || value.path.root != request.root
        || value.path.validate().is_err()
    })
  {
    return Err(RelationshipPageValidationError::InconsistentReference);
  }
  Ok(())
}

fn validate_generated(
  values: &[PageGeneratedExample],
) -> Result<(), RelationshipPageValidationError> {
  if values.iter().any(|value| {
    !value.generated
      || !valid_text(&value.source_text, 512)
      || !valid_text(&value.translated_text, 512)
  }) {
    return Err(RelationshipPageValidationError::InvalidValue);
  }
  Ok(())
}

fn validate_inferred(
  values: &[PageInferredExplanation],
  facts: &BTreeSet<CanonicalId>,
) -> Result<(), RelationshipPageValidationError> {
  if values.len() > 4
    || values.iter().any(|value| {
      !valid_text(&value.text, 1_024)
        || value.supporting_assertion_ids.is_empty()
        || value
          .supporting_assertion_ids
          .windows(2)
          .any(|pair| pair[0] >= pair[1])
        || value
          .supporting_assertion_ids
          .iter()
          .any(|id| !facts.contains(id))
    })
  {
    return Err(RelationshipPageValidationError::InconsistentReference);
  }
  Ok(())
}

fn validate_exploratory(
  values: &[PageExploratoryItem],
) -> Result<(), RelationshipPageValidationError> {
  if values.len() > 8
    || values.iter().any(|value| !valid_text(&value.reason, 512))
    || values.windows(2).any(|pair| pair[0].node >= pair[1].node)
  {
    return Err(RelationshipPageValidationError::InvalidValue);
  }
  Ok(())
}

fn validate_alternatives(
  values: &[LabeledAlternative],
  per_unit_limit: usize,
) -> Result<(), RelationshipPageValidationError> {
  let mut units = BTreeSet::new();
  let mut counts = std::collections::BTreeMap::new();
  if values.iter().any(|value| {
    let count = counts.entry(value.unit_index).or_insert(0_usize);
    *count += 1;
    !valid_text(&value.text, 512)
      || !valid_text(&value.consequence, 512)
      || !valid_text(&value.usefulness_reason, 512)
      || !units.insert((value.unit_index, value.text.clone()))
      || *count > per_unit_limit
  }) {
    return Err(RelationshipPageValidationError::InvalidValue);
  }
  Ok(())
}

fn project_alternatives(
  values: &[LabeledAlternative],
  per_unit_limit: usize,
) -> Vec<LabeledAlternative> {
  let mut counts = std::collections::BTreeMap::new();
  values
    .iter()
    .filter(|value| {
      let count = counts.entry(value.unit_index).or_insert(0_usize);
      if *count == per_unit_limit {
        return false;
      }
      *count += 1;
      true
    })
    .cloned()
    .collect()
}

fn validate_gaps(
  values: &[RelationshipGapProposal],
) -> Result<(), RelationshipPageValidationError> {
  if values.len() > 8
    || values.iter().any(|value| {
      value.source == value.target
        || !valid_text(&value.rationale, 1_024)
        || value.candidate_evidence_ids.is_empty()
        || value
          .candidate_evidence_ids
          .windows(2)
          .any(|pair| pair[0] >= pair[1])
    })
  {
    return Err(RelationshipPageValidationError::InvalidValue);
  }
  Ok(())
}

fn valid_text(value: &str, max_chars: usize) -> bool {
  value.trim() == value && !value.is_empty() && value.chars().count() <= max_chars
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{
    assertion::CanonicalNodeFamily,
    knowledge_hydration::{HydratedScaleMember, SemanticScaleDirection},
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn node(value: &str) -> CanonicalNodeId {
    CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id(value))
  }

  fn pin() -> CanonicalReleasePin {
    CanonicalReleasePin::new(id("release-1"), "canonical-v1".into()).unwrap()
  }

  fn superset(level: ResponseLevel) -> RelationshipPageSuperset {
    let root = node("root");
    let related = node("related");
    let release = pin();
    let projection = HydratedAssertionProjection::topology_fixture(
      related.clone(),
      root.clone(),
      id("edge-1"),
      id("assertion-1"),
      release.release_id.clone(),
    );
    let step = crate::domain::knowledge_view::VerifiedKnowledgeStep::from_hydrated(
      projection.clone(),
      release.clone(),
    )
    .unwrap();
    let path = UsefulRootPath {
      root: root.clone(),
      item: related.clone(),
      steps: vec![step],
    };
    RelationshipPageSuperset {
      request: RelationshipPageRequest {
        root: root.clone(),
        target_language: LanguageTag::parse("en").unwrap(),
        response_level: level,
        release: release.clone(),
        max_alternatives: 2,
      },
      summary: RelationshipPageSummaryRef::Concept {
        concept: root.clone(),
      },
      domain: PageDomainContext {
        assessment: DomainAssessment::Existing {
          domain_ids: vec![DomainId::new("domain_1").unwrap()],
          reason: "Reviewed domain match.".into(),
        },
        profiles: vec![PageDomainProfile {
          domain_id: DomainId::new("domain_1").unwrap(),
          available_fact_families: vec!["definition".into()],
          languages: vec![LanguageTag::parse("en").unwrap()],
          verified_fact_count: 1,
          coverage: DomainCoverageState::Seed,
        }],
      },
      facts: vec![PageFact {
        projection,
        release: release.clone(),
      }],
      scales: vec![PageSemanticScale {
        scale: HydratedSemanticScale {
          scale_id: id("scale-1"),
          revision: 1,
          dimension: "intensity".into(),
          direction: SemanticScaleDirection::Increasing,
          domain_ids: Vec::new(),
          conditions: Vec::new(),
          members: vec![
            HydratedScaleMember {
              node_id: id("root"),
              position: 1,
            },
            HydratedScaleMember {
              node_id: id("related"),
              position: 2,
            },
          ],
          evidence_ids: vec![id("evidence-1")],
        },
        release: release.clone(),
      }],
      groups: vec![PageRelationshipGroup {
        kind: RelationshipGroupKind::Meaning,
        relationships: vec![PageRelationship {
          node: related.clone(),
          assertion_id: id("assertion-1"),
          path_to_root: path.clone(),
          usefulness: 100,
        }],
      }],
      paths: vec![PageNamedPath {
        label: "How the concepts connect".into(),
        path,
      }],
      generated_examples: vec![PageGeneratedExample {
        source_text: "Example.".into(),
        translated_text: "示例。".into(),
        generated: true,
      }],
      inferred_explanations: vec![PageInferredExplanation {
        text: "Grounded synthesis.".into(),
        supporting_assertion_ids: vec![id("assertion-1")],
      }],
      exploratory_items: vec![PageExploratoryItem {
        node: node("possible"),
        reason: "Similarity nomination only.".into(),
      }],
      alternatives: vec![
        LabeledAlternative {
          unit_index: 0,
          text: "alternative one".into(),
          dimension: AlternativeDimension::Register,
          consequence: "More formal.".into(),
          usefulness_reason: "The requested audience is professional.".into(),
        },
        LabeledAlternative {
          unit_index: 0,
          text: "alternative two".into(),
          dimension: AlternativeDimension::Degree,
          consequence: "Stronger intensity.".into(),
          usefulness_reason: "The source is materially ambiguous in degree.".into(),
        },
      ],
      gap_proposals: vec![RelationshipGapProposal {
        source: root,
        target: related,
        relation_type_id: id("relation-1"),
        rationale: "Expected reviewed relationship is absent.".into(),
        candidate_evidence_ids: vec![id("evidence-2")],
      }],
      versions: RelationshipPageVersionMetadata::default(),
    }
  }

  #[test]
  fn projections_are_monotonic_and_never_expose_gap_proposals() {
    let brief = superset(ResponseLevel::Brief).project().unwrap();
    let standard = superset(ResponseLevel::Standard).project().unwrap();
    let full = superset(ResponseLevel::Full).project().unwrap();
    assert!(brief.generated_examples.is_empty());
    assert!(brief.alternatives.is_empty());
    assert_eq!(standard.generated_examples.len(), 1);
    assert_eq!(standard.alternatives.len(), 1);
    assert_eq!(full.alternatives.len(), 2);
    assert!(brief.groups.len() <= standard.groups.len());
    assert!(standard.groups.len() <= full.groups.len());
  }

  #[test]
  fn alternatives_require_explicit_bound_and_material_explanation() {
    let mut page = superset(ResponseLevel::Full);
    page.request.max_alternatives = 1;
    assert_eq!(
      page.validate(),
      Err(RelationshipPageValidationError::InvalidValue)
    );
    page.alternatives.truncate(1);
    page.alternatives[0].consequence.clear();
    assert_eq!(
      page.validate(),
      Err(RelationshipPageValidationError::InvalidValue)
    );
  }

  #[test]
  fn facts_scales_and_inference_must_resolve_under_one_pin() {
    let mut page = superset(ResponseLevel::Full);
    page.facts[0].release =
      CanonicalReleasePin::new(id("release-2"), "canonical-v1".into()).unwrap();
    assert_eq!(
      page.validate(),
      Err(RelationshipPageValidationError::ReleaseMismatch)
    );
    let mut page = superset(ResponseLevel::Full);
    page.inferred_explanations[0].supporting_assertion_ids = vec![id("unknown")];
    assert_eq!(
      page.validate(),
      Err(RelationshipPageValidationError::InconsistentReference)
    );
  }

  #[test]
  fn proposed_domains_are_full_only_and_never_claim_canonical_profiles() {
    let proposal = super::super::domain_assessment::ProposedDomain::new(
      super::super::domain_assessment::LocalizedDomainTerm::new(
        LanguageTag::parse("en").unwrap(),
        "Proposed scope",
      )
      .unwrap(),
      "A request-local candidate scope.",
      Vec::new(),
      "No reviewed canonical scope fits.",
    )
    .unwrap();
    let mut page = superset(ResponseLevel::Standard);
    page.domain = PageDomainContext {
      assessment: DomainAssessment::ProposedNew(proposal.clone()),
      profiles: Vec::new(),
    };
    assert!(page.project().unwrap().domain.is_none());

    page.request.response_level = ResponseLevel::Full;
    assert!(page.project().unwrap().domain.is_some());

    page.domain.profiles.push(PageDomainProfile {
      domain_id: DomainId::new("domain_1").unwrap(),
      available_fact_families: Vec::new(),
      languages: Vec::new(),
      verified_fact_count: 0,
      coverage: DomainCoverageState::Seed,
    });
    assert_eq!(
      page.validate(),
      Err(RelationshipPageValidationError::InconsistentReference)
    );
  }
}
