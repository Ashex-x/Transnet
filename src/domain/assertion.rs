//! Canonical n-ary assertions and explicit binary traversal admission.
//!
//! Assertions are publisher-owned canonical records. This module validates their role, scope,
//! condition, evidence, release, and traversal declarations without assigning identities or
//! inferring binary edges from prose.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use super::{
  canonical::{CanonicalId, EvidenceId, ReleaseId},
  canonical_content::CanonicalEvidenceLineage,
  canonical_translation::DomainId,
  graph::{GraphNodeKey, PublishedRelationship, RelationshipVerificationState},
};

/// Maximum participants admitted for one canonical assertion.
pub const MAX_ASSERTION_PARTICIPANTS: usize = 16;
/// Maximum canonical domains admitted for one assertion.
pub const MAX_ASSERTION_DOMAINS: usize = 16;
/// Maximum structured conditions admitted for one assertion.
pub const MAX_ASSERTION_CONDITIONS: usize = 16;
/// Maximum parameters admitted for one structured condition.
pub const MAX_CONDITION_PARAMETERS: usize = 16;

/// Closed canonical entity families that may participate in the assertion graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalNodeFamily {
  /// One language-specific lexical form and part of speech.
  Lexeme,
  /// One independently selectable lexical meaning.
  LexicalSense,
  /// One established multi-token expression.
  Phrase,
  /// One explicitly aligned multilingual term.
  MultilingualTerm,
  /// One language-independent concept.
  Concept,
  /// One named real or abstract entity.
  Entity,
  /// One observable phenomenon.
  Phenomenon,
  /// One explanatory mechanism.
  Mechanism,
  /// One ordered process.
  Process,
  /// One canonical equation.
  Equation,
  /// One measurable quantity.
  Quantity,
  /// One canonical material.
  Material,
  /// One instrument.
  Instrument,
  /// One method.
  Method,
  /// One technology.
  Technology,
  /// One application.
  Application,
  /// One published standard.
  Standard,
  /// One organization.
  Organization,
  /// One person.
  Person,
  /// One place.
  Place,
  /// One idiomatic expression.
  Idiom,
  /// One reviewed metaphorical extension.
  Metaphor,
  /// One grammar pattern or construction.
  GrammarPattern,
  /// One collocation.
  Collocation,
  /// One documented misconception.
  Misconception,
  /// One canonical domain.
  Domain,
  /// One evidence-backed semantic scale.
  SemanticScale,
}

impl From<super::retrieval_data::RetrievalNodeType> for CanonicalNodeFamily {
  fn from(value: super::retrieval_data::RetrievalNodeType) -> Self {
    use super::retrieval_data::RetrievalNodeType;
    match value {
      RetrievalNodeType::LexicalSense => Self::LexicalSense,
      RetrievalNodeType::Phrase => Self::Phrase,
      RetrievalNodeType::MultilingualTerm => Self::MultilingualTerm,
      RetrievalNodeType::Concept => Self::Concept,
      RetrievalNodeType::Entity => Self::Entity,
      RetrievalNodeType::Phenomenon => Self::Phenomenon,
      RetrievalNodeType::Mechanism => Self::Mechanism,
      RetrievalNodeType::Process => Self::Process,
      RetrievalNodeType::Equation => Self::Equation,
      RetrievalNodeType::Quantity => Self::Quantity,
      RetrievalNodeType::Material => Self::Material,
      RetrievalNodeType::Instrument => Self::Instrument,
      RetrievalNodeType::Method => Self::Method,
      RetrievalNodeType::Technology => Self::Technology,
      RetrievalNodeType::Application => Self::Application,
      RetrievalNodeType::Standard => Self::Standard,
      RetrievalNodeType::Organization => Self::Organization,
      RetrievalNodeType::Person => Self::Person,
      RetrievalNodeType::Place => Self::Place,
      RetrievalNodeType::Idiom => Self::Idiom,
      RetrievalNodeType::Metaphor => Self::Metaphor,
      RetrievalNodeType::GrammarPattern => Self::GrammarPattern,
      RetrievalNodeType::Collocation => Self::Collocation,
      RetrievalNodeType::Misconception => Self::Misconception,
      RetrievalNodeType::Domain => Self::Domain,
      RetrievalNodeType::SemanticScale => Self::SemanticScale,
    }
  }
}

/// Publisher-assigned, family-qualified canonical node identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalNodeId {
  family: CanonicalNodeFamily,
  id: CanonicalId,
}

impl CanonicalNodeId {
  /// Retains an explicitly supplied publisher identity under its declared family.
  pub fn publisher_assigned(family: CanonicalNodeFamily, id: CanonicalId) -> Self {
    Self { family, id }
  }

  /// Returns the closed entity family.
  pub const fn family(&self) -> CanonicalNodeFamily {
    self.family
  }

  /// Returns the opaque publisher-assigned identity.
  pub fn id(&self) -> &CanonicalId {
    &self.id
  }
}

