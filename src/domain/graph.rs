//! Typed, evidence-backed graph topology and bounded traversal values.

use std::collections::BTreeSet;

use thiserror::Error;

use super::canonical::{
  CanonicalId, CanonicalValidationError, EvidenceConfidence, EvidenceId, EvidenceUse, LanguageTag,
  LexicalPartOfSpeech, ReleaseId,
};
use super::canonical_content::CanonicalEvidenceLineage;

/// Default number of hops returned for a graph read.
pub const DEFAULT_GRAPH_DEPTH: u8 = 1;
/// Largest supported graph traversal depth.
pub const MAX_GRAPH_DEPTH: u8 = 2;
/// Default and maximum number of nodes in one graph response.
pub const DEFAULT_GRAPH_NODE_LIMIT: usize = 75;
/// Largest node cap accepted by the graph foundation.
pub const MAX_GRAPH_NODE_LIMIT: usize = 75;
/// Default and maximum number of edges in one graph response.
pub const DEFAULT_GRAPH_EDGE_LIMIT: usize = 200;
/// Largest edge cap accepted by the graph foundation.
pub const MAX_GRAPH_EDGE_LIMIT: usize = 200;
/// Largest graph score represented in basis points.
pub const MAX_GRAPH_SCORE_BASIS_POINTS: u16 = 10_000;

/// Validation failure for graph values and bounded traversal requests.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphValidationError {
  /// A graph traversal depth was outside the supported bounded range.
  #[error("graph depth must be between 0 and {MAX_GRAPH_DEPTH}")]
  InvalidDepth,
  /// A graph node cap was outside the supported bounded range.
  #[error("graph node limit must be between 1 and {MAX_GRAPH_NODE_LIMIT}")]
  InvalidNodeLimit,
  /// A graph edge cap was outside the supported bounded range.
  #[error("graph edge limit must be between 1 and {MAX_GRAPH_EDGE_LIMIT}")]
  InvalidEdgeLimit,
  /// A graph score was larger than one.
  #[error("graph score must be at most {MAX_GRAPH_SCORE_BASIS_POINTS} basis points")]
  InvalidScore,
  /// A relation version must be positive because zero is not a published version.
  #[error("relation version must be greater than zero")]
  InvalidRelationVersion,
  /// A user-visible graph label was blank after trimming surrounding whitespace.
  #[error("graph label must not be blank")]
  BlankLabel,
  /// A graph relation connected a node to itself.
  #[error("graph relation endpoints must differ")]
  DuplicateRelationEndpoints,
  /// A symmetric relation did not store endpoints in canonical typed-key order.
  #[error("symmetric relation endpoints must be stored in canonical order")]
  UnorderedSymmetricEndpoints,
  /// A canonical relation attempted to persist a relation reserved for scale projection.
  #[error("scale degree relations are derived and cannot be stored canonically")]
  StoredScaleProjection,
  /// A canonical relation had no independently citable supporting evidence.
  #[error("graph relation evidence must not be empty")]
  EmptyEvidence,
  /// A relation registry rule rejected the source node family.
  #[error("graph relation source node kind is not allowed")]
  InvalidRelationSourceKind,
  /// A relation registry rule rejected the target node family.
  #[error("graph relation target node kind is not allowed")]
  InvalidRelationTargetKind,
  /// The target Qdrant contract has not frozen a wire name for this relation.
  #[error("graph relation has no authoritative Qdrant wire mapping")]
  UnresolvedQdrantRelation,
  /// A relationship revision has not passed canonical publication review.
  #[error("graph relationship revision is not verified")]
  UnverifiedRelationship,
  /// A relationship or endpoint belongs to a different immutable release.
  #[error("graph relationship and endpoints must share one release")]
  RelationshipReleaseMismatch,
  /// One release repeated the same typed source-target assertion.
  #[error("graph release contains a duplicate typed relationship")]
  DuplicateTypedRelationship,
  /// A caller declared a Qdrant direction that differs from the registry mapping.
  #[error("graph relationship wire direction contradicts the registry")]
  RelationshipDirectionMismatch,
  /// A caller declared inverse semantics that differ from the registry.
  #[error("graph relationship inverse semantics contradict the registry")]
  RelationshipInverseMismatch,
  /// The relationship's resolved evidence did not exactly match its evidence identifiers.
  #[error("graph relationship evidence lineage is incomplete or contradictory")]
  RelationshipEvidenceMismatch,
  /// Resolved evidence belongs to another immutable release.
  #[error("graph relationship evidence belongs to another release")]
  RelationshipEvidenceReleaseMismatch,
  /// Resolved evidence lacks permission or lifecycle eligibility for vector projection.
  #[error("graph relationship evidence is not eligible for projection")]
  RelationshipEvidenceNotPermitted,
  /// Current canonical-domain scope cannot yet be represented without a guessed identity.
  #[error("graph relationship domain scope semantics are unresolved")]
  UnresolvedRelationshipDomainScope,
  /// Free-text conditions are not admitted as canonical relationship conditions.
  #[error("graph relationship condition semantics are unsupported")]
  UnsupportedRelationshipCondition,
  /// A supported scope field was blank or exceeded its bounded representation.
  #[error("graph relationship scope is invalid")]
  InvalidRelationshipScope,
  /// A semantic scale contained a non-sense member.
  #[error("semantic scale members must be sense nodes")]
  InvalidScaleMember,
  /// A semantic scale repeated an ordinal position.
  #[error("semantic scale member ordinals must be unique")]
  DuplicateScaleOrdinal,
  /// A semantic scale repeated a member node.
  #[error("semantic scale members must be unique")]
  DuplicateScaleMember,
  /// A cursor issued for one root was used with another root.
  #[error("graph cursor belongs to a different root node")]
  CursorRootMismatch,
  /// A cursor issued for one graph content version was used with another version.
  #[error("graph cursor belongs to a different graph content version")]
  CursorContentMismatch,
  /// A cursor issued with one normalized relation filter was used with another filter.
  #[error("graph cursor belongs to a different relation filter")]
  CursorFilterMismatch,
  /// A direct-neighbor page did not reserve room for one root and one adjacent endpoint.
  #[error("graph neighbor node limit must be between 2 and {MAX_GRAPH_NODE_LIMIT}")]
  InvalidNeighborNodeLimit,
}

/// Kind of canonical entity represented by a graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphNodeKind {
  /// One distinct lexical sense.
  Sense,
  /// A language-specific lemma and part of speech.
  Lexeme,
  /// A structured grammar or collocation construction.
  Construction,
  /// A context-qualified ordered semantic scale.
  Scale,
  /// One established phrase.
  Phrase,
  /// One multilingual term.
  MultilingualTerm,
  /// One language-independent concept.
  Concept,
  /// One named entity.
  Entity,
  /// One phenomenon.
  Phenomenon,
  /// One mechanism.
  Mechanism,
  /// One process.
  Process,
  /// One equation.
  Equation,
  /// One quantity.
  Quantity,
  /// One material.
  Material,
  /// One instrument.
  Instrument,
  /// One method.
  Method,
  /// One technology.
  Technology,
  /// One application.
  Application,
  /// One standard.
  Standard,
  /// One organization.
  Organization,
  /// One person.
  Person,
  /// One place.
  Place,
  /// One idiom.
  Idiom,
  /// One metaphor.
  Metaphor,
  /// One collocation.
  Collocation,
  /// One misconception.
  Misconception,
  /// One canonical domain.
  Domain,
}

/// Stable typed key for every graph node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphNodeKey {
  /// Entity family that determines how `id` is interpreted.
  pub kind: GraphNodeKind,
  /// Opaque stable identifier within `kind`.
  pub id: CanonicalId,
}

impl GraphNodeKey {
  /// Creates a typed key from a canonical entity identifier.
  pub fn new(kind: GraphNodeKind, id: CanonicalId) -> Self {
    Self { kind, id }
  }
}

/// Stable public representation of one graph node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphNode {
  /// Typed stable identity of this node.
  pub key: GraphNodeKey,
  /// Concise reader-visible label.
  pub label: String,
  /// Node language when the entity has one.
  pub language: Option<LanguageTag>,
  /// Part of speech for lexeme and sense nodes when available.
  pub part_of_speech: Option<LexicalPartOfSpeech>,
  /// Short canonical definition for sense nodes when available.
  pub definition_short: Option<String>,
  /// Whether this node may have more eligible neighbors.
  pub expandable: bool,
}

