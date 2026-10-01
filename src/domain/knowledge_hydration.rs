//! Authoritative, release-pinned values hydrated after retrieval nomination.

use std::collections::BTreeSet;

use thiserror::Error;

use super::{
  canonical::{CanonicalId, LanguageTag},
  canonical_translation::DomainId,
  retrieval_data::{RetrievalNodeType, RetrievalRelation, RELATION_REGISTRY_VERSION},
};

/// Maximum exact records accepted by one canonical hydration operation.
pub const MAX_KNOWLEDGE_HYDRATION_ITEMS: usize = 50;
/// Maximum structured conditions attached to one fact or scale.
pub const MAX_KNOWLEDGE_CONDITIONS: usize = 16;
/// Maximum identifiers accepted in one bounded support list.
pub const MAX_KNOWLEDGE_SUPPORT_IDS: usize = 32;
/// Maximum scalar length of one authoritative statement.
pub const MAX_KNOWLEDGE_STATEMENT_CHARS: usize = 4_096;

/// Closed validation failures for authoritative knowledge hydration.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgeHydrationValidationError {
  /// An item count or a nested collection exceeded its contract bound.
  #[error("knowledge hydration collection is invalid")]
  InvalidCollection,
  /// A required revision or ordinal was not positive.
  #[error("knowledge hydration revision or position is invalid")]
  InvalidRevision,
  /// Text was blank, untrimmed, or exceeded its scalar bound.
  #[error("knowledge hydration text is invalid")]
  InvalidText,
  /// A fact used an unknown registry version or malformed topology.
  #[error("knowledge hydration relation is invalid")]
  InvalidRelation,
  /// A support list was empty, duplicated, or not strictly ordered.
  #[error("knowledge hydration support is invalid")]
  InvalidSupport,
}

/// Exact immutable fact revision requested after a retrieval nomination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalFactRef {
  /// Publisher-owned stable fact identity.
  pub fact_id: CanonicalId,
  /// Positive immutable fact revision.
  pub revision: u32,
}

/// Structured applicability condition owned by canonical data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeCondition {
  /// Stable canonical condition identity.
  pub condition_id: CanonicalId,
  /// Closed publisher-defined condition family.
  pub condition_type: String,
  /// Strictly ordered canonical parameter identities.
  pub parameter_ids: Vec<CanonicalId>,
}

impl KnowledgeCondition {
  /// Validates the bounded condition name and its ordered parameters.
  pub fn validate(&self) -> Result<(), KnowledgeHydrationValidationError> {
    if !valid_token(&self.condition_type, 64)
      || self.parameter_ids.is_empty()
      || self.parameter_ids.len() > MAX_KNOWLEDGE_SUPPORT_IDS
      || !strictly_ordered(&self.parameter_ids)
    {
      return Err(KnowledgeHydrationValidationError::InvalidSupport);
    }
    Ok(())
  }
}

/// One authoritative atomic fact and its exact evidence references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeFact {
  /// Publisher-owned stable fact identity.
  pub fact_id: CanonicalId,
  /// Positive immutable fact revision.
  pub revision: u32,
  /// Canonical statement, never generated retrieval prose.
  pub statement: String,
  /// Canonical subject endpoint.
  pub subject_node_id: CanonicalId,
  /// Exact frozen relation.
  pub predicate: RetrievalRelation,
  /// Exact frozen relationship-registry version.
  pub relation_registry_version: u32,
  /// Canonical object endpoint.
  pub object_node_id: CanonicalId,
  /// Strictly ordered canonical domain scope.
  pub domain_ids: Vec<DomainId>,
  /// Strictly ordered applicable sense identities.
  pub applicable_sense_ids: Vec<CanonicalId>,
  /// Bounded structured applicability conditions.
  pub conditions: Vec<KnowledgeCondition>,
  /// Strictly ordered evidence identities eligible for this read.
  pub evidence_ids: Vec<CanonicalId>,
  /// Strictly ordered canonical source identities.
  pub provenance: Vec<CanonicalId>,
}

