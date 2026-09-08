//! Core translation service and its minimal HTTP interface.

/// HTTP routing and status mapping.
pub mod api;
/// Runtime configuration types.
pub mod config;
/// OpenAI-compatible model clients and routing.
pub mod provider;
/// Public HTTP request and response types.
pub mod types;

pub use api::{app_router, AppState};
pub use config::{AppConfig, ProviderConfig, TranslationConfig};
pub use provider::{TranslationError, TranslationService};
pub use types::{TranslateRequest, TranslateResponse};
