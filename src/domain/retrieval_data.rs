//! Bounded request and result values for the outbound retrieval-data-v1 boundary.

use std::fmt;

use thiserror::Error;

use super::{
  canonical::{CanonicalId, LanguageTag, ReleaseId},
  canonical_translation::DomainId,
  graph::GraphRelationType,
};

use super::{
  embedding_input::{NODE_DENSE_INPUT_VERSION, NODE_LEXICAL_INPUT_VERSION},
  knowledge_projection::{LEXICAL_ENCODER_IDENTITY, LEXICAL_ENCODER_REVISION},
};

/// Transport schema required by every retrieval-data operation.
pub const RETRIEVAL_DATA_SCHEMA_VERSION: &str = "retrieval-data-v1";
/// Largest result limit accepted by any retrieval-data search.
pub const MAX_RETRIEVAL_DATA_RESULTS: usize = 50;
/// Exact dense query-vector dimension frozen by retrieval-data-v1.
pub const DENSE_QUERY_VECTOR_DIMENSIONS: usize = 1_024;
/// Exact relationship-registry version accepted by retrieval-data-v1.
pub const RELATION_REGISTRY_VERSION: u32 = 1;
/// Largest sparse query vector accepted at the storage-neutral boundary.
pub const MAX_SPARSE_VECTOR_TERMS: usize = 4_096;
/// Largest opaque pagination cursor accepted from island-port.
pub const MAX_RETRIEVAL_CURSOR_BYTES: usize = 512;

/// Closed validation failures that occur before an outbound request is sent.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RetrievalDataValidationError {
  /// The request limit is zero or exceeds the interface-wide bound.
  #[error("retrieval-data result limit is invalid")]
  InvalidLimit,
  /// A required filter list is empty or a filter list exceeds its bound.
  #[error("retrieval-data filters are invalid")]
  InvalidFilters,
  /// A dense or sparse query vector is empty, oversized, non-finite, or malformed.
  #[error("retrieval-data query vector is invalid")]
  InvalidVector,
  /// A cursor is empty, oversized, or not identifier-safe ASCII.
  #[error("retrieval-data cursor is invalid")]
  InvalidCursor,
  /// A score is not finite or falls outside the closed zero-to-one range.
  #[error("retrieval-data score is invalid")]
  InvalidScore,
  /// A relation name is not an exact value from the frozen v1 registry.
  #[error("retrieval-data relation is not in the frozen registry")]
  InvalidRelation,
  /// A response candidate contradicted the request's eligibility filters.
  #[error("retrieval-data candidate is ineligible")]
  IneligibleCandidate,
}

/// Publication lifecycle values that may be selected before limiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetrievalPublicationState {
  /// Only content in an immutable published collection is eligible.
  Published,
}

/// Verification classes kept separate during retrieval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetrievalVerificationState {
  /// Reviewed canonical content backed by eligible evidence.
  Verified,
  /// Request-time candidate content that must never be presented as a fact.
  Exploratory,
}

/// Direction selected for a direct-neighbor lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeighborDirection {
  /// Only edges whose stored source is the selected root.
  Outgoing,
  /// Only edges whose stored target is the selected root.
  Incoming,
  /// Eligible direct edges in either direction.
  Both,
}

/// Closed canonical node-family catalog understood by retrieval-data-v1.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetrievalNodeType {
  /// One independently selectable lexical sense.
  LexicalSense,
  /// One established multi-token lexical phrase.
  Phrase,
  /// One explicitly aligned multilingual term.
  MultilingualTerm,
  /// One non-lexical canonical concept.
  Concept,
  /// One named entity.
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
  /// One material.
  Material,
  /// One instrument.
  Instrument,
  /// One method.
  Method,
  /// One technology.
  Technology,
  /// One application of a method or technology.
  Application,
  /// One canonical standard.
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
  /// One structured grammar pattern.
  GrammarPattern,
  /// One structured collocation.
  Collocation,
  /// One reviewed misconception.
  Misconception,
  /// One canonical domain.
  Domain,
  /// One complete evidence-backed semantic scale.
  SemanticScale,
}

