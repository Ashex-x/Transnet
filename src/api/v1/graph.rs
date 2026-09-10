//! `GET /v1/graph` and bounded typed-neighbor graph-read transport contracts.
//!
//! This module translates public query parameters and opaque cursors into the bounded graph
//! application requests. It deliberately contains no graph persistence, private feedback overlay,
//! saved layout, authentication, or rendering behavior.

use std::collections::{BTreeMap, BTreeSet};

use axum::{
  extract::{
    rejection::{PathRejection, QueryRejection},
    Extension, Path, Query, State,
  },
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
  aead::{Aead, KeyInit, Payload},
  XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};

use crate::{
  api::{
    problem::{self, FieldError},
    request_id::RequestId,
    AppState,
  },
  application::{
    graph::GraphReadError,
    graph_topology_cache::{
      GraphTopologySnapshotCacheError, GraphTopologySnapshotCacheResult,
      GraphTopologySnapshotValidationError,
    },
  },
  domain::{
    canonical::{CanonicalId, EvidenceConfidence, LexicalPartOfSpeech},
    graph::{
      GraphContentVersion, GraphCursor, GraphEdge, GraphEdgeId, GraphEdgeOrderingKey,
      GraphEdgeOrigin, GraphFeedbackCapability, GraphFilter, GraphNeighborRequest, GraphNode,
      GraphNodeKey, GraphNodeKind, GraphRanking, GraphReadRequest, GraphReadResult,
      GraphRelationType, GraphScope, GraphScore, GraphValidationError, RelationVersion,
      DEFAULT_GRAPH_DEPTH, DEFAULT_GRAPH_EDGE_LIMIT, DEFAULT_GRAPH_NODE_LIMIT,
    },
  },
  ports::graph_repository::GraphRepositoryError,
};

const MAX_GRAPH_ID_LENGTH: usize = 256;
const MAX_GRAPH_RELATION_TYPES_LENGTH: usize = 512;
const MAX_GRAPH_RELATION_TYPE_COUNT: usize = 21;
const MAX_GRAPH_CURSOR_LENGTH: usize = 4_096;
const GRAPH_CURSOR_PREFIX: &str = "g2";
const GRAPH_CURSOR_VERSION: u8 = 2;
const GRAPH_CURSOR_NONCE_BYTES: usize = 24;
const GRAPH_CURSOR_AUTH_TAG_BYTES: usize = 16;
const GRAPH_CURSOR_AAD: &[u8] = b"transnet.graph.neighbor-cursor.g2";

/// Reads a bounded graph from one typed root when a graph service has been injected.
pub(super) async fn read(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  query: Result<Query<GraphReadQuery>, QueryRejection>,
) -> Response {
  let Query(query) = match query {
    Ok(query) => query,
    Err(_) => return malformed_query_problem(&request_id),
  };
  let request = match graph_read_request(query) {
    Ok(request) => request,
    Err(error) => return invalid_graph_request(error, &request_id),
  };
  if let Some(cache) = state.graph_topology_snapshot_cache_service() {
    return graph_response(
      cache
        .read(request)
        .await
        .map(GraphTopologySnapshotCacheResult::into_result)
        .map_err(graph_topology_cache_error),
      &state,
      &request_id,
    );
  }
  let Some(service) = state.graph_service() else {
    return graph_unavailable(&request_id, true);
  };

  graph_response(service.read(request).await, &state, &request_id)
}

/// Expands one typed node through one opaque, bounded direct-neighbor page.
pub(super) async fn neighbors(
  State(state): State<AppState>,
  Extension(request_id): Extension<RequestId>,
  path: Result<Path<GraphNeighborPath>, PathRejection>,
  query: Result<Query<GraphNeighborQuery>, QueryRejection>,
) -> Response {
  let Path(path) = match path {
    Ok(path) => path,
    Err(_) => return malformed_path_problem(&request_id),
  };
  let Query(query) = match query {
    Ok(query) => query,
    Err(_) => return malformed_query_problem(&request_id),
  };
  let cursor_codec = GraphCursorCodec::new(state.graph_cursor_protection_key());
  let request = match graph_neighbor_request(path, query, &cursor_codec) {
    Ok(request) => request,
    Err(error) => return invalid_graph_request(error, &request_id),
  };
  let Some(service) = state.graph_service() else {
    return graph_unavailable(&request_id, true);
  };

  graph_response(service.neighbors(request).await, &state, &request_id)
}

