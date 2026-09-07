//! Shared domain, wire, configuration, and error types.
//!
//! These types define Transnet's transport-independent contract. Provider wire
//! types remain private to the provider adapter.

use std::{
  collections::HashMap,
  sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

static TRANSLATION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Returns the next process-local translation identifier.
///
/// Identifiers are unique only within one running process and may restart at one
/// after a server restart. They are suitable for response correlation, not
/// durable database identity.
pub fn next_translation_id() -> u64 {
  TRANSLATION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Granularity used to select a translation prompt and response schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  /// Infer the type from the submitted text.
  #[default]
  Auto,
  /// A single whitespace-delimited token.
  Word,
  /// A short fragment without sentence-ending punctuation.
  Phrase,
  /// One short sentence.
  Sentence,
  /// Text within the configured paragraph heuristic limits.
  Paragraph,
  /// Text exceeding paragraph heuristic limits.
  Essay,
}

/// Requested depth of the provider's translation response.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
  /// Return the smallest schema supported for the input type.
  #[default]
  Basic,
  /// Add usage and contextual explanation.
  Explain,
  /// Add lexical or relationship analysis where supported.
  FullAnalysis,
}

impl InputType {
  /// Resolves [`InputType::Auto`] with [`infer_input_type`].
  ///
  /// Explicit variants are returned unchanged.
  pub fn resolve(self, text: &str) -> Self {
    match self {
      Self::Auto => infer_input_type(text),
      explicit => explicit,
    }
  }
}

/// JSON request accepted by the translation endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct TranslateRequest {
  /// Source text to translate; blank text is rejected.
  pub text: String,
  /// Provider-facing source language name or code.
  pub source_lang: String,
  /// Provider-facing target language name or code.
  pub target_lang: String,
  /// Desired response depth; defaults to [`TranslationMode::Basic`].
  pub mode: Option<TranslationMode>,
  /// Explicit text granularity; defaults to [`InputType::Auto`].
  pub input_type: Option<InputType>,
}

/// Successful translation result returned inside [`SuccessResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateResponse {
  /// Process-local correlation identifier.
  pub translation_id: u64,
  /// Original text submitted by the caller.
  pub text: String,
  /// Validated provider JSON whose shape depends on `input_type` and mode.
  pub translation: Value,
  /// Source language as submitted by the caller.
  pub source_lang: String,
  /// Target language as submitted by the caller.
  pub target_lang: String,
  /// Resolved input type used to select and validate the prompt.
  pub input_type: InputType,
}

/// Readiness information returned by the health endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  /// Human-readable readiness state.
  pub status: String,
  /// Stable service name.
  pub service: String,
  /// Optional dependency status values keyed by dependency name.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub checks: Option<HashMap<String, String>>,
}

/// Standard success envelope for HTTP responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuccessResponse<T> {
  /// Always `true` for this envelope.
  pub success: bool,
  /// Endpoint-specific response payload.
  pub data: T,
}

impl<T> SuccessResponse<T> {
  /// Wraps a response payload in the standard success envelope.
  pub fn new(data: T) -> Self {
    Self {
      success: true,
      data,
    }
  }
}

/// Machine-readable error details returned to HTTP clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
  /// Stable uppercase error category.
  pub code: String,
  /// Human-readable error description.
  pub message: String,
}

/// Standard error envelope for HTTP responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
  /// Always `false` for this envelope.
  pub success: bool,
  /// Error category and diagnostic message.
  pub error: ErrorInfo,
}

impl ErrorResponse {
  /// Creates a standard error envelope from string-like values.
  pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      success: false,
      error: ErrorInfo {
        code: code.into(),
        message: message.into(),
      },
    }
  }
}

/// Top-level server configuration file.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerFileConfig {
  /// TCP listener settings.
  pub server: ServerConfig,
  /// Process logging settings.
  pub logging: LoggingConfig,
}

