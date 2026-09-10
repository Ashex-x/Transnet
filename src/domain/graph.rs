//! Typed, evidence-backed graph topology and bounded traversal values.

use std::collections::BTreeSet;

use thiserror::Error;

use super::canonical::{
  CanonicalId, CanonicalValidationError, EvidenceConfidence, EvidenceId, LanguageTag,
  LexicalPartOfSpeech, ReleaseId,
};

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
  /// Concise learner-visible label.
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
  /// Creates a graph node after validating its learner-visible label.
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
  /// Similar meaning that requires a learner-visible contrast.
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

impl GraphRelationType {
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
  /// Last edge returned on the preceding page.
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
}