/// Closed node-nomination mechanisms returned by retrieval-data-v1.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeMatchMechanism {
  /// Dense semantic-vector nomination.
  Dense,
  /// Sparse lexical-vector nomination.
  Sparse,
  /// Exact canonical-label match.
  CanonicalLabel,
  /// Published alias match.
  Alias,
  /// Reviewed translation match.
  Translation,
  /// Published transliteration match.
  Transliteration,
  /// Published abbreviation match.
  Abbreviation,
  /// Canonical formula match.
  Formula,
  /// Canonical domain-term match.
  DomainTerm,
}

impl NodeMatchMechanism {
  /// Parses one exact retrieval-data-v1 wire value.
  pub fn from_wire_name(value: &str) -> Result<Self, RetrievalDataValidationError> {
    match value {
      "dense" => Ok(Self::Dense),
      "sparse" => Ok(Self::Sparse),
      "canonical_label" => Ok(Self::CanonicalLabel),
      "alias" => Ok(Self::Alias),
      "translation" => Ok(Self::Translation),
      "transliteration" => Ok(Self::Transliteration),
      "abbreviation" => Ok(Self::Abbreviation),
      "formula" => Ok(Self::Formula),
      "domain_term" => Ok(Self::DomainTerm),
      _ => Err(RetrievalDataValidationError::IneligibleCandidate),
    }
  }

  /// Returns the exact retrieval-data-v1 wire value.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Dense => "dense",
      Self::Sparse => "sparse",
      Self::CanonicalLabel => "canonical_label",
      Self::Alias => "alias",
      Self::Translation => "translation",
      Self::Transliteration => "transliteration",
      Self::Abbreviation => "abbreviation",
      Self::Formula => "formula",
      Self::DomainTerm => "domain_term",
    }
  }
}

impl RetrievalNodeType {
  /// Parses one exact node-family wire name from the closed v1 catalog.
  ///
  /// # Errors
  ///
  /// Returns an error rather than accepting an unknown or derived family name.
  pub fn new(value: impl Into<String>) -> Result<Self, RetrievalDataValidationError> {
    let value = value.into();
    match value.as_str() {
      "lexical_sense" => Ok(Self::LexicalSense),
      "phrase" => Ok(Self::Phrase),
      "multilingual_term" => Ok(Self::MultilingualTerm),
      "concept" => Ok(Self::Concept),
      "entity" => Ok(Self::Entity),
      "phenomenon" => Ok(Self::Phenomenon),
      "mechanism" => Ok(Self::Mechanism),
      "process" => Ok(Self::Process),
      "equation" => Ok(Self::Equation),
      "quantity" => Ok(Self::Quantity),
      "material" => Ok(Self::Material),
      "instrument" => Ok(Self::Instrument),
      "method" => Ok(Self::Method),
      "technology" => Ok(Self::Technology),
      "application" => Ok(Self::Application),
      "standard" => Ok(Self::Standard),
      "organization" => Ok(Self::Organization),
      "person" => Ok(Self::Person),
      "place" => Ok(Self::Place),
      "idiom" => Ok(Self::Idiom),
      "metaphor" => Ok(Self::Metaphor),
      "grammar_pattern" => Ok(Self::GrammarPattern),
      "collocation" => Ok(Self::Collocation),
      "misconception" => Ok(Self::Misconception),
      "domain" => Ok(Self::Domain),
      "semantic_scale" => Ok(Self::SemanticScale),
      _ => Err(RetrievalDataValidationError::InvalidFilters),
    }
  }

  /// Returns the exact v1 wire name.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::LexicalSense => "lexical_sense",
      Self::Phrase => "phrase",
      Self::MultilingualTerm => "multilingual_term",
      Self::Concept => "concept",
      Self::Entity => "entity",
      Self::Phenomenon => "phenomenon",
      Self::Mechanism => "mechanism",
      Self::Process => "process",
      Self::Equation => "equation",
      Self::Quantity => "quantity",
      Self::Material => "material",
      Self::Instrument => "instrument",
      Self::Method => "method",
      Self::Technology => "technology",
      Self::Application => "application",
      Self::Standard => "standard",
      Self::Organization => "organization",
      Self::Person => "person",
      Self::Place => "place",
      Self::Idiom => "idiom",
      Self::Metaphor => "metaphor",
      Self::GrammarPattern => "grammar_pattern",
      Self::Collocation => "collocation",
      Self::Misconception => "misconception",
      Self::Domain => "domain",
      Self::SemanticScale => "semantic_scale",
    }
  }
}

