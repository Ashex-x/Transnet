//! Strict `POST /api/v1/knowledge/views` HTTP boundary and opaque cursor mapping.

use std::sync::Arc;

use axum::{
  extract::{rejection::JsonRejection, Extension},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::post,
  Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
  application::knowledge_views::{
    knowledge_node_family_wire, KnowledgeViewService, KnowledgeViewServiceError,
  },
  domain::{
    assertion::{CanonicalNodeFamily, CanonicalNodeId},
    canonical::{CanonicalId, LanguageTag},
    knowledge_cursor::{
      KnowledgeCursor, KnowledgeCursorBinding, KnowledgeCursorCodec, KnowledgeCursorRoot,
    },
    knowledge_release::{EdgeCollectionId, NodeCollectionId},
    knowledge_view::{
      KnowledgeEvidenceState, KnowledgeLens, KnowledgeRelevanceReason, KnowledgeRoot,
      KnowledgeViewItem, KnowledgeViewRequest, KnowledgeViewSuperset, VerifiedKnowledgeStep,
    },
    request_context::RequestContext,
    retrieval_data::RetrievalNodeType,
    translation_turn::ResponseLevel,
  },
  ports::active_knowledge_release::ActiveKnowledgeReleasePort,
};

use super::super::{
  envelope::SuccessEnvelope,
  problem::{self, FieldError},
};
use super::knowledge_paths::{self, KnowledgePathUseCase, RequestCancellationFactory};

const RESULT_SCHEMA_VERSION: &str = "knowledge-view-result-v1";
const ASSERTION_VERSION: &str = "canonical-assertions-v1";
const PROJECTION_VERSION: &str = "knowledge-projection-v1";
const LENS_POLICY_VERSION: &str = "knowledge-lenses-v1";
const ORDERING_VERSION: &str = "knowledge-order-v1";

/// Dependencies required to install both target knowledge routes atomically.
#[derive(Clone)]
pub struct KnowledgeRouteDependencies {
  service: Arc<KnowledgeViewService>,
  cursors: Arc<KnowledgeCursorCodec>,
  paths: Arc<dyn KnowledgePathUseCase>,
  runtime_cancellation: Arc<crate::domain::model_runtime::CancellationSignal>,
  readiness: Arc<dyn crate::api::Readiness>,
}

/// Invalid atomic knowledge-route dependency composition.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgeRouteDependenciesError {
  /// The view and path services do not use the same complete immutable projection expectation.
  #[error("knowledge route services use different immutable projection expectations")]
  ProjectionMismatch,
  /// The shared immutable projection expectation is invalid.
  #[error("knowledge route projection expectation is invalid")]
  InvalidProjection,
}

impl KnowledgeRouteDependencies {
  /// Creates the atomic route dependency bundle.
  pub fn new(
    service: Arc<KnowledgeViewService>,
    cursors: Arc<KnowledgeCursorCodec>,
    paths: Arc<dyn KnowledgePathUseCase>,
    active_release: Arc<dyn ActiveKnowledgeReleasePort>,
    readiness_timeout: std::time::Duration,
  ) -> Result<Self, KnowledgeRouteDependenciesError> {
    let execution = service.execution().clone();
    if execution.validate().is_err() {
      return Err(KnowledgeRouteDependenciesError::InvalidProjection);
    }
    if paths.execution_expectation() != &execution {
      return Err(KnowledgeRouteDependenciesError::ProjectionMismatch);
    }
    Ok(Self {
      service,
      cursors,
      paths,
      runtime_cancellation: Arc::new(crate::domain::model_runtime::CancellationSignal::default()),
      readiness: Arc::new(crate::api::CompositeKnowledgeReadiness::configured(
        active_release,
        execution,
        readiness_timeout,
      )),
    })
  }

  /// Connects request cancellation to the runtime drain signal used by the eventual server.
  pub fn with_runtime_cancellation(
    mut self,
    runtime_cancellation: Arc<crate::domain::model_runtime::CancellationSignal>,
  ) -> Self {
    self.runtime_cancellation = runtime_cancellation;
    self
  }

  pub(crate) fn readiness(&self) -> Arc<dyn crate::api::Readiness> {
    self.readiness.clone()
  }
}

/// Builds both strict target knowledge routes for one merge beneath `/api/v1`.
pub fn knowledge_router<S>(dependencies: KnowledgeRouteDependencies) -> Router<S>
where
  S: Clone + Send + Sync + 'static,
{
  routes(dependencies).method_not_allowed_fallback(super::method_not_allowed)
}

