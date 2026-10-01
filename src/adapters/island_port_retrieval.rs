//! Strict outbound retrieval-data-v1 client over island-port's injected UDS transport.

use std::{collections::BTreeSet, fmt, sync::Arc};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
  adapters::island_port::{IslandPortClientError, IslandPortTransport},
  domain::{
    canonical::{CanonicalId, ReleaseId},
    request_context::RequestContext,
    retrieval_data::{
      validate_cursor, validate_limit, EdgeCandidate, EdgeSearchRequest, EdgeSearchResult,
      NeighborCandidate, NeighborDirection, NeighborEdge, NeighborNode, NeighborSearchRequest,
      NeighborSearchResult, NodeCandidate, NodeCandidatePayload, NodeSearchRequest,
      NodeSearchResult, RetrievalDataScore, RetrievalDataValidationError, RetrievalFilters,
      RetrievalNodeType, RetrievalPublicationState, RetrievalRelation, RetrievalVerificationState,
      ScaleCandidate, ScaleSearchRequest, ScaleSearchResult, RELATION_REGISTRY_VERSION,
      RETRIEVAL_DATA_SCHEMA_VERSION,
    },
  },
  ports::retrieval_data::{RetrievalDataError, RetrievalDataPort},
};

const MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_REQUEST_BYTES: usize = 1_048_576;

/// Strict storage-neutral retrieval-data client with no direct Qdrant capability.
pub struct IslandPortRetrievalClient {
  transport: Arc<dyn IslandPortTransport>,
}

impl IslandPortRetrievalClient {
  /// Creates a client over an injected bounded transport.
  pub fn new(transport: Arc<dyn IslandPortTransport>) -> Self {
    Self { transport }
  }

  async fn call<T: Serialize, R: DeserializeOwned>(
    &self,
    path: &'static str,
    context: &RequestContext,
    input: T,
  ) -> Result<R, RetrievalDataError> {
    let timeout = validate_context(context)?;
    let envelope = RequestEnvelope {
      context: RequestContextDto {
        request_id: context.request_id().as_str(),
        deadline_at: context.deadline_rfc3339(),
        schema_version: RETRIEVAL_DATA_SCHEMA_VERSION,
        content_release: context
          .content_release()
          .map(CanonicalId::as_str)
          .unwrap_or(""),
      },
      input,
    };
    let body = serde_json::to_vec(&envelope).map_err(|_| RetrievalDataError::InvalidRequest)?;
    if body.len() > MAX_REQUEST_BYTES {
      return Err(RetrievalDataError::InvalidRequest);
    }
    let response = self
      .transport
      .post_json(path, body, timeout)
      .await
      .map_err(map_transport_error)?;
    if response.len() > MAX_RESPONSE_BYTES {
      return Err(RetrievalDataError::InconsistentData);
    }
    serde_json::from_slice(&response).map_err(|_| RetrievalDataError::InconsistentData)
  }
}

impl fmt::Debug for IslandPortRetrievalClient {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortRetrievalClient([redacted])")
  }
}

