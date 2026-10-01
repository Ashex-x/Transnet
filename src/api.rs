//! HTTP boundary, platform middleware, and versioned API routing.

use std::{sync::Arc, time::Duration};

use axum::{
  extract::{DefaultBodyLimit, MatchedPath, Request},
  http::StatusCode,
  middleware,
  response::{IntoResponse, Response},
  Router,
};
use tower_http::{
  limit::RequestBodyLimitLayer,
  trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
  application::{
    canonical_read::CanonicalReadService, relationship_page::RelationshipPageRuntime,
    translation::TranslationOrchestrator,
  },
  config::{HttpConfig, HttpConfigError, DEFAULT_MAX_REQUEST_BODY_BYTES},
  domain::capabilities::{KnowledgeCapabilityBundle, ServiceCapabilities},
};

mod envelope;
mod problem;
mod readiness;
mod request_context;
mod request_id;
mod stateless;
mod trace_context;
mod v1;

pub use envelope::{SuccessEnvelope, SuccessMeta};
pub use readiness::{
  AlwaysReady, CanonicalDependencyReadiness, CompositeKnowledgeReadiness,
  KnowledgeProjectionReadiness, KnowledgeReadinessComponents, Readiness, ReadinessComponentState,
  ReadinessReport,
};
pub use v1::{
  knowledge_paths::KnowledgePathUseCase,
  knowledge_views::{
    knowledge_router, KnowledgeRouteDependencies, KnowledgeRouteDependenciesError,
  },
};

use request_id::RequestId;

/// Shared dependencies used by request handlers.
#[derive(Clone)]
pub struct AppState {
  translation_orchestrator: Option<Arc<TranslationOrchestrator>>,
  relationship_page_runtime: Option<Arc<RelationshipPageRuntime>>,
  canonical_read: Option<Arc<CanonicalReadService>>,
  canonical_read_timeout: Option<Duration>,
  readiness: Arc<dyn Readiness>,
  capabilities: ServiceCapabilities,
  knowledge_routes: Option<KnowledgeRouteDependencies>,
  runtime_cancellation: Arc<crate::domain::model_runtime::CancellationSignal>,
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

impl AppState {
  /// Creates application state for the target translation service.
  pub fn new() -> Self {
    Self {
      translation_orchestrator: None,
      relationship_page_runtime: None,
      canonical_read: None,
      canonical_read_timeout: None,
      readiness: Arc::new(AlwaysReady),
      capabilities: ServiceCapabilities::current(DEFAULT_MAX_REQUEST_BODY_BYTES),
      knowledge_routes: None,
      runtime_cancellation: Arc::new(crate::domain::model_runtime::CancellationSignal::default()),
    }
  }

  /// Installs the process-owned signal cancelled when listener drain begins.
  pub fn with_runtime_cancellation(
    mut self,
    cancellation: Arc<crate::domain::model_runtime::CancellationSignal>,
  ) -> Self {
    self.runtime_cancellation = cancellation.clone();
    self.knowledge_routes = self
      .knowledge_routes
      .take()
      .map(|dependencies| dependencies.with_runtime_cancellation(cancellation));
    self
  }

  /// Adds the unified request-local translation operation used by `/api/v1/translations`.
  pub fn with_translation_orchestrator(
    mut self,
    orchestrator: Arc<TranslationOrchestrator>,
  ) -> Self {
    let live_retrieval_available = orchestrator.live_retrieval_available();
    self.translation_orchestrator = Some(orchestrator);
    self.capabilities = self.capabilities.with_translation_orchestrator(true);
    self.capabilities = self
      .capabilities
      .with_live_retrieval(live_retrieval_available);
    self.capabilities = self.capabilities.with_relationship_pages(
      self.relationship_page_runtime.is_some() && self.translation_orchestrator.is_some(),
    );
    self
  }

  /// Enables embedded lexical relationship pages from one complete request-local authority.
  pub fn with_relationship_page_runtime(mut self, runtime: Arc<RelationshipPageRuntime>) -> Self {
    self.relationship_page_runtime = Some(runtime);
    self.capabilities = self.capabilities.with_relationship_pages(
      self.relationship_page_runtime.is_some() && self.translation_orchestrator.is_some(),
    );
    self
  }