/// Typed scalar value for an assertion role that is not an entity endpoint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssertionLiteral {
  /// Bounded language-neutral text whose interpretation is defined by the role schema.
  Text(String),
  /// Signed whole number.
  Integer(i64),
  /// Decimal encoded canonically by the publisher, including its unit when the role requires one.
  Decimal(String),
  /// Closed boolean value.
  Boolean(bool),
}

/// Exactly one entity or typed literal bound to an assertion role.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssertionParticipantValue {
  /// Canonical entity participant.
  Entity(CanonicalNodeId),
  /// Schema-validated typed literal participant.
  Literal(AssertionLiteral),
}

/// One ordered, registry-owned assertion participant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssertionParticipant {
  /// Stable role identity from the pinned relation registry.
  pub role_id: CanonicalId,
  /// Zero-based position within a repeatable role.
  pub ordinal: u16,
  /// Entity or literal value; the variants make mutual exclusivity structural.
  pub value: AssertionParticipantValue,
}

/// Structured applicability condition owned by the pinned condition registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssertionCondition {
  /// Stable publisher-owned condition identity.
  pub condition_id: CanonicalId,
  /// Closed condition type registered for the assertion relation.
  pub condition_type: CanonicalId,
  /// Sorted unique canonical parameter identities.
  pub parameter_ids: Vec<CanonicalId>,
}

/// Release-pinned canonical domain resolved by publication tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDomainReference {
  /// Exact canonical domain identity.
  pub domain_id: DomainId,
  /// Immutable release owning the domain revision.
  pub release_id: ReleaseId,
}

/// Release-pinned condition registry record resolved by publication tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConditionReference {
  /// Exact condition identity.
  pub condition_id: CanonicalId,
  /// Registered closed condition type.
  pub condition_type: CanonicalId,
  /// Exact sorted parameter identities admitted by this condition revision.
  pub parameter_ids: Vec<CanonicalId>,
  /// Immutable release owning the condition and parameter revisions.
  pub release_id: ReleaseId,
}

/// Whether a participant role accepts an entity or a typed literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantValueRule {
  /// Entity whose family must appear in the closed allowed set.
  Entity(BTreeSet<CanonicalNodeFamily>),
  /// Text literal.
  Text,
  /// Integer literal.
  Integer,
  /// Canonical decimal literal.
  Decimal,
  /// Boolean literal.
  Boolean,
}

/// Cardinality and value requirements for one registry-owned participant role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantRoleRule {
  /// Stable role identity.
  pub role_id: CanonicalId,
  /// Minimum number of participants carrying this role.
  pub minimum: u16,
  /// Maximum number of participants carrying this role.
  pub maximum: u16,
  /// Closed value schema for the role.
  pub value_rule: ParticipantValueRule,
}

/// Explicit role mapping for one permitted binary traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryTraversalRule {
  /// Stable declaration identity within the registry revision.
  pub traversal_id: CanonicalId,
  /// Role used as the stored source endpoint.
  pub source_role_id: CanonicalId,
  /// Role used as the stored target endpoint.
  pub target_role_id: CanonicalId,
  /// Exact graph relation and wire semantics reused by projection admission.
  pub relation_type: super::graph::GraphRelationType,
}

/// One immutable relation-registry entry pinned by an assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionRegistryEntry {
  /// Stable relation type identity.
  pub relation_type_id: CanonicalId,
  /// Positive immutable registry revision.
  pub registry_revision: u32,
  /// Closed participant-role schema.
  pub participant_roles: Vec<ParticipantRoleRule>,
  /// Release-resolved domains eligible for structured scope.
  pub resolved_domains: Vec<ResolvedDomainReference>,
  /// Release-resolved condition and parameter records eligible for structured scope.
  pub resolved_conditions: Vec<ResolvedConditionReference>,
  /// Explicit binary traversals; absence means the assertion is never projected as an edge.
  pub binary_traversals: Vec<BinaryTraversalRule>,
  /// Whether independently citable evidence is required.
  pub requires_evidence: bool,
}

/// Immutable publisher-owned canonical assertion revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAssertion {
  /// Stable assertion identity assigned by the publisher.
  pub assertion_id: CanonicalId,
  /// Positive immutable assertion revision.
  pub assertion_revision: u32,
  /// Immutable release owning the assertion and all referenced records.
  pub release_id: ReleaseId,
  /// Relation type identity from the pinned registry.
  pub relation_type_id: CanonicalId,
  /// Exact positive registry revision.
  pub relation_registry_revision: u32,
  /// Ordered role participants.
  pub participants: Vec<AssertionParticipant>,
  /// Sorted unique canonical domain identities.
  pub domain_ids: Vec<DomainId>,
  /// Sorted unique structured applicability conditions.
  pub conditions: Vec<AssertionCondition>,
  /// Sorted unique evidence identities supporting this revision.
  pub evidence_ids: Vec<EvidenceId>,
  /// Resolved evidence lineage used for release and permission checks.
  pub evidence_lineage: Vec<CanonicalEvidenceLineage>,
  /// Publication lifecycle state.
  pub verification_state: RelationshipVerificationState,
}

