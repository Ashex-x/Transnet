use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  #[default]
  Auto,
  Word,
  Phrase,
  Sentence,
  Text,
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
  pub mode: Option<String>,
  pub input_type: Option<InputType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateResponse {
  pub text: String,
  pub translation: String,
  pub source_lang: String,
  pub target_lang: String,
  pub input_type: InputType,
  pub provider: String,
  pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthData {
  pub status: String,
  pub service: String,
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

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  if s.is_empty() {
    Ok(None)
  } else {
    Ok(Some(s))
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
  pub api_key: String,
  pub base_url: String,
  pub model: String,
  pub timeout_seconds: u64,
  pub max_retries: u32,
  #[serde(deserialize_with = "deserialize_optional_string", default)]
  pub normal_lang_base_url: Option<String>,
  #[serde(deserialize_with = "deserialize_optional_string", default)]
  pub normal_lang_model: Option<String>,
}

const NORMAL_LANGUAGES: &[&str] = &[
  "english", "en",
  "chinese", "zh", "mandarin",
  "spanish", "es",
  "french", "fr",
  "japanese", "ja",
  "korean", "ko",
  "german", "de",
];

pub fn is_normal_language(lang: &str) -> bool {
  let lang_lower = lang.to_lowercase();
  NORMAL_LANGUAGES.contains(&lang_lower.as_str())
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
    return InputType::Text;
  }

  let word_count = trimmed.split_whitespace().count();
  let sentence_markers = trimmed
    .chars()
    .filter(|ch| matches!(ch, '.' | '!' | '?' | '\n'))
    .count();

  if word_count <= 1 {
    InputType::Word
  } else if word_count <= 6 && sentence_markers == 0 {
    InputType::Phrase
  } else if sentence_markers <= 1 && word_count <= 24 {
    InputType::Sentence
  } else {
    InputType::Text
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
  fn infers_text_input() {
    assert_eq!(
      infer_input_type("One sentence.\nAnother sentence on the next line."),
      InputType::Text
    );
  }
}