impl GraphNode {
  /// Creates a graph node after validating its reader-visible label.
  ///
  /// # Errors
  ///
  /// Returns an error when `label` is blank after trimming surrounding whitespace.
  pub fn new(
    key: GraphNodeKey,
    label: impl Into<String>,
    language: Option<LanguageTag>,
    part_of_speech: Option<LexicalPartOfSpeech>,
    definition_short: Option<String>,
    expandable: bool,
  ) -> Result<Self, GraphValidationError> {
    let label = label.into().trim().to_string();
    if label.is_empty() {
      return Err(GraphValidationError::BlankLabel);
    }
    Ok(Self {
      key,
      label,
      language,
      part_of_speech,
      definition_short,
      expandable,
    })
  }
}

/// Stable ID for a stored relation or a release-scoped derived projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphEdgeId(String);

impl GraphEdgeId {
  /// Parses one stable stored or derived graph edge identifier.
  ///
  /// # Errors
  ///
  /// Returns an error when `value` is blank after trimming surrounding whitespace.
  pub fn parse(value: impl AsRef<str>) -> Result<Self, CanonicalValidationError> {
    Ok(Self(CanonicalId::new(value)?.to_string()))
  }

  /// Creates the public ID of a feedback-enabled stored relation.
  pub fn stored(id: CanonicalId) -> Self {
    Self(id.to_string())
  }

  /// Creates the deterministic ID of one derived adjacent-scale projection.
  pub fn derived_scale(
    release_id: &ReleaseId,
    scale_id: &CanonicalId,
    lower: &GraphNodeKey,
    higher: &GraphNodeKey,
  ) -> Self {
    Self(format!(
      "derived:scale:{}:{}:{}:{}",
      release_id, scale_id, lower.id, higher.id
    ))
  }

  /// Creates the deterministic ID of one derived scale-membership projection.
  pub fn derived_scale_membership(
    release_id: &ReleaseId,
    scale_id: &CanonicalId,
    ordinal: u32,
    member: &GraphNodeKey,
  ) -> Self {
    Self(format!(
      "derived:scale:{}:{}:member:{ordinal:010}:{}",
      release_id, scale_id, member.id
    ))
  }

  /// Returns the public edge ID as a string slice.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Immutable positive version of a feedback-enabled stored relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationVersion(u32);

impl RelationVersion {
  /// Validates a published positive relation version.
  ///
  /// # Errors
  ///
  /// Returns an error when `value` is zero.
  pub const fn new(value: u32) -> Result<Self, GraphValidationError> {
    if value == 0 {
      return Err(GraphValidationError::InvalidRelationVersion);
    }
    Ok(Self(value))
  }

  /// Returns the published relation version number.
  pub const fn get(self) -> u32 {
    self.0
  }
}

/// Relation family shown on a graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphRelationType {
  /// Equivalent meaning within an explicitly supported scope.
  Synonym,
  /// Similar meaning that requires a reader-visible contrast.
  NearSynonym,
  /// Explicit equivalence across languages or lexicalizations.
  TranslationEquivalent,
  /// Opposing meanings within an explicitly supported scope.
  Antonym,
  /// Source is a broader category than target.
  Hypernym,
  /// Source is a narrower category than target.
  Hyponym,
  /// Source is a whole of which target is a part.
  Holonym,
  /// Source is a part of which target is a whole.
  Meronym,
  /// Terms are commonly confused by learners.
  ConfusableWith,
  /// A deliberately weak topical association.
  AssociatedWith,
  /// Source is an inflected form of target.
  InflectionOf,
  /// Target has source as one of its documented inflections.
  HasInflection,
  /// Documented modern derivational relationship.
  DerivationallyRelatedTo,
  /// Source historically derives from target.
  EtymologicallyDerivedFrom,
  /// Target historically derives from source.
  EtymologicalSourceOf,
  /// Source participates in target construction.
  ConstructionMember,
  /// Target has source as a construction participant.
  HasConstructionMember,
  /// Source scale contains target as an ordered member.
  ScaleContains,
  /// Source is an ordered member of target scale.
  MemberOfScale,
  /// Target is the adjacent lower member of a projected semantic scale.
  LowerDegree,
  /// Target is the adjacent higher member of a projected semantic scale.
  HigherDegree,
}

/// Directionality declared by the authoritative relationship registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
  /// Source and target have distinct roles and inversion changes the relation type.
  Directed,
  /// Endpoint order has no semantic effect after canonical key ordering.
  Symmetric,
}

/// Whether the authoritative contract declares a transitive or causal property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipProperty {
  /// The contract explicitly declares that the property applies.
  Declared,
  /// The contract explicitly declares that the property does not apply.
  NotApplicable,
  /// The property remains unspecified and consumers must not infer it.
  Unspecified,
}

/// Scope fields the current graph domain can carry without inventing publication metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationshipScopeFields {
  /// Whether an explicit dialect restriction may qualify the assertion.
  pub dialect: bool,
  /// Whether an explicit canonical-domain restriction may qualify the assertion.
  pub domain: bool,
  /// Whether an explicit register restriction may qualify the assertion.
  pub register: bool,
  /// Whether a concise evidence-backed condition may qualify the assertion.
  pub condition: bool,
}

/// Immutable semantics for one supported internal relationship type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationshipRule {
  /// Internal relation identity used by the graph domain.
  pub relation_type: GraphRelationType,
  /// Qdrant payload name when the authoritative contract defines one exactly.
  pub qdrant_wire_name: Option<&'static str>,
  /// Node families permitted at the stored source endpoint.
  pub source_kinds: &'static [GraphNodeKind],
  /// Node families permitted at the stored target endpoint.
  pub target_kinds: &'static [GraphNodeKind],
  /// Directed or symmetric endpoint semantics.
  pub direction: RelationshipDirection,
  /// Relation visible from the opposite endpoint.
  pub inverse: GraphRelationType,
  /// Transitivity semantics; unspecified is fail-closed and never means transitive.
  pub transitivity: RelationshipProperty,
  /// Causality semantics; unspecified is fail-closed and never proves causation.
  pub causality: RelationshipProperty,
  /// Explicit scope fields carried by the current graph contract.
  pub allowed_scope_fields: RelationshipScopeFields,
  /// Whether independently citable evidence is mandatory for publication.
  pub requires_evidence: bool,
  /// Whether source-qualified confidence is mandatory for publication.
  pub requires_confidence: bool,
  /// Whether only a verified canonical revision may be published.
  pub requires_verified_revision: bool,
  /// Whether endpoints and assertion must belong to the same immutable release.
  pub requires_release_ownership: bool,
}

const SENSE_KINDS: &[GraphNodeKind] = &[GraphNodeKind::Sense];
const LEXICAL_KINDS: &[GraphNodeKind] = &[GraphNodeKind::Sense, GraphNodeKind::Lexeme];
const CONSTRUCTION_SOURCE_KINDS: &[GraphNodeKind] = &[GraphNodeKind::Sense, GraphNodeKind::Lexeme];
const CONSTRUCTION_TARGET_KINDS: &[GraphNodeKind] = &[GraphNodeKind::Construction];
const SCALE_KINDS: &[GraphNodeKind] = &[GraphNodeKind::Scale];

impl RelationshipRule {
  /// Validates the typed endpoints without inferring semantics from labels or wire names.
  ///
  /// # Errors
  ///
  /// Returns an error when either endpoint family is not allowed by this registry rule.
  pub fn validate_endpoint_kinds(
    &self,
    source: GraphNodeKind,
    target: GraphNodeKind,
  ) -> Result<(), GraphValidationError> {
    if !self.source_kinds.contains(&source) {
      return Err(GraphValidationError::InvalidRelationSourceKind);
    }
    if !self.target_kinds.contains(&target) {
      return Err(GraphValidationError::InvalidRelationTargetKind);
    }
    Ok(())
  }