/// Publisher-supplied linkage from one assertion revision to one binary relationship projection.
#[derive(Debug, Clone, Copy)]
pub struct BinaryAssertionProjection<'a> {
  /// Exact assertion identity represented by the traversal.
  pub assertion_id: &'a CanonicalId,
  /// Exact immutable assertion revision represented by the traversal.
  pub assertion_revision: u32,
  /// Explicit traversal declaration selected from the pinned registry entry.
  pub traversal_id: &'a CanonicalId,
  /// Existing binary projection candidate subjected to graph publication admission.
  pub relationship: &'a PublishedRelationship,
}

/// Assertion admission failures exposed without request content.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AssertionValidationError {
  /// The relation type or registry revision was not the pinned entry.
  #[error("assertion relation registry entry is unresolved")]
  UnknownRegistryEntry,
  /// An immutable revision was zero.
  #[error("assertion and registry revisions must be positive")]
  InvalidRevision,
  /// Participant count or role cardinality violates the registry.
  #[error("assertion participant cardinality is invalid")]
  InvalidParticipantCardinality,
  /// A participant used an unknown role or incompatible value schema.
  #[error("assertion participant role or value is invalid")]
  InvalidParticipantRole,
  /// Participant ordinals were duplicated or non-contiguous for a role.
  #[error("assertion participant ordering is invalid")]
  InvalidParticipantOrder,
  /// A literal is blank, oversized, or not canonical.
  #[error("assertion literal is invalid")]
  InvalidLiteral,
  /// Domain scope is excessive, duplicated, or unsorted.
  #[error("assertion domain scope is invalid")]
  InvalidDomainScope,
  /// A condition is unresolved or structurally invalid.
  #[error("assertion condition is unsupported")]
  UnsupportedCondition,
  /// Evidence is missing, duplicated, unresolved, or contradictory.
  #[error("assertion evidence lineage is incomplete or contradictory")]
  InvalidEvidence,
  /// Evidence belongs to another release or lacks publication permissions.
  #[error("assertion evidence is not eligible for this release")]
  IneligibleEvidence,
  /// Only verified assertion revisions can be projected.
  #[error("assertion revision is not verified")]
  UnverifiedAssertion,
  /// A requested binary traversal was not explicitly declared.
  #[error("binary traversal is not declared by the registry")]
  UnknownBinaryTraversal,
  /// Traversal roles do not resolve to exactly one compatible entity endpoint each.
  #[error("binary traversal endpoints do not match assertion roles")]
  InvalidTraversalEndpoints,
  /// The binary relationship does not identify this exact assertion revision and release.
  #[error("binary relationship does not match its authoritative assertion")]
  ProjectionMismatch,
  /// Structured assertion scope has no lossless representation in the selected edge projection.
  #[error("assertion structured scope is not represented by the binary projection")]
  ProjectionScopeMismatch,
  /// Existing graph publication admission rejected the binary projection.
  #[error("binary relationship failed graph publication admission")]
  Graph(#[source] super::graph::GraphValidationError),
}

impl From<super::graph::GraphValidationError> for AssertionValidationError {
  fn from(value: super::graph::GraphValidationError) -> Self {
    Self::Graph(value)
  }
}

impl AssertionRegistryEntry {
  /// Validates registry structure before it is used to admit assertions.
  ///
  /// # Errors
  ///
  /// Returns an error for zero revisions, duplicate roles or traversals, impossible cardinalities,
  /// or traversal declarations that do not name entity roles compatible with graph endpoints.
  pub fn validate(&self) -> Result<(), AssertionValidationError> {
    if self.registry_revision == 0 {
      return Err(AssertionValidationError::InvalidRevision);
    }
    let mut roles = BTreeMap::new();
    for role in &self.participant_roles {
      if role.maximum == 0
        || role.minimum > role.maximum
        || roles.insert(&role.role_id, role).is_some()
      {
        return Err(AssertionValidationError::InvalidParticipantCardinality);
      }
      if matches!(&role.value_rule, ParticipantValueRule::Entity(kinds) if kinds.is_empty()) {
        return Err(AssertionValidationError::InvalidParticipantRole);
      }
    }
    let mut traversals = BTreeSet::new();
    for traversal in &self.binary_traversals {
      if !traversals.insert(&traversal.traversal_id)
        || traversal.source_role_id == traversal.target_role_id
      {
        return Err(AssertionValidationError::UnknownBinaryTraversal);
      }
      for role_id in [&traversal.source_role_id, &traversal.target_role_id] {
        let Some(role) = roles.get(role_id) else {
          return Err(AssertionValidationError::InvalidTraversalEndpoints);
        };
        if role.minimum != 1
          || role.maximum != 1
          || !matches!(role.value_rule, ParticipantValueRule::Entity(_))
        {
          return Err(AssertionValidationError::InvalidTraversalEndpoints);
        }
      }
    }
    if self
      .resolved_domains
      .windows(2)
      .any(|pair| pair[0].domain_id >= pair[1].domain_id)
      || self
        .resolved_conditions
        .windows(2)
        .any(|pair| pair[0].condition_id >= pair[1].condition_id)
    {
      return Err(AssertionValidationError::UnknownRegistryEntry);
    }
    for condition in &self.resolved_conditions {
      if condition.parameter_ids.len() > MAX_CONDITION_PARAMETERS
        || condition
          .parameter_ids
          .windows(2)
          .any(|pair| pair[0] >= pair[1])
      {
        return Err(AssertionValidationError::UnsupportedCondition);
      }
    }
    Ok(())
  }
}

