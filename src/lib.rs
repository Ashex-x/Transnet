//! Core translation service and its minimal HTTP interface.

#![recursion_limit = "256"]

/// Infrastructure implementations of application ports.
pub mod adapters;
/// HTTP routing and status mapping.
pub mod api;
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

pub use api::{app_router, AppState};
pub use config::{AppConfig, ProviderConfig, TranslationConfig};
pub use provider::{TranslationError, TranslationService};
pub use types::{TranslateRequest, TranslateResponse};
