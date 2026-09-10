//! HTTP boundary, platform middleware, and versioned API routing.

use std::sync::Arc;

use axum::{
  extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
  http::{header, HeaderName, Method, StatusCode},
  middleware,
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use tower_http::{
  cors::{AllowCredentials, AllowOrigin, CorsLayer},
  limit::RequestBodyLimitLayer,
  trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
  application::{
    canonical_lookup::CanonicalLookupService, lookup::LookupService, lookup_job::LookupJobService,
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

/// Shared dependencies used by request handlers.
#[derive(Clone)]
pub struct AppState {
  service: Arc<TranslationService>,
  lookup: Option<Arc<LookupService>>,
  canonical_lookup: Option<Arc<CanonicalLookupService>>,
  lookup_jobs: Option<Arc<LookupJobService>>,
  readiness: Arc<dyn Readiness>,
}

impl AppState {
  /// Creates application state for a translation service.
  pub fn new(service: TranslationService) -> Self {
    Self {
      service: Arc::new(service),
      lookup: None,
      canonical_lookup: None,
      lookup_jobs: None,
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

  fn has_lookup_job_service(&self) -> bool {
    self.lookup_jobs.is_some()
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
    .nest("/v1", v1::router(state.has_lookup_job_service()))
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
          tracing::info_span!(
            "http.request",
            request_id = %request_id,
            method = %request.method(),
            path = %request.uri().path(),
          )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN)),
    )
    .layer(middleware::from_fn(request_id::propagate_request_id))
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
