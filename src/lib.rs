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
/// Public HTTP request and response types.
pub mod types;

pub use adapters::learning_model::OpenAiLearningModel;
pub use api::{
  app_router, app_router_with_http_config, AlwaysReady, AppState, AuthenticatedLookupJobOwner,
  Readiness,
};
pub use config::{AppConfig, HttpConfig, HttpConfigError, ProviderConfig, TranslationConfig};
pub use provider::{TranslationError, TranslationService};
pub use types::{TranslateRequest, TranslateResponse};
