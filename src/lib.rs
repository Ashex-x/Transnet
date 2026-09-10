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
/// Interfaces implemented by external infrastructure.
pub mod ports;
/// OpenAI-compatible model clients and routing.
pub mod provider;
/// Bounded, redacted resilience controls for outbound providers.
pub mod resilience;
/// Public HTTP request and response types.
pub mod types;

pub use adapters::learning_model::OpenAiLearningModel;
pub use api::{
  app_router, app_router_with_http_config, AlwaysReady, AppState, AuthenticatedLookupJobOwner,
  Readiness,
};
pub use config::{
  AppConfig, HttpConfig, HttpConfigError, ProviderApiKey, ProviderConfig, ProviderResilienceConfig,
  ProviderResilienceConfigs, TranslationConfig,
};
pub use provider::{TranslationError, TranslationProviderMetrics, TranslationService};
pub use resilience::{ProviderMetricsSnapshot, ProviderPolicy, ProviderPolicyError};
pub use types::{TranslateRequest, TranslateResponse};
