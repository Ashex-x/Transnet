use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

// Global self-increment ID counter for translations
static TRANSLATION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate the next self-increment translation ID
pub fn next_translation_id() -> u64 {
  TRANSLATION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  #[default]
  Auto,
  Word,
  Phrase,
  Sentence,
  Paragraph,
  Essay,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
  #[default]
  Basic,
  Explain,
  FullAnalysis,
}

impl InputType {
  pub fn resolve(self, text: &str) -> Self {
    match self {
      Self::Auto => infer_input_type(text),
      explicit => explicit,
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslateRequest {
  pub text: String,
  pub source_lang: String,
  pub target_lang: String,
  pub mode: Option<TranslationMode>,
  pub input_type: Option<InputType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateResponse {
  pub translation_id: u64,
  pub text: String,
  pub translation: Value,
  pub source_lang: String,
  pub target_lang: String,
  pub input_type: InputType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  pub status: String,
  pub service: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub checks: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuccessResponse<T> {
  pub success: bool,
  pub data: T,
}

impl<T> SuccessResponse<T> {
  pub fn new(data: T) -> Self {
    Self {
      success: true,
      data,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
  pub code: String,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
  pub success: bool,
  pub error: ErrorInfo,
}

impl ErrorResponse {
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

#[derive(Debug, Clone, Deserialize)]
pub struct ServerFileConfig {
  pub server: ServerConfig,
  pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  pub host: String,
  pub port: u16,
  pub workers: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
  pub level: String,
  pub format: String,
  pub file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmFileConfig {
  pub openai: LlmConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
  pub api_key: String,
  pub base_url: String,
  pub model: String,
  pub timeout_seconds: u64,
  pub max_retries: u32,
  pub normal_lang_base_url: Option<String>,
  pub normal_lang_model: Option<String>,
}

#[derive(Debug, Error)]
pub enum TransnetError {
  #[error("{0}")]
  Validation(String),
  #[error("llm request failed: {0}")]
  Llm(String),
  #[error("configuration error: {0}")]
  Config(String),
  #[error("internal error: {0}")]
  Internal(String),
}

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

  // Match the detection algorithm from types.md
  // Algorithm is applied in order, so more specific checks come first
  if word_count == 1 {
    InputType::Word
  } else if word_count >= 2 && word_count <= 8 && char_count <= 80 && sentence_markers_count == 0 && !has_line_break {
    // Phrase: 2-8 words, <= 80 chars, no sentence-ending punctuation, no line breaks
    InputType::Phrase
  } else if word_count >= 3 && word_count <= 40 && char_count <= 300 && sentence_markers_count == 1 && !has_line_break {
    // Sentence: 3-40 words, <= 300 chars, single sentence-ending punctuation, no line breaks
    InputType::Sentence
  } else if word_count <= 500 && char_count <= 4000 {
    // Paragraph: <= 500 words, <= 4000 chars (multiple sentences or doesn't fit above criteria)
    InputType::Paragraph
  } else {
    // Essay: anything longer
    InputType::Essay
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn infers_word_input() {
    assert_eq!(infer_input_type("hello"), InputType::Word);
  }

  #[test]
  fn infers_phrase_input() {
    assert_eq!(infer_input_type("very good morning"), InputType::Phrase);
  }

  #[test]
  fn infers_sentence_input() {
    assert_eq!(
      infer_input_type("The quick brown fox jumps."),
      InputType::Sentence
    );
  }

  #[test]
  fn infers_paragraph_input() {
    assert_eq!(
      infer_input_type("One sentence.\nAnother sentence on the next line."),
      InputType::Paragraph
    );
  }

  #[test]
  fn infers_essay_input() {
    let long_text = "This is a very long text that exceeds the paragraph limits. ".repeat(100);
    assert_eq!(infer_input_type(&long_text), InputType::Essay);
  }
}