pub(super) fn routes<S>(dependencies: KnowledgeRouteDependencies) -> Router<S>
where
  S: Clone + Send + Sync + 'static,
{
  let paths = knowledge_paths::router(dependencies.paths.clone());
  let cancellations = RequestCancellationFactory::new(dependencies.runtime_cancellation.clone());
  Router::new()
    .route("/knowledge/views", post(view))
    .layer(Extension(dependencies))
    .merge(paths)
    .layer(Extension(cancellations))
}

async fn view(
  Extension(state): Extension<KnowledgeRouteDependencies>,
  Extension(context): Extension<RequestContext>,
  payload: Result<Json<KnowledgeViewHttpRequest>, JsonRejection>,
) -> Response {
  let Json(input) = match payload {
    Ok(value) => value,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      return problem::payload_too_large(context.request_id())
    }
    Err(_) => return invalid_json(&context),
  };
  let request = match input.into_domain(state.service.execution()) {
    Ok(value) => value,
    Err(ViewRequestError::Invalid(field, message)) => {
      return invalid_field(&context, field, message)
    }
    Err(ViewRequestError::UnavailableRelease) => return unavailable_release(&context),
  };
  let binding = match cursor_binding(&request, state.service.execution()) {
    Ok(value) => value,
    Err(()) => return inconsistent_configuration(&context),
  };
  let resume_after = match request.cursor.as_deref() {
    Some(encoded) => match state.cursors.decode_for(encoded, &binding) {
      Ok(cursor) if cursor.continuation_tokens().is_empty() => {
        Some(cursor.ordering_key().to_string())
      }
      _ => {
        return invalid_field(
          &context,
          "cursor",
          "must be a valid cursor for this request.",
        )
      }
    },
    None => None,
  };
  let mut domain_request = request;
  domain_request.cursor = None;
  let pinned_context = match context
    .clone()
    .with_content_release(domain_request.release.release_id.clone())
  {
    Ok(value) => value,
    Err(_) => return inconsistent_configuration(&context),
  };
  match state
    .service
    .build_after(&pinned_context, domain_request, resume_after.as_deref())
    .await
  {
    Ok(result) => match KnowledgeViewResponse::from_domain(result, &binding, &state.cursors) {
      Ok(data) => problem::no_store(
        Json(SuccessEnvelope::new(
          data,
          &pinned_context,
          RESULT_SCHEMA_VERSION,
        ))
        .into_response(),
      ),
      Err(()) => inconsistent_configuration(&pinned_context),
    },
    Err(error) => map_service_error(error, &pinned_context),
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeViewHttpRequest {
  root: NodeRefRequest,
  lens: String,
  target_language: String,
  response_level: String,
  content_release: String,
  #[serde(default)]
  cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeRefRequest {
  kind: String,
  id: String,
}

impl KnowledgeViewHttpRequest {
  fn into_domain(
    self,
    execution: &crate::domain::retrieval_data::NeighborProjectionExecutionExpectation,
  ) -> Result<KnowledgeViewRequest, ViewRequestError> {
    let family = parse_node_family(&self.root.kind).ok_or(ViewRequestError::Invalid(
      "root.kind",
      "must be a supported canonical node kind.",
    ))?;
    let node_id = CanonicalId::new(&self.root.id).map_err(|_| {
      ViewRequestError::Invalid("root.id", "must be a valid canonical node identifier.")
    })?;
    if self.root.id.chars().count() > 256 {
      return Err(ViewRequestError::Invalid(
        "root.id",
        "must be at most 256 Unicode characters.",
      ));
    }
    let lens = KnowledgeLens::parse(&self.lens).ok_or(ViewRequestError::Invalid(
      "lens",
      "must be one of the eight closed knowledge lenses.",
    ))?;
    let target_language = LanguageTag::parse(&self.target_language).map_err(|_| {
      ViewRequestError::Invalid("target_language", "must be a valid BCP-47 language tag.")
    })?;
    let response_level = ResponseLevel::parse(&self.response_level).ok_or(
      ViewRequestError::Invalid("response_level", "must be brief, standard, or full."),
    )?;
    if self.content_release != execution.content.release_id.as_str() {
      CanonicalId::new(&self.content_release).map_err(|_| {
        ViewRequestError::Invalid(
          "content_release",
          "must be a valid canonical release identifier.",
        )
      })?;
      return Err(ViewRequestError::UnavailableRelease);
    }
    Ok(KnowledgeViewRequest {
      root: KnowledgeRoot {
        node: CanonicalNodeId::publisher_assigned(family, node_id),
      },
      lens,
      target_language,
      response_level,
      release: execution.content.clone(),
      cursor: self.cursor,
    })
  }
}

#[derive(Debug)]
enum ViewRequestError {
  Invalid(&'static str, &'static str),
  UnavailableRelease,
}

fn cursor_binding(
  request: &KnowledgeViewRequest,
  execution: &crate::domain::retrieval_data::NeighborProjectionExecutionExpectation,
) -> Result<KnowledgeCursorBinding, ()> {
  Ok(KnowledgeCursorBinding {
    root: KnowledgeCursorRoot::new(
      knowledge_node_family_wire(request.root.node.family()),
      request.root.node.id().clone(),
    )
    .map_err(|_| ())?,
    lens: request.lens,
    language: request.target_language.clone(),
    response_level: request.response_level,
    canonical_release: request.release.clone(),
    node_collection_id: NodeCollectionId::parse(execution.node_collection_id.as_str())
      .map_err(|_| ())?,
    node_collection_hash: execution.node_collection_content_hash.clone(),
    edge_collection_id: EdgeCollectionId::parse(execution.edge_collection_id.as_str())
      .map_err(|_| ())?,
    edge_collection_hash: execution.edge_collection_content_hash.clone(),
    assertion_version: ASSERTION_VERSION.into(),
    registry_version: format!("relations-v{}", execution.relationship_registry_version),
    projection_version: PROJECTION_VERSION.into(),
    lens_policy_version: LENS_POLICY_VERSION.into(),
    ordering_version: ORDERING_VERSION.into(),
  })
}

#[derive(Serialize)]
struct KnowledgeViewResponse {
  root: NodeRefResponse,
  lens: &'static str,
  content_release: String,
  canonical_schema_version: String,
  branches: Vec<BranchResponse>,
  items: Vec<ItemResponse>,
  truncated: bool,
  next_cursor: Option<String>,
}

impl KnowledgeViewResponse {
  fn from_domain(
    result: KnowledgeViewSuperset,
    binding: &KnowledgeCursorBinding,
    codec: &KnowledgeCursorCodec,
  ) -> Result<Self, ()> {
    let projected = result.project();
    let admitted = projected
      .iter()
      .map(|item| item.node.clone())
      .collect::<std::collections::BTreeSet<_>>();
    let items = projected
      .into_iter()
      .map(ItemResponse::try_from)
      .collect::<Result<Vec<_>, _>>()?;
    let branches = result
      .branches
      .iter()
      .filter_map(|branch| {
        let item_ids = branch
          .item_ids
          .iter()
          .filter(|id| admitted.contains(*id))
          .map(NodeRefResponse::from)
          .collect::<Vec<_>>();
        (!item_ids.is_empty()).then_some(BranchResponse {
          order: branch.order,
          reason: relevance_name(branch.reason),
          item_ids,
        })
      })
      .collect();
    let next_cursor = match result.next_cursor {
      Some(ordering_key) => Some(
        codec
          .encode(&KnowledgeCursor::new(binding.clone(), ordering_key, Vec::new()).map_err(|_| ())?)
          .map_err(|_| ())?,
      ),
      None => None,
    };
    Ok(Self {
      root: NodeRefResponse::from(&result.request.root.node),
      lens: result.request.lens.as_str(),
      content_release: result.request.release.release_id.to_string(),
      canonical_schema_version: result.request.release.canonical_schema_version.clone(),
      branches,
      items,
      truncated: result.truncated,
      next_cursor,
    })
  }
}

#[derive(Serialize)]
struct BranchResponse {
  order: u16,
  reason: &'static str,
  item_ids: Vec<NodeRefResponse>,
}

#[derive(Serialize)]
struct ItemResponse {
  node: NodeRefResponse,
  order: u16,
  relevance_reason: &'static str,
  evidence_state: &'static str,
  path_to_root: Option<PathResponse>,
}

impl TryFrom<&KnowledgeViewItem> for ItemResponse {
  type Error = ();

  fn try_from(item: &KnowledgeViewItem) -> Result<Self, Self::Error> {
    Ok(Self {
      node: NodeRefResponse::from(&item.node),
      order: item.order,
      relevance_reason: relevance_name(item.branch),
      evidence_state: evidence_state_name(item.evidence_state),
      path_to_root: item
        .path_to_root
        .as_ref()
        .map(|path| {
          Ok::<PathResponse, ()>(PathResponse {
            root: NodeRefResponse::from(&path.root),
            item: NodeRefResponse::from(&path.item),
            steps: path
              .steps
              .iter()
              .map(StepResponse::try_from)
              .collect::<Result<Vec<_>, _>>()?,
          })
        })
        .transpose()?,
    })
  }
}

#[derive(Serialize)]
struct PathResponse {
  root: NodeRefResponse,
  item: NodeRefResponse,
  steps: Vec<StepResponse>,
}

#[derive(Serialize)]
struct StepResponse {
  edge_id: String,
  relationship_revision: u32,
  assertion_id: String,
  assertion_revision: u32,
  traversal_id: String,
  relation_registry_revision: u32,
  relation: &'static str,
  source: NodeRefResponse,
  target: NodeRefResponse,
  evidence_ids: Vec<String>,
  conditions: Vec<ConditionResponse>,
  content_release: String,
  canonical_schema_version: String,
}

impl TryFrom<&VerifiedKnowledgeStep> for StepResponse {
  type Error = ();

  fn try_from(step: &VerifiedKnowledgeStep) -> Result<Self, Self::Error> {
    let assertion = step.projection().assertion();
    let traversal = step.projection().traversal();
    let relation = traversal
      .relation_type
      .rule()
      .require_qdrant_wire_name()
      .map_err(|_| ())?;
    Ok(Self {
      edge_id: traversal.edge_id.to_string(),
      relationship_revision: traversal.relationship_revision,
      assertion_id: assertion.assertion_id.to_string(),
      assertion_revision: assertion.assertion_revision,
      traversal_id: traversal.traversal_id.to_string(),
      relation_registry_revision: traversal.relation_registry_revision,
      relation,
      source: NodeRefResponse::from(&traversal.source),
      target: NodeRefResponse::from(&traversal.target),
      evidence_ids: assertion
        .evidence_ids
        .iter()
        .map(ToString::to_string)
        .collect(),
      conditions: assertion
        .conditions
        .iter()
        .map(|condition| ConditionResponse {
          condition_id: condition.condition_id.to_string(),
          condition_type: condition.condition_type.to_string(),
          parameter_ids: condition
            .parameter_ids
            .iter()
            .map(ToString::to_string)
            .collect(),
        })
        .collect(),
      content_release: step.release().release_id.to_string(),
      canonical_schema_version: step.release().canonical_schema_version.clone(),
    })
  }
}

#[derive(Serialize)]
struct ConditionResponse {
  condition_id: String,
  condition_type: String,
  parameter_ids: Vec<String>,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct NodeRefResponse {
  kind: &'static str,
  id: String,
}

impl From<&CanonicalNodeId> for NodeRefResponse {
  fn from(node: &CanonicalNodeId) -> Self {
    Self {
      kind: knowledge_node_family_wire(node.family()),
      id: node.id().to_string(),
    }
  }
}

fn parse_node_family(value: &str) -> Option<CanonicalNodeFamily> {
  if value == "sense" || value == "lexical_sense" {
    return Some(CanonicalNodeFamily::LexicalSense);
  }
  RetrievalNodeType::new(value)
    .ok()
    .map(CanonicalNodeFamily::from)
}

fn evidence_state_name(value: KnowledgeEvidenceState) -> &'static str {
  match value {
    KnowledgeEvidenceState::Verified => "verified",
    KnowledgeEvidenceState::Inferred => "inferred",
    KnowledgeEvidenceState::Exploratory => "exploratory",
  }
}

fn relevance_name(value: KnowledgeRelevanceReason) -> &'static str {
  match value {
    KnowledgeRelevanceReason::Meaning => "meaning",
    KnowledgeRelevanceReason::Contrast => "contrast",
    KnowledgeRelevanceReason::Usage => "usage",
    KnowledgeRelevanceReason::Form => "form",
    KnowledgeRelevanceReason::Origin => "origin",
    KnowledgeRelevanceReason::Domain => "domain",
    KnowledgeRelevanceReason::Mechanism => "mechanism",
    KnowledgeRelevanceReason::Application => "application",
  }
}

fn invalid_json(context: &RequestContext) -> Response {
  problem::response(
    StatusCode::BAD_REQUEST,
    "invalid_json",
    "Invalid JSON request",
    "The request body is not valid knowledge-view JSON.",
    context.request_id(),
    false,
    Vec::new(),
  )
}

fn invalid_field(context: &RequestContext, field: &'static str, message: &'static str) -> Response {
  problem::response(
    StatusCode::UNPROCESSABLE_ENTITY,
    "invalid_knowledge_view_request",
    "Invalid knowledge-view request",
    "One or more knowledge-view fields are invalid.",
    context.request_id(),
    false,
    vec![FieldError::new(field, message)],
  )
}

fn inconsistent_configuration(context: &RequestContext) -> Response {
  problem::response(
    StatusCode::SERVICE_UNAVAILABLE,
    "knowledge_view_unavailable",
    "Knowledge view unavailable",
    "The configured knowledge projection cannot serve this request.",
    context.request_id(),
    true,
    Vec::new(),
  )
}

fn unavailable_release(context: &RequestContext) -> Response {
  problem::response(
    StatusCode::CONFLICT,
    "content_release_unavailable",
    "Content release unavailable",
    "The requested immutable content release is not available.",
    context.request_id(),
    false,
    Vec::new(),
  )
}

fn map_service_error(error: KnowledgeViewServiceError, context: &RequestContext) -> Response {
  let (status, code, title, detail, retryable) = match error {
    KnowledgeViewServiceError::LensUnavailable => (
      StatusCode::NOT_IMPLEMENTED,
      "knowledge_lens_unavailable",
      "Knowledge lens unavailable",
      "The selected lens has no sound traversal policy in this release.",
      false,
    ),
    KnowledgeViewServiceError::PartialPublication => (
      StatusCode::SERVICE_UNAVAILABLE,
      "knowledge_publication_partial",
      "Knowledge publication incomplete",
      "The immutable knowledge projection is missing authoritative records.",
      true,
    ),
    KnowledgeViewServiceError::DependencyUnavailable => (
      StatusCode::SERVICE_UNAVAILABLE,
      "knowledge_dependency_unavailable",
      "Knowledge dependency unavailable",
      "A required knowledge dependency is unavailable.",
      true,
    ),
    KnowledgeViewServiceError::DeadlineExceeded => (
      StatusCode::GATEWAY_TIMEOUT,
      "deadline_exceeded",
      "Request deadline exceeded",
      "The knowledge-view operation exceeded its shared deadline.",
      true,
    ),
    KnowledgeViewServiceError::InconsistentData => (
      StatusCode::BAD_GATEWAY,
      "knowledge_data_inconsistent",
      "Knowledge data inconsistent",
      "A knowledge dependency contradicted the pinned request.",
      false,
    ),
  };
  problem::response(
    status,
    code,
    title,
    detail,
    context.request_id(),
    retryable,
    Vec::new(),
  )
}

#[cfg(test)]
mod tests {
  use async_trait::async_trait;
  use axum::{
    body::{to_bytes, Body},
    http::Request,
  };
  use time::{format_description::well_known::Rfc3339, OffsetDateTime};
  use tower::ServiceExt;

  use super::*;
  use crate::{
    api::{app_router, AppState},
    config::{ProviderApiKey, ProviderConfig, TranslationConfig},
    domain::{
      canonical::CanonicalReleasePin,
      canonical_content::CanonicalSenseDetails,
      canonical_translation::CanonicalTranslationRevision,
      domain_assessment::DomainInventory,
      embedding_input::{EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION},
      knowledge_hydration::{HydratedAssertionProjection, HydratedKnowledgeNode},
      request_context::RequestId,
      retrieval::RepositoryMatch,
      retrieval_data::{
        EdgeSearchRequest, EdgeSearchResult, NeighborProjectionExecutionExpectation,
        NeighborProjectionExecutionProof, NeighborSearchRequest, NeighborSearchResult,
        NodeSearchRequest, NodeSearchResult, ScaleSearchRequest, ScaleSearchResult,
        RELATION_REGISTRY_VERSION,
      },
    },
    ports::{
      canonical_read::{
        CanonicalAssertionQuery, CanonicalCandidateQuery, CanonicalDomainQuery,
        CanonicalKnowledgeNodeQuery, CanonicalReadContext, CanonicalReadError, CanonicalReadPort,
        CanonicalScaleQuery, CanonicalSenseQuery, CanonicalTranslationQuery,
      },
      retrieval_data::{RetrievalDataError, RetrievalDataPort},
    },
    provider::TranslationService,
  };

  struct EmptyRetrieval {
    proof: NeighborProjectionExecutionProof,
  }

  #[async_trait]
  impl RetrievalDataPort for EmptyRetrieval {
    async fn search_nodes(
      &self,
      _: &RequestContext,
      _: NodeSearchRequest,
    ) -> Result<NodeSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_scales(
      &self,
      _: &RequestContext,
      _: ScaleSearchRequest,
    ) -> Result<ScaleSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_edges(
      &self,
      _: &RequestContext,
      _: EdgeSearchRequest,
    ) -> Result<EdgeSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_neighbors(
      &self,
      _: &RequestContext,
      request: NeighborSearchRequest,
    ) -> Result<NeighborSearchResult, RetrievalDataError> {
      Ok(NeighborSearchResult {
        release_id: request.release_id,
        execution: self.proof.clone(),
        root_node_id: request.node_id,
        neighbors: Vec::new(),
        next_cursor: None,
      })
    }
  }

  struct RootCanonical;

  struct UnusedPathUseCase {
    execution: NeighborProjectionExecutionExpectation,
  }

  #[async_trait]
  impl KnowledgePathUseCase for UnusedPathUseCase {
    fn execution_expectation(&self) -> &NeighborProjectionExecutionExpectation {
      &self.execution
    }

    async fn find(
      &self,
      _: &RequestContext,
      _: &crate::domain::model_runtime::CancellationSignal,
      _: crate::domain::knowledge_view::KnowledgePathRequest,
    ) -> Result<
      crate::domain::knowledge_view::KnowledgePathResult,
      crate::application::knowledge_paths::KnowledgePathSearchError,
    > {
      Err(crate::application::knowledge_paths::KnowledgePathSearchError::DependencyUnavailable)
    }
  }

  struct ActiveRelease(NeighborProjectionExecutionExpectation);

  struct NeverReady;

  #[async_trait]
  impl crate::api::Readiness for NeverReady {
    async fn is_ready(&self) -> bool {
      false
    }
  }

  #[async_trait]
  impl crate::ports::active_knowledge_release::ActiveKnowledgeReleasePort for ActiveRelease {
    async fn active_knowledge_release(
      &self,
      _: &crate::ports::canonical_read::CanonicalReadContext,
    ) -> Result<
      Option<NeighborProjectionExecutionExpectation>,
      crate::ports::active_knowledge_release::ActiveKnowledgeReleaseError,
    > {
      Ok(Some(self.0.clone()))
    }
  }

  #[async_trait]
  impl CanonicalReadPort for RootCanonical {
    async fn active_release(
      &self,
      _: &CanonicalReadContext,
    ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn translations(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalTranslationQuery,
    ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn candidates(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalCandidateQuery,
    ) -> Result<Vec<RepositoryMatch>, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn sense(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalSenseQuery,
    ) -> Result<CanonicalSenseDetails, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn domains(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalDomainQuery,
    ) -> Result<DomainInventory, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn canonical_assertions(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalAssertionQuery,
    ) -> Result<
      Vec<crate::domain::knowledge_hydration::HydratedAssertionProjection>,
      CanonicalReadError,
    > {
      Ok(Vec::new())
    }
    async fn semantic_scales(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalScaleQuery,
    ) -> Result<Vec<crate::domain::knowledge_hydration::HydratedSemanticScale>, CanonicalReadError>
    {
      Ok(Vec::new())
    }
    async fn knowledge_nodes(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      query: CanonicalKnowledgeNodeQuery,
    ) -> Result<Vec<HydratedKnowledgeNode>, CanonicalReadError> {
      Ok(
        query
          .node_ids
          .into_iter()
          .map(|node_id| HydratedKnowledgeNode {
            node_id,
            revision: 1,
            node_type: RetrievalNodeType::Concept,
            sense_id: None,
            canonical_label: "Root".into(),
            language: None,
            domain_ids: Vec::new(),
            evidence_ids: vec![id("evidence-1")],
          })
          .collect(),
      )
    }
  }

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn execution() -> NeighborProjectionExecutionExpectation {
    NeighborProjectionExecutionExpectation {
      content: CanonicalReleasePin::new(id("release-1"), "canonical-v1".into()).unwrap(),
      node_collection_id: id("nodes-1"),
      node_collection_content_hash: format!("sha256:{}", "a".repeat(64)),
      edge_collection_id: id("edges-1"),
      edge_collection_content_hash: format!("sha256:{}", "b".repeat(64)),
      relationship_registry_version: RELATION_REGISTRY_VERSION,
      edge_dense_input_version: EDGE_DENSE_INPUT_VERSION.into(),
      edge_lexical_input_version: EDGE_LEXICAL_INPUT_VERSION.into(),
    }
  }

  fn proof(expected: &NeighborProjectionExecutionExpectation) -> NeighborProjectionExecutionProof {
    NeighborProjectionExecutionProof {
      content: expected.content.clone(),
      node_collection_id: expected.node_collection_id.clone(),
      node_collection_content_hash: expected.node_collection_content_hash.clone(),
      edge_collection_id: expected.edge_collection_id.clone(),
      edge_collection_content_hash: expected.edge_collection_content_hash.clone(),
      relationship_registry_version: expected.relationship_registry_version,
      edge_dense_input_version: expected.edge_dense_input_version.clone(),
      edge_lexical_input_version: expected.edge_lexical_input_version.clone(),
    }
  }

  fn context() -> RequestContext {
    RequestContext::new(
      RequestId::new("request-1").unwrap(),
      OffsetDateTime::parse("2099-01-01T00:00:00.000000Z", &Rfc3339).unwrap(),
      "transnet-v1",
      None,
    )
    .unwrap()
  }

  fn state() -> KnowledgeRouteDependencies {
    let expected = execution();
    KnowledgeRouteDependencies::new(
      Arc::new(KnowledgeViewService::new(
        Arc::new(EmptyRetrieval {
          proof: proof(&expected),
        }),
        Arc::new(RootCanonical),
        expected.clone(),
      )),
      Arc::new(KnowledgeCursorCodec::new(
        crate::domain::knowledge_cursor::KnowledgeCursorProtectionKey::new([9_u8; 32]).unwrap(),
      )),
      Arc::new(UnusedPathUseCase {
        execution: expected.clone(),
      }),
      Arc::new(ActiveRelease(expected)),
      std::time::Duration::from_secs(1),
    )
    .unwrap()
  }

  fn body(cursor: Option<&str>, extra: &str) -> String {
    let cursor = cursor.map_or("null".into(), |value| serde_json::to_string(value).unwrap());
    format!(
      r#"{{"root":{{"kind":"concept","id":"root-1"}},"lens":"meaning","target_language":"en","response_level":"brief","content_release":"release-1","cursor":{cursor}{extra}}}"#
    )
  }

  async fn send(body: String) -> Response {
    knowledge_router(state())
      .layer(Extension(context()))
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/knowledge/views")
          .header("content-type", "application/json")
          .body(Body::from(body))
          .unwrap(),
      )
      .await
      .unwrap()
  }

  fn composed_app(with_knowledge: bool) -> Router {
    let provider = ProviderConfig {
      base_url: "http://127.0.0.1:1/v1".into(),
      model: "unused".into(),
      api_key: ProviderApiKey::new("unused"),
    };
    let legacy = TranslationService::new(
      TranslationConfig {
        long_text_chars: 4_000,
        timeout_seconds: 1,
        max_retries: 0,
        retry_delay_ms: 0,
      },
      provider.clone(),
      provider,
    )
    .unwrap();
    let app_state = AppState::new(legacy);
    app_router(if with_knowledge {
      app_state
        .with_knowledge_routes(state())
        .with_readiness(Arc::new(NeverReady))
    } else {
      app_state
    })
  }

  async fn json(response: Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
  }

  #[tokio::test]
  async fn runtime_composition_registers_both_routes_and_exactly_six_lenses_atomically() {
    let disabled = composed_app(false)
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/v1/knowledge/views")
          .header("content-type", "application/json")
          .body(Body::from(body(None, "")))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

    let capabilities = composed_app(true)
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/v1/capabilities")
          .header("content-type", "application/json")
          .body(Body::from("{}"))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(
      json(capabilities).await["data"]["knowledge_lenses"],
      serde_json::json!(["meaning", "contrast", "usage", "form", "origin", "domain"])
    );

    let view = composed_app(true)
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/v1/knowledge/views")
          .header("content-type", "application/json")
          .body(Body::from(body(None, "")))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(view.status(), StatusCode::OK);

    let path = composed_app(true)
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/v1/knowledge/paths")
          .header("content-type", "application/json")
          .body(Body::from(
            r#"{"from":{"kind":"concept","id":"a"},"to":{"kind":"concept","id":"b"},"target_language":"en","content_release":"release-1","canonical_schema_version":"canonical-v1"}"#,
          ))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(path.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(path.headers()["cache-control"], "no-store");

    let wrong_method = composed_app(true)
      .oneshot(
        Request::builder()
          .method("GET")
          .uri("/api/v1/knowledge/views")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(wrong_method.headers()["cache-control"], "no-store");

    let readiness = composed_app(true)
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/v1/readyz")
          .header("content-type", "application/json")
          .body(Body::from("{}"))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(readiness.status(), StatusCode::OK);
    assert_eq!(
      json(readiness).await["data"]["components"],
      serde_json::json!({
        "canonical_data":"available",
        "retrieval_data":"available",
        "knowledge_projection":"available"
      })
    );
  }

  #[test]
  fn dependency_bundle_rejects_mismatched_view_and_path_projection_snapshots() {
    let expected = execution();
    let mut mismatched = expected.clone();
    mismatched.edge_collection_id = id("edges-2");
    let result = KnowledgeRouteDependencies::new(
      Arc::new(KnowledgeViewService::new(
        Arc::new(EmptyRetrieval {
          proof: proof(&expected),
        }),
        Arc::new(RootCanonical),
        expected.clone(),
      )),
      Arc::new(KnowledgeCursorCodec::new(
        crate::domain::knowledge_cursor::KnowledgeCursorProtectionKey::new([7_u8; 32]).unwrap(),
      )),
      Arc::new(UnusedPathUseCase {
        execution: mismatched,
      }),
      Arc::new(ActiveRelease(expected)),
      std::time::Duration::from_secs(1),
    );
    assert!(matches!(
      result,
      Err(KnowledgeRouteDependenciesError::ProjectionMismatch)
    ));
  }

  #[tokio::test]
  async fn serves_root_only_success_in_shared_envelope_with_no_store() {
    let response = send(body(None, "")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let value: serde_json::Value =
      serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(value["meta"]["request_id"], "request-1");
    assert_eq!(value["meta"]["content_release"], "release-1");
    assert_eq!(value["data"]["items"][0]["node"]["id"], "root-1");
    assert_eq!(value["data"]["truncated"], false);
  }

  #[tokio::test]
  async fn rejects_unknown_fields_as_strict_json_problem() {
    let response = send(body(None, r#", "raw_filters":[]"#)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
      response.headers()["content-type"],
      "application/problem+json"
    );
  }

  #[tokio::test]
  async fn rejects_unavailable_release_and_wrong_method_with_no_store() {
    let response = send(body(None, "").replace("release-1", "release-2")).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()["cache-control"], "no-store");

    let response = knowledge_router(state())
      .layer(Extension(context()))
      .layer(Extension(RequestId::new("request-1").unwrap()))
      .oneshot(
        Request::builder()
          .method("GET")
          .uri("/knowledge/views")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()["cache-control"], "no-store");
  }

  #[tokio::test]
  async fn rejects_tampered_or_differently_bound_cursor_before_service() {
    let response = send(body(Some("k1.invalid.invalid"), "")).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let route_state = state();
    let input = KnowledgeViewHttpRequest {
      root: NodeRefRequest {
        kind: "concept".into(),
        id: "root-1".into(),
      },
      lens: "contrast".into(),
      target_language: "en".into(),
      response_level: "brief".into(),
      content_release: "release-1".into(),
      cursor: None,
    }
    .into_domain(route_state.service.execution())
    .unwrap();
    let binding = cursor_binding(&input, route_state.service.execution()).unwrap();
    let wrong = route_state
      .cursors
      .encode(&KnowledgeCursor::new(binding, "sha256:resume", Vec::new()).unwrap())
      .unwrap();
    let response = knowledge_router(route_state)
      .layer(Extension(context()))
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/knowledge/views")
          .header("content-type", "application/json")
          .body(Body::from(body(Some(&wrong), "")))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()["cache-control"], "no-store");
  }

  #[test]
  fn verified_step_serialization_preserves_the_full_pin() {
    let pin = execution().content;
    let projection = HydratedAssertionProjection::topology_fixture(
      CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id("source")),
      CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id("target")),
      id("edge-1"),
      id("assertion-1"),
      pin.release_id.clone(),
    );
    let step = VerifiedKnowledgeStep::from_hydrated(projection, pin.clone()).unwrap();
    let response = StepResponse::try_from(&step).unwrap();
    assert_eq!(response.content_release, pin.release_id.as_str());
    assert_eq!(
      response.canonical_schema_version,
      pin.canonical_schema_version
    );
  }
}