/// TCP server settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  /// Interface or host name passed to the TCP listener.
  pub host: String,
  /// TCP port passed to the listener.
  pub port: u16,
  /// Configured worker count, retained for deployment compatibility.
  pub workers: usize,
}

/// Structured logging settings.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
  /// Default tracing filter when `RUST_LOG` is unset.
  pub level: String,
  /// Output format; `json` selects JSON and other values select compact text.
  pub format: String,
  /// Reserved log-file path; the current executable writes to standard output.
  pub file: Option<String>,
}

/// Top-level LLM provider configuration file.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmFileConfig {
  /// OpenAI-compatible provider settings.
  pub openai: LlmConfig,
}

/// OpenAI-compatible translation provider settings.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
  /// Bearer credential sent to the provider.
  pub api_key: String,
  /// Base URL containing the provider's `/v1` API root.
  pub base_url: String,
  /// Default model identifier.
  pub model: String,
  /// Per-request HTTP timeout in seconds.
  pub timeout_seconds: u64,
  /// Number of retries after the initial provider request.
  pub max_retries: u32,
  /// Optional provider base URL for the common-language route.
  pub normal_lang_base_url: Option<String>,
  /// Optional model for the common-language route.
  pub normal_lang_model: Option<String>,
}

/// Domain error surfaced by the translation service.
#[derive(Debug, Error)]
pub enum TransnetError {
  /// Caller input or requested mode is invalid.
  #[error("{0}")]
  Validation(String),
  /// The provider request or response failed.
  #[error("llm request failed: {0}")]
  Llm(String),
  /// Runtime configuration is missing or invalid.
  #[error("configuration error: {0}")]
  Config(String),
  /// An uncategorized internal operation failed.
  #[error("internal error: {0}")]
  Internal(String),
}

/// Classifies text by ordered length, punctuation, and line-break heuristics.
///
/// Empty input resolves to [`InputType::Paragraph`]; request validation rejects
/// empty text before translation. Counts use Unicode scalar values and
/// whitespace-delimited words.
pub fn infer_input_type(text: &str) -> InputType {
  let trimmed = text.trim();
  if trimmed.is_empty() {
    return InputType::Paragraph;
  }

  let word_count = trimmed.split_whitespace().count();
  let char_count = trimmed.chars().count();
  let sentence_markers_count = trimmed
    .chars()
    .filter(|ch| matches!(ch, '.' | '!' | '?' | '\n'))
    .count();
  let has_line_break = trimmed.contains('\n');

  if word_count == 1 {
    InputType::Word
  } else if (2..=8).contains(&word_count)
    && char_count <= 80
    && sentence_markers_count == 0
    && !has_line_break
  {
    InputType::Phrase
  } else if (3..=40).contains(&word_count)
    && char_count <= 300
    && sentence_markers_count == 1
    && !has_line_break
  {
    InputType::Sentence
  } else if word_count <= 500 && char_count <= 4000 {
    InputType::Paragraph
  } else {
    InputType::Essay
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_infer_input_type_classifies_word() {
    assert_eq!(infer_input_type("hello"), InputType::Word);
  }

  #[test]
  fn test_infer_input_type_classifies_phrase() {
    assert_eq!(infer_input_type("very good morning"), InputType::Phrase);
  }

  #[test]
  fn test_infer_input_type_classifies_sentence() {
    assert_eq!(
      infer_input_type("The quick brown fox jumps."),
      InputType::Sentence
    );
  }

  #[test]
  fn test_infer_input_type_classifies_paragraph() {
    assert_eq!(
      infer_input_type("One sentence.\nAnother sentence on the next line."),
      InputType::Paragraph
    );
  }

  #[test]
  fn test_infer_input_type_classifies_essay() {
    let long_text = "This is a very long text that exceeds the paragraph limits. ".repeat(100);
    assert_eq!(infer_input_type(&long_text), InputType::Essay);
  }
}