/// One exact relation name backed by the frozen relationship registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetrievalRelation(GraphRelationType);

impl RetrievalRelation {
  /// Resolves an exact Qdrant wire name through the frozen registry.
  ///
  /// # Errors
  ///
  /// Returns an error rather than accepting aliases or deriving enum spelling.
  pub fn from_wire_name(value: &str) -> Result<Self, RetrievalDataValidationError> {
    ALL_RELATIONS
      .iter()
      .copied()
      .find(|relation| relation.rule().qdrant_wire_name == Some(value))
      .map(Self)
      .ok_or(RetrievalDataValidationError::InvalidRelation)
  }

  /// Returns the exact registry-backed wire name.
  pub fn wire_name(self) -> &'static str {
    self.0.rule().qdrant_wire_name.unwrap_or("")
  }

  /// Returns the internal typed registry identity.
  pub const fn relation_type(self) -> GraphRelationType {
    self.0
  }
}

const ALL_RELATIONS: &[GraphRelationType] = &[
  GraphRelationType::Synonym,
  GraphRelationType::NearSynonym,
  GraphRelationType::TranslationEquivalent,
  GraphRelationType::Antonym,
  GraphRelationType::Hypernym,
  GraphRelationType::Hyponym,
  GraphRelationType::Holonym,
  GraphRelationType::Meronym,
  GraphRelationType::ConfusableWith,
  GraphRelationType::AssociatedWith,
  GraphRelationType::InflectionOf,
  GraphRelationType::HasInflection,
  GraphRelationType::DerivationallyRelatedTo,
  GraphRelationType::EtymologicallyDerivedFrom,
  GraphRelationType::EtymologicalSourceOf,
  GraphRelationType::ConstructionMember,
  GraphRelationType::HasConstructionMember,
  GraphRelationType::ScaleContains,
  GraphRelationType::MemberOfScale,
  GraphRelationType::LowerDegree,
  GraphRelationType::HigherDegree,
];

/// Finite query vector used only for the current request.
#[derive(Clone, PartialEq)]
pub struct DenseQueryVector(Vec<f32>);

impl DenseQueryVector {
  /// Validates a bounded non-empty finite vector.
  ///
  /// # Errors
  ///
  /// Returns an error for empty, oversized, or non-finite input.
  pub fn new(values: Vec<f32>) -> Result<Self, RetrievalDataValidationError> {
    if values.len() != DENSE_QUERY_VECTOR_DIMENSIONS
      || values.iter().any(|value| !value.is_finite())
    {
      return Err(RetrievalDataValidationError::InvalidVector);
    }
    Ok(Self(values))
  }

  /// Returns the ephemeral vector values.
  pub fn values(&self) -> &[f32] {
    &self.0
  }
}

impl fmt::Debug for DenseQueryVector {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("DenseQueryVector(REDACTED)")
  }
}

/// Sorted sparse-vector indices and finite values used for one request.
#[derive(Clone, PartialEq)]
pub struct SparseQueryVector {
  indices: Vec<u32>,
  values: Vec<f32>,
}

impl SparseQueryVector {
  /// Validates equal lengths, strict index ordering, and finite values.
  ///
  /// # Errors
  ///
  /// Returns an error when the sparse representation is empty, oversized, or malformed.
  pub fn new(indices: Vec<u32>, values: Vec<f32>) -> Result<Self, RetrievalDataValidationError> {
    if indices.is_empty()
      || indices.len() > MAX_SPARSE_VECTOR_TERMS
      || indices.len() != values.len()
      || indices.windows(2).any(|pair| pair[0] >= pair[1])
      || values.iter().any(|value| !value.is_finite())
    {
      return Err(RetrievalDataValidationError::InvalidVector);
    }
    Ok(Self { indices, values })
  }

  /// Returns the sorted dictionary indices.
  pub fn indices(&self) -> &[u32] {
    &self.indices
  }

  /// Returns the weights paired with [`Self::indices`].
  pub fn values(&self) -> &[f32] {
    &self.values
  }
}

impl fmt::Debug for SparseQueryVector {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("SparseQueryVector(REDACTED)")
  }
}