fn graph_response(
  result: Result<GraphReadResult, GraphReadError>,
  state: &AppState,
  request_id: &RequestId,
) -> Response {
  match result {
    Ok(result) => {
      match GraphResponse::from_result(
        result,
        GraphCursorCodec::new(state.graph_cursor_protection_key()),
      ) {
        Ok(response) => problem::no_store((StatusCode::OK, Json(response)).into_response()),
        Err(_) => {
          tracing::warn!(
            error_category = "graph_response_incomplete",
            "could not construct a complete graph response"
          );
          graph_unavailable(request_id, false)
        }
      }
    }
    Err(GraphReadError::RootNotFound) => problem::response(
      StatusCode::NOT_FOUND,
      "graph_root_not_found",
      "Graph root not found",
      "The requested typed graph root is not available in the active graph content.",
      request_id,
      false,
      Vec::new(),
    ),
    Err(GraphReadError::Validation(error)) => {
      invalid_graph_request(RequestValidation::from_graph_validation(error), request_id)
    }
    Err(GraphReadError::Repository(GraphRepositoryError::Unavailable)) => {
      tracing::warn!(
        error_category = "graph_repository_unavailable",
        "graph read dependency unavailable"
      );
      graph_unavailable(request_id, true)
    }
    Err(GraphReadError::Repository(GraphRepositoryError::InconsistentData)) => {
      tracing::warn!(
        error_category = "graph_repository_inconsistent",
        "graph read dependency returned inconsistent data"
      );
      graph_unavailable(request_id, false)
    }
  }
}

fn graph_topology_cache_error(error: GraphTopologySnapshotCacheError) -> GraphReadError {
  match error {
    GraphTopologySnapshotCacheError::Graph(error) => error,
    GraphTopologySnapshotCacheError::Contract(GraphTopologySnapshotValidationError::Request(
      error,
    )) => GraphReadError::Validation(error),
    GraphTopologySnapshotCacheError::Contract(_) | GraphTopologySnapshotCacheError::ZeroTtl => {
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    }
  }
}

fn graph_unavailable(request_id: &RequestId, retryable: bool) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "graph_unavailable",
    "Graph unavailable",
    "The canonical graph is temporarily unavailable.",
    request_id,
    retryable,
    Vec::new(),
  )
}

fn malformed_query_problem(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_graph_request",
    "Invalid graph request",
    "The graph query parameters are malformed.",
    request_id,
    false,
    Vec::new(),
  )
}

fn malformed_path_problem(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_graph_request",
    "Invalid graph request",
    "The graph node path is malformed.",
    request_id,
    false,
    Vec::new(),
  )
}