  /// Retains the opt-in canonical-only application dependency for BasicCard delivery.
  pub fn with_canonical_read_service(mut self, service: Arc<CanonicalReadService>) -> Self {
    self.canonical_read = Some(service);
    self.canonical_read_timeout = Some(Duration::from_secs(2));
    self
  }

  /// Adds the canonical-only service with its validated per-request deadline bound.
  pub fn with_canonical_read_service_timeout(
    mut self,
    service: Arc<CanonicalReadService>,
    timeout: Duration,
  ) -> Self {
    self.canonical_read = Some(service);
    self.canonical_read_timeout = Some(timeout);
    self
  }

  /// Returns the configured canonical-only application dependency, if explicitly enabled.
  pub fn canonical_read_service(&self) -> Option<&Arc<CanonicalReadService>> {
    self.canonical_read.as_ref()
  }

  /// Returns the configured deadline bound for canonical HTTP requests.
  pub(crate) fn canonical_read_timeout(&self) -> Option<Duration> {
    self.canonical_read_timeout
  }

  /// Adds the dependency probe used by `POST /api/v1/readyz` when no knowledge bundle owns readiness.
  pub fn with_readiness(mut self, readiness: Arc<dyn Readiness>) -> Self {
    if self.knowledge_routes.is_none() {
      self.readiness = readiness;
    }
    self
  }

  /// Replaces the content-free capability declaration derived by runtime composition.
  pub fn with_capabilities(mut self, capabilities: ServiceCapabilities) -> Self {
    self.capabilities =
      capabilities.with_translation_orchestrator(self.translation_orchestrator.is_some());
    self.capabilities = self.capabilities.with_live_retrieval(
      self
        .translation_orchestrator
        .as_ref()
        .is_some_and(|orchestrator| orchestrator.live_retrieval_available()),
    );
    self.capabilities = self.capabilities.with_relationship_pages(
      self.relationship_page_runtime.is_some() && self.translation_orchestrator.is_some(),
    );
    self
  }

  /// Atomically enables the guided knowledge-view and verified-path HTTP routes.
  ///
  /// The dependency bundle is indivisible, so application state cannot register only one route or
  /// advertise knowledge lenses without the complete route composition.
  pub fn with_knowledge_routes(mut self, dependencies: KnowledgeRouteDependencies) -> Self {
    let dependencies = dependencies.with_runtime_cancellation(self.runtime_cancellation.clone());
    self.readiness = dependencies.readiness();
    self.knowledge_routes = Some(dependencies);
    self
  }

  pub(crate) fn translation_orchestrator(&self) -> Option<&Arc<TranslationOrchestrator>> {
    self.translation_orchestrator.as_ref()
  }

  pub(crate) fn relationship_page_runtime(&self) -> Option<&Arc<RelationshipPageRuntime>> {
    self.relationship_page_runtime.as_ref()
  }
  pub(crate) fn capabilities(&self) -> &ServiceCapabilities {
    &self.capabilities
  }