/// Filters applied by island-port before result limiting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalFilters {
  /// Duplicate release field that must equal the request-context pin.
  pub release_id: ReleaseId,
  /// Eligible publication states.
  pub publication_states: Vec<RetrievalPublicationState>,
  /// Eligible verification states.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Optional node-family restrictions.
  pub node_types: Vec<RetrievalNodeType>,
  /// Optional language restrictions.
  pub languages: Vec<LanguageTag>,
  /// Optional dialect restrictions.
  pub dialects: Vec<LanguageTag>,
  /// Optional bounded region labels.
  pub regions: Vec<String>,
  /// Optional bounded historical-period labels.
  pub periods: Vec<String>,
  /// Optional canonical-domain restrictions.
  pub domain_ids: Vec<DomainId>,
  /// Optional evidence allow-list.
  pub eligible_evidence_ids: Vec<CanonicalId>,
}

impl RetrievalFilters {
  /// Creates minimum safe filters for one immutable release.
  pub fn verified(release_id: ReleaseId) -> Self {
    Self {
      release_id,
      publication_states: vec![RetrievalPublicationState::Published],
      verification_states: vec![RetrievalVerificationState::Verified],
      node_types: Vec::new(),
      languages: Vec::new(),
      dialects: Vec::new(),
      regions: Vec::new(),
      periods: Vec::new(),
      domain_ids: Vec::new(),
      eligible_evidence_ids: Vec::new(),
    }
  }

  /// Validates all filter-list and text bounds.
  ///
  /// # Errors
  ///
  /// Returns an error for missing lifecycle filters or an oversized/malformed list.
  pub fn validate(&self) -> Result<(), RetrievalDataValidationError> {
    if self.publication_states.is_empty()
      || self.verification_states.is_empty()
      || [
        self.node_types.len(),
        self.languages.len(),
        self.dialects.len(),
        self.regions.len(),
        self.periods.len(),
        self.domain_ids.len(),
        self.eligible_evidence_ids.len(),
      ]
      .into_iter()
      .any(|length| length > MAX_RETRIEVAL_DATA_RESULTS)
      || self.regions.iter().any(|value| !valid_name(value, 64))
      || self.periods.iter().any(|value| !valid_name(value, 64))
    {
      return Err(RetrievalDataValidationError::InvalidFilters);
    }
    Ok(())
  }
}

/// Search request for canonical nodes.
#[derive(Clone, Debug)]
pub struct NodeSearchRequest {
  /// Ephemeral dense nomination signal.
  pub dense_vector: DenseQueryVector,
  /// Ephemeral sparse nomination signal.
  pub sparse_vector: SparseQueryVector,
  /// Exact query-encoder revisions and input contracts expected from the selected projection.
  pub execution: NodeProjectionExecutionExpectation,
  /// Eligibility filters applied before limiting.
  pub filters: RetrievalFilters,
  /// Maximum candidates returned.
  pub limit: usize,
}

/// Query-side execution versions that the immutable node projection must echo exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeProjectionExecutionExpectation {
  /// Immutable dense artifact revision used for the ephemeral query vector.
  pub dense_artifact_revision: String,
  /// Exact dense canonical-input contract.
  pub dense_input_specification: String,
  /// Frozen lexical encoder identity.
  pub lexical_encoder_identity: String,
  /// Frozen lexical encoder revision.
  pub lexical_encoder_revision: String,
  /// Exact lexical canonical-input contract.
  pub lexical_input_specification: String,
}

impl NodeProjectionExecutionExpectation {
  /// Creates the closed node-projection expectation for one immutable dense artifact.
  ///
  /// # Errors
  ///
  /// Returns an error when the artifact revision is blank, untrimmed, or oversized.
  pub fn v1(
    dense_artifact_revision: impl Into<String>,
  ) -> Result<Self, RetrievalDataValidationError> {
    let dense_artifact_revision = dense_artifact_revision.into();
    if !valid_version(&dense_artifact_revision) {
      return Err(RetrievalDataValidationError::InvalidFilters);
    }
    Ok(Self {
      dense_artifact_revision,
      dense_input_specification: NODE_DENSE_INPUT_VERSION.to_string(),
      lexical_encoder_identity: LEXICAL_ENCODER_IDENTITY.to_string(),
      lexical_encoder_revision: LEXICAL_ENCODER_REVISION.to_string(),
      lexical_input_specification: NODE_LEXICAL_INPUT_VERSION.to_string(),
    })
  }