impl CanonicalAssertion {
  /// Validates an assertion against one exact registry entry and immutable release.
  ///
  /// # Errors
  ///
  /// Returns an error for unresolved registry data, invalid roles/order/scope, unverified state,
  /// or evidence that is missing, cross-release, inactive, or not permitted for projection.
  pub fn validate(
    &self,
    registry: &AssertionRegistryEntry,
  ) -> Result<(), AssertionValidationError> {
    registry.validate()?;
    if self.relation_type_id != registry.relation_type_id
      || self.relation_registry_revision != registry.registry_revision
    {
      return Err(AssertionValidationError::UnknownRegistryEntry);
    }
    if self.assertion_revision == 0 {
      return Err(AssertionValidationError::InvalidRevision);
    }
    if self.verification_state != RelationshipVerificationState::Verified {
      return Err(AssertionValidationError::UnverifiedAssertion);
    }
    self.validate_participants(registry)?;
    self.validate_scope(registry)?;
    self.validate_evidence(registry.requires_evidence)?;
    Ok(())
  }

  /// Validates that an existing binary relationship is a declared, lossless projection of this
  /// assertion. This method performs assertion checks before invoking graph publication admission.
  ///
  /// # Errors
  ///
  /// Returns an error unless the traversal is declared and its two roles map exactly to the
  /// relationship endpoints, release, publisher identity, revision, evidence, and relation type.
  pub fn validate_binary_projection(
    &self,
    registry: &AssertionRegistryEntry,
    projection: BinaryAssertionProjection<'_>,
  ) -> Result<(), AssertionValidationError> {
    self.validate(registry)?;
    let traversal = registry
      .binary_traversals
      .iter()
      .find(|candidate| &candidate.traversal_id == projection.traversal_id)
      .ok_or(AssertionValidationError::UnknownBinaryTraversal)?;
    let source = self.single_entity(&traversal.source_role_id)?;
    let target = self.single_entity(&traversal.target_role_id)?;
    let relationship = projection.relationship;
    if projection.assertion_id != &self.assertion_id
      || projection.assertion_revision != self.assertion_revision
      || traversal.relation_type != relationship.relation.relation_type
      || source.family() != graph_family(&relationship.relation.source)
      || target.family() != graph_family(&relationship.relation.target)
      || source.id() != &relationship.relation.source.id
      || target.id() != &relationship.relation.target.id
      || self.release_id != relationship.relation_release_id
      || self.release_id != relationship.source_release_id
      || self.release_id != relationship.target_release_id
      || self.evidence_ids != relationship.relation.evidence.evidence_ids
      || self.evidence_lineage != relationship.evidence_lineage
    {
      return Err(AssertionValidationError::ProjectionMismatch);
    }
    if !self.domain_ids.is_empty() || !self.conditions.is_empty() {
      return Err(AssertionValidationError::ProjectionScopeMismatch);
    }
    relationship.validate()?;
    Ok(())
  }

  fn validate_participants(
    &self,
    registry: &AssertionRegistryEntry,
  ) -> Result<(), AssertionValidationError> {
    if self.participants.is_empty() || self.participants.len() > MAX_ASSERTION_PARTICIPANTS {
      return Err(AssertionValidationError::InvalidParticipantCardinality);
    }
    let rules = registry
      .participant_roles
      .iter()
      .map(|rule| (&rule.role_id, rule))
      .collect::<BTreeMap<_, _>>();
    let mut counts = BTreeMap::<&CanonicalId, u16>::new();
    let mut order = BTreeMap::<&CanonicalId, BTreeSet<u16>>::new();
    for participant in &self.participants {
      let Some(rule) = rules.get(&participant.role_id) else {
        return Err(AssertionValidationError::InvalidParticipantRole);
      };
      validate_participant_value(&participant.value, &rule.value_rule)?;
      *counts.entry(&participant.role_id).or_default() += 1;
      if !order
        .entry(&participant.role_id)
        .or_default()
        .insert(participant.ordinal)
      {
        return Err(AssertionValidationError::InvalidParticipantOrder);
      }
    }
    for rule in &registry.participant_roles {
      let count = counts.get(&rule.role_id).copied().unwrap_or_default();
      if count < rule.minimum || count > rule.maximum {
        return Err(AssertionValidationError::InvalidParticipantCardinality);
      }
      let expected = (0..count).collect::<BTreeSet<_>>();
      if order.get(&rule.role_id).cloned().unwrap_or_default() != expected {
        return Err(AssertionValidationError::InvalidParticipantOrder);
      }
    }
    Ok(())
  }