fn invalid_graph_request(error: RequestValidation, request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_graph_request",
    "Invalid graph request",
    "One or more graph request fields are invalid.",
    request_id,
    false,
    vec![FieldError::new(error.field, error.message)],
  )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GraphReadQuery {
  root_kind: String,
  root_id: String,
  depth: Option<u8>,
  node_limit: Option<usize>,
  edge_limit: Option<usize>,
  relation_types: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GraphNeighborPath {
  node_kind: String,
  node_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GraphNeighborQuery {
  node_limit: Option<usize>,
  edge_limit: Option<usize>,
  relation_types: Option<String>,
  cursor: Option<String>,
}

#[derive(Debug)]
struct RequestValidation {
  field: &'static str,
  message: &'static str,
}

impl RequestValidation {
  fn new(field: &'static str, message: &'static str) -> Self {
    Self { field, message }
  }

  fn from_graph_validation(error: GraphValidationError) -> Self {
    match error {
      GraphValidationError::InvalidDepth => {
        Self::new("depth", "must be within the bounded graph traversal depth.")
      }
      GraphValidationError::InvalidNodeLimit => {
        Self::new("node_limit", "must be within the bounded graph node limit.")
      }
      GraphValidationError::InvalidEdgeLimit => {
        Self::new("edge_limit", "must be within the bounded graph edge limit.")
      }
      GraphValidationError::CursorRootMismatch
      | GraphValidationError::CursorContentMismatch
      | GraphValidationError::CursorFilterMismatch => Self::new(
        "cursor",
        "does not match this graph node, active content version, or relation filter.",
      ),
      GraphValidationError::InvalidNeighborNodeLimit => Self::new(
        "node_limit",
        "must leave room for the root and one adjacent graph node.",
      ),
      _ => Self::new("graph", "is not available in the requested form."),
    }
  }
}

fn graph_read_request(query: GraphReadQuery) -> Result<GraphReadRequest, RequestValidation> {
  let root = parse_node_key(&query.root_kind, &query.root_id, "root_kind", "root_id")?;
  let filter = parse_relation_filter(query.relation_types.as_deref())?;
  GraphReadRequest::new(
    root,
    query.depth.unwrap_or(DEFAULT_GRAPH_DEPTH),
    query.node_limit.unwrap_or(DEFAULT_GRAPH_NODE_LIMIT),
    query.edge_limit.unwrap_or(DEFAULT_GRAPH_EDGE_LIMIT),
    filter,
  )
  .map_err(RequestValidation::from_graph_validation)
}

fn graph_neighbor_request(
  path: GraphNeighborPath,
  query: GraphNeighborQuery,
  cursor_codec: &GraphCursorCodec<'_>,
) -> Result<GraphNeighborRequest, RequestValidation> {
  let root = parse_node_key(&path.node_kind, &path.node_id, "node_kind", "node_id")?;
  let filter = parse_relation_filter(query.relation_types.as_deref())?;
  let cursor = match query.cursor {
    Some(cursor) => Some(
      cursor_codec
        .decode(&cursor)
        .map_err(|_| RequestValidation::new("cursor", "must be a valid opaque graph cursor."))?,
    ),
    None => None,
  };
  GraphNeighborRequest::new(
    root,
    query.node_limit.unwrap_or(DEFAULT_GRAPH_NODE_LIMIT),
    query.edge_limit.unwrap_or(DEFAULT_GRAPH_EDGE_LIMIT),
    filter,
    cursor,
  )
  .map_err(RequestValidation::from_graph_validation)
}

fn parse_node_key(
  kind: &str,
  id: &str,
  kind_field: &'static str,
  id_field: &'static str,
) -> Result<GraphNodeKey, RequestValidation> {
  let kind = parse_node_kind(kind)
    .ok_or_else(|| RequestValidation::new(kind_field, "must be a supported graph node kind."))?;
  let id = parse_canonical_id(id, id_field)?;
  Ok(GraphNodeKey::new(kind, id))
}

fn parse_canonical_id(value: &str, field: &'static str) -> Result<CanonicalId, RequestValidation> {
  if value.chars().count() > MAX_GRAPH_ID_LENGTH {
    return Err(RequestValidation::new(
      field,
      "must not exceed the graph identifier length limit.",
    ));
  }
  CanonicalId::new(value)
    .map_err(|_| RequestValidation::new(field, "must be a nonblank canonical identifier."))
}

fn parse_relation_filter(value: Option<&str>) -> Result<GraphFilter, RequestValidation> {
  let Some(value) = value else {
    return Ok(GraphFilter::default());
  };
  if value.len() > MAX_GRAPH_RELATION_TYPES_LENGTH {
    return Err(RequestValidation::new(
      "relation_types",
      "must not exceed the graph relation-filter length limit.",
    ));
  }

  let mut relation_types = BTreeSet::new();
  for (index, relation_type) in value.split(',').enumerate() {
    if index >= MAX_GRAPH_RELATION_TYPE_COUNT {
      return Err(RequestValidation::new(
        "relation_types",
        "must not include more than the supported graph relation types.",
      ));
    }
    let relation_type = parse_relation_type(relation_type).ok_or_else(|| {
      RequestValidation::new(
        "relation_types",
        "must contain supported comma-separated graph relation types.",
      )
    })?;
    relation_types.insert(relation_type);
  }
  if relation_types.is_empty() {
    return Err(RequestValidation::new(
      "relation_types",
      "must contain at least one supported graph relation type when provided.",
    ));
  }

  Ok(GraphFilter { relation_types })
}

fn parse_node_kind(value: &str) -> Option<GraphNodeKind> {
  match value {
    "sense" => Some(GraphNodeKind::Sense),
    "lexeme" => Some(GraphNodeKind::Lexeme),
    "construction" => Some(GraphNodeKind::Construction),
    "scale" => Some(GraphNodeKind::Scale),
    _ => None,
  }
}

fn parse_relation_type(value: &str) -> Option<GraphRelationType> {
  match value {
    "synonym" => Some(GraphRelationType::Synonym),
    "near_synonym" => Some(GraphRelationType::NearSynonym),
    "translation_equivalent" => Some(GraphRelationType::TranslationEquivalent),
    "antonym" => Some(GraphRelationType::Antonym),
    "hypernym" => Some(GraphRelationType::Hypernym),
    "hyponym" => Some(GraphRelationType::Hyponym),
    "holonym" => Some(GraphRelationType::Holonym),
    "meronym" => Some(GraphRelationType::Meronym),
    "confusable_with" => Some(GraphRelationType::ConfusableWith),
    "associated_with" => Some(GraphRelationType::AssociatedWith),
    "inflection_of" => Some(GraphRelationType::InflectionOf),
    "has_inflection" => Some(GraphRelationType::HasInflection),
    "derivationally_related_to" => Some(GraphRelationType::DerivationallyRelatedTo),
    "etymologically_derived_from" => Some(GraphRelationType::EtymologicallyDerivedFrom),
    "etymological_source_of" => Some(GraphRelationType::EtymologicalSourceOf),
    "construction_member" => Some(GraphRelationType::ConstructionMember),
    "has_construction_member" => Some(GraphRelationType::HasConstructionMember),
    "scale_contains" => Some(GraphRelationType::ScaleContains),
    "member_of_scale" => Some(GraphRelationType::MemberOfScale),
    "lower_degree" => Some(GraphRelationType::LowerDegree),
    "higher_degree" => Some(GraphRelationType::HigherDegree),
    _ => None,
  }
}

#[derive(Debug, Serialize)]
struct GraphResponse {
  schema_version: &'static str,
  root: GraphNodeReference,
  content_version: GraphContentVersionResponse,
  nodes: Vec<GraphNodeResponse>,
  edges: Vec<GraphEdgeResponse>,
  relation_list: Vec<GraphRelationListItem>,
  truncated: bool,
  next_cursor: Option<String>,
}

impl GraphResponse {
  fn from_result(
    result: GraphReadResult,
    cursor_codec: GraphCursorCodec<'_>,
  ) -> Result<Self, GraphResponseError> {
    let node_labels = result
      .nodes
      .iter()
      .map(|node| (node.key.clone(), node.label.clone()))
      .collect::<BTreeMap<_, _>>();
    let relation_list = result
      .edges
      .iter()
      .map(|edge| GraphRelationListItem::from_edge(edge, &node_labels))
      .collect::<Result<Vec<_>, _>>()?;
    let edges = result
      .edges
      .into_iter()
      .map(GraphEdgeResponse::from)
      .collect();
    let nodes = result
      .nodes
      .into_iter()
      .map(GraphNodeResponse::from)
      .collect();
    let next_cursor = result
      .next_cursor
      .as_ref()
      .map(|cursor| cursor_codec.encode(cursor))
      .transpose()
      .map_err(|_| GraphResponseError::Cursor)?;

    Ok(Self {
      schema_version: "1.0",
      root: GraphNodeReference::from(result.root),
      content_version: GraphContentVersionResponse::from(result.content),
      nodes,
      edges,
      relation_list,
      truncated: result.truncated,
      next_cursor,
    })
  }
}

#[derive(Debug, Clone, Serialize)]
struct GraphNodeReference {
  kind: &'static str,
  id: String,
}

impl From<GraphNodeKey> for GraphNodeReference {
  fn from(value: GraphNodeKey) -> Self {
    Self::from(&value)
  }
}

impl From<&GraphNodeKey> for GraphNodeReference {
  fn from(value: &GraphNodeKey) -> Self {
    Self {
      kind: graph_node_kind(value.kind),
      id: value.id.to_string(),
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphNodeResponse {
  kind: &'static str,
  id: String,
  label: String,
  language: Option<String>,
  part_of_speech: Option<&'static str>,
  definition_short: Option<String>,
  expandable: bool,
}

impl From<GraphNode> for GraphNodeResponse {
  fn from(value: GraphNode) -> Self {
    Self {
      kind: graph_node_kind(value.key.kind),
      id: value.key.id.to_string(),
      label: value.label,
      language: value.language.map(|language| language.to_string()),
      part_of_speech: value.part_of_speech.map(part_of_speech),
      definition_short: value.definition_short,
      expandable: value.expandable,
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphContentVersionResponse {
  release_id: String,
  ranking_version: String,
  community_aggregate_version: String,
}

impl From<GraphContentVersion> for GraphContentVersionResponse {
  fn from(value: GraphContentVersion) -> Self {
    Self {
      release_id: value.release_id.to_string(),
      ranking_version: value.ranking_version,
      community_aggregate_version: value.community_aggregate_version,
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphEdgeResponse {
  id: String,
  source: GraphNodeReference,
  target: GraphNodeReference,
  relation_type: &'static str,
  direction: GraphDirectionResponse,
  relation_version: Option<u32>,
  evidence: GraphEvidenceResponse,
  scope: GraphScopeResponse,
  feedback_capabilities: Vec<&'static str>,
  ranking: GraphRankingResponse,
}

impl From<GraphEdge> for GraphEdgeResponse {
  fn from(value: GraphEdge) -> Self {
    Self {
      id: value.id.as_str().to_string(),
      source: GraphNodeReference::from(value.source),
      target: GraphNodeReference::from(value.target),
      relation_type: graph_relation_type(value.relation_type),
      direction: GraphDirectionResponse::new(value.directed, value.origin),
      relation_version: value.relation_version.map(RelationVersion::get),
      evidence: GraphEvidenceResponse {
        evidence_ids: value
          .evidence
          .evidence_ids
          .into_iter()
          .map(|id| id.to_string())
          .collect(),
        confidence: evidence_confidence(value.evidence.confidence),
      },
      scope: GraphScopeResponse::from(value.scope),
      feedback_capabilities: value
        .feedback_capabilities
        .into_iter()
        .map(graph_feedback_capability)
        .collect(),
      ranking: GraphRankingResponse::from(value.ranking),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
struct GraphDirectionResponse {
  directed: bool,
  canonical_projection: GraphCanonicalProjectionResponse,
}

impl GraphDirectionResponse {
  fn new(directed: bool, origin: GraphEdgeOrigin) -> Self {
    Self {
      directed,
      canonical_projection: GraphCanonicalProjectionResponse::from(origin),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GraphCanonicalProjectionResponse {
  Stored,
  InverseProjection,
  ScaleAdjacency {
    scale_id: String,
  },
  ScaleMembership {
    scale_id: String,
    member_ordinal: u32,
  },
}

impl From<GraphEdgeOrigin> for GraphCanonicalProjectionResponse {
  fn from(value: GraphEdgeOrigin) -> Self {
    match value {
      GraphEdgeOrigin::Canonical => Self::Stored,
      GraphEdgeOrigin::InverseProjection => Self::InverseProjection,
      GraphEdgeOrigin::ScaleAdjacency { scale_id } => Self::ScaleAdjacency {
        scale_id: scale_id.to_string(),
      },
      GraphEdgeOrigin::ScaleMembership { scale_id, ordinal } => Self::ScaleMembership {
        scale_id: scale_id.to_string(),
        member_ordinal: ordinal,
      },
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphEvidenceResponse {
  evidence_ids: Vec<String>,
  confidence: &'static str,
}

#[derive(Debug, Serialize)]
struct GraphScopeResponse {
  dialect: Option<String>,
  domain: Option<String>,
  register: Option<String>,
  note: Option<String>,
}

impl From<GraphScope> for GraphScopeResponse {
  fn from(value: GraphScope) -> Self {
    Self {
      dialect: value.dialect.map(|dialect| dialect.to_string()),
      domain: value.domain,
      register: value.register,
      note: value.note,
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphRankingResponse {
  display_rank_basis_points: u16,
  components: GraphRankingComponentsResponse,
  version: String,
}

impl From<GraphRanking> for GraphRankingResponse {
  fn from(value: GraphRanking) -> Self {
    Self {
      display_rank_basis_points: value.display_rank.basis_points(),
      components: GraphRankingComponentsResponse {
        evidence_basis_points: value.components.evidence.basis_points(),
        community_basis_points: value.components.community.map(GraphScore::basis_points),
        pedagogical_basis_points: value.components.pedagogical.map(GraphScore::basis_points),
      },
      version: value.ranking_version,
    }
  }
}

#[derive(Debug, Serialize)]
struct GraphRankingComponentsResponse {
  evidence_basis_points: u16,
  community_basis_points: Option<u16>,
  pedagogical_basis_points: Option<u16>,
}

#[derive(Debug, Serialize)]
struct GraphRelationListItem {
  edge_id: String,
  source: GraphLabeledNodeReference,
  relation_type: &'static str,
  direction: GraphDirectionResponse,
  target: GraphLabeledNodeReference,
}

impl GraphRelationListItem {
  fn from_edge(
    edge: &GraphEdge,
    node_labels: &BTreeMap<GraphNodeKey, String>,
  ) -> Result<Self, GraphResponseError> {
    Ok(Self {
      edge_id: edge.id.as_str().to_string(),
      source: GraphLabeledNodeReference::new(&edge.source, node_labels)?,
      relation_type: graph_relation_type(edge.relation_type),
      direction: GraphDirectionResponse::new(edge.directed, edge.origin.clone()),
      target: GraphLabeledNodeReference::new(&edge.target, node_labels)?,
    })
  }
}

#[derive(Debug, Serialize)]
struct GraphLabeledNodeReference {
  kind: &'static str,
  id: String,
  label: String,
}

impl GraphLabeledNodeReference {
  fn new(
    key: &GraphNodeKey,
    node_labels: &BTreeMap<GraphNodeKey, String>,
  ) -> Result<Self, GraphResponseError> {
    let label = node_labels
      .get(key)
      .cloned()
      .ok_or(GraphResponseError::MissingEndpoint)?;
    Ok(Self {
      kind: graph_node_kind(key.kind),
      id: key.id.to_string(),
      label,
    })
  }
}

#[derive(Debug)]
enum GraphResponseError {
  Cursor,
  MissingEndpoint,
}

fn graph_node_kind(value: GraphNodeKind) -> &'static str {
  match value {
    GraphNodeKind::Sense => "sense",
    GraphNodeKind::Lexeme => "lexeme",
    GraphNodeKind::Construction => "construction",
    GraphNodeKind::Scale => "scale",
  }
}

fn graph_relation_type(value: GraphRelationType) -> &'static str {
  match value {
    GraphRelationType::Synonym => "synonym",
    GraphRelationType::NearSynonym => "near_synonym",
    GraphRelationType::TranslationEquivalent => "translation_equivalent",
    GraphRelationType::Antonym => "antonym",
    GraphRelationType::Hypernym => "hypernym",
    GraphRelationType::Hyponym => "hyponym",
    GraphRelationType::Holonym => "holonym",
    GraphRelationType::Meronym => "meronym",
    GraphRelationType::ConfusableWith => "confusable_with",
    GraphRelationType::AssociatedWith => "associated_with",
    GraphRelationType::InflectionOf => "inflection_of",
    GraphRelationType::HasInflection => "has_inflection",
    GraphRelationType::DerivationallyRelatedTo => "derivationally_related_to",
    GraphRelationType::EtymologicallyDerivedFrom => "etymologically_derived_from",
    GraphRelationType::EtymologicalSourceOf => "etymological_source_of",
    GraphRelationType::ConstructionMember => "construction_member",
    GraphRelationType::HasConstructionMember => "has_construction_member",
    GraphRelationType::ScaleContains => "scale_contains",
    GraphRelationType::MemberOfScale => "member_of_scale",
    GraphRelationType::LowerDegree => "lower_degree",
    GraphRelationType::HigherDegree => "higher_degree",
  }
}

fn graph_feedback_capability(value: GraphFeedbackCapability) -> &'static str {
  match value {
    GraphFeedbackCapability::Usefulness => "usefulness",
    GraphFeedbackCapability::Accuracy => "accuracy",
  }
}

fn evidence_confidence(value: EvidenceConfidence) -> &'static str {
  match value {
    EvidenceConfidence::High => "high",
    EvidenceConfidence::Medium => "medium",
    EvidenceConfidence::Low => "low",
  }
}

fn part_of_speech(value: LexicalPartOfSpeech) -> &'static str {
  match value {
    LexicalPartOfSpeech::Noun => "noun",
    LexicalPartOfSpeech::Verb => "verb",
    LexicalPartOfSpeech::Adjective => "adjective",
    LexicalPartOfSpeech::Adverb => "adverb",
    LexicalPartOfSpeech::Pronoun => "pronoun",
    LexicalPartOfSpeech::Preposition => "preposition",
    LexicalPartOfSpeech::Conjunction => "conjunction",
    LexicalPartOfSpeech::Determiner => "determiner",
    LexicalPartOfSpeech::Interjection => "interjection",
    LexicalPartOfSpeech::Numeral => "numeral",
    LexicalPartOfSpeech::Other => "other",
  }
}

struct GraphCursorCodec<'a> {
  key: &'a [u8],
}

impl<'a> GraphCursorCodec<'a> {
  fn new(key: &'a [u8]) -> Self {
    Self { key }
  }

  fn encode(&self, cursor: &GraphCursor) -> Result<String, GraphCursorCodecError> {
    let payload = serde_json::to_vec(&GraphCursorPayload::from(cursor))
      .map_err(|_| GraphCursorCodecError::Encoding)?;
    let cipher =
      XChaCha20Poly1305::new_from_slice(self.key).map_err(|_| GraphCursorCodecError::Encoding)?;
    let mut nonce = [0_u8; GRAPH_CURSOR_NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| GraphCursorCodecError::Encoding)?;
    let ciphertext = cipher
      .encrypt(
        XNonce::from_slice(&nonce),
        Payload {
          msg: &payload,
          aad: GRAPH_CURSOR_AAD,
        },
      )
      .map_err(|_| GraphCursorCodecError::Encoding)?;
    let cursor = format!(
      "{GRAPH_CURSOR_PREFIX}.{}.{}",
      URL_SAFE_NO_PAD.encode(nonce),
      URL_SAFE_NO_PAD.encode(ciphertext),
    );
    if cursor.len() > MAX_GRAPH_CURSOR_LENGTH {
      return Err(GraphCursorCodecError::Encoding);
    }
    Ok(cursor)
  }

  fn decode(&self, value: &str) -> Result<GraphCursor, GraphCursorCodecError> {
    if value.is_empty() || value.len() > MAX_GRAPH_CURSOR_LENGTH {
      return Err(GraphCursorCodecError::Malformed);
    }
    let mut parts = value.split('.');
    let (Some(prefix), Some(nonce), Some(ciphertext), None) =
      (parts.next(), parts.next(), parts.next(), parts.next())
    else {
      return Err(GraphCursorCodecError::Malformed);
    };
    if prefix != GRAPH_CURSOR_PREFIX {
      return Err(GraphCursorCodecError::Malformed);
    }
    let nonce = URL_SAFE_NO_PAD
      .decode(nonce)
      .map_err(|_| GraphCursorCodecError::Malformed)?;
    if nonce.len() != GRAPH_CURSOR_NONCE_BYTES {
      return Err(GraphCursorCodecError::Malformed);
    }
    let ciphertext = URL_SAFE_NO_PAD
      .decode(ciphertext)
      .map_err(|_| GraphCursorCodecError::Malformed)?;
    if ciphertext.len() < GRAPH_CURSOR_AUTH_TAG_BYTES {
      return Err(GraphCursorCodecError::Malformed);
    }
    let cipher =
      XChaCha20Poly1305::new_from_slice(self.key).map_err(|_| GraphCursorCodecError::Malformed)?;
    let payload = cipher
      .decrypt(
        XNonce::from_slice(&nonce),
        Payload {
          msg: &ciphertext,
          aad: GRAPH_CURSOR_AAD,
        },
      )
      .map_err(|_| GraphCursorCodecError::Malformed)?;
    let payload = serde_json::from_slice::<GraphCursorPayload>(&payload)
      .map_err(|_| GraphCursorCodecError::Malformed)?;
    payload.into_cursor()
  }
}

#[derive(Debug)]
enum GraphCursorCodecError {
  Malformed,
  Encoding,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphCursorPayload {
  version: u8,
  root: GraphCursorNodeKey,
  content: GraphCursorContentVersion,
  filter: GraphCursorFilter,
  after: GraphCursorOrderingKey,
}

impl From<&GraphCursor> for GraphCursorPayload {
  fn from(value: &GraphCursor) -> Self {
    Self {
      version: GRAPH_CURSOR_VERSION,
      root: GraphCursorNodeKey::from(&value.root),
      content: GraphCursorContentVersion::from(&value.content),
      filter: GraphCursorFilter::from(&value.filter),
      after: GraphCursorOrderingKey::from(&value.after),
    }
  }
}

impl GraphCursorPayload {
  fn into_cursor(self) -> Result<GraphCursor, GraphCursorCodecError> {
    if self.version != GRAPH_CURSOR_VERSION {
      return Err(GraphCursorCodecError::Malformed);
    }
    Ok(GraphCursor {
      root: self.root.into_node_key()?,
      content: self.content.into_content_version()?,
      filter: self.filter.into_filter()?,
      after: self.after.into_ordering_key()?,
    })
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphCursorNodeKey {
  kind: String,
  id: String,
}

impl From<&GraphNodeKey> for GraphCursorNodeKey {
  fn from(value: &GraphNodeKey) -> Self {
    Self {
      kind: graph_node_kind(value.kind).to_string(),
      id: value.id.to_string(),
    }
  }
}

impl GraphCursorNodeKey {
  fn into_node_key(self) -> Result<GraphNodeKey, GraphCursorCodecError> {
    let kind = parse_node_kind(&self.kind).ok_or(GraphCursorCodecError::Malformed)?;
    let id = CanonicalId::new(self.id).map_err(|_| GraphCursorCodecError::Malformed)?;
    Ok(GraphNodeKey::new(kind, id))
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphCursorContentVersion {
  release_id: String,
  ranking_version: String,
  community_aggregate_version: String,
}

impl From<&GraphContentVersion> for GraphCursorContentVersion {
  fn from(value: &GraphContentVersion) -> Self {
    Self {
      release_id: value.release_id.to_string(),
      ranking_version: value.ranking_version.clone(),
      community_aggregate_version: value.community_aggregate_version.clone(),
    }
  }
}

impl GraphCursorContentVersion {
  fn into_content_version(self) -> Result<GraphContentVersion, GraphCursorCodecError> {
    let release_id =
      CanonicalId::new(self.release_id).map_err(|_| GraphCursorCodecError::Malformed)?;
    if self.ranking_version.is_empty() || self.community_aggregate_version.is_empty() {
      return Err(GraphCursorCodecError::Malformed);
    }
    Ok(GraphContentVersion {
      release_id,
      ranking_version: self.ranking_version,
      community_aggregate_version: self.community_aggregate_version,
    })
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphCursorFilter {
  relation_types: Vec<String>,
}

impl From<&GraphFilter> for GraphCursorFilter {
  fn from(value: &GraphFilter) -> Self {
    Self {
      relation_types: value
        .relation_types
        .iter()
        .copied()
        .map(graph_relation_type)
        .map(str::to_string)
        .collect(),
    }
  }
}

impl GraphCursorFilter {
  fn into_filter(self) -> Result<GraphFilter, GraphCursorCodecError> {
    if self.relation_types.len() > MAX_GRAPH_RELATION_TYPE_COUNT {
      return Err(GraphCursorCodecError::Malformed);
    }
    let mut relation_types = BTreeSet::new();
    for relation_type in self.relation_types {
      let relation_type =
        parse_relation_type(&relation_type).ok_or(GraphCursorCodecError::Malformed)?;
      if !relation_types.insert(relation_type) {
        return Err(GraphCursorCodecError::Malformed);
      }
    }
    Ok(GraphFilter { relation_types })
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphCursorOrderingKey {
  display_rank_basis_points: u16,
  edge_id: String,
  source: GraphCursorNodeKey,
  target: GraphCursorNodeKey,
  relation_type: String,
}

impl From<&GraphEdgeOrderingKey> for GraphCursorOrderingKey {
  fn from(value: &GraphEdgeOrderingKey) -> Self {
    Self {
      display_rank_basis_points: value.display_rank.basis_points(),
      edge_id: value.edge_id.as_str().to_string(),
      source: GraphCursorNodeKey::from(&value.source),
      target: GraphCursorNodeKey::from(&value.target),
      relation_type: graph_relation_type(value.relation_type).to_string(),
    }
  }
}

impl GraphCursorOrderingKey {
  fn into_ordering_key(self) -> Result<GraphEdgeOrderingKey, GraphCursorCodecError> {
    Ok(GraphEdgeOrderingKey {
      display_rank: GraphScore::new(self.display_rank_basis_points)
        .map_err(|_| GraphCursorCodecError::Malformed)?,
      edge_id: GraphEdgeId::parse(self.edge_id).map_err(|_| GraphCursorCodecError::Malformed)?,
      source: self.source.into_node_key()?,
      target: self.target.into_node_key()?,
      relation_type: parse_relation_type(&self.relation_type)
        .ok_or(GraphCursorCodecError::Malformed)?,
    })
  }
}