  /// Returns the exact Qdrant payload name or fails closed when it is not frozen.
  ///
  /// # Errors
  ///
  /// Returns an error rather than deriving a wire value from the Rust enum variant.
  pub fn require_qdrant_wire_name(&self) -> Result<&'static str, GraphValidationError> {
    self
      .qdrant_wire_name
      .ok_or(GraphValidationError::UnresolvedQdrantRelation)
  }
}

impl GraphRelationType {
  /// Returns the immutable registry rule for this internal relation identity.
  pub const fn rule(self) -> RelationshipRule {
    let (source_kinds, target_kinds) = match self {
      Self::InflectionOf
      | Self::HasInflection
      | Self::DerivationallyRelatedTo
      | Self::EtymologicallyDerivedFrom
      | Self::EtymologicalSourceOf => (LEXICAL_KINDS, LEXICAL_KINDS),
      Self::ConstructionMember => (CONSTRUCTION_SOURCE_KINDS, CONSTRUCTION_TARGET_KINDS),
      Self::HasConstructionMember => (CONSTRUCTION_TARGET_KINDS, CONSTRUCTION_SOURCE_KINDS),
      Self::ScaleContains => (SCALE_KINDS, SENSE_KINDS),
      Self::MemberOfScale => (SENSE_KINDS, SCALE_KINDS),
      _ => (SENSE_KINDS, SENSE_KINDS),
    };
    let qdrant_wire_name = match self {
      // The graph domain stores broader -> narrower while the Qdrant contract names that
      // direction `has_subtype`; the inverse narrower -> broader direction is `is_a`.
      Self::Hypernym => Some("has_subtype"),
      Self::Hyponym => Some("is_a"),
      Self::LowerDegree => Some("lower_degree_than"),
      Self::HigherDegree => Some("higher_degree_than"),
      Self::Synonym => Some("synonym"),
      Self::NearSynonym => Some("near_synonym"),
      Self::TranslationEquivalent => Some("translation_equivalent"),
      Self::Antonym => Some("antonym"),
      Self::Holonym => Some("has_part"),
      Self::Meronym => Some("part_of"),
      Self::ConfusableWith => Some("confusable_with"),
      Self::AssociatedWith => Some("associated_with"),
      Self::InflectionOf => Some("inflection_of"),
      Self::HasInflection => Some("has_inflection"),
      Self::DerivationallyRelatedTo => Some("derivationally_related_to"),
      Self::EtymologicallyDerivedFrom => Some("etymologically_derived_from"),
      Self::EtymologicalSourceOf => Some("etymological_source_of"),
      Self::ConstructionMember => Some("member_of_construction"),
      Self::HasConstructionMember => Some("has_construction_member"),
      Self::ScaleContains => Some("scale_contains"),
      Self::MemberOfScale => Some("member_of_scale"),
    };
    let direction = if self.is_symmetric() {
      RelationshipDirection::Symmetric
    } else {
      RelationshipDirection::Directed
    };
    RelationshipRule {
      relation_type: self,
      qdrant_wire_name,
      source_kinds,
      target_kinds,
      direction,
      inverse: self.inverse(),
      transitivity: if matches!(self, Self::Hypernym | Self::Hyponym) {
        RelationshipProperty::Declared
      } else {
        RelationshipProperty::NotApplicable
      },
      causality: RelationshipProperty::NotApplicable,
      allowed_scope_fields: RelationshipScopeFields {
        dialect: true,
        domain: false,
        register: true,
        condition: false,
      },
      requires_evidence: true,
      requires_confidence: true,
      requires_verified_revision: true,
      requires_release_ownership: true,
    }
  }

  /// Returns the relation type visible when the same fact is read from its opposite endpoint.
  pub const fn inverse(self) -> Self {
    match self {
      Self::Synonym => Self::Synonym,
      Self::NearSynonym => Self::NearSynonym,
      Self::TranslationEquivalent => Self::TranslationEquivalent,
      Self::Antonym => Self::Antonym,
      Self::Hypernym => Self::Hyponym,
      Self::Hyponym => Self::Hypernym,
      Self::Holonym => Self::Meronym,
      Self::Meronym => Self::Holonym,
      Self::ConfusableWith => Self::ConfusableWith,
      Self::AssociatedWith => Self::AssociatedWith,
      Self::InflectionOf => Self::HasInflection,
      Self::HasInflection => Self::InflectionOf,
      Self::DerivationallyRelatedTo => Self::DerivationallyRelatedTo,
      Self::EtymologicallyDerivedFrom => Self::EtymologicalSourceOf,
      Self::EtymologicalSourceOf => Self::EtymologicallyDerivedFrom,
      Self::ConstructionMember => Self::HasConstructionMember,
      Self::HasConstructionMember => Self::ConstructionMember,
      Self::ScaleContains => Self::MemberOfScale,
      Self::MemberOfScale => Self::ScaleContains,
      Self::LowerDegree => Self::HigherDegree,
      Self::HigherDegree => Self::LowerDegree,
    }
  }

  /// Returns whether canonical records for this type must use endpoint key order.
  pub const fn is_symmetric(self) -> bool {
    matches!(
      self,
      Self::Synonym
        | Self::NearSynonym
        | Self::TranslationEquivalent
        | Self::Antonym
        | Self::ConfusableWith
        | Self::AssociatedWith
        | Self::DerivationallyRelatedTo
    )
  }

  /// Returns whether this type exists only as a read-time scale projection.
  pub const fn is_scale_projection(self) -> bool {
    matches!(
      self,
      Self::ScaleContains | Self::MemberOfScale | Self::LowerDegree | Self::HigherDegree
    )
  }
}

/// Feedback dimension accepted by a stored relation version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphFeedbackCapability {
  /// A learner may report personal usefulness.
  Usefulness,
  /// A learner may report factual accuracy.
  Accuracy,
}

/// Evidence and source-qualified confidence supporting a graph assertion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEvidence {
  /// Independently citable evidence fragments supporting this assertion.
  pub evidence_ids: Vec<EvidenceId>,
  /// Source-qualified confidence kept separate from graph rank.
  pub confidence: EvidenceConfidence,
}

impl GraphEvidence {
  /// Creates evidence after sorting and deduplicating its identifiers.
  ///
  /// # Errors
  ///
  /// Returns an error when no evidence identifiers are supplied.
  pub fn new(
    mut evidence_ids: Vec<EvidenceId>,
    confidence: EvidenceConfidence,
  ) -> Result<Self, GraphValidationError> {
    evidence_ids.sort();
    evidence_ids.dedup();
    if evidence_ids.is_empty() {
      return Err(GraphValidationError::EmptyEvidence);
    }
    Ok(Self {
      evidence_ids,
      confidence,
    })
  }
}

/// Context that qualifies where a graph relation is supported.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphScope {
  /// Dialect restriction when the relation is not language-wide.
  pub dialect: Option<LanguageTag>,
  /// Domain restriction such as medicine, law, or computing.
  pub domain: Option<String>,
  /// Register restriction such as formal or informal.
  pub register: Option<String>,
  /// Concise additional qualification retained from source evidence.
  pub note: Option<String>,
}

/// Deterministic graph score represented in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphScore(u16);

impl GraphScore {
  /// Creates a score from zero through 10,000 basis points.
  ///
  /// # Errors
  ///
  /// Returns an error when `basis_points` exceeds 10,000.
  pub const fn new(basis_points: u16) -> Result<Self, GraphValidationError> {
    if basis_points > MAX_GRAPH_SCORE_BASIS_POINTS {
      return Err(GraphValidationError::InvalidScore);
    }
    Ok(Self(basis_points))
  }

  /// Returns the score in basis points.
  pub const fn basis_points(self) -> u16 {
    self.0
  }
}

/// Independent rank components retained for explainable graph ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphScoreComponents {
  /// Evidence-derived eligibility and confidence contribution.
  pub evidence: GraphScore,
  /// Community aggregate contribution, if enough reviewed feedback exists.
  pub community: Option<GraphScore>,
  /// Learner-neutral pedagogical contribution, if configured.
  pub pedagogical: Option<GraphScore>,
}