  /// Validates that no caller substituted a competing input or encoder contract.
  pub fn validate(&self) -> Result<(), RetrievalDataValidationError> {
    if !valid_version(&self.dense_artifact_revision)
      || self.dense_input_specification != NODE_DENSE_INPUT_VERSION
      || self.lexical_encoder_identity != LEXICAL_ENCODER_IDENTITY
      || self.lexical_encoder_revision != LEXICAL_ENCODER_REVISION
      || self.lexical_input_specification != NODE_LEXICAL_INPUT_VERSION
    {
      return Err(RetrievalDataValidationError::InvalidFilters);
    }
    Ok(())
  }
}

/// Server-observed immutable node-projection execution proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeProjectionExecutionProof {
  /// Immutable physical node collection selected for the pinned release.
  pub collection_id: CanonicalId,
  /// SHA-256 content hash of that frozen collection.
  pub collection_content_hash: String,
  /// Dense artifact revision observed by the projection.
  pub dense_artifact_revision: String,
  /// Dense canonical-input contract observed by the projection.
  pub dense_input_specification: String,
  /// Lexical encoder identity observed by the projection.
  pub lexical_encoder_identity: String,
  /// Lexical encoder revision observed by the projection.
  pub lexical_encoder_revision: String,
  /// Lexical canonical-input contract observed by the projection.
  pub lexical_input_specification: String,
}

impl NodeProjectionExecutionProof {
  /// Validates the immutable collection proof against the query-side expectation.
  pub fn validate_against(
    &self,
    expected: &NodeProjectionExecutionExpectation,
  ) -> Result<(), RetrievalDataValidationError> {
    expected.validate()?;
    if !valid_sha256(&self.collection_content_hash)
      || self.dense_artifact_revision != expected.dense_artifact_revision
      || self.dense_input_specification != expected.dense_input_specification
      || self.lexical_encoder_identity != expected.lexical_encoder_identity
      || self.lexical_encoder_revision != expected.lexical_encoder_revision
      || self.lexical_input_specification != expected.lexical_input_specification
    {
      return Err(RetrievalDataValidationError::IneligibleCandidate);
    }
    Ok(())
  }
}

/// Index request for complete semantic-scale candidates.
#[derive(Clone, Debug)]
pub struct ScaleSearchRequest {
  /// Canonical node that must be an explicit scale member.
  pub member_node_id: CanonicalId,
  /// Eligibility filters applied before limiting.
  pub filters: RetrievalFilters,
  /// Maximum candidates returned.
  pub limit: usize,
}

/// Search request for canonical structured relationships.
#[derive(Clone, Debug)]
pub struct EdgeSearchRequest {
  /// Ephemeral dense relationship nomination signal.
  pub dense_vector: DenseQueryVector,
  /// Ephemeral sparse relationship nomination signal.
  pub sparse_vector: SparseQueryVector,
  /// Eligibility filters applied before limiting.
  pub filters: RetrievalFilters,
  /// Exact frozen relation families eligible for the result.
  pub relation_types: Vec<RetrievalRelation>,
  /// Applicable canonical senses.
  pub applicable_sense_ids: Vec<CanonicalId>,
  /// Maximum candidates returned.
  pub limit: usize,
}

/// Direct one-hop neighbor request for one canonical root.
#[derive(Clone, Debug)]
pub struct NeighborSearchRequest {
  /// Canonical root whose endpoint indexes are read.
  pub node_id: CanonicalId,
  /// Incoming, outgoing, or both endpoint directions.
  pub direction: NeighborDirection,
  /// Exact frozen relation families eligible for the result.
  pub relation_types: Vec<RetrievalRelation>,
  /// Eligible verification states.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Optional language restrictions.
  pub languages: Vec<LanguageTag>,
  /// Optional canonical-domain restrictions.
  pub domain_ids: Vec<DomainId>,
  /// Duplicate release field that must equal the request-context pin.
  pub release_id: ReleaseId,
  /// Maximum neighbors returned.
  pub limit: usize,
  /// Opaque cursor issued by the same operation and scope.
  pub cursor: Option<String>,
}

/// One validated score comparable only within the same release and model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetrievalDataScore(f32);