impl KnowledgeFact {
  /// Validates revision, topology, text, scopes, and evidence support.
  pub fn validate(&self) -> Result<(), KnowledgeHydrationValidationError> {
    if self.revision == 0 {
      return Err(KnowledgeHydrationValidationError::InvalidRevision);
    }
    if self.subject_node_id == self.object_node_id
      || self.relation_registry_version != RELATION_REGISTRY_VERSION
    {
      return Err(KnowledgeHydrationValidationError::InvalidRelation);
    }
    if !valid_text(&self.statement, MAX_KNOWLEDGE_STATEMENT_CHARS) {
      return Err(KnowledgeHydrationValidationError::InvalidText);
    }
    validate_optional_ids(&self.domain_ids)?;
    validate_optional_ids(&self.applicable_sense_ids)?;
    validate_required_ids(&self.evidence_ids)?;
    validate_required_ids(&self.provenance)?;
    if self.conditions.len() > MAX_KNOWLEDGE_CONDITIONS {
      return Err(KnowledgeHydrationValidationError::InvalidCollection);
    }
    for condition in &self.conditions {
      condition.validate()?;
    }
    if self
      .conditions
      .windows(2)
      .any(|pair| pair[0].condition_id >= pair[1].condition_id)
    {
      return Err(KnowledgeHydrationValidationError::InvalidSupport);
    }
    Ok(())
  }
}

/// Direction in which an authoritative semantic scale increases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticScaleDirection {
  /// Later positions represent more of the named dimension.
  Increasing,
  /// Later positions represent less of the named dimension.
  Decreasing,
}

/// One explicit member of a complete semantic scale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedScaleMember {
  /// Canonical node identity.
  pub node_id: CanonicalId,
  /// Positive ordinal position; it is not a numeric intensity interval.
  pub position: u32,
}

/// One complete authoritative semantic scale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedSemanticScale {
  /// Stable canonical scale identity.
  pub scale_id: CanonicalId,
  /// Positive immutable scale revision.
  pub revision: u32,
  /// Bounded machine-readable dimension.
  pub dimension: String,
  /// Direction in which member positions progress.
  pub direction: SemanticScaleDirection,
  /// Strictly ordered canonical domain scope.
  pub domain_ids: Vec<DomainId>,
  /// Bounded structured applicability conditions.
  pub conditions: Vec<KnowledgeCondition>,
  /// Complete members in strictly increasing position order.
  pub members: Vec<HydratedScaleMember>,
  /// Strictly ordered evidence identities supporting the complete scale.
  pub evidence_ids: Vec<CanonicalId>,
}

impl HydratedSemanticScale {
  /// Validates a complete, ordered, evidence-backed scale.
  pub fn validate(&self) -> Result<(), KnowledgeHydrationValidationError> {
    if self.revision == 0 || self.members.iter().any(|member| member.position == 0) {
      return Err(KnowledgeHydrationValidationError::InvalidRevision);
    }
    if !valid_token(&self.dimension, 128) {
      return Err(KnowledgeHydrationValidationError::InvalidText);
    }
    validate_optional_ids(&self.domain_ids)?;
    validate_required_ids(&self.evidence_ids)?;
    if self.conditions.len() > MAX_KNOWLEDGE_CONDITIONS
      || self.members.len() < 2
      || self.members.len() > MAX_KNOWLEDGE_SUPPORT_IDS
    {
      return Err(KnowledgeHydrationValidationError::InvalidCollection);
    }
    for condition in &self.conditions {
      condition.validate()?;
    }
    if self
      .conditions
      .windows(2)
      .any(|pair| pair[0].condition_id >= pair[1].condition_id)
    {
      return Err(KnowledgeHydrationValidationError::InvalidSupport);
    }
    if self
      .members
      .windows(2)
      .any(|pair| pair[0].position >= pair[1].position)
      || self
        .members
        .iter()
        .map(|member| &member.node_id)
        .collect::<BTreeSet<_>>()
        .len()
        != self.members.len()
    {
      return Err(KnowledgeHydrationValidationError::InvalidSupport);
    }
    Ok(())
  }
}

/// Authoritative display-safe values for one nominated knowledge node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HydratedKnowledgeNode {
  /// Stable canonical node identity.
  pub node_id: CanonicalId,
  /// Positive immutable node revision.
  pub revision: u32,
  /// Closed node family.
  pub node_type: RetrievalNodeType,
  /// Canonical sense identity for lexical-sense nodes and no other family.
  pub sense_id: Option<CanonicalId>,
  /// Concise reviewed label.
  pub canonical_label: String,
  /// Optional canonical language for language-bearing nodes.
  pub language: Option<LanguageTag>,
  /// Strictly ordered canonical domain memberships.
  pub domain_ids: Vec<DomainId>,
  /// Strictly ordered evidence identities supporting the returned values.
  pub evidence_ids: Vec<CanonicalId>,
}