/// Rank and algorithm version attached to a graph edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphRanking {
  /// Current display ordering score; it is not semantic truth or a probability.
  pub display_rank: GraphScore,
  /// Independent feature values used by the versioned ranker.
  pub components: GraphScoreComponents,
  /// Deterministic ranker implementation version.
  pub ranking_version: String,
}

/// Pinned versions required to perform a repeatable graph read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphContentVersion {
  /// Immutable lexical content release that owns the graph facts.
  pub release_id: ReleaseId,
  /// Version of the deterministic graph ranking implementation.
  pub ranking_version: String,
  /// Version of public community aggregate projections.
  pub community_aggregate_version: String,
}

/// Canonical stored relation before read-time inverse projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoredGraphRelation {
  /// Stable public edge ID shared by every projection of this assertion.
  pub edge_id: GraphEdgeId,
  /// Immutable version to which feedback events are pinned.
  pub relation_version: RelationVersion,
  /// Authoritative source endpoint.
  pub source: GraphNodeKey,
  /// Authoritative target endpoint.
  pub target: GraphNodeKey,
  /// Authoritative directed or symmetric relation type.
  pub relation_type: GraphRelationType,
  /// Evidence supporting the canonical assertion.
  pub evidence: GraphEvidence,
  /// Source-qualified scope restrictions.
  pub scope: GraphScope,
  /// Feedback dimensions allowed for this stored relation version.
  pub feedback_capabilities: BTreeSet<GraphFeedbackCapability>,
  /// Versioned rank and score components.
  pub ranking: GraphRanking,
}

/// Publication lifecycle state attached to an authoritative relationship revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipVerificationState {
  /// The assertion passed the evidence, rights, and editorial review workflow.
  Verified,
  /// The assertion remains a candidate and cannot enter a canonical projection.
  Exploratory,
}

/// Release ownership attached to one relationship before Qdrant projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRelationship {
  /// Canonical relation revision being projected.
  pub relation: StoredGraphRelation,
  /// Release that owns the relation revision.
  pub relation_release_id: ReleaseId,
  /// Release that owns the source endpoint revision.
  pub source_release_id: ReleaseId,
  /// Release that owns the target endpoint revision.
  pub target_release_id: ReleaseId,
  /// Exact Qdrant relation name declared by the projection candidate.
  pub declared_wire_relation: String,
  /// Inverse relation identity declared by the projection candidate.
  pub declared_inverse: GraphRelationType,
  /// Fully resolved M2 evidence lineage referenced by the relation.
  pub evidence_lineage: Vec<CanonicalEvidenceLineage>,
  /// Review lifecycle state of the relation revision.
  pub verification_state: RelationshipVerificationState,
}

/// Stable semantic identity used to detect duplicate typed edge projections.
///
/// Evidence revisions are intentionally excluded until the authoritative contract decides whether
/// evidence changes create a new edge identity or only a new relationship revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublishedEdgeIdentity {
  /// Immutable release owning the assertion and both endpoints.
  pub release_id: ReleaseId,
  /// Stable canonical relationship identity assigned by the publisher.
  pub relationship_id: GraphEdgeId,
  /// Immutable positive revision of the canonical relationship.
  pub relationship_revision: RelationVersion,
  /// Canonical source endpoint after symmetric canonicalization.
  pub source: GraphNodeKey,
  /// Canonical target endpoint after symmetric canonicalization.
  pub target: GraphNodeKey,
  /// Internal typed relation identity.
  pub relation_type: GraphRelationType,
  /// Scope and condition identity admitted by the current registry.
  pub scope: GraphScope,
}

impl PublishedRelationship {
  /// Validates relationship registry, evidence, review, and same-release ownership rules.
  ///
  /// # Errors
  ///
  /// Returns an error when the relation itself is invalid, its endpoint kinds violate the
  /// registry, it is not verified, or any endpoint belongs to another release.
  pub fn validate(&self) -> Result<(), GraphValidationError> {
    self.relation.validate()?;
    let rule = self.relation.relation_type.rule();
    rule.validate_endpoint_kinds(self.relation.source.kind, self.relation.target.kind)?;
    if rule.require_qdrant_wire_name()? != self.declared_wire_relation {
      return Err(GraphValidationError::RelationshipDirectionMismatch);
    }
    if rule.inverse != self.declared_inverse {
      return Err(GraphValidationError::RelationshipInverseMismatch);
    }
    if self.verification_state != RelationshipVerificationState::Verified {
      return Err(GraphValidationError::UnverifiedRelationship);
    }
    if self.source_release_id != self.relation_release_id
      || self.target_release_id != self.relation_release_id
    {
      return Err(GraphValidationError::RelationshipReleaseMismatch);
    }
    validate_relationship_scope(&self.relation.scope, &rule)?;
    self.validate_evidence()?;
    Ok(())
  }

  /// Returns the deterministic semantic identity used for duplicate rejection.
  pub fn identity(&self) -> PublishedEdgeIdentity {
    let (source, target) = if self.relation.relation_type.is_symmetric()
      && self.relation.source > self.relation.target
    {
      (self.relation.target.clone(), self.relation.source.clone())
    } else {
      (self.relation.source.clone(), self.relation.target.clone())
    };
    PublishedEdgeIdentity {
      release_id: self.relation_release_id.clone(),
      relationship_id: self.relation.edge_id.clone(),
      relationship_revision: self.relation.relation_version,
      source,
      target,
      relation_type: self.relation.relation_type,
      scope: self.relation.scope.clone(),
    }
  }

  fn validate_evidence(&self) -> Result<(), GraphValidationError> {
    if self.evidence_lineage.is_empty() {
      return Err(GraphValidationError::EmptyEvidence);
    }
    let expected = self
      .relation
      .evidence
      .evidence_ids
      .iter()
      .cloned()
      .collect::<BTreeSet<_>>();
    let actual = self
      .evidence_lineage
      .iter()
      .map(|lineage| lineage.fragment().id.clone())
      .collect::<BTreeSet<_>>();
    if actual.len() != self.evidence_lineage.len() || actual != expected {
      return Err(GraphValidationError::RelationshipEvidenceMismatch);
    }
    for lineage in &self.evidence_lineage {
      if lineage.fragment().release_id != self.relation_release_id {
        return Err(GraphValidationError::RelationshipEvidenceReleaseMismatch);
      }
      if !lineage.permits(&self.relation_release_id, EvidenceUse::Embedding) {
        return Err(GraphValidationError::RelationshipEvidenceNotPermitted);
      }
    }
    Ok(())
  }
}

/// Validates a release's relationships and rejects duplicate typed endpoint assertions.
///
/// Duplicate identity is defined by stored source, target, and internal relation type rather than
/// by an independently supplied edge ID.
///
/// # Errors
///
/// Returns an error when a relationship is invalid or a typed endpoint assertion is repeated.
pub fn validate_published_relationships(
  relationships: &[PublishedRelationship],
) -> Result<(), GraphValidationError> {
  let mut ordered = relationships.iter().collect::<Vec<_>>();
  ordered.sort_by_key(|published| published.identity());
  let mut identities = BTreeSet::new();
  for published in ordered {
    published.validate()?;
    let identity = published.identity();
    let typed_assertion = (
      identity.release_id,
      identity.source,
      identity.target,
      identity.relation_type,
      identity.scope,
    );
    if !identities.insert(typed_assertion) {
      return Err(GraphValidationError::DuplicateTypedRelationship);
    }
  }
  Ok(())
}

fn validate_relationship_scope(
  scope: &GraphScope,
  rule: &RelationshipRule,
) -> Result<(), GraphValidationError> {
  if scope.domain.is_some() && !rule.allowed_scope_fields.domain {
    return Err(GraphValidationError::UnresolvedRelationshipDomainScope);
  }
  if scope.note.is_some() && !rule.allowed_scope_fields.condition {
    return Err(GraphValidationError::UnsupportedRelationshipCondition);
  }
  if scope
    .register
    .as_ref()
    .is_some_and(|value| value.trim().is_empty() || value.len() > 64)
  {
    return Err(GraphValidationError::InvalidRelationshipScope);
  }
  Ok(())
}

