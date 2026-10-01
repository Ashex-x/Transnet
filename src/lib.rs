//! Core translation service and its minimal HTTP interface.

#![recursion_limit = "256"]

/// Infrastructure implementations of application ports.
pub mod adapters;
/// HTTP routing and status mapping.
pub mod api;
/// Use-case orchestration.
pub mod application;
/// Runtime configuration types.
pub mod config;
/// Pure business types and invariants.
pub mod domain;
/// Process-wide structured file logging.
pub mod logger;
/// Interfaces implemented by external infrastructure.
pub mod ports;
/// OpenAI-compatible model clients and routing.
pub mod provider;
/// Bounded, redacted resilience controls for outbound providers.
pub mod resilience;
/// Owned Unix-domain listener lifecycle and HTTP serving.
#[cfg(unix)]
pub mod server;
/// Public HTTP request and response types.
pub mod types;

pub use adapters::learning_model::OpenAiLearningModel;
pub use adapters::model_runtime::{OpenAiEmbeddingAdapter, OpenAiGenerationAdapter};
pub use api::{
  app_router, app_router_with_http_config, AlwaysReady, AppState, CompositeKnowledgeReadiness,
  GraphCursorProtectionKey, GraphCursorProtectionKeyError, KnowledgePathUseCase,
  KnowledgeProjectionReadiness, KnowledgeReadinessComponents, KnowledgeRouteDependencies,
  KnowledgeRouteDependenciesError, Readiness, ReadinessComponentState, ReadinessReport,
  SuccessEnvelope, SuccessMeta, MIN_GRAPH_CURSOR_PROTECTION_KEY_BYTES,
};
pub use application::canonical_lookup::{CanonicalLookupError, CanonicalLookupService};
pub use application::observability::{
  ClosedMetricsDispatcher, TelemetryDropSnapshot, MAX_IN_FLIGHT_METRIC_RECORDS,
};
pub use config::{
  AppConfig, EnabledCanonicalRuntimeConfig, EnabledKnowledgeRuntimeConfig, HttpConfig,
  HttpConfigError, ProviderApiKey, ProviderConfig, ProviderResilienceConfig,
  ProviderResilienceConfigs, TranslationConfig,
};
pub use domain::capabilities::{
  AnnotationFamilyCapability, CapabilityLimits, GenerationProfileCapability,
  ImageMediaTypeCapability, InputTypeCapability, KnowledgeCapabilityBundle,
  KnowledgeLensCapability, LiveRetrievalCapability, LiveRetrievalDefault, PurposeCapability,
  SchemaVersionCapability, ServiceCapabilities, SourceLanguageCapability, TargetLanguageCapability,
};
pub use domain::model_runtime::{
  CancellationSignal, EmbeddingInput, EphemeralEmbedding, GenerationInput, GenerationOutput,
  GenerationProfile, ModelValueError, ModelVersion, ReasoningBudget,
};
pub use domain::request_context::{RequestContext, RequestContextError, RequestId};
pub use domain::translation_turn::{
  CitationReference, ExternalSourceReference, ImageRegionTranslationResult,
  ProjectedTranslationResult, SegmentTranslationResult, TerminologyDecision, TranslationAnnotation,
  TranslationAnnotationCode, TranslationResultKind, TranslationResultValidationError,
  TranslationReview, TranslationReviewIssue, TranslationReviewState, TranslationTurnResult,
  TranslationVersionMetadata, TurnDetails, TurnTranslation,
};
pub use ports::model_runtime::{
  EmbeddingPort, EmbeddingRequest, GenerationPort, GenerationRequest, GenerationResponse,
  ModelOperationContext, ModelOperationError,
};
pub use provider::{TranslationError, TranslationProviderMetrics, TranslationService};
pub use resilience::{ProviderMetricsSnapshot, ProviderPolicy, ProviderPolicyError};
pub use types::{TranslateRequest, TranslateResponse};