impl HydratedKnowledgeNode {
  /// Validates revision, label, domain, and evidence bounds.
  pub fn validate(&self) -> Result<(), KnowledgeHydrationValidationError> {
    if self.revision == 0 {
      return Err(KnowledgeHydrationValidationError::InvalidRevision);
    }
    if !valid_text(&self.canonical_label, 512) {
      return Err(KnowledgeHydrationValidationError::InvalidText);
    }
    if (self.node_type == RetrievalNodeType::LexicalSense) != self.sense_id.is_some() {
      return Err(KnowledgeHydrationValidationError::InvalidRelation);
    }
    validate_optional_ids(&self.domain_ids)?;
    validate_required_ids(&self.evidence_ids)
  }
}

fn validate_required_ids<T: Ord>(values: &[T]) -> Result<(), KnowledgeHydrationValidationError> {
  if values.is_empty() {
    return Err(KnowledgeHydrationValidationError::InvalidSupport);
  }
  validate_optional_ids(values)
}

fn validate_optional_ids<T: Ord>(values: &[T]) -> Result<(), KnowledgeHydrationValidationError> {
  if values.len() > MAX_KNOWLEDGE_SUPPORT_IDS || !strictly_ordered(values) {
    return Err(KnowledgeHydrationValidationError::InvalidSupport);
  }
  Ok(())
}

fn strictly_ordered<T: Ord>(values: &[T]) -> bool {
  values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_text(value: &str, max_chars: usize) -> bool {
  value.trim() == value && !value.is_empty() && value.chars().count() <= max_chars
}

fn valid_token(value: &str, max_chars: usize) -> bool {
  valid_text(value, max_chars)
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
  use super::*;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  #[test]
  fn fact_requires_exact_registry_positive_revision_and_ordered_support() {
    let mut fact = KnowledgeFact {
      fact_id: id("fact-1"),
      revision: 1,
      statement: "A is related to B.".into(),
      subject_node_id: id("node-a"),
      predicate: RetrievalRelation::from_wire_name("associated_with").unwrap(),
      relation_registry_version: RELATION_REGISTRY_VERSION,
      object_node_id: id("node-b"),
      domain_ids: vec![],
      applicable_sense_ids: vec![],
      conditions: vec![],
      evidence_ids: vec![id("evidence-1")],
      provenance: vec![id("source-1")],
    };
    assert_eq!(fact.validate(), Ok(()));
    fact.relation_registry_version += 1;
    assert_eq!(
      fact.validate(),
      Err(KnowledgeHydrationValidationError::InvalidRelation)
    );
  }

  #[test]
  fn scale_requires_complete_strictly_ordered_members() {
    let scale = HydratedSemanticScale {
      scale_id: id("scale-1"),
      revision: 1,
      dimension: "heat_intensity".into(),
      direction: SemanticScaleDirection::Increasing,
      domain_ids: vec![],
      conditions: vec![],
      members: vec![
        HydratedScaleMember {
          node_id: id("node-a"),
          position: 10,
        },
        HydratedScaleMember {
          node_id: id("node-b"),
          position: 20,
        },
      ],
      evidence_ids: vec![id("evidence-1")],
    };
    assert_eq!(scale.validate(), Ok(()));
  }

  #[test]
  fn knowledge_node_sense_identity_matches_its_family() {
    let mut node = HydratedKnowledgeNode {
      node_id: id("node-sense"),
      revision: 1,
      node_type: RetrievalNodeType::LexicalSense,
      sense_id: Some(id("sense-1")),
      canonical_label: "heat".into(),
      language: Some(LanguageTag::parse("en").unwrap()),
      domain_ids: vec![],
      evidence_ids: vec![id("evidence-1")],
    };
    assert_eq!(node.validate(), Ok(()));
    node.sense_id = None;
    assert_eq!(
      node.validate(),
      Err(KnowledgeHydrationValidationError::InvalidRelation)
    );
    node.node_type = RetrievalNodeType::Concept;
    node.sense_id = Some(id("sense-1"));
    assert_eq!(
      node.validate(),
      Err(KnowledgeHydrationValidationError::InvalidRelation)
    );
  }
}