  fn validate_scope(
    &self,
    registry: &AssertionRegistryEntry,
  ) -> Result<(), AssertionValidationError> {
    if self.domain_ids.len() > MAX_ASSERTION_DOMAINS
      || self.domain_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
      return Err(AssertionValidationError::InvalidDomainScope);
    }
    if self.conditions.len() > MAX_ASSERTION_CONDITIONS
      || self.conditions.windows(2).any(|pair| pair[0] >= pair[1])
    {
      return Err(AssertionValidationError::UnsupportedCondition);
    }
    for domain_id in &self.domain_ids {
      let Some(resolved) = registry
        .resolved_domains
        .iter()
        .find(|resolved| &resolved.domain_id == domain_id)
      else {
        return Err(AssertionValidationError::InvalidDomainScope);
      };
      if resolved.release_id != self.release_id {
        return Err(AssertionValidationError::InvalidDomainScope);
      }
    }
    let mut condition_ids = BTreeSet::new();
    for condition in &self.conditions {
      if !condition_ids.insert(&condition.condition_id)
        || condition.parameter_ids.len() > MAX_CONDITION_PARAMETERS
        || condition
          .parameter_ids
          .windows(2)
          .any(|pair| pair[0] >= pair[1])
      {
        return Err(AssertionValidationError::UnsupportedCondition);
      }
      let Some(resolved) = registry
        .resolved_conditions
        .iter()
        .find(|resolved| resolved.condition_id == condition.condition_id)
      else {
        return Err(AssertionValidationError::UnsupportedCondition);
      };
      if resolved.release_id != self.release_id
        || resolved.condition_type != condition.condition_type
        || resolved.parameter_ids != condition.parameter_ids
      {
        return Err(AssertionValidationError::UnsupportedCondition);
      }
    }
    Ok(())
  }

  fn validate_evidence(&self, required: bool) -> Result<(), AssertionValidationError> {
    if (required && self.evidence_ids.is_empty())
      || self.evidence_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
      return Err(AssertionValidationError::InvalidEvidence);
    }
    let lineage_ids = self
      .evidence_lineage
      .iter()
      .map(|lineage| lineage.fragment().id.clone())
      .collect::<BTreeSet<_>>();
    if lineage_ids.len() != self.evidence_lineage.len()
      || lineage_ids.iter().ne(self.evidence_ids.iter())
    {
      return Err(AssertionValidationError::InvalidEvidence);
    }
    for lineage in &self.evidence_lineage {
      if lineage.fragment().release_id != self.release_id
        || !lineage.permits(&self.release_id, super::canonical::EvidenceUse::Embedding)
      {
        return Err(AssertionValidationError::IneligibleEvidence);
      }
    }
    Ok(())
  }

  fn single_entity(
    &self,
    role_id: &CanonicalId,
  ) -> Result<&CanonicalNodeId, AssertionValidationError> {
    let mut values = self
      .participants
      .iter()
      .filter(|participant| &participant.role_id == role_id);
    let Some(AssertionParticipantValue::Entity(value)) = values.next().map(|item| &item.value)
    else {
      return Err(AssertionValidationError::InvalidTraversalEndpoints);
    };
    if values.next().is_some() {
      return Err(AssertionValidationError::InvalidTraversalEndpoints);
    }
    Ok(value)
  }
}

fn validate_participant_value(
  value: &AssertionParticipantValue,
  rule: &ParticipantValueRule,
) -> Result<(), AssertionValidationError> {
  let valid = match (value, rule) {
    (AssertionParticipantValue::Entity(node), ParticipantValueRule::Entity(families)) => {
      families.contains(&node.family())
    }
    (
      AssertionParticipantValue::Literal(AssertionLiteral::Text(value)),
      ParticipantValueRule::Text,
    ) => !value.trim().is_empty() && value.chars().count() <= 1_024,
    (
      AssertionParticipantValue::Literal(AssertionLiteral::Integer(_)),
      ParticipantValueRule::Integer,
    )
    | (
      AssertionParticipantValue::Literal(AssertionLiteral::Boolean(_)),
      ParticipantValueRule::Boolean,
    ) => true,
    (
      AssertionParticipantValue::Literal(AssertionLiteral::Decimal(value)),
      ParticipantValueRule::Decimal,
    ) => is_canonical_decimal(value),
    _ => false,
  };
  if valid {
    Ok(())
  } else if matches!(value, AssertionParticipantValue::Literal(_)) {
    Err(AssertionValidationError::InvalidLiteral)
  } else {
    Err(AssertionValidationError::InvalidParticipantRole)
  }
}