#[async_trait]
impl RetrievalDataPort for IslandPortRetrievalClient {
  async fn search_nodes(
    &self,
    context: &RequestContext,
    request: NodeSearchRequest,
  ) -> Result<NodeSearchResult, RetrievalDataError> {
    validate_filters(context, &request.filters)?;
    validate_limit(request.limit)?;
    let allowed_verification = request.filters.verification_states.clone();
    let allowed_node_types = request.filters.node_types.clone();
    let limit = request.limit;
    let response: ResponseDto<CandidatesDto<NodeCandidateDto>> = self
      .call(
        "/api/v1/nodes/search",
        context,
        NodeSearchInputDto::from(request),
      )
      .await?;
    let (release_id, value) = response.into_value(context)?;
    if value.candidates.len() > limit {
      return Err(RetrievalDataError::InconsistentData);
    }
    let candidates = value
      .candidates
      .into_iter()
      .map(NodeCandidateDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    if candidates.iter().any(|candidate| {
      !allowed_verification.contains(&candidate.payload.verification_state)
        || (!allowed_node_types.is_empty()
          && !allowed_node_types.contains(&candidate.payload.node_type))
    }) {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NodeSearchResult {
      release_id,
      candidates,
    })
  }

  async fn search_scales(
    &self,
    context: &RequestContext,
    request: ScaleSearchRequest,
  ) -> Result<ScaleSearchResult, RetrievalDataError> {
    validate_filters(context, &request.filters)?;
    if !request.filters.node_types.is_empty()
      || !request.filters.languages.is_empty()
      || !request.filters.dialects.is_empty()
      || !request.filters.regions.is_empty()
      || !request.filters.periods.is_empty()
      || !request.filters.eligible_evidence_ids.is_empty()
    {
      return Err(RetrievalDataError::InvalidRequest);
    }
    validate_limit(request.limit)?;
    let allowed_verification = request.filters.verification_states.clone();
    let limit = request.limit;
    let response: ResponseDto<CandidatesDto<ScaleCandidateDto>> = self
      .call(
        "/api/v1/scales/search",
        context,
        ScaleSearchInputDto::from(request),
      )
      .await?;
    let (release_id, value) = response.into_value(context)?;
    if value.candidates.len() > limit {
      return Err(RetrievalDataError::InconsistentData);
    }
    let candidates = value
      .candidates
      .into_iter()
      .map(ScaleCandidateDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    if candidates
      .iter()
      .any(|candidate| !allowed_verification.contains(&candidate.verification_state))
    {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(ScaleSearchResult {
      release_id,
      candidates,
    })
  }

  async fn search_edges(
    &self,
    context: &RequestContext,
    request: EdgeSearchRequest,
  ) -> Result<EdgeSearchResult, RetrievalDataError> {
    validate_filters(context, &request.filters)?;
    if !request.filters.node_types.is_empty() {
      return Err(RetrievalDataError::InvalidRequest);
    }
    validate_limit(request.limit)?;
    if request.relation_types.is_empty()
      || request.relation_types.len() > 50
      || request.applicable_sense_ids.len() > 50
    {
      return Err(RetrievalDataError::InvalidRequest);
    }
    let allowed_relations = request.relation_types.clone();
    let allowed_verification = request.filters.verification_states.clone();
    let limit = request.limit;
    let response: ResponseDto<CandidatesDto<EdgeCandidateDto>> = self
      .call(
        "/api/v1/edges/search",
        context,
        EdgeSearchInputDto::from(request),
      )
      .await?;
    let (release_id, value) = response.into_value(context)?;
    if value.candidates.len() > limit {
      return Err(RetrievalDataError::InconsistentData);
    }
    let candidates = value
      .candidates
      .into_iter()
      .map(EdgeCandidateDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    if candidates.iter().any(|candidate| {
      !allowed_relations.contains(&candidate.relation_type)
        || !allowed_verification.contains(&candidate.verification_state)
    }) {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(EdgeSearchResult {
      release_id,
      candidates,
    })
  }

  async fn search_neighbors(
    &self,
    context: &RequestContext,
    request: NeighborSearchRequest,
  ) -> Result<NeighborSearchResult, RetrievalDataError> {
    validate_release(context, &request.release_id)?;
    validate_limit(request.limit)?;
    if request.relation_types.is_empty()
      || request.relation_types.len() > 50
      || request.verification_states.is_empty()
      || request.verification_states.len() > 50
      || request.languages.len() > 50
      || request.domain_ids.len() > 50
    {
      return Err(RetrievalDataError::InvalidRequest);
    }
    if let Some(cursor) = request.cursor.as_deref() {
      validate_cursor(cursor)?;
    }
    let root_node_id = request.node_id.clone();
    let allowed_relations = request.relation_types.clone();
    let allowed_verification = request.verification_states.clone();
    let direction = request.direction;
    let limit = request.limit;
    let response: ResponseDto<NeighborsDto> = self
      .call(
        "/api/v1/neighbors/search",
        context,
        NeighborSearchInputDto::from(request),
      )
      .await?;
    let (release_id, value) = response.into_value(context)?;
    let echoed_root = parse_id(value.root_node_id)?;
    if echoed_root != root_node_id || value.neighbors.len() > limit {
      return Err(RetrievalDataError::InconsistentData);
    }
    if let Some(cursor) = value.next_cursor.as_deref() {
      validate_cursor(cursor).map_err(|_| RetrievalDataError::InconsistentData)?;
    }
    let neighbors = value
      .neighbors
      .into_iter()
      .map(NeighborCandidateDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    if neighbors.iter().any(|candidate| {
      !allowed_relations.contains(&candidate.edge.relation_type)
        || !allowed_verification.contains(&candidate.edge.verification_state)
        || !edge_touches_root(&candidate.edge, &root_node_id, direction)
        || candidate.node.node_id == root_node_id
        || (candidate.node.node_id != candidate.edge.source_node_id
          && candidate.node.node_id != candidate.edge.target_node_id)
    }) {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NeighborSearchResult {
      release_id,
      root_node_id: echoed_root,
      neighbors,
      next_cursor: value.next_cursor,
    })
  }
}

fn validate_context(context: &RequestContext) -> Result<std::time::Duration, RetrievalDataError> {
  if context.schema_version() != RETRIEVAL_DATA_SCHEMA_VERSION
    || context.content_release().is_none()
  {
    return Err(RetrievalDataError::InvalidRequest);
  }
  let timeout = context.remaining_budget();
  if timeout.is_zero() {
    return Err(RetrievalDataError::Timeout);
  }
  Ok(timeout)
}

fn validate_filters(
  context: &RequestContext,
  filters: &RetrievalFilters,
) -> Result<(), RetrievalDataError> {
  filters.validate()?;
  validate_release(context, &filters.release_id)
}

fn validate_release(
  context: &RequestContext,
  duplicated_release: &ReleaseId,
) -> Result<(), RetrievalDataError> {
  match context.content_release() {
    Some(release) if release == duplicated_release => Ok(()),
    _ => Err(RetrievalDataError::InvalidRequest),
  }
}

fn edge_touches_root(
  edge: &NeighborEdge,
  root: &CanonicalId,
  direction: NeighborDirection,
) -> bool {
  match direction {
    NeighborDirection::Outgoing => edge.source_node_id == *root,
    NeighborDirection::Incoming => edge.target_node_id == *root,
    NeighborDirection::Both => edge.source_node_id == *root || edge.target_node_id == *root,
  }
}

fn map_transport_error(error: IslandPortClientError) -> RetrievalDataError {
  match error {
    IslandPortClientError::InvalidRequest => RetrievalDataError::InvalidRequest,
    IslandPortClientError::NotFound => RetrievalDataError::NotFound,
    IslandPortClientError::ContentReleaseUnavailable => {
      RetrievalDataError::ContentReleaseUnavailable
    }
    IslandPortClientError::SchemaIncompatible => RetrievalDataError::SchemaIncompatible,
    IslandPortClientError::Unavailable => RetrievalDataError::Unavailable,
    IslandPortClientError::Timeout => RetrievalDataError::Timeout,
    IslandPortClientError::InconsistentData => RetrievalDataError::InconsistentData,
  }
}

impl From<RetrievalDataValidationError> for RetrievalDataError {
  fn from(_: RetrievalDataValidationError) -> Self {
    Self::InvalidRequest
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope<'a, T> {
  context: RequestContextDto<'a>,
  input: T,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestContextDto<'a> {
  request_id: &'a str,
  deadline_at: String,
  schema_version: &'static str,
  content_release: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SparseVectorDto {
  indices: Vec<u32>,
  values: Vec<f32>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct NodeFiltersDto {
  release_id: String,
  publication_states: Vec<&'static str>,
  verification_states: Vec<&'static str>,
  node_types: Vec<String>,
  languages: Vec<String>,
  dialects: Vec<String>,
  regions: Vec<String>,
  periods: Vec<String>,
  domain_ids: Vec<String>,
  eligible_evidence_ids: Vec<String>,
}

impl From<RetrievalFilters> for NodeFiltersDto {
  fn from(value: RetrievalFilters) -> Self {
    Self {
      release_id: value.release_id.to_string(),
      publication_states: value
        .publication_states
        .into_iter()
        .map(publication_state_name)
        .collect(),
      verification_states: value
        .verification_states
        .into_iter()
        .map(verification_state_name)
        .collect(),
      node_types: value
        .node_types
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      languages: value
        .languages
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      dialects: value
        .dialects
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      regions: value.regions,
      periods: value.periods,
      domain_ids: value
        .domain_ids
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      eligible_evidence_ids: value
        .eligible_evidence_ids
        .into_iter()
        .map(|value| value.to_string())
        .collect(),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct NodeSearchInputDto {
  dense_vector: Vec<f32>,
  sparse_vector: SparseVectorDto,
  filters: NodeFiltersDto,
  limit: usize,
}

impl From<NodeSearchRequest> for NodeSearchInputDto {
  fn from(value: NodeSearchRequest) -> Self {
    Self {
      dense_vector: value.dense_vector.values().to_vec(),
      sparse_vector: SparseVectorDto {
        indices: value.sparse_vector.indices().to_vec(),
        values: value.sparse_vector.values().to_vec(),
      },
      filters: value.filters.into(),
      limit: value.limit,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ScaleFiltersDto {
  release_id: String,
  publication_states: Vec<&'static str>,
  verification_states: Vec<&'static str>,
  domain_ids: Vec<String>,
}

impl From<RetrievalFilters> for ScaleFiltersDto {
  fn from(value: RetrievalFilters) -> Self {
    Self {
      release_id: value.release_id.to_string(),
      publication_states: value
        .publication_states
        .into_iter()
        .map(publication_state_name)
        .collect(),
      verification_states: value
        .verification_states
        .into_iter()
        .map(verification_state_name)
        .collect(),
      domain_ids: value
        .domain_ids
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ScaleSearchInputDto {
  member_node_id: String,
  filters: ScaleFiltersDto,
  limit: usize,
}

impl From<ScaleSearchRequest> for ScaleSearchInputDto {
  fn from(value: ScaleSearchRequest) -> Self {
    Self {
      member_node_id: value.member_node_id.to_string(),
      filters: ScaleFiltersDto::from(value.filters),
      limit: value.limit,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct EdgeFiltersDto {
  release_id: String,
  publication_states: Vec<&'static str>,
  relation_types: Vec<&'static str>,
  verification_states: Vec<&'static str>,
  languages: Vec<String>,
  dialects: Vec<String>,
  regions: Vec<String>,
  periods: Vec<String>,
  domain_ids: Vec<String>,
  applicable_sense_ids: Vec<String>,
  eligible_evidence_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct EdgeSearchInputDto {
  dense_vector: Vec<f32>,
  sparse_vector: SparseVectorDto,
  filters: EdgeFiltersDto,
  limit: usize,
}

impl From<EdgeSearchRequest> for EdgeSearchInputDto {
  fn from(value: EdgeSearchRequest) -> Self {
    let EdgeSearchRequest {
      dense_vector,
      sparse_vector,
      filters,
      relation_types,
      applicable_sense_ids,
      limit,
    } = value;
    Self {
      dense_vector: dense_vector.values().to_vec(),
      sparse_vector: SparseVectorDto {
        indices: sparse_vector.indices().to_vec(),
        values: sparse_vector.values().to_vec(),
      },
      filters: EdgeFiltersDto {
        release_id: filters.release_id.to_string(),
        publication_states: filters
          .publication_states
          .into_iter()
          .map(publication_state_name)
          .collect(),
        relation_types: relation_types
          .into_iter()
          .map(RetrievalRelation::wire_name)
          .collect(),
        verification_states: filters
          .verification_states
          .into_iter()
          .map(verification_state_name)
          .collect(),
        languages: filters
          .languages
          .into_iter()
          .map(|value| value.as_str().to_string())
          .collect(),
        dialects: filters
          .dialects
          .into_iter()
          .map(|value| value.as_str().to_string())
          .collect(),
        regions: filters.regions,
        periods: filters.periods,
        domain_ids: filters
          .domain_ids
          .into_iter()
          .map(|value| value.as_str().to_string())
          .collect(),
        applicable_sense_ids: applicable_sense_ids
          .into_iter()
          .map(|value| value.to_string())
          .collect(),
        eligible_evidence_ids: filters
          .eligible_evidence_ids
          .into_iter()
          .map(|value| value.to_string())
          .collect(),
      },
      limit,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct NeighborSearchInputDto {
  node_id: String,
  direction: &'static str,
  relation_types: Vec<&'static str>,
  verification_states: Vec<&'static str>,
  languages: Vec<String>,
  domain_ids: Vec<String>,
  release_id: String,
  limit: usize,
  cursor: Option<String>,
}

impl From<NeighborSearchRequest> for NeighborSearchInputDto {
  fn from(value: NeighborSearchRequest) -> Self {
    Self {
      node_id: value.node_id.to_string(),
      direction: match value.direction {
        NeighborDirection::Outgoing => "outgoing",
        NeighborDirection::Incoming => "incoming",
        NeighborDirection::Both => "both",
      },
      relation_types: value
        .relation_types
        .into_iter()
        .map(RetrievalRelation::wire_name)
        .collect(),
      verification_states: value
        .verification_states
        .into_iter()
        .map(verification_state_name)
        .collect(),
      languages: value
        .languages
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      domain_ids: value
        .domain_ids
        .into_iter()
        .map(|value| value.as_str().to_string())
        .collect(),
      release_id: value.release_id.to_string(),
      limit: value.limit,
      cursor: value.cursor,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseDto<T> {
  request_id: String,
  schema_version: String,
  outcome: OutcomeDto,
  value: Option<T>,
  error: Option<ErrorDto>,
  release_id: String,
}

impl<T> ResponseDto<T> {
  fn into_value(self, context: &RequestContext) -> Result<(ReleaseId, T), RetrievalDataError> {
    if self.request_id != context.request_id().as_str()
      || self.schema_version != RETRIEVAL_DATA_SCHEMA_VERSION
      || self.release_id
        != context
          .content_release()
          .map(CanonicalId::as_str)
          .unwrap_or("")
    {
      return Err(RetrievalDataError::InconsistentData);
    }
    let release_id = parse_id(self.release_id)?;
    if matches!(self.outcome, OutcomeDto::Ok) {
      return match (self.value, self.error) {
        (Some(value), None) => Ok((release_id, value)),
        _ => Err(RetrievalDataError::InconsistentData),
      };
    }
    if self.value.is_some() {
      return Err(RetrievalDataError::InconsistentData);
    }
    let code = self
      .error
      .and_then(|error| RetrievalErrorCode::parse(&error.code))
      .ok_or(RetrievalDataError::InconsistentData)?;
    if !self.outcome.accepts(code) {
      return Err(RetrievalDataError::InconsistentData);
    }
    match code {
      RetrievalErrorCode::NotFound => Err(RetrievalDataError::NotFound),
      RetrievalErrorCode::InvalidPayload => Err(RetrievalDataError::InvalidRequest),
      RetrievalErrorCode::SchemaIncompatible => Err(RetrievalDataError::SchemaIncompatible),
      RetrievalErrorCode::ContentReleaseUnavailable => {
        Err(RetrievalDataError::ContentReleaseUnavailable)
      }
      RetrievalErrorCode::DependencyUnavailable => Err(RetrievalDataError::Unavailable),
      RetrievalErrorCode::Timeout => Err(RetrievalDataError::Timeout),
    }
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeDto {
  Ok,
  Missing,
  InvalidPayload,
  VersionMismatch,
  Unavailable,
  Timeout,
}

impl OutcomeDto {
  fn accepts(&self, code: RetrievalErrorCode) -> bool {
    match self {
      Self::Missing => code == RetrievalErrorCode::NotFound,
      Self::InvalidPayload => code == RetrievalErrorCode::InvalidPayload,
      Self::VersionMismatch => matches!(
        code,
        RetrievalErrorCode::SchemaIncompatible | RetrievalErrorCode::ContentReleaseUnavailable
      ),
      Self::Unavailable => code == RetrievalErrorCode::DependencyUnavailable,
      Self::Timeout => code == RetrievalErrorCode::Timeout,
      Self::Ok => false,
    }
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RetrievalErrorCode {
  NotFound,
  InvalidPayload,
  SchemaIncompatible,
  ContentReleaseUnavailable,
  DependencyUnavailable,
  Timeout,
}

impl RetrievalErrorCode {
  fn parse(value: &str) -> Option<Self> {
    match value {
      "not_found" => Some(Self::NotFound),
      "invalid_payload" => Some(Self::InvalidPayload),
      "schema_incompatible" => Some(Self::SchemaIncompatible),
      "content_release_unavailable" => Some(Self::ContentReleaseUnavailable),
      "dependency_unavailable" => Some(Self::DependencyUnavailable),
      "timeout" => Some(Self::Timeout),
      _ => None,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorDto {
  code: String,
  #[serde(rename = "message")]
  _message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidatesDto<T> {
  candidates: Vec<T>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeCandidateDto {
  node_id: String,
  score: f32,
  matched_by: Vec<String>,
  payload: NodeCandidatePayloadDto,
}

impl NodeCandidateDto {
  fn into_domain(self) -> Result<NodeCandidate, RetrievalDataError> {
    const MATCH_TYPES: &[&str] = &[
      "dense",
      "sparse",
      "canonical_label",
      "alias",
      "translation",
      "transliteration",
      "abbreviation",
      "formula",
      "domain_term",
    ];
    if self.matched_by.is_empty()
      || self.matched_by.len() > MATCH_TYPES.len()
      || self
        .matched_by
        .iter()
        .any(|value| !MATCH_TYPES.contains(&value.as_str()))
      || self.matched_by.iter().collect::<BTreeSet<_>>().len() != self.matched_by.len()
    {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NodeCandidate {
      node_id: parse_id(self.node_id)?,
      score: parse_score(self.score)?,
      matched_by: self.matched_by,
      payload: self.payload.into_domain()?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeCandidatePayloadDto {
  node_type: String,
  sense_id: Option<String>,
  canonical_label: String,
  verification_state: String,
}

impl NodeCandidatePayloadDto {
  fn into_domain(self) -> Result<NodeCandidatePayload, RetrievalDataError> {
    if self.canonical_label.trim().is_empty() || self.canonical_label.chars().count() > 512 {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NodeCandidatePayload {
      node_type: RetrievalNodeType::new(self.node_type)
        .map_err(|_| RetrievalDataError::InconsistentData)?,
      sense_id: self.sense_id.map(parse_id).transpose()?,
      canonical_label: self.canonical_label,
      verification_state: parse_verification(&self.verification_state)?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScaleCandidateDto {
  scale_id: String,
  score: f32,
  member_position: u32,
  verification_state: String,
  fact_ids: Vec<String>,
}

impl ScaleCandidateDto {
  fn into_domain(self) -> Result<ScaleCandidate, RetrievalDataError> {
    if self.member_position == 0 || self.fact_ids.is_empty() || self.fact_ids.len() > 50 {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(ScaleCandidate {
      scale_id: parse_id(self.scale_id)?,
      score: parse_score(self.score)?,
      member_position: self.member_position,
      verification_state: parse_verification(&self.verification_state)?,
      fact_ids: self
        .fact_ids
        .into_iter()
        .map(parse_id)
        .collect::<Result<_, _>>()?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EdgeCandidateDto {
  edge_id: String,
  score: f32,
  source_node_id: String,
  target_node_id: String,
  relation_type: String,
  relation_registry_version: u32,
  fact_id: String,
  fact_revision: u32,
  verification_state: String,
}

impl EdgeCandidateDto {
  fn into_domain(self) -> Result<EdgeCandidate, RetrievalDataError> {
    if self.relation_registry_version != RELATION_REGISTRY_VERSION || self.fact_revision == 0 {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(EdgeCandidate {
      edge_id: parse_id(self.edge_id)?,
      score: parse_score(self.score)?,
      source_node_id: parse_id(self.source_node_id)?,
      target_node_id: parse_id(self.target_node_id)?,
      relation_type: RetrievalRelation::from_wire_name(&self.relation_type)
        .map_err(|_| RetrievalDataError::InconsistentData)?,
      relation_registry_version: self.relation_registry_version,
      fact_id: parse_id(self.fact_id)?,
      fact_revision: self.fact_revision,
      verification_state: parse_verification(&self.verification_state)?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeighborsDto {
  root_node_id: String,
  neighbors: Vec<NeighborCandidateDto>,
  next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeighborCandidateDto {
  edge: NeighborEdgeDto,
  node: NeighborNodeDto,
}

impl NeighborCandidateDto {
  fn into_domain(self) -> Result<NeighborCandidate, RetrievalDataError> {
    Ok(NeighborCandidate {
      edge: self.edge.into_domain()?,
      node: self.node.into_domain()?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeighborEdgeDto {
  edge_id: String,
  source_node_id: String,
  target_node_id: String,
  relation_type: String,
  relation_registry_version: u32,
  fact_id: String,
  fact_revision: u32,
  verification_state: String,
}

impl NeighborEdgeDto {
  fn into_domain(self) -> Result<NeighborEdge, RetrievalDataError> {
    if self.relation_registry_version != RELATION_REGISTRY_VERSION || self.fact_revision == 0 {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NeighborEdge {
      edge_id: parse_id(self.edge_id)?,
      source_node_id: parse_id(self.source_node_id)?,
      target_node_id: parse_id(self.target_node_id)?,
      relation_type: RetrievalRelation::from_wire_name(&self.relation_type)
        .map_err(|_| RetrievalDataError::InconsistentData)?,
      relation_registry_version: self.relation_registry_version,
      fact_id: parse_id(self.fact_id)?,
      fact_revision: self.fact_revision,
      verification_state: parse_verification(&self.verification_state)?,
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeighborNodeDto {
  node_id: String,
  node_type: String,
  canonical_label: String,
}

impl NeighborNodeDto {
  fn into_domain(self) -> Result<NeighborNode, RetrievalDataError> {
    if self.canonical_label.trim().is_empty() || self.canonical_label.chars().count() > 512 {
      return Err(RetrievalDataError::InconsistentData);
    }
    Ok(NeighborNode {
      node_id: parse_id(self.node_id)?,
      node_type: RetrievalNodeType::new(self.node_type)
        .map_err(|_| RetrievalDataError::InconsistentData)?,
      canonical_label: self.canonical_label,
    })
  }
}

fn parse_id(value: impl AsRef<str>) -> Result<CanonicalId, RetrievalDataError> {
  CanonicalId::new(value).map_err(|_| RetrievalDataError::InconsistentData)
}

fn parse_score(value: f32) -> Result<RetrievalDataScore, RetrievalDataError> {
  RetrievalDataScore::new(value).map_err(|_| RetrievalDataError::InconsistentData)
}

fn parse_verification(value: &str) -> Result<RetrievalVerificationState, RetrievalDataError> {
  match value {
    "verified" => Ok(RetrievalVerificationState::Verified),
    "exploratory" => Ok(RetrievalVerificationState::Exploratory),
    _ => Err(RetrievalDataError::InconsistentData),
  }
}

const fn publication_state_name(value: RetrievalPublicationState) -> &'static str {
  match value {
    RetrievalPublicationState::Published => "published",
  }
}

const fn verification_state_name(value: RetrievalVerificationState) -> &'static str {
  match value {
    RetrievalVerificationState::Verified => "verified",
    RetrievalVerificationState::Exploratory => "exploratory",
  }
}

#[cfg(test)]
mod tests {
  use std::{sync::Mutex, time::Duration};

  use time::{format_description::well_known::Rfc3339, OffsetDateTime};

  use super::*;
  use crate::domain::{
    canonical::LanguageTag,
    request_context::RequestId,
    retrieval_data::{DenseQueryVector, SparseQueryVector, DENSE_QUERY_VECTOR_DIMENSIONS},
  };

  struct FakeTransport {
    response: Mutex<Vec<u8>>,
    request: Mutex<Option<(&'static str, Vec<u8>)>>,
  }

  #[async_trait]
  impl IslandPortTransport for FakeTransport {
    async fn post_json(
      &self,
      path: &'static str,
      body: Vec<u8>,
      _timeout: Duration,
    ) -> Result<Vec<u8>, IslandPortClientError> {
      *self.request.lock().unwrap() = Some((path, body));
      Ok(self.response.lock().unwrap().clone())
    }
  }

  fn context() -> RequestContext {
    let deadline = (OffsetDateTime::now_utc() + time::Duration::seconds(30))
      .replace_nanosecond(0)
      .unwrap();
    RequestContext::new(
      RequestId::new("request-1").unwrap(),
      deadline,
      RETRIEVAL_DATA_SCHEMA_VERSION,
      Some(CanonicalId::new("knowledge-2026-09").unwrap()),
    )
    .unwrap()
  }

  fn transport(response: &str) -> Arc<FakeTransport> {
    Arc::new(FakeTransport {
      response: Mutex::new(response.as_bytes().to_vec()),
      request: Mutex::new(None),
    })
  }

  fn filters(release: &str) -> RetrievalFilters {
    RetrievalFilters::verified(CanonicalId::new(release).unwrap())
  }

  fn vectors() -> (DenseQueryVector, SparseQueryVector) {
    (
      DenseQueryVector::new(vec![0.25; DENSE_QUERY_VECTOR_DIMENSIONS]).unwrap(),
      SparseQueryVector::new(vec![1, 4], vec![1.0, 0.5]).unwrap(),
    )
  }

  #[tokio::test]
  async fn sends_strict_envelope_and_accepts_valid_node_candidates() {
    let response = r#"{
      "request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok",
      "value":{"candidates":[{"node_id":"node-1","score":0.93,"matched_by":["dense"],
      "payload":{"node_type":"lexical_sense","sense_id":"sense-1","canonical_label":"hot","verification_state":"verified"}}]},
      "error":null,"release_id":"knowledge-2026-09"}"#;
    let transport = transport(response);
    let client = IslandPortRetrievalClient::new(transport.clone());
    let (dense_vector, sparse_vector) = vectors();
    let result = client
      .search_nodes(
        &context(),
        NodeSearchRequest {
          dense_vector,
          sparse_vector,
          filters: filters("knowledge-2026-09"),
          limit: 20,
        },
      )
      .await
      .unwrap();
    assert_eq!(result.candidates[0].node_id.as_str(), "node-1");
    let request = transport.request.lock().unwrap();
    let (path, body) = request.as_ref().unwrap();
    assert_eq!(*path, "/api/v1/nodes/search");
    let body: serde_json::Value = serde_json::from_slice(body).unwrap();
    assert_eq!(body["context"]["content_release"], "knowledge-2026-09");
    assert_eq!(body["input"]["filters"]["release_id"], "knowledge-2026-09");
  }

  #[tokio::test]
  async fn rejects_duplicate_release_mismatch_before_transport() {
    let transport = transport("{}");
    let client = IslandPortRetrievalClient::new(transport.clone());
    let result = client
      .search_scales(
        &context(),
        ScaleSearchRequest {
          member_node_id: CanonicalId::new("node-1").unwrap(),
          filters: filters("another-release"),
          limit: 5,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InvalidRequest));
    assert!(transport.request.lock().unwrap().is_none());
  }

  #[tokio::test]
  async fn rejects_response_echo_and_unknown_fields() {
    let wrong_echo = transport(
      r#"{"request_id":"other","schema_version":"retrieval-data-v1","outcome":"ok","value":{"candidates":[]},"error":null,"release_id":"knowledge-2026-09"}"#,
    );
    let client = IslandPortRetrievalClient::new(wrong_echo);
    let result = client
      .search_scales(
        &context(),
        ScaleSearchRequest {
          member_node_id: CanonicalId::new("node-1").unwrap(),
          filters: filters("knowledge-2026-09"),
          limit: 5,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));

    let unknown = transport(
      r#"{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{"candidates":[]},"error":null,"release_id":"knowledge-2026-09","extra":true}"#,
    );
    let client = IslandPortRetrievalClient::new(unknown);
    let result = client
      .search_scales(
        &context(),
        ScaleSearchRequest {
          member_node_id: CanonicalId::new("node-1").unwrap(),
          filters: filters("knowledge-2026-09"),
          limit: 5,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));
  }

  #[tokio::test]
  async fn rejects_unrequested_relation_and_invalid_neighbor_topology() {
    let edge = transport(
      r#"{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{"candidates":[{"edge_id":"edge-1","score":0.9,"source_node_id":"node-1","target_node_id":"node-2","relation_type":"antonym","relation_registry_version":1,"fact_id":"fact-1","fact_revision":1,"verification_state":"verified"}]},"error":null,"release_id":"knowledge-2026-09"}"#,
    );
    let client = IslandPortRetrievalClient::new(edge);
    let (dense_vector, sparse_vector) = vectors();
    let result = client
      .search_edges(
        &context(),
        EdgeSearchRequest {
          dense_vector,
          sparse_vector,
          filters: filters("knowledge-2026-09"),
          relation_types: vec![RetrievalRelation::from_wire_name("higher_degree_than").unwrap()],
          applicable_sense_ids: vec![],
          limit: 20,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));

    let neighbor = transport(
      r#"{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{"root_node_id":"node-1","neighbors":[{"edge":{"edge_id":"edge-1","source_node_id":"node-2","target_node_id":"node-3","relation_type":"higher_degree_than","relation_registry_version":1,"fact_id":"fact-1","fact_revision":2,"verification_state":"verified"},"node":{"node_id":"node-3","node_type":"lexical_sense","canonical_label":"scorching"}}],"next_cursor":null},"error":null,"release_id":"knowledge-2026-09"}"#,
    );
    let client = IslandPortRetrievalClient::new(neighbor);
    let result = client
      .search_neighbors(
        &context(),
        NeighborSearchRequest {
          node_id: CanonicalId::new("node-1").unwrap(),
          direction: NeighborDirection::Both,
          relation_types: vec![RetrievalRelation::from_wire_name("higher_degree_than").unwrap()],
          verification_states: vec![RetrievalVerificationState::Verified],
          languages: vec![LanguageTag::parse("en").unwrap()],
          domain_ids: vec![],
          release_id: CanonicalId::new("knowledge-2026-09").unwrap(),
          limit: 20,
          cursor: None,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));
  }

  #[tokio::test]
  async fn rejects_unknown_node_families_and_non_v1_relation_registry_responses() {
    let unknown_family = transport(
      r#"{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{"candidates":[{"node_id":"node-1","score":0.9,"matched_by":["dense"],"payload":{"node_type":"custom_concept","sense_id":null,"canonical_label":"unsafe","verification_state":"verified"}}]},"error":null,"release_id":"knowledge-2026-09"}"#,
    );
    let client = IslandPortRetrievalClient::new(unknown_family);
    let (dense_vector, sparse_vector) = vectors();
    let result = client
      .search_nodes(
        &context(),
        NodeSearchRequest {
          dense_vector,
          sparse_vector,
          filters: filters("knowledge-2026-09"),
          limit: 20,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));

    let wrong_registry = transport(
      r#"{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{"candidates":[{"edge_id":"edge-1","score":0.9,"source_node_id":"node-1","target_node_id":"node-2","relation_type":"higher_degree_than","relation_registry_version":2,"fact_id":"fact-1","fact_revision":1,"verification_state":"verified"}]},"error":null,"release_id":"knowledge-2026-09"}"#,
    );
    let client = IslandPortRetrievalClient::new(wrong_registry);
    let (dense_vector, sparse_vector) = vectors();
    let result = client
      .search_edges(
        &context(),
        EdgeSearchRequest {
          dense_vector,
          sparse_vector,
          filters: filters("knowledge-2026-09"),
          relation_types: vec![RetrievalRelation::from_wire_name("higher_degree_than").unwrap()],
          applicable_sense_ids: vec![],
          limit: 20,
        },
      )
      .await;
    assert_eq!(result, Err(RetrievalDataError::InconsistentData));
  }

  #[tokio::test]
  async fn neighbors_require_exact_fact_revision_and_registry_version() {
    for (registry, revision) in [(2, 1), (1, 0)] {
      let response = format!(
        r#"{{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"ok","value":{{"root_node_id":"node-1","neighbors":[{{"edge":{{"edge_id":"edge-1","source_node_id":"node-1","target_node_id":"node-2","relation_type":"higher_degree_than","relation_registry_version":{registry},"fact_id":"fact-1","fact_revision":{revision},"verification_state":"verified"}},"node":{{"node_id":"node-2","node_type":"lexical_sense","canonical_label":"scorching"}}}}],"next_cursor":null}},"error":null,"release_id":"knowledge-2026-09"}}"#,
      );
      let client = IslandPortRetrievalClient::new(transport(&response));
      let result = client
        .search_neighbors(
          &context(),
          NeighborSearchRequest {
            node_id: CanonicalId::new("node-1").unwrap(),
            direction: NeighborDirection::Both,
            relation_types: vec![RetrievalRelation::from_wire_name("higher_degree_than").unwrap()],
            verification_states: vec![RetrievalVerificationState::Verified],
            languages: vec![LanguageTag::parse("en").unwrap()],
            domain_ids: vec![],
            release_id: CanonicalId::new("knowledge-2026-09").unwrap(),
            limit: 20,
            cursor: None,
          },
        )
        .await;
      assert_eq!(result, Err(RetrievalDataError::InconsistentData));
    }
  }

  #[tokio::test]
  async fn enforces_exact_outcome_error_code_topology() {
    let cases = [
      ("missing", "not_found", RetrievalDataError::NotFound),
      (
        "invalid_payload",
        "invalid_payload",
        RetrievalDataError::InvalidRequest,
      ),
      (
        "version_mismatch",
        "schema_incompatible",
        RetrievalDataError::SchemaIncompatible,
      ),
      (
        "version_mismatch",
        "content_release_unavailable",
        RetrievalDataError::ContentReleaseUnavailable,
      ),
      (
        "unavailable",
        "dependency_unavailable",
        RetrievalDataError::Unavailable,
      ),
      ("timeout", "timeout", RetrievalDataError::Timeout),
    ];
    for (outcome, code, expected) in cases {
      let response = format!(
        r#"{{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"{outcome}","value":null,"error":{{"code":"{code}","message":"ignored"}},"release_id":"knowledge-2026-09"}}"#,
      );
      let client = IslandPortRetrievalClient::new(transport(&response));
      let result = client
        .search_scales(
          &context(),
          ScaleSearchRequest {
            member_node_id: CanonicalId::new("node-1").unwrap(),
            filters: filters("knowledge-2026-09"),
            limit: 5,
          },
        )
        .await;
      assert_eq!(result, Err(expected));
    }

    for (outcome, code) in [
      ("missing", "timeout"),
      ("unavailable", "not_found"),
      ("invalid_payload", "dependency_unavailable"),
      ("timeout", "unknown_timeout"),
    ] {
      let response = format!(
        r#"{{"request_id":"request-1","schema_version":"retrieval-data-v1","outcome":"{outcome}","value":null,"error":{{"code":"{code}","message":"must not classify"}},"release_id":"knowledge-2026-09"}}"#,
      );
      let client = IslandPortRetrievalClient::new(transport(&response));
      let result = client
        .search_scales(
          &context(),
          ScaleSearchRequest {
            member_node_id: CanonicalId::new("node-1").unwrap(),
            filters: filters("knowledge-2026-09"),
            limit: 5,
          },
        )
        .await;
      assert_eq!(result, Err(RetrievalDataError::InconsistentData));
    }
  }

  #[test]
  fn errors_and_debug_are_content_free() {
    let transport = transport("{}");
    let client = IslandPortRetrievalClient::new(transport);
    assert_eq!(
      format!("{client:?}"),
      "IslandPortRetrievalClient([redacted])"
    );
    assert_eq!(
      RetrievalDataError::InconsistentData.to_string(),
      "retrieval-data response is inconsistent"
    );
    let parsed = OffsetDateTime::parse("2026-10-01T12:00:00Z", &Rfc3339).unwrap();
    assert_eq!(parsed.year(), 2026);
  }
}
