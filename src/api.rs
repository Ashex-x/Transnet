//! HTTP boundary, platform middleware, and versioned API routing.

use std::{fmt, sync::Arc};

use axum::{
  extract::{rejection::JsonRejection, DefaultBodyLimit, MatchedPath, Request, State},
  http::{header, HeaderName, Method, StatusCode},
  middleware,
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tower_http::{
  cors::{AllowCredentials, AllowOrigin, CorsLayer},
  limit::RequestBodyLimitLayer,
  trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
  application::{
    canonical_lookup::CanonicalLookupService, graph::GraphService, lookup::LookupService,
    lookup_job::LookupJobService,
  },
  config::{HttpConfig, HttpConfigError, DEFAULT_MAX_REQUEST_BODY_BYTES},
  ports::{
    learning_model::LearningModel,
    lookup_job::{LookupJobOwner, LookupJobStore},
  },
  provider::{TranslationError, TranslationService},
  types::{ErrorResponse, HealthResponse, TranslateRequest},
};

mod problem;
mod readiness;
mod request_id;
mod v1;

pub use readiness::{AlwaysReady, Readiness};

use request_id::RequestId;

/// Minimum number of secret bytes accepted for graph-cursor confidentiality and integrity.
pub const MIN_GRAPH_CURSOR_PROTECTION_KEY_BYTES: usize = 32;

/// Validated secret used to protect opaque graph neighbor cursors.
///
/// This type deliberately redacts its contents in `Debug` output. Its normalized key material is
/// used for both confidentiality and integrity. Hosts serving graph pagination across restarts or
/// multiple replicas must inject the same high-entropy value through
/// [`AppState::with_graph_cursor_protection_key`].
#[derive(Clone)]
pub struct GraphCursorProtectionKey(Arc<[u8]>);

impl GraphCursorProtectionKey {
  /// Creates a graph-cursor protection key from at least 32 bytes of high-entropy secret material.
  ///
  /// # Errors
  ///
  /// Returns an error when `secret` is shorter than the minimum protection-key length.
  pub fn new(secret: impl AsRef<[u8]>) -> Result<Self, GraphCursorProtectionKeyError> {
    let secret = secret.as_ref();
    if secret.len() < MIN_GRAPH_CURSOR_PROTECTION_KEY_BYTES {
      return Err(GraphCursorProtectionKeyError::TooShort);
    }
    Ok(Self(Arc::from(Sha256::digest(secret).to_vec())))
  }

  fn ephemeral() -> Self {
    let mut secret = Vec::with_capacity(MIN_GRAPH_CURSOR_PROTECTION_KEY_BYTES);
    secret.extend(ulid::Ulid::new().to_bytes());
    secret.extend(ulid::Ulid::new().to_bytes());
    Self(Arc::from(Sha256::digest(secret).to_vec()))
  }

  pub(crate) fn as_bytes(&self) -> &[u8] {
    &self.0
  }
}

impl fmt::Debug for GraphCursorProtectionKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("GraphCursorProtectionKey(REDACTED)")
  }
}

/// Validation failure for graph-cursor protection-key material.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum GraphCursorProtectionKeyError {
  /// The supplied key cannot safely provide the required cursor protection.
  #[error(
    "graph cursor protection key must contain at least {MIN_GRAPH_CURSOR_PROTECTION_KEY_BYTES} bytes"
  )]
  TooShort,
}

/// Shared dependencies used by request handlers.
#[derive(Clone)]
pub struct AppState {
  service: Arc<TranslationService>,
  lookup: Option<Arc<LookupService>>,
  canonical_lookup: Option<Arc<CanonicalLookupService>>,
  lookup_jobs: Option<Arc<LookupJobService>>,
  graph: Option<Arc<GraphService>>,
  graph_cursor_protection_key: GraphCursorProtectionKey,
  readiness: Arc<dyn Readiness>,
}

impl AppState {
  /// Creates application state for a translation service.
  ///
  /// Graph pagination starts with an ephemeral process-local protection key. A graph-serving host
  /// must replace it with [`Self::with_graph_cursor_protection_key`] when cursors must survive a
  /// restart or move between replicas.
  pub fn new(service: TranslationService) -> Self {
    Self {
      service: Arc::new(service),
      lookup: None,
      canonical_lookup: None,
      lookup_jobs: None,
      graph: None,
      graph_cursor_protection_key: GraphCursorProtectionKey::ephemeral(),
      readiness: Arc::new(AlwaysReady),
    }
  }