impl RetrievalDataScore {
  /// Validates a finite score in the inclusive zero-to-one range.
  ///
  /// # Errors
  ///
  /// Returns an error when a response score is non-finite or outside the range.
  pub fn new(value: f32) -> Result<Self, RetrievalDataValidationError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
      return Err(RetrievalDataValidationError::InvalidScore);
    }
    Ok(Self(value))
  }

  /// Returns the validated score.
  pub const fn get(self) -> f32 {
    self.0
  }
}

/// Minimal payload retained for a nominated canonical node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeCandidatePayload {
  /// Frozen node family.
  pub node_type: RetrievalNodeType,
  /// Optional canonical sense identity.
  pub sense_id: Option<CanonicalId>,
  /// Canonical display label hydrated only for nomination.
  pub canonical_label: String,
  /// Verification class echoed by the projection.
  pub verification_state: RetrievalVerificationState,
}

/// One node nomination; it is not factual proof.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeCandidate {
  /// Canonical node identity.
  pub node_id: CanonicalId,
  /// Release-local nomination score.
  pub score: RetrievalDataScore,
  /// Closed retrieval mechanisms that nominated this candidate.
  pub matched_by: Vec<NodeMatchMechanism>,
  /// Minimal index payload requiring authoritative hydration before factual use.
  pub payload: NodeCandidatePayload,
}

/// One complete-scale pointer requiring canonical SQL hydration.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleCandidate {
  /// Canonical semantic-scale identity.
  pub scale_id: CanonicalId,
  /// Release-local nomination score.
  pub score: RetrievalDataScore,
  /// Explicit member position within the complete scale.
  pub member_position: u32,
  /// Verification class echoed by the projection.
  pub verification_state: RetrievalVerificationState,
  /// Canonical fact revisions that must be hydrated before presentation.
  pub fact_ids: Vec<CanonicalId>,
}

/// One relationship pointer requiring canonical fact hydration.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeCandidate {
  /// Publisher-owned stable edge identity.
  pub edge_id: CanonicalId,
  /// Release-local nomination score.
  pub score: RetrievalDataScore,
  /// Canonical source node.
  pub source_node_id: CanonicalId,
  /// Canonical target node.
  pub target_node_id: CanonicalId,
  /// Exact frozen registry relation.
  pub relation_type: RetrievalRelation,
  /// Relationship-registry version used for the projection.
  pub relation_registry_version: u32,
  /// Canonical fact identity to hydrate.
  pub fact_id: CanonicalId,
  /// Positive immutable fact revision.
  pub fact_revision: u32,
  /// Verification class echoed by the projection.
  pub verification_state: RetrievalVerificationState,
}

/// Minimal opposite endpoint returned with a direct edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeighborNode {
  /// Canonical opposite-node identity.
  pub node_id: CanonicalId,
  /// Frozen node family.
  pub node_type: RetrievalNodeType,
  /// Concise canonical label.
  pub canonical_label: String,
}

/// Edge metadata returned by direct-neighbor traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeighborEdge {
  /// Publisher-owned stable edge identity.
  pub edge_id: CanonicalId,
  /// Canonical source node.
  pub source_node_id: CanonicalId,
  /// Canonical target node.
  pub target_node_id: CanonicalId,
  /// Exact frozen registry relation.
  pub relation_type: RetrievalRelation,
  /// Exact frozen relationship-registry version.
  pub relation_registry_version: u32,
  /// Canonical fact identity that must be hydrated before factual use.
  pub fact_id: CanonicalId,
  /// Positive immutable fact revision that must be hydrated exactly.
  pub fact_revision: u32,
  /// Verification class echoed by the projection.
  pub verification_state: RetrievalVerificationState,
}

/// One direct edge and its opposite endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeighborCandidate {
  /// Direct canonical edge.
  pub edge: NeighborEdge,
  /// Opposite endpoint for the selected root.
  pub node: NeighborNode,
}

/// Strict result of a node search.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSearchResult {
  /// Immutable release echoed by island-port.
  pub release_id: ReleaseId,
  /// Exact immutable collection and encoder proof used for this search.
  pub execution: NodeProjectionExecutionProof,
  /// Bounded candidates in authoritative retrieval order.
  pub candidates: Vec<NodeCandidate>,
}

/// Strict result of a scale search.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleSearchResult {
  /// Immutable release echoed by island-port.
  pub release_id: ReleaseId,
  /// Bounded candidates in authoritative retrieval order.
  pub candidates: Vec<ScaleCandidate>,
}