  fn has_knowledge_routes(&self) -> bool {
    self.knowledge_routes.is_some()
  }
}

/// Builds the complete Transnet HTTP router with a safe default HTTP boundary.
pub fn app_router(state: AppState) -> Router {
  build_router(state, DEFAULT_MAX_REQUEST_BODY_BYTES)
}

/// Builds the complete Transnet HTTP router with validated runtime HTTP configuration.
///
/// # Errors
///
/// Returns an error when the request-size configuration is invalid.
pub fn app_router_with_http_config(
  state: AppState,
  config: &HttpConfig,
) -> Result<Router, HttpConfigError> {
  config.validate()?;
  Ok(build_router(state, config.max_request_body_bytes))
}

fn build_router(mut state: AppState, max_request_body_bytes: usize) -> Router {
  let has_knowledge_routes = state.has_knowledge_routes();
  state.capabilities = state
    .capabilities
    .with_knowledge_bundle(if has_knowledge_routes {
      KnowledgeCapabilityBundle::FullyConfigured
    } else {
      KnowledgeCapabilityBundle::Disabled
    })
    .with_max_request_body_bytes(max_request_body_bytes);
  let knowledge_routes = state.knowledge_routes.clone();
  let runtime_cancellation = state.runtime_cancellation.clone();
  let router = Router::new()
    .nest(
      "/api/v1",
      v1::target_router(knowledge_routes, runtime_cancellation),
    )
    .with_state(state)
    .layer(DefaultBodyLimit::max(max_request_body_bytes))
    .layer(RequestBodyLimitLayer::new(max_request_body_bytes))
    .layer(middleware::from_fn(payload_limit_response));
  router
    .layer(
      TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
          let request_id = request
            .extensions()
            .get::<RequestId>()
            .map_or("missing", RequestId::as_str);
          let route = trace_route(request);
          tracing::info_span!(
            "http.request",
            request_id = %request_id,
            method = %request.method(),
            route = %route,
          )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN)),
    )
    .layer(middleware::from_fn(stateless::admit))
    .layer(middleware::from_fn(request_context::establish))
    .layer(middleware::from_fn(trace_context::propagate_trace_parent))
    .layer(middleware::from_fn(request_id::propagate_request_id))
}

fn trace_route<B>(request: &Request<B>) -> &str {
  request
    .extensions()
    .get::<MatchedPath>()
    .map_or("unmatched", MatchedPath::as_str)
}

async fn payload_limit_response(request: Request, next: middleware::Next) -> Response {
  let is_v1 = request.uri().path() == "/api/v1" || request.uri().path().starts_with("/api/v1/");
  let request_id = request.extensions().get::<RequestId>().cloned();
  let response = next.run(request).await;
  if response.status() != StatusCode::PAYLOAD_TOO_LARGE {
    return response;
  }

  if is_v1 {
    if let Some(request_id) = request_id {
      return problem::payload_too_large(&request_id);
    }
  }
  StatusCode::NOT_FOUND.into_response()
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use axum::{
    body::Body,
    extract::{Request as AxumRequest, State},
    http::{Request, StatusCode},
    middleware,
    response::Response,
    routing::get,
    Router,
  };
  use tower::ServiceExt;

  use super::trace_route;

  #[test]
  fn trace_route_never_falls_back_to_a_raw_unmatched_path() {
    let request =
      Request::get("/v1/graph/nodes/sense/source-text-that-must-not-be-logged/neighbors")
        .body(())
        .unwrap();

    assert_eq!(trace_route(&request), "unmatched");
  }

  #[tokio::test]
  async fn trace_route_uses_the_matched_template_for_dynamic_graph_identifiers() {
    let recorded = Arc::new(Mutex::new(None));
    let router = Router::new()
      .route(
        "/v1/graph/nodes/:node_kind/:node_id/neighbors",
        get(|| async { StatusCode::OK }),
      )
      .layer(middleware::from_fn_with_state(
        recorded.clone(),
        capture_route,
      ));

    let response = router
      .oneshot(
        Request::get("/v1/graph/nodes/sense/source-text-that-must-not-be-logged/neighbors")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      recorded.lock().unwrap().as_deref(),
      Some("/v1/graph/nodes/:node_kind/:node_id/neighbors")
    );
  }

  #[tokio::test]
  async fn trace_route_uses_the_matched_template_for_dynamic_sense_identifiers() {
    let recorded = Arc::new(Mutex::new(None));
    let router = Router::new()
      .route("/v1/senses/:sense_id", get(|| async { StatusCode::OK }))
      .layer(middleware::from_fn_with_state(
        recorded.clone(),
        capture_route,
      ));

    let response = router
      .oneshot(
        Request::get("/v1/senses/sense-source-text-that-must-not-be-logged")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      recorded.lock().unwrap().as_deref(),
      Some("/v1/senses/:sense_id")
    );
  }

  async fn capture_route(
    State(recorded): State<Arc<Mutex<Option<String>>>>,
    request: AxumRequest,
    next: middleware::Next,
  ) -> Response {
    *recorded.lock().unwrap() = Some(trace_route(&request).to_string());
    next.run(request).await
  }
}