fn is_canonical_decimal(value: &str) -> bool {
  if value.is_empty() || value.len() > 128 || value.starts_with('+') || value.ends_with('.') {
    return false;
  }
  let number = value.strip_prefix('-').unwrap_or(value);
  if number.is_empty() {
    return false;
  }
  let mut parts = number.split('.');
  let whole = parts.next().unwrap_or_default();
  let fraction = parts.next();
  whole.chars().all(|character| character.is_ascii_digit())
    && (whole == "0" || !whole.starts_with('0'))
    && fraction.is_none_or(|digits| {
      !digits.is_empty()
        && digits.chars().all(|character| character.is_ascii_digit())
        && !digits.ends_with('0')
    })
    && parts.next().is_none()
}

/// Converts a graph endpoint kind to the corresponding canonical assertion family.
pub const fn graph_family(key: &GraphNodeKey) -> CanonicalNodeFamily {
  match key.kind {
    super::graph::GraphNodeKind::Sense => CanonicalNodeFamily::LexicalSense,
    super::graph::GraphNodeKind::Lexeme => CanonicalNodeFamily::Lexeme,
    super::graph::GraphNodeKind::Construction => CanonicalNodeFamily::GrammarPattern,
    super::graph::GraphNodeKind::Scale => CanonicalNodeFamily::SemanticScale,
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeSet;

  use super::*;
  use crate::domain::{
    canonical::{
      CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind, LanguageTag,
      LexicalSource, SourcePermissions,
    },
    canonical_content::CanonicalEvidenceOrigin,
    graph::{
      GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphNodeKind, GraphRanking,
      GraphRelationType, GraphScope, GraphScore, GraphScoreComponents, RelationVersion,
      StoredGraphRelation,
    },
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn node(family: CanonicalNodeFamily, value: &str) -> CanonicalNodeId {
    CanonicalNodeId::publisher_assigned(family, id(value))
  }

  fn role(role_id: &str, family: CanonicalNodeFamily) -> ParticipantRoleRule {
    ParticipantRoleRule {
      role_id: id(role_id),
      minimum: 1,
      maximum: 1,
      value_rule: ParticipantValueRule::Entity(BTreeSet::from([family])),
    }
  }

  fn participant(role_id: &str, family: CanonicalNodeFamily, value: &str) -> AssertionParticipant {
    AssertionParticipant {
      role_id: id(role_id),
      ordinal: 0,
      value: AssertionParticipantValue::Entity(node(family, value)),
    }
  }

  fn permissions() -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding: true,
      model_processing: false,
      api_redistribution: true,
    }
  }

  fn lineage(release: &str) -> CanonicalEvidenceLineage {
    lineage_with_hash(release, "sha256:evidence-1")
  }

  fn lineage_with_hash(release: &str, content_hash: &str) -> CanonicalEvidenceLineage {
    let source_id = id("source-1");
    CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".to_string(),
        version: "2026-01".to_string(),
        license: "reviewed".to_string(),
        attribution: Some("Reviewed source".to_string()),
        permissions: permissions(),
      },
      EvidenceFragment {
        id: id("evidence-1"),
        source_id,
        source_reference: "entry-1".to_string(),
        release_id: id(release),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Reviewed support.".to_string(),
        content_hash: content_hash.to_string(),
        permissions: permissions(),
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap()
  }

  fn registry() -> AssertionRegistryEntry {
    AssertionRegistryEntry {
      relation_type_id: id("relation-taxonomy"),
      registry_revision: 1,
      participant_roles: vec![
        role("role-narrower", CanonicalNodeFamily::LexicalSense),
        role("role-broader", CanonicalNodeFamily::LexicalSense),
      ],
      resolved_domains: vec![ResolvedDomainReference {
        domain_id: DomainId::new("domain_weather").unwrap(),
        release_id: id("release-1"),
      }],
      resolved_conditions: vec![ResolvedConditionReference {
        condition_id: id("condition-weather"),
        condition_type: id("usage-context"),
        parameter_ids: vec![id("context-weather")],
        release_id: id("release-1"),
      }],
      binary_traversals: vec![BinaryTraversalRule {
        traversal_id: id("traversal-has-subtype"),
        source_role_id: id("role-broader"),
        target_role_id: id("role-narrower"),
        relation_type: GraphRelationType::Hypernym,
      }],
      requires_evidence: true,
    }
  }

  fn assertion() -> CanonicalAssertion {
    CanonicalAssertion {
      assertion_id: id("fact-taxonomy-1"),
      assertion_revision: 1,
      release_id: id("release-1"),
      relation_type_id: id("relation-taxonomy"),
      relation_registry_revision: 1,
      participants: vec![
        participant("role-broader", CanonicalNodeFamily::LexicalSense, "sense-b"),
        participant(
          "role-narrower",
          CanonicalNodeFamily::LexicalSense,
          "sense-a",
        ),
      ],
      domain_ids: vec![DomainId::new("domain_weather").unwrap()],
      conditions: vec![AssertionCondition {
        condition_id: id("condition-weather"),
        condition_type: id("usage-context"),
        parameter_ids: vec![id("context-weather")],
      }],
      evidence_ids: vec![id("evidence-1")],
      evidence_lineage: vec![lineage("release-1")],
      verification_state: RelationshipVerificationState::Verified,
    }
  }

  fn published() -> PublishedRelationship {
    let relation_type = GraphRelationType::Hypernym;
    let rule = relation_type.rule();
    PublishedRelationship {
      relation: StoredGraphRelation {
        edge_id: GraphEdgeId::stored(id("edge-taxonomy-1")),
        relation_version: RelationVersion::new(1).unwrap(),
        source: GraphNodeKey::new(GraphNodeKind::Sense, id("sense-b")),
        target: GraphNodeKey::new(GraphNodeKind::Sense, id("sense-a")),
        relation_type,
        evidence: GraphEvidence::new(vec![id("evidence-1")], EvidenceConfidence::High).unwrap(),
        scope: GraphScope::default(),
        feedback_capabilities: BTreeSet::from([GraphFeedbackCapability::Accuracy]),
        ranking: GraphRanking {
          display_rank: GraphScore::new(9_000).unwrap(),
          components: GraphScoreComponents {
            evidence: GraphScore::new(9_000).unwrap(),
            community: None,
            pedagogical: None,
          },
          ranking_version: "graph-rank-v1".to_string(),
        },
      },
      relation_release_id: id("release-1"),
      source_release_id: id("release-1"),
      target_release_id: id("release-1"),
      declared_wire_relation: rule.qdrant_wire_name.unwrap().to_string(),
      declared_inverse: rule.inverse,
      evidence_lineage: vec![lineage("release-1")],
      verification_state: RelationshipVerificationState::Verified,
    }
  }

  #[test]
  fn catalog_covers_every_documented_node_family_without_generating_ids() {
    let families = [
      CanonicalNodeFamily::Lexeme,
      CanonicalNodeFamily::LexicalSense,
      CanonicalNodeFamily::Phrase,
      CanonicalNodeFamily::MultilingualTerm,
      CanonicalNodeFamily::Concept,
      CanonicalNodeFamily::Entity,
      CanonicalNodeFamily::Phenomenon,
      CanonicalNodeFamily::Mechanism,
      CanonicalNodeFamily::Process,
      CanonicalNodeFamily::Equation,
      CanonicalNodeFamily::Quantity,
      CanonicalNodeFamily::Material,
      CanonicalNodeFamily::Instrument,
      CanonicalNodeFamily::Method,
      CanonicalNodeFamily::Technology,
      CanonicalNodeFamily::Application,
      CanonicalNodeFamily::Standard,
      CanonicalNodeFamily::Organization,
      CanonicalNodeFamily::Person,
      CanonicalNodeFamily::Place,
      CanonicalNodeFamily::Idiom,
      CanonicalNodeFamily::Metaphor,
      CanonicalNodeFamily::GrammarPattern,
      CanonicalNodeFamily::Collocation,
      CanonicalNodeFamily::Misconception,
      CanonicalNodeFamily::Domain,
      CanonicalNodeFamily::SemanticScale,
    ];
    for (index, family) in families.into_iter().enumerate() {
      let supplied = format!("publisher-{index}");
      let value = node(family, &supplied);
      assert_eq!(value.family(), family);
      assert_eq!(value.id().as_str(), supplied);
    }
  }

  #[test]
  fn validates_nary_entity_and_literal_roles_without_pairwise_rewriting() {
    let registry = AssertionRegistryEntry {
      relation_type_id: id("relation-measurement"),
      registry_revision: 3,
      participant_roles: vec![
        role("role-quantity", CanonicalNodeFamily::Quantity),
        role("role-method", CanonicalNodeFamily::Method),
        ParticipantRoleRule {
          role_id: id("role-value"),
          minimum: 1,
          maximum: 1,
          value_rule: ParticipantValueRule::Decimal,
        },
      ],
      resolved_domains: Vec::new(),
      resolved_conditions: Vec::new(),
      binary_traversals: Vec::new(),
      requires_evidence: false,
    };
    let assertion = CanonicalAssertion {
      assertion_id: id("fact-measurement-1"),
      assertion_revision: 2,
      release_id: id("release-1"),
      relation_type_id: id("relation-measurement"),
      relation_registry_revision: 3,
      participants: vec![
        participant("role-quantity", CanonicalNodeFamily::Quantity, "quantity-1"),
        participant("role-method", CanonicalNodeFamily::Method, "method-1"),
        AssertionParticipant {
          role_id: id("role-value"),
          ordinal: 0,
          value: AssertionParticipantValue::Literal(AssertionLiteral::Decimal("12.5".to_string())),
        },
      ],
      domain_ids: Vec::new(),
      conditions: Vec::new(),
      evidence_ids: Vec::new(),
      evidence_lineage: Vec::new(),
      verification_state: RelationshipVerificationState::Verified,
    };

    assert_eq!(assertion.validate(&registry), Ok(()));
    assert_eq!(registry.binary_traversals, Vec::new());
  }

  #[test]
  fn admits_only_the_explicit_lossless_binary_traversal() {
    let mut assertion = assertion();
    assertion.domain_ids.clear();
    assertion.conditions.clear();
    let relationship = published();
    let assertion_id = assertion.assertion_id.clone();
    let assertion_revision = assertion.assertion_revision;
    let projection = BinaryAssertionProjection {
      assertion_id: &assertion_id,
      assertion_revision,
      traversal_id: &id("traversal-has-subtype"),
      relationship: &relationship,
    };

    assert_eq!(
      assertion.validate_binary_projection(&registry(), projection),
      Ok(())
    );
  }

  #[test]
  fn rejects_unknown_registry_conditions_and_noncontiguous_role_order() {
    let mut unknown_registry = assertion();
    unknown_registry.relation_registry_revision = 2;
    assert_eq!(
      unknown_registry.validate(&registry()),
      Err(AssertionValidationError::UnknownRegistryEntry)
    );

    let mut condition = assertion();
    condition.conditions[0].condition_type = id("free-text-condition");
    assert_eq!(
      condition.validate(&registry()),
      Err(AssertionValidationError::UnsupportedCondition)
    );

    let mut order = assertion();
    order.participants[0].ordinal = 1;
    assert_eq!(
      order.validate(&registry()),
      Err(AssertionValidationError::InvalidParticipantOrder)
    );
  }

  #[test]
  fn rejects_cross_release_evidence_and_projection_endpoint_changes() {
    let mut cross_release = assertion();
    cross_release.evidence_lineage = vec![lineage("release-2")];
    assert_eq!(
      cross_release.validate(&registry()),
      Err(AssertionValidationError::IneligibleEvidence)
    );

    let mut assertion = assertion();
    assertion.domain_ids.clear();
    assertion.conditions.clear();
    let mut relationship = published();
    relationship.relation.target = GraphNodeKey::new(GraphNodeKind::Sense, id("sense-c"));
    let projection = BinaryAssertionProjection {
      assertion_id: &assertion.assertion_id,
      assertion_revision: assertion.assertion_revision,
      traversal_id: &id("traversal-has-subtype"),
      relationship: &relationship,
    };
    assert_eq!(
      assertion.validate_binary_projection(&registry(), projection),
      Err(AssertionValidationError::ProjectionMismatch)
    );
  }

  #[test]
  fn binary_projection_requires_independent_identity_scope_and_full_lineage_match() {
    let scoped = assertion();
    let relationship = published();
    let assertion_id = scoped.assertion_id.clone();
    let projection = BinaryAssertionProjection {
      assertion_id: &assertion_id,
      assertion_revision: scoped.assertion_revision,
      traversal_id: &id("traversal-has-subtype"),
      relationship: &relationship,
    };
    assert_eq!(
      scoped.validate_binary_projection(&registry(), projection),
      Err(AssertionValidationError::ProjectionScopeMismatch)
    );

    let mut unscoped = scoped;
    unscoped.domain_ids.clear();
    unscoped.conditions.clear();
    let wrong_id = id("fact-other");
    let projection = BinaryAssertionProjection {
      assertion_id: &wrong_id,
      assertion_revision: unscoped.assertion_revision,
      traversal_id: &id("traversal-has-subtype"),
      relationship: &relationship,
    };
    assert_eq!(
      unscoped.validate_binary_projection(&registry(), projection),
      Err(AssertionValidationError::ProjectionMismatch)
    );

    let mut changed_lineage = relationship;
    changed_lineage.evidence_lineage[0] = lineage_with_hash("release-1", "sha256:changed");
    let assertion_id = unscoped.assertion_id.clone();
    let projection = BinaryAssertionProjection {
      assertion_id: &assertion_id,
      assertion_revision: unscoped.assertion_revision,
      traversal_id: &id("traversal-has-subtype"),
      relationship: &changed_lineage,
    };
    assert_eq!(
      unscoped.validate_binary_projection(&registry(), projection),
      Err(AssertionValidationError::ProjectionMismatch)
    );
  }

  #[test]
  fn rejects_duplicate_domains_parameters_and_invalid_literals() {
    let mut domains = assertion();
    domains
      .domain_ids
      .push(DomainId::new("domain_weather").unwrap());
    assert_eq!(
      domains.validate(&registry()),
      Err(AssertionValidationError::InvalidDomainScope)
    );

    let mut parameters = assertion();
    parameters.conditions[0]
      .parameter_ids
      .push(id("context-weather"));
    assert_eq!(
      parameters.validate(&registry()),
      Err(AssertionValidationError::UnsupportedCondition)
    );

    assert!(!is_canonical_decimal("01.0"));
    assert!(!is_canonical_decimal("1.20"));
    assert!(is_canonical_decimal("-12.5"));

    let mut wrong_release_registry = registry();
    wrong_release_registry.resolved_domains[0].release_id = id("release-2");
    assert_eq!(
      assertion().validate(&wrong_release_registry),
      Err(AssertionValidationError::InvalidDomainScope)
    );
  }
}