impl StoredGraphRelation {
  /// Validates invariants that every persisted canonical relation must satisfy.
  ///
  /// # Errors
  ///
  /// Returns an error for self-relations, unordered symmetric endpoints, or stored scale types.
  pub fn validate(&self) -> Result<(), GraphValidationError> {
    if self.source == self.target {
      return Err(GraphValidationError::DuplicateRelationEndpoints);
    }
    if self.relation_type.is_symmetric() && self.source > self.target {
      return Err(GraphValidationError::UnorderedSymmetricEndpoints);
    }
    if self.relation_type.is_scale_projection() {
      return Err(GraphValidationError::StoredScaleProjection);
    }
    if self.evidence.evidence_ids.is_empty() {
      return Err(GraphValidationError::EmptyEvidence);
    }
    Ok(())
  }

  /// Projects this relation from `node` when it is one of the relation endpoints.
  pub fn project_from(&self, node: &GraphNodeKey) -> Option<GraphEdge> {
    if node == &self.source {
      return Some(GraphEdge {
        id: self.edge_id.clone(),
        source: self.source.clone(),
        target: self.target.clone(),
        relation_type: self.relation_type,
        directed: !self.relation_type.is_symmetric(),
        relation_version: Some(self.relation_version),
        evidence: self.evidence.clone(),
        scope: self.scope.clone(),
        feedback_capabilities: self.feedback_capabilities.clone(),
        ranking: self.ranking.clone(),
        origin: GraphEdgeOrigin::Canonical,
      });
    }
    if node == &self.target {
      return Some(GraphEdge {
        id: self.edge_id.clone(),
        source: self.target.clone(),
        target: self.source.clone(),
        relation_type: self.relation_type.inverse(),
        directed: !self.relation_type.is_symmetric(),
        relation_version: Some(self.relation_version),
        evidence: self.evidence.clone(),
        scope: self.scope.clone(),
        feedback_capabilities: self.feedback_capabilities.clone(),
        ranking: self.ranking.clone(),
        origin: GraphEdgeOrigin::InverseProjection,
      });
    }
    None
  }
}

/// One ordered sense in a semantic scale.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticScaleMember {
  /// Sense represented at this point on the scale.
  pub sense: GraphNodeKey,
  /// Stable source-qualified order within the scale.
  pub ordinal: u32,
  /// Evidence that supports this member's placement.
  pub evidence: GraphEvidence,
}

/// Canonical ordered semantic scale used to derive adjacent degree edges.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticScale {
  /// Stable scale identifier.
  pub id: CanonicalId,
  /// Release that owns the scale and every member assertion.
  pub release_id: ReleaseId,
  /// Learner-visible scale dimension.
  pub label: String,
  /// Context that qualifies all derived adjacency projections.
  pub scope: GraphScope,
  /// Evidence supporting the scale definition.
  pub evidence: GraphEvidence,
  /// Ordered source-backed members from lower to higher degree.
  pub members: Vec<SemanticScaleMember>,
  /// Rank inherited by derived adjacency edges.
  pub ranking: GraphRanking,
}

impl SemanticScale {
  /// Validates and orders scale members by their stable ordinal.
  ///
  /// # Errors
  ///
  /// Returns an error for non-sense, repeated, or unevidenced members.
  pub fn validate(&self) -> Result<(), GraphValidationError> {
    let mut ordinals = BTreeSet::new();
    let mut senses = BTreeSet::new();
    for member in &self.members {
      if member.sense.kind != GraphNodeKind::Sense {
        return Err(GraphValidationError::InvalidScaleMember);
      }
      if !ordinals.insert(member.ordinal) {
        return Err(GraphValidationError::DuplicateScaleOrdinal);
      }
      if !senses.insert(member.sense.clone()) {
        return Err(GraphValidationError::DuplicateScaleMember);
      }
      if member.evidence.evidence_ids.is_empty() {
        return Err(GraphValidationError::EmptyEvidence);
      }
    }
    Ok(())
  }

  /// Projects derived membership and adjacent-degree edges visible from `node`.
  pub fn project_from(&self, node: &GraphNodeKey) -> Vec<GraphEdge> {
    let mut members = self.members.clone();
    members.sort_by_key(|member| member.ordinal);
    let scale_node = self.node_key();
    if node == &scale_node {
      return members
        .iter()
        .map(|member| {
          self.membership_edge(
            node,
            &member.sense,
            GraphRelationType::ScaleContains,
            member,
          )
        })
        .collect();
    }
    let Some(index) = members.iter().position(|member| &member.sense == node) else {
      return Vec::new();
    };

    let mut edges = Vec::with_capacity(3);
    edges.push(self.membership_edge(
      node,
      &scale_node,
      GraphRelationType::MemberOfScale,
      &members[index],
    ));
    if let Some(lower) = index.checked_sub(1).and_then(|index| members.get(index)) {
      edges.push(self.derived_edge(node, &lower.sense, GraphRelationType::LowerDegree));
    }
    if let Some(higher) = members.get(index + 1) {
      edges.push(self.derived_edge(node, &higher.sense, GraphRelationType::HigherDegree));
    }
    edges
  }

  fn derived_edge(
    &self,
    source: &GraphNodeKey,
    target: &GraphNodeKey,
    relation_type: GraphRelationType,
  ) -> GraphEdge {
    let (lower, higher) = if relation_type == GraphRelationType::LowerDegree {
      (target, source)
    } else {
      (source, target)
    };
    GraphEdge {
      id: GraphEdgeId::derived_scale(&self.release_id, &self.id, lower, higher),
      source: source.clone(),
      target: target.clone(),
      relation_type,
      directed: true,
      relation_version: None,
      evidence: self.evidence.clone(),
      scope: self.scope.clone(),
      feedback_capabilities: BTreeSet::new(),
      ranking: self.ranking.clone(),
      origin: GraphEdgeOrigin::ScaleAdjacency {
        scale_id: self.id.clone(),
      },
    }
  }

  fn membership_edge(
    &self,
    source: &GraphNodeKey,
    target: &GraphNodeKey,
    relation_type: GraphRelationType,
    member: &SemanticScaleMember,
  ) -> GraphEdge {
    GraphEdge {
      id: GraphEdgeId::derived_scale_membership(
        &self.release_id,
        &self.id,
        member.ordinal,
        &member.sense,
      ),
      source: source.clone(),
      target: target.clone(),
      relation_type,
      directed: true,
      relation_version: None,
      evidence: member.evidence.clone(),
      scope: self.scope.clone(),
      feedback_capabilities: BTreeSet::new(),
      ranking: self.ranking.clone(),
      origin: GraphEdgeOrigin::ScaleMembership {
        scale_id: self.id.clone(),
        ordinal: member.ordinal,
      },
    }
  }

  fn node_key(&self) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Scale, self.id.clone())
  }
}

/// Source of an edge returned by a graph read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphEdgeOrigin {
  /// The stored authoritative direction was read directly.
  Canonical,
  /// The stored relation was read from its target and inverse-projected in memory.
  InverseProjection,
  /// Adjacent members of an ordered scale were projected in memory.
  ScaleAdjacency {
    /// Scale that owns the ordered member assertions.
    scale_id: CanonicalId,
  },
  /// Membership in an ordered scale was projected in memory.
  ScaleMembership {
    /// Scale that owns the membership assertion.
    scale_id: CanonicalId,
    /// Stable source-qualified member ordinal.
    ordinal: u32,
  },
}

/// Stable graph edge DTO returned by bounded graph reads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEdge {
  /// Stable stored or derived public identifier.
  pub id: GraphEdgeId,
  /// Node from which this projection is read.
  pub source: GraphNodeKey,
  /// Adjacent node reached by this projection.
  pub target: GraphNodeKey,
  /// Learner-visible relation direction and family.
  pub relation_type: GraphRelationType,
  /// Whether source-to-target direction carries semantic meaning.
  pub directed: bool,
  /// Stored relation version when feedback can be pinned; `None` for derived edges.
  pub relation_version: Option<RelationVersion>,
  /// Supporting canonical evidence.
  pub evidence: GraphEvidence,
  /// Source-qualified scope restriction.
  pub scope: GraphScope,
  /// Allowed feedback dimensions; every derived scale edge exposes an empty set.
  pub feedback_capabilities: BTreeSet<GraphFeedbackCapability>,
  /// Deterministic score components and ranker version.
  pub ranking: GraphRanking,
  /// Whether this edge is stored, inverse-projected, or scale-derived.
  pub origin: GraphEdgeOrigin,
}

