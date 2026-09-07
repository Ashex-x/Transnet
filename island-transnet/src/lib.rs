//! Translation service core and HTTP transport.
//!
//! The crate owns request classification, prompt construction, provider response
//! validation, and the Axum router. Persistence and operating-system IPC belong
//! in separate adapters.

/// Axum routes and response-to-status mapping.
pub mod api;
/// Parsing and structural validation for model-generated JSON.
pub mod format;
/// OpenAI-compatible translation provider adapter.
pub mod llm;
/// Prompt selection and construction.
pub mod prompt;
/// Public request, response, configuration, and error types.
pub mod types;