  /// Adds the structured learning-model dependency used by `/v1/lookups`.
  pub fn with_learning_model(mut self, model: Arc<dyn LearningModel>) -> Self {
    self.lookup = Some(Arc::new(LookupService::new(model)));
    self
  }

  /// Adds the deterministic canonical lookup dependency used by eligible `/v1/lookups` requests.
  ///
  /// Requests with automatic language detection or nonblank context intentionally remain on the
  /// injected learning-model path because this foundation does not perform language analysis or
  /// private contextual policy.
  pub fn with_canonical_lookup(mut self, lookup: Arc<CanonicalLookupService>) -> Self {
    self.canonical_lookup = Some(lookup);
    self
  }

  /// Adds the lookup-job store required to expose authorized asynchronous polling.
  ///
  /// Without this injected dependency, the lookup-job route is intentionally not registered.
  pub fn with_lookup_job_store(mut self, store: Arc<dyn LookupJobStore>) -> Self {
    self.lookup_jobs = Some(Arc::new(LookupJobService::new(store)));
    self
  }

  /// Adds the canonical graph service used by the conditional graph-read routes.
  ///
  /// Without this injected dependency, graph routes are intentionally not registered so the
  /// default model-only runtime cannot imply that canonical graph content is available.
  pub fn with_graph_service(mut self, service: Arc<GraphService>) -> Self {
    self.graph = Some(service);
    self
  }

  /// Replaces the process-local graph-cursor protection key with stable secret material.
  ///
  /// Every graph-serving replica and replacement process must use the same high-entropy key when
  /// clients need to resume opaque neighbor cursors across a restart or load-balanced request.
  pub fn with_graph_cursor_protection_key(mut self, key: GraphCursorProtectionKey) -> Self {
    self.graph_cursor_protection_key = key;
    self
  }

  /// Adds the dependency probe used by `GET /readyz`.
  pub fn with_readiness(mut self, readiness: Arc<dyn Readiness>) -> Self {
    self.readiness = readiness;
    self
  }

  pub(crate) fn lookup_job_service(&self) -> Option<&Arc<LookupJobService>> {
    self.lookup_jobs.as_ref()
  }

  pub(crate) fn canonical_lookup_service(&self) -> Option<&Arc<CanonicalLookupService>> {
    self.canonical_lookup.as_ref()
  }

  pub(crate) fn graph_service(&self) -> Option<&Arc<GraphService>> {
    self.graph.as_ref()
  }

  pub(crate) fn graph_cursor_protection_key(&self) -> &[u8] {
    self.graph_cursor_protection_key.as_bytes()
  }

  fn has_lookup_job_service(&self) -> bool {
    self.lookup_jobs.is_some()
  }

  fn has_graph_service(&self) -> bool {
    self.graph.is_some()
  }
}

/// Authenticated owner principal that authorization middleware may attach to a lookup-job poll.
///
/// The HTTP handler never accepts an owner identity from a request header. Middleware must derive
/// this opaque value from an authenticated session before inserting this extension.
#[derive(Clone)]
pub struct AuthenticatedLookupJobOwner(LookupJobOwner);

impl AuthenticatedLookupJobOwner {
  /// Creates an authenticated lookup-job owner extension from a validated opaque principal.
  pub fn new(owner: LookupJobOwner) -> Self {
    Self(owner)
  }

  pub(crate) fn owner(&self) -> &LookupJobOwner {
    &self.0
  }
}

/// Builds the complete Transnet HTTP router with a safe default HTTP boundary.
pub fn app_router(state: AppState) -> Router {
  build_router(state, DEFAULT_MAX_REQUEST_BODY_BYTES, None)
}

/// Builds the complete Transnet HTTP router with validated runtime HTTP configuration.
///
/// # Errors
///
/// Returns an error when the request-size or CORS origin configuration is invalid.
pub fn app_router_with_http_config(
  state: AppState,
  config: &HttpConfig,
) -> Result<Router, HttpConfigError> {
  let cors = cors_layer(config)?;
  Ok(build_router(state, config.max_request_body_bytes, cors))
}