impl GraphEdge {
  /// Returns whether feedback may be attached to this exact edge version.
  pub fn accepts_feedback(&self) -> bool {
    self.relation_version.is_some() && !self.feedback_capabilities.is_empty()
  }

  /// Returns the stable ordering key for descending rank pagination.
  pub fn ordering_key(&self) -> GraphEdgeOrderingKey {
    GraphEdgeOrderingKey {
      display_rank: self.ranking.display_rank,
      edge_id: self.id.clone(),
      source: self.source.clone(),
      target: self.target.clone(),
      relation_type: self.relation_type,
    }
  }
}

/// Fully deterministic tie-breaker for graph edge ranking and cursors.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEdgeOrderingKey {
  /// Descending display score, applied by [`compare_graph_edges`].
  pub display_rank: GraphScore,
  /// Stable edge public ID.
  pub edge_id: GraphEdgeId,
  /// Stable projection source key.
  pub source: GraphNodeKey,
  /// Stable projection target key.
  pub target: GraphNodeKey,
  /// Stable relation-type tie breaker.
  pub relation_type: GraphRelationType,
}

/// Compares edges in their public deterministic order.
pub fn compare_graph_edges(left: &GraphEdge, right: &GraphEdge) -> std::cmp::Ordering {
  compare_graph_edge_ordering_keys(&left.ordering_key(), &right.ordering_key())
}

/// Compares edge ordering keys in the public deterministic order.
pub fn compare_graph_edge_ordering_keys(
  left: &GraphEdgeOrderingKey,
  right: &GraphEdgeOrderingKey,
) -> std::cmp::Ordering {
  right
    .display_rank
    .cmp(&left.display_rank)
    .then_with(|| left.edge_id.cmp(&right.edge_id))
    .then_with(|| left.source.cmp(&right.source))
    .then_with(|| left.target.cmp(&right.target))
    .then_with(|| left.relation_type.cmp(&right.relation_type))
}

/// Optional relation-type filter applied before node and edge limits.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphFilter {
  /// Included relation types; an empty set includes every relation type.
  pub relation_types: BTreeSet<GraphRelationType>,
}

impl GraphFilter {
  /// Returns whether `relation_type` is allowed by this filter.
  pub fn allows(&self, relation_type: GraphRelationType) -> bool {
    self.relation_types.is_empty() || self.relation_types.contains(&relation_type)
  }
}

/// Cursor used to resume deterministic edges adjacent to one typed root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphCursor {
  /// Root node whose neighbor ordering is being resumed.
  pub root: GraphNodeKey,
  /// Content version against which the ordering was issued.
  pub content: GraphContentVersion,
  /// Exact normalized relation filter against which the ordering was issued.
  pub filter: GraphFilter,
  /// Last ordering key safely processed on the preceding page.
  ///
  /// This can refer to an omitted incomplete edge so a later complete edge is never blocked or
  /// skipped by pagination.
  pub after: GraphEdgeOrderingKey,
}

/// Validated bounded request for a multi-hop graph read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphReadRequest {
  /// Typed canonical root node.
  pub root: GraphNodeKey,
  /// Number of breadth-first hops to expand from `root`.
  pub depth: u8,
  /// Maximum number of returned nodes, including root and every edge endpoint.
  pub node_limit: usize,
  /// Maximum number of returned edges.
  pub edge_limit: usize,
  /// Relation types eligible for traversal.
  pub filter: GraphFilter,
}

impl GraphReadRequest {
  /// Creates a graph request with explicit bounded limits.
  ///
  /// # Errors
  ///
  /// Returns an error when depth or either limit exceeds the graph contract.
  pub fn new(
    root: GraphNodeKey,
    depth: u8,
    node_limit: usize,
    edge_limit: usize,
    filter: GraphFilter,
  ) -> Result<Self, GraphValidationError> {
    if depth > MAX_GRAPH_DEPTH {
      return Err(GraphValidationError::InvalidDepth);
    }
    if !(1..=MAX_GRAPH_NODE_LIMIT).contains(&node_limit) {
      return Err(GraphValidationError::InvalidNodeLimit);
    }
    if !(1..=MAX_GRAPH_EDGE_LIMIT).contains(&edge_limit) {
      return Err(GraphValidationError::InvalidEdgeLimit);
    }
    Ok(Self {
      root,
      depth,
      node_limit,
      edge_limit,
      filter,
    })
  }

  /// Creates the default depth-one graph request.
  pub fn defaults(root: GraphNodeKey) -> Self {
    Self {
      root,
      depth: DEFAULT_GRAPH_DEPTH,
      node_limit: DEFAULT_GRAPH_NODE_LIMIT,
      edge_limit: DEFAULT_GRAPH_EDGE_LIMIT,
      filter: GraphFilter::default(),
    }
  }
}

/// Validated paginated request for one typed node's direct neighbors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNeighborRequest {
  /// Node whose direct projections are requested.
  pub root: GraphNodeKey,
  /// Maximum number of returned nodes, including root and every edge endpoint.
  pub node_limit: usize,
  /// Maximum number of returned edges and endpoint nodes on this page.
  pub edge_limit: usize,
  /// Relation types eligible for expansion.
  pub filter: GraphFilter,
  /// Prior page cursor, if any.
  pub cursor: Option<GraphCursor>,
}

impl GraphNeighborRequest {
  /// Creates a direct-neighbor request with a bounded page size.
  ///
  /// # Errors
  ///
  /// Returns an error when either limit cannot produce a complete direct-neighbor page.
  pub fn new(
    root: GraphNodeKey,
    node_limit: usize,
    edge_limit: usize,
    filter: GraphFilter,
    cursor: Option<GraphCursor>,
  ) -> Result<Self, GraphValidationError> {
    if !(2..=MAX_GRAPH_NODE_LIMIT).contains(&node_limit) {
      return Err(GraphValidationError::InvalidNeighborNodeLimit);
    }
    if !(1..=MAX_GRAPH_EDGE_LIMIT).contains(&edge_limit) {
      return Err(GraphValidationError::InvalidEdgeLimit);
    }
    Ok(Self {
      root,
      node_limit,
      edge_limit,
      filter,
      cursor,
    })
  }

  /// Creates the default direct-neighbor request for `root`.
  pub fn defaults(root: GraphNodeKey) -> Self {
    Self {
      root,
      node_limit: DEFAULT_GRAPH_NODE_LIMIT,
      edge_limit: DEFAULT_GRAPH_EDGE_LIMIT,
      filter: GraphFilter::default(),
      cursor: None,
    }
  }
}

