//! Authoritative, release-pinned values hydrated after retrieval nomination.

use std::{collections::BTreeSet, fmt};

use thiserror::Error;

use super::{
  assertion::{AssertionRegistryEntry, CanonicalAssertion, CanonicalNodeId},
  canonical::{CanonicalId, EvidenceUse, LanguageTag, ReleaseId},
  canonical_translation::DomainId,
  graph::GraphRelationType,
  retrieval_data::RetrievalNodeType,
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

/// Exact immutable assertion projection requested after a retrieval nomination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalAssertionProjectionRef {
  /// Publisher-owned stable edge identity.
  pub edge_id: CanonicalId,
  /// Positive immutable relationship revision.
  pub relationship_revision: u32,
  /// Publisher-owned stable assertion identity.
  pub assertion_id: CanonicalId,
  /// Positive immutable assertion revision.
  pub assertion_revision: u32,
  /// Explicit registry traversal selected by the projection.
  pub traversal_id: CanonicalId,
  /// Canonical source node identity.
  pub source_node_id: CanonicalId,
  /// Canonical target node identity.
  pub target_node_id: CanonicalId,
  /// Exact typed relation selected by the traversal.
  pub relation_type: GraphRelationType,
  /// Exact positive registry revision.
  pub relation_registry_revision: u32,
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

/// Independently returned binary traversal proof validated against one canonical assertion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedBinaryTraversal {
  /// Publisher-owned stable edge identity.
  pub edge_id: CanonicalId,
  /// Positive immutable relationship revision.
  pub relationship_revision: u32,
  /// Assertion identity independently echoed by the projection record.
  pub assertion_id: CanonicalId,
  /// Assertion revision independently echoed by the projection record.
  pub assertion_revision: u32,
  /// Explicit registry traversal selected by the projection.
  pub traversal_id: CanonicalId,
  /// Typed source endpoint resolved from its participant role.
  pub source: CanonicalNodeId,
  /// Typed target endpoint resolved from its participant role.
  pub target: CanonicalNodeId,
  /// Exact relation declared by the selected traversal.
  pub relation_type: GraphRelationType,
  /// Exact registry revision echoed by the projection.
  pub relation_registry_revision: u32,
}

/// Sole validated read projection used by future knowledge views and path steps.
#[derive(Clone, Eq, PartialEq)]
pub struct HydratedAssertionProjection {
  assertion: CanonicalAssertion,
  registry: AssertionRegistryEntry,
  traversal: SelectedBinaryTraversal,
}

impl fmt::Debug for HydratedAssertionProjection {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("HydratedAssertionProjection(REDACTED)")
  }
}

impl HydratedAssertionProjection {
  /// Constructs a read projection only when assertion, registry, traversal, and nomination agree.
  ///
  /// # Errors
  ///
  /// Returns an error for invalid evidence permission, release, registry, role, endpoint, relation,
  /// identity, revision, or independently echoed projection proof.
  pub fn new(
    release_id: &ReleaseId,
    evidence_use: EvidenceUse,
    requested: &CanonicalAssertionProjectionRef,
    assertion: CanonicalAssertion,
    registry: AssertionRegistryEntry,
    traversal: SelectedBinaryTraversal,
  ) -> Result<Self, KnowledgeHydrationValidationError> {
    assertion
      .validate_for(&registry, evidence_use)
      .map_err(|_| KnowledgeHydrationValidationError::InvalidSupport)?;
    let (source, target, relation_type) = assertion
      .traversal_endpoints(&registry, &traversal.traversal_id)
      .map_err(|_| KnowledgeHydrationValidationError::InvalidRelation)?;
    if &assertion.release_id != release_id
      || traversal.relationship_revision == 0
      || traversal.assertion_id != assertion.assertion_id
      || traversal.assertion_revision != assertion.assertion_revision
      || traversal.relation_registry_revision != assertion.relation_registry_revision
      || &traversal.source != source
      || &traversal.target != target
      || traversal.relation_type != relation_type
      || requested.edge_id != traversal.edge_id
      || requested.relationship_revision != traversal.relationship_revision
      || requested.assertion_id != traversal.assertion_id
      || requested.assertion_revision != traversal.assertion_revision
      || requested.traversal_id != traversal.traversal_id
      || requested.source_node_id != *traversal.source.id()
      || requested.target_node_id != *traversal.target.id()
      || requested.relation_type != traversal.relation_type
      || requested.relation_registry_revision != traversal.relation_registry_revision
    {
      return Err(KnowledgeHydrationValidationError::InvalidRelation);
    }
    Ok(Self {
      assertion,
      registry,
      traversal,
    })
  }

  /// Returns the authoritative assertion revision.
  pub fn assertion(&self) -> &CanonicalAssertion {
    &self.assertion
  }

  /// Returns the exact validated registry proof.
  pub fn registry(&self) -> &AssertionRegistryEntry {
    &self.registry
  }

  /// Returns the selected traversal projection for view and path composition.
  pub fn traversal(&self) -> &SelectedBinaryTraversal {
    &self.traversal
  }

  #[cfg(test)]
  pub(crate) fn topology_fixture(
    source: CanonicalNodeId,
    target: CanonicalNodeId,
    edge_id: CanonicalId,
    assertion_id: CanonicalId,
    release_id: ReleaseId,
  ) -> Self {
    use super::graph::RelationshipVerificationState;

    let evidence_id = CanonicalId::new("evidence-fixture").unwrap();
    Self {
      assertion: CanonicalAssertion {
        assertion_id: assertion_id.clone(),
        assertion_revision: 1,
        release_id,
        relation_type_id: CanonicalId::new("relation-fixture").unwrap(),
        relation_registry_revision: 1,
        statement: "Validated test fixture.".into(),
        participants: Vec::new(),
        domain_ids: Vec::new(),
        conditions: Vec::new(),
        applicable_sense_ids: Vec::new(),
        evidence_ids: vec![evidence_id],
        evidence_lineage: Vec::new(),
        provenance_ids: Vec::new(),
        verification_state: RelationshipVerificationState::Verified,
      },
      registry: AssertionRegistryEntry {
        relation_type_id: CanonicalId::new("relation-fixture").unwrap(),
        registry_revision: 1,
        participant_roles: Vec::new(),
        resolved_domains: Vec::new(),
        resolved_conditions: Vec::new(),
        binary_traversals: Vec::new(),
        requires_evidence: true,
      },
      traversal: SelectedBinaryTraversal {
        edge_id,
        relationship_revision: 1,
        assertion_id,
        assertion_revision: 1,
        traversal_id: CanonicalId::new("traversal-fixture").unwrap(),
        source,
        target,
        relation_type: GraphRelationType::Hypernym,
        relation_registry_revision: 1,
      },
    }
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