/// Strict result of an edge search.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeSearchResult {
  /// Immutable release echoed by island-port.
  pub release_id: ReleaseId,
  /// Bounded candidates in authoritative retrieval order.
  pub candidates: Vec<EdgeCandidate>,
}

/// Strict result of a one-hop neighbor search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeighborSearchResult {
  /// Immutable release echoed by island-port.
  pub release_id: ReleaseId,
  /// Root echoed by the endpoint.
  pub root_node_id: CanonicalId,
  /// Bounded direct neighbors in authoritative retrieval order.
  pub neighbors: Vec<NeighborCandidate>,
  /// Opaque continuation token, when more eligible direct neighbors exist.
  pub next_cursor: Option<String>,
}

/// Validates one route-specific limit.
pub fn validate_limit(limit: usize) -> Result<(), RetrievalDataValidationError> {
  if !(1..=MAX_RETRIEVAL_DATA_RESULTS).contains(&limit) {
    return Err(RetrievalDataValidationError::InvalidLimit);
  }
  Ok(())
}

/// Validates one opaque cursor without interpreting its contents.
pub fn validate_cursor(cursor: &str) -> Result<(), RetrievalDataValidationError> {
  if cursor.is_empty()
    || cursor.len() > MAX_RETRIEVAL_CURSOR_BYTES
    || !cursor.bytes().all(|byte| byte.is_ascii_graphic())
  {
    return Err(RetrievalDataValidationError::InvalidCursor);
  }
  Ok(())
}

fn valid_name(value: &str, max: usize) -> bool {
  !value.is_empty()
    && value.len() <= max
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-'))
}

fn valid_version(value: &str) -> bool {
  value.trim() == value && !value.is_empty() && value.len() <= 256 && value.is_ascii()
}

fn valid_sha256(value: &str) -> bool {
  value.len() == 71
    && value.starts_with("sha256:")
    && value[7..]
      .bytes()
      .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn relation_names_are_exact_registry_values() {
    let relation = RetrievalRelation::from_wire_name("higher_degree_than").unwrap();
    assert_eq!(relation.relation_type(), GraphRelationType::HigherDegree);
    assert_eq!(relation.wire_name(), "higher_degree_than");
    assert_eq!(
      RetrievalRelation::from_wire_name("higher_degree"),
      Err(RetrievalDataValidationError::InvalidRelation)
    );
  }

  #[test]
  fn vectors_reject_unbounded_or_non_finite_values() {
    assert_eq!(
      DenseQueryVector::new(vec![f32::NAN; DENSE_QUERY_VECTOR_DIMENSIONS]),
      Err(RetrievalDataValidationError::InvalidVector)
    );
    assert_eq!(
      DenseQueryVector::new(vec![0.0; DENSE_QUERY_VECTOR_DIMENSIONS - 1]),
      Err(RetrievalDataValidationError::InvalidVector)
    );
    assert!(DenseQueryVector::new(vec![0.0; DENSE_QUERY_VECTOR_DIMENSIONS]).is_ok());
    assert_eq!(
      SparseQueryVector::new(vec![2, 2], vec![1.0, 0.5]),
      Err(RetrievalDataValidationError::InvalidVector)
    );
  }

  #[test]
  fn vector_debug_output_is_content_free() {
    let dense = DenseQueryVector::new(vec![0.1234; DENSE_QUERY_VECTOR_DIMENSIONS]).unwrap();
    let sparse = SparseQueryVector::new(vec![42], vec![0.9876]).unwrap();
    assert_eq!(format!("{dense:?}"), "DenseQueryVector(REDACTED)");
    assert_eq!(format!("{sparse:?}"), "SparseQueryVector(REDACTED)");
  }

  #[test]
  fn node_family_catalog_rejects_plausible_unknown_values() {
    assert_eq!(
      RetrievalNodeType::new("lexical_senses"),
      Err(RetrievalDataValidationError::InvalidFilters)
    );
    assert_eq!(
      RetrievalNodeType::new("custom_concept"),
      Err(RetrievalDataValidationError::InvalidFilters)
    );
    assert_eq!(
      RetrievalNodeType::new("semantic_scale").unwrap().as_str(),
      "semantic_scale"
    );
  }
}