/// Internally complete bounded topology returned by graph and neighbor expansion reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphReadResult {
  /// Pinned public graph content and ranking versions.
  pub content: GraphContentVersion,
  /// Typed root node requested by the caller.
  pub root: GraphNodeKey,
  /// Every returned node, including every edge endpoint.
  pub nodes: Vec<GraphNode>,
  /// Deterministically ranked bounded edge projections.
  pub edges: Vec<GraphEdge>,
  /// Whether eligible topology was omitted because of a cap or a remaining page.
  pub truncated: bool,
  /// Cursor for the next direct-neighbor page, when this result is paginated.
  pub next_cursor: Option<GraphCursor>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{
    canonical::{
      CanonicalStatus, EvidenceFragment, EvidenceKind, LexicalSource, SourcePermissions,
    },
    canonical_content::{
      CanonicalEvidenceLineage, CanonicalEvidenceOrigin, GeneratedEvidenceReview,
    },
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn key(value: &str) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Sense, id(value))
  }

  fn evidence() -> GraphEvidence {
    GraphEvidence::new(vec![id("evidence-1")], EvidenceConfidence::High).unwrap()
  }

  fn ranking(score: u16) -> GraphRanking {
    GraphRanking {
      display_rank: GraphScore::new(score).unwrap(),
      components: GraphScoreComponents {
        evidence: GraphScore::new(score).unwrap(),
        community: None,
        pedagogical: None,
      },
      ranking_version: "graph-rank-v1".to_string(),
    }
  }

  fn relation(relation_type: GraphRelationType) -> StoredGraphRelation {
    StoredGraphRelation {
      edge_id: GraphEdgeId::stored(id("edge-1")),
      relation_version: RelationVersion::new(1).unwrap(),
      source: key("sense-a"),
      target: key("sense-b"),
      relation_type,
      evidence: evidence(),
      scope: GraphScope::default(),
      feedback_capabilities: BTreeSet::from([GraphFeedbackCapability::Accuracy]),
      ranking: ranking(9_000),
    }
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

  fn lineage(release: &str, embedding: bool) -> CanonicalEvidenceLineage {
    let source_id = id("source-1");
    let permissions = permissions(embedding);
    CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".to_string(),
        version: "2026-01".to_string(),
        license: "reviewed".to_string(),
        attribution: Some("Reviewed source".to_string()),
        permissions,
      },
      EvidenceFragment {
        id: id("evidence-1"),
        source_id,
        source_reference: "entry-1".to_string(),
        release_id: id(release),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "A reviewed definition.".to_string(),
        content_hash: "sha256:evidence".to_string(),
        permissions,
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap()
  }

  fn published(relation_type: GraphRelationType) -> PublishedRelationship {
    let release = id("release-1");
    let rule = relation_type.rule();
    PublishedRelationship {
      relation: relation(relation_type),
      relation_release_id: release.clone(),
      source_release_id: release.clone(),
      target_release_id: release,
      declared_wire_relation: rule.qdrant_wire_name.unwrap_or("unresolved").to_string(),
      declared_inverse: rule.inverse,
      evidence_lineage: vec![lineage("release-1", true)],
      verification_state: RelationshipVerificationState::Verified,
    }
  }

  #[test]
  fn edge_id_parsing_accepts_stored_and_derived_public_identifiers() {
    assert_eq!(
      GraphEdgeId::parse("derived:scale:release-1:temperature:warm:hot")
        .unwrap()
        .as_str(),
      "derived:scale:release-1:temperature:warm:hot"
    );
    assert!(GraphEdgeId::parse("   ").is_err());
  }

  #[test]
  fn inverse_projection_preserves_relation_version_and_feedback_capability() {
    let edge = relation(GraphRelationType::Hypernym)
      .project_from(&key("sense-b"))
      .unwrap();

    assert_eq!(edge.source, key("sense-b"));
    assert_eq!(edge.target, key("sense-a"));
    assert_eq!(edge.relation_type, GraphRelationType::Hyponym);
    assert_eq!(
      edge.relation_version,
      Some(RelationVersion::new(1).unwrap())
    );
    assert!(edge.accepts_feedback());
    assert_eq!(edge.origin, GraphEdgeOrigin::InverseProjection);
  }

  #[test]
  fn scale_projection_is_derived_and_never_feedback_enabled() {
    let scale = SemanticScale {
      id: id("temperature"),
      release_id: id("release-1"),
      label: "temperature".to_string(),
      scope: GraphScope::default(),
      evidence: evidence(),
      members: vec![
        SemanticScaleMember {
          sense: key("warm"),
          ordinal: 1,
          evidence: evidence(),
        },
        SemanticScaleMember {
          sense: key("hot"),
          ordinal: 2,
          evidence: evidence(),
        },
      ],
      ranking: ranking(8_000),
    };

    let edge = scale.project_from(&key("warm")).pop().unwrap();

    assert_eq!(edge.relation_type, GraphRelationType::HigherDegree);
    assert_eq!(edge.relation_version, None);
    assert!(edge.feedback_capabilities.is_empty());
    assert!(!edge.accepts_feedback());
    assert!(matches!(
      edge.origin,
      GraphEdgeOrigin::ScaleAdjacency { .. }
    ));
  }

  #[test]
  fn scale_root_projects_ordered_non_feedback_memberships() {
    let scale = SemanticScale {
      id: id("temperature"),
      release_id: id("release-1"),
      label: "temperature".to_string(),
      scope: GraphScope::default(),
      evidence: evidence(),
      members: vec![
        SemanticScaleMember {
          sense: key("hot"),
          ordinal: 2,
          evidence: evidence(),
        },
        SemanticScaleMember {
          sense: key("warm"),
          ordinal: 1,
          evidence: evidence(),
        },
      ],
      ranking: ranking(8_000),
    };
    let root = GraphNodeKey::new(GraphNodeKind::Scale, id("temperature"));
    let edges = scale.project_from(&root);

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].target, key("warm"));
    assert_eq!(edges[1].target, key("hot"));
    assert!(edges.iter().all(|edge| {
      edge.source == root
        && edge.relation_type == GraphRelationType::ScaleContains
        && edge.relation_version.is_none()
        && !edge.accepts_feedback()
        && matches!(&edge.origin, GraphEdgeOrigin::ScaleMembership { .. })
    }));
  }

  #[test]
  fn graph_request_rejects_unbounded_values() {
    assert!(GraphReadRequest::new(
      key("root"),
      MAX_GRAPH_DEPTH + 1,
      DEFAULT_GRAPH_NODE_LIMIT,
      DEFAULT_GRAPH_EDGE_LIMIT,
      GraphFilter::default(),
    )
    .is_err());
    assert!(GraphReadRequest::new(
      key("root"),
      1,
      MAX_GRAPH_NODE_LIMIT + 1,
      DEFAULT_GRAPH_EDGE_LIMIT,
      GraphFilter::default(),
    )
    .is_err());
    assert!(GraphNeighborRequest::new(
      key("root"),
      1,
      DEFAULT_GRAPH_EDGE_LIMIT,
      GraphFilter::default(),
      None,
    )
    .is_err());
    assert!(GraphNeighborRequest::new(
      key("root"),
      DEFAULT_GRAPH_NODE_LIMIT,
      MAX_GRAPH_EDGE_LIMIT + 1,
      GraphFilter::default(),
      None,
    )
    .is_err());
  }

  #[test]
  fn edge_ordering_is_descending_then_stable() {
    let mut later = relation(GraphRelationType::Hypernym)
      .project_from(&key("sense-a"))
      .unwrap();
    later.id = GraphEdgeId::stored(id("edge-z"));
    let earlier = relation(GraphRelationType::Hypernym)
      .project_from(&key("sense-a"))
      .unwrap();
    let mut edges = [later, earlier];

    edges.sort_by(compare_graph_edges);

    assert_eq!(edges[0].id.as_str(), "edge-1");
  }

  #[test]
  fn relationship_registry_maps_taxonomy_direction_explicitly() {
    let broader_to_narrower = GraphRelationType::Hypernym.rule();
    let narrower_to_broader = GraphRelationType::Hyponym.rule();

    assert_eq!(
      broader_to_narrower.require_qdrant_wire_name(),
      Ok("has_subtype")
    );
    assert_eq!(narrower_to_broader.require_qdrant_wire_name(), Ok("is_a"));
    assert_eq!(broader_to_narrower.inverse, GraphRelationType::Hyponym);
    assert_eq!(narrower_to_broader.inverse, GraphRelationType::Hypernym);
    assert_eq!(
      broader_to_narrower.direction,
      RelationshipDirection::Directed
    );
  }

  #[test]
  fn symmetric_registry_rules_preserve_inverse_identity() {
    for relation_type in [
      GraphRelationType::Synonym,
      GraphRelationType::Antonym,
      GraphRelationType::TranslationEquivalent,
    ] {
      let rule = relation_type.rule();
      assert_eq!(rule.direction, RelationshipDirection::Symmetric);
      assert_eq!(rule.inverse, relation_type);
      assert!(rule.requires_evidence);
      assert!(rule.requires_confidence);
    }
  }

  #[test]
  fn relationship_registry_freezes_every_v1_wire_name_and_property() {
    let expected = [
      (GraphRelationType::Synonym, "synonym"),
      (GraphRelationType::NearSynonym, "near_synonym"),
      (
        GraphRelationType::TranslationEquivalent,
        "translation_equivalent",
      ),
      (GraphRelationType::Antonym, "antonym"),
      (GraphRelationType::Hypernym, "has_subtype"),
      (GraphRelationType::Hyponym, "is_a"),
      (GraphRelationType::Holonym, "has_part"),
      (GraphRelationType::Meronym, "part_of"),
      (GraphRelationType::ConfusableWith, "confusable_with"),
      (GraphRelationType::AssociatedWith, "associated_with"),
      (GraphRelationType::InflectionOf, "inflection_of"),
      (GraphRelationType::HasInflection, "has_inflection"),
      (
        GraphRelationType::DerivationallyRelatedTo,
        "derivationally_related_to",
      ),
      (
        GraphRelationType::EtymologicallyDerivedFrom,
        "etymologically_derived_from",
      ),
      (
        GraphRelationType::EtymologicalSourceOf,
        "etymological_source_of",
      ),
      (
        GraphRelationType::ConstructionMember,
        "member_of_construction",
      ),
      (
        GraphRelationType::HasConstructionMember,
        "has_construction_member",
      ),
      (GraphRelationType::ScaleContains, "scale_contains"),
      (GraphRelationType::MemberOfScale, "member_of_scale"),
      (GraphRelationType::LowerDegree, "lower_degree_than"),
      (GraphRelationType::HigherDegree, "higher_degree_than"),
    ];

    for (relation_type, wire_name) in expected {
      let rule = relation_type.rule();
      assert_eq!(rule.require_qdrant_wire_name(), Ok(wire_name));
      assert_eq!(rule.inverse.inverse(), relation_type);
      assert_eq!(rule.causality, RelationshipProperty::NotApplicable);
      assert_eq!(
        rule.transitivity,
        if matches!(
          relation_type,
          GraphRelationType::Hypernym | GraphRelationType::Hyponym
        ) {
          RelationshipProperty::Declared
        } else {
          RelationshipProperty::NotApplicable
        }
      );
    }
  }

  #[test]
  fn relationship_registry_rejects_invalid_endpoint_kinds() {
    let rule = GraphRelationType::ConstructionMember.rule();
    assert_eq!(
      rule.validate_endpoint_kinds(GraphNodeKind::Scale, GraphNodeKind::Construction),
      Err(GraphValidationError::InvalidRelationSourceKind)
    );
    assert_eq!(
      rule.validate_endpoint_kinds(GraphNodeKind::Sense, GraphNodeKind::Sense),
      Err(GraphValidationError::InvalidRelationTargetKind)
    );
  }

  #[test]
  fn publication_requires_verified_same_release_relationships() {
    let mut published = published(GraphRelationType::Hypernym);
    assert_eq!(published.validate(), Ok(()));

    published.verification_state = RelationshipVerificationState::Exploratory;
    assert_eq!(
      published.validate(),
      Err(GraphValidationError::UnverifiedRelationship)
    );
    published.verification_state = RelationshipVerificationState::Verified;
    published.target_release_id = id("release-2");
    assert_eq!(
      published.validate(),
      Err(GraphValidationError::RelationshipReleaseMismatch)
    );
  }

  #[test]
  fn publication_rejects_duplicate_typed_relationship_identity() {
    let first = published(GraphRelationType::Hypernym);
    let mut duplicate = first.clone();
    duplicate.relation.edge_id = GraphEdgeId::stored(id("different-edge-id"));

    assert_eq!(
      validate_published_relationships(&[first, duplicate]),
      Err(GraphValidationError::DuplicateTypedRelationship)
    );
  }

  #[test]
  fn admission_accepts_frozen_names_and_rejects_reversed_wire_direction() {
    assert_eq!(published(GraphRelationType::Synonym).validate(), Ok(()));
    let mut reversed = published(GraphRelationType::Hypernym);
    reversed.declared_wire_relation = "is_a".to_string();
    assert_eq!(
      reversed.validate(),
      Err(GraphValidationError::RelationshipDirectionMismatch)
    );
  }

  #[test]
  fn admission_rejects_invalid_inverse_declaration() {
    let mut candidate = published(GraphRelationType::Hypernym);
    candidate.declared_inverse = GraphRelationType::Hypernym;
    assert_eq!(
      candidate.validate(),
      Err(GraphValidationError::RelationshipInverseMismatch)
    );
  }

  #[test]
  fn symmetric_identity_canonicalizes_endpoint_order() {
    let first = published(GraphRelationType::Synonym);
    let mut reversed = first.clone();
    std::mem::swap(&mut reversed.relation.source, &mut reversed.relation.target);

    assert_eq!(first.identity(), reversed.identity());
  }

  #[test]
  fn distinct_relation_types_have_distinct_edge_identity() {
    let hypernym = published(GraphRelationType::Hypernym);
    let hyponym = published(GraphRelationType::Hyponym);

    assert_ne!(hypernym.identity(), hyponym.identity());
  }

  #[test]
  fn admission_requires_exact_release_bound_evidence() {
    let mut missing = published(GraphRelationType::Hypernym);
    missing.evidence_lineage.clear();
    assert_eq!(missing.validate(), Err(GraphValidationError::EmptyEvidence));

    let mut wrong_release = published(GraphRelationType::Hypernym);
    wrong_release.evidence_lineage = vec![lineage("release-2", true)];
    assert_eq!(
      wrong_release.validate(),
      Err(GraphValidationError::RelationshipEvidenceReleaseMismatch)
    );

    let mut forbidden = published(GraphRelationType::Hypernym);
    forbidden.evidence_lineage = vec![lineage("release-1", false)];
    assert_eq!(
      forbidden.validate(),
      Err(GraphValidationError::RelationshipEvidenceNotPermitted)
    );
  }

  #[test]
  fn admission_rejects_duplicate_or_contradictory_evidence_resolution() {
    let mut duplicate = published(GraphRelationType::Hypernym);
    duplicate.evidence_lineage.push(lineage("release-1", true));
    assert_eq!(
      duplicate.validate(),
      Err(GraphValidationError::RelationshipEvidenceMismatch)
    );

    let mut contradictory = published(GraphRelationType::Hypernym);
    contradictory.relation.evidence.evidence_ids = vec![id("another-evidence")];
    assert_eq!(
      contradictory.validate(),
      Err(GraphValidationError::RelationshipEvidenceMismatch)
    );
  }

  #[test]
  fn generated_evidence_must_be_reviewed_before_admission() {
    let source_id = id("source-1");
    let permissions = permissions(true);
    let result = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".to_string(),
        version: "2026-01".to_string(),
        license: "reviewed".to_string(),
        attribution: None,
        permissions,
      },
      EvidenceFragment {
        id: id("evidence-1"),
        source_id,
        source_reference: "entry-1".to_string(),
        release_id: id("release-1"),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Generated candidate.".to_string(),
        content_hash: "sha256:generated".to_string(),
        permissions,
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::Generated {
        generation_id: id("generation-1"),
        review: GeneratedEvidenceReview::Unreviewed,
      },
    );

    assert!(result.is_err());
  }

  #[test]
  fn unresolved_domain_and_condition_scopes_fail_closed() {
    let mut domain = published(GraphRelationType::Hypernym);
    domain.relation.scope.domain = Some("medicine".to_string());
    assert_eq!(
      domain.validate(),
      Err(GraphValidationError::UnresolvedRelationshipDomainScope)
    );

    let mut condition = published(GraphRelationType::Hypernym);
    condition.relation.scope.note = Some("when used figuratively".to_string());
    assert_eq!(
      condition.validate(),
      Err(GraphValidationError::UnsupportedRelationshipCondition)
    );
  }

  #[test]
  fn invalid_bounded_scope_is_rejected() {
    let mut candidate = published(GraphRelationType::Hypernym);
    candidate.relation.scope.register = Some(" ".to_string());
    assert_eq!(
      candidate.validate(),
      Err(GraphValidationError::InvalidRelationshipScope)
    );
  }

  #[test]
  fn validation_result_is_independent_of_input_order() {
    let first = published(GraphRelationType::Hypernym);
    let mut duplicate = first.clone();
    duplicate.relation.edge_id = GraphEdgeId::stored(id("another-edge"));

    let forward = validate_published_relationships(&[first.clone(), duplicate.clone()]);
    let reverse = validate_published_relationships(&[duplicate, first]);
    assert_eq!(forward, reverse);
    assert_eq!(
      forward,
      Err(GraphValidationError::DuplicateTypedRelationship)
    );
  }
}