fn build_router(state: AppState, max_request_body_bytes: usize, cors: Option<CorsLayer>) -> Router {
  let router = Router::new()
    .route("/health", get(health))
    .route("/livez", get(livez))
    .route("/readyz", get(readyz))
    .route("/translate", post(translate))
    .nest(
      "/v1",
      v1::router(state.has_lookup_job_service(), state.has_graph_service()),
    )
    .with_state(state)
    .layer(DefaultBodyLimit::max(max_request_body_bytes))
    .layer(RequestBodyLimitLayer::new(max_request_body_bytes))
    .layer(middleware::from_fn(payload_limit_response));
  let router = match cors {
    Some(cors) => router.layer(cors),
    None => router,
  };

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
    .layer(middleware::from_fn(request_id::propagate_request_id))
}

fn trace_route<B>(request: &Request<B>) -> &str {
  request
    .extensions()
    .get::<MatchedPath>()
    .map_or("unmatched", MatchedPath::as_str)
}

fn cors_layer(config: &HttpConfig) -> Result<Option<CorsLayer>, HttpConfigError> {
  let origins = config.origin_header_values()?;
  if origins.is_empty() {
    return Ok(None);
  }

  let credential_origins = origins.clone();
  let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::list(origins))
    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    .allow_headers([
      header::CONTENT_TYPE,
      HeaderName::from_static("x-request-id"),
      HeaderName::from_static("lookup-capability"),
    ])
    .expose_headers([HeaderName::from_static("x-request-id")]);
  if config.allow_credentials {
    Ok(Some(cors.allow_credentials(AllowCredentials::predicate(
      move |origin, _| credential_origins.contains(origin),
    ))))
  } else {
    Ok(Some(cors))
  }
}

async fn payload_limit_response(request: Request, next: middleware::Next) -> Response {
  let is_v1 = request.uri().path() == "/v1" || request.uri().path().starts_with("/v1/");
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
  error(StatusCode::PAYLOAD_TOO_LARGE, "request body too large")
}

async fn health() -> Json<HealthResponse> {
  Json(HealthResponse { status: "ok" })
}

async fn livez() -> Json<HealthResponse> {
  Json(HealthResponse { status: "ok" })
}

async fn readyz(State(state): State<AppState>) -> Response {
  if state.readiness.is_ready().await {
    return (StatusCode::OK, Json(HealthResponse { status: "ok" })).into_response();
  }

  tracing::warn!("readiness probe reported an unavailable dependency");
  (
    StatusCode::SERVICE_UNAVAILABLE,
    Json(HealthResponse {
      status: "unavailable",
    }),
  )
    .into_response()
}

async fn translate(
  State(state): State<AppState>,
  payload: Result<Json<TranslateRequest>, JsonRejection>,
) -> Response {
  let Json(request) = match payload {
    Ok(request) => request,
    Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
      return error(StatusCode::PAYLOAD_TOO_LARGE, "request body too large")
    }
    Err(_) => return error(StatusCode::BAD_REQUEST, "invalid JSON request"),
  };

  match state.service.translate(request).await {
    Ok(response) => (StatusCode::OK, Json(response)).into_response(),
    Err(TranslationError::Validation(message)) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    Err(TranslationError::Provider) => error(
      StatusCode::SERVICE_UNAVAILABLE,
      "translation provider unavailable",
    ),
  }
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
  (
    status,
    Json(ErrorResponse {
      error: message.into(),
    }),
  )
    .into_response()
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

  use super::{trace_route, GraphCursorProtectionKey, GraphCursorProtectionKeyError};

  #[test]
  fn graph_cursor_protection_keys_require_length_and_redact_debug_output() {
    let secret = b"protection-key-must-not-appear-in-debug";
    let key = GraphCursorProtectionKey::new(secret).unwrap();

    assert_eq!(format!("{key:?}"), "GraphCursorProtectionKey(REDACTED)");
    assert!(GraphCursorProtectionKey::new([0_u8; 31]).is_err());
    assert!(matches!(
      GraphCursorProtectionKey::new([0_u8; 31]),
      Err(GraphCursorProtectionKeyError::TooShort)
    ));
  }

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

  async fn capture_route(
    State(recorded): State<Arc<Mutex<Option<String>>>>,
    request: AxumRequest,
    next: middleware::Next,
  ) -> Response {
    *recorded.lock().unwrap() = Some(trace_route(&request).to_string());
    next.run(request).await
  }
}
