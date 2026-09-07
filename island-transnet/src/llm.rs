//! OpenAI-compatible provider adapter and translation orchestration.
//!
//! The service validates caller input, selects a provider/model, retries failed
//! requests, and validates model JSON. It does not own HTTP routing or storage.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::format::{parse_llm_response, validate_translation_structure};
use crate::prompt::{build_system_prompt, build_user_prompt};
use crate::types::{
  next_translation_id, InputType, LlmConfig, TranslateRequest, TranslateResponse, TranslationMode,
  TransnetError,
};

/// Translation core backed by an OpenAI-compatible HTTP provider.
///
/// Clones share the underlying connection pool. A service is safe to reuse
/// concurrently; each call owns its request and response values.
#[derive(Clone)]
pub struct TranslationService {
  client: reqwest::Client,
  config: LlmConfig,
}

impl TranslationService {
  /// Creates a provider client from validated runtime configuration.
  ///
  /// # Errors
  ///
  /// Returns an error if the configured HTTP client cannot be constructed.
  pub fn new(config: LlmConfig) -> Result<Self> {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(config.timeout_seconds))
      .build()
      .context("failed to build http client")?;

    Ok(Self { client, config })
  }

  /// Translates one request and returns a structurally validated provider result.
  ///
  /// A call performs at most `max_retries + 1` provider requests. Retries use a
  /// fixed 250 ms delay and apply to transport, provider-status, parsing, and
  /// schema failures.
  ///
  /// # Errors
  ///
  /// Returns [`TransnetError::Validation`] for invalid caller input or unsupported
  /// modes, and [`TransnetError::Llm`] after provider attempts are exhausted.
  pub async fn translate(
    &self,
    request: TranslateRequest,
  ) -> Result<TranslateResponse, TransnetError> {
    validate_request(&request)?;

    let resolved_input_type = request
      .input_type
      .unwrap_or_default()
      .resolve(&request.text);
    let mode = request.mode.unwrap_or_default();

    validate_mode_combination(resolved_input_type, mode)?;

    let user_prompt = build_user_prompt(
      &request.text,
      &request.source_lang,
      &request.target_lang,
      resolved_input_type,
      mode,
    );

    let (base_url, model) = self.provider_for(&request.source_lang, &request.target_lang);
    let system_prompt = build_system_prompt(model);

    let body = ChatCompletionRequest {
      model: model.to_owned(),
      messages: vec![
        ChatMessage {
          role: "system",
          content: system_prompt.to_string(),
        },
        ChatMessage {
          role: "user",
          content: user_prompt,
        },
      ],
      temperature: 0.2,
      chat_template_kwargs: if model.to_lowercase().contains("qwen") {
        Some(ChatTemplateKwargs {
          enable_thinking: false,
        })
      } else {
        None
      },
    };

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 0..=self.config.max_retries {
      match self
        .send_request(&endpoint, &body, resolved_input_type, mode)
        .await
      {
        Ok(model_response) => {
          return Ok(TranslateResponse {
            translation_id: next_translation_id(),
            text: request.text,
            translation: model_response.translation,
            source_lang: request.source_lang,
            target_lang: request.target_lang,
            input_type: resolved_input_type,
          });
        }
        Err(err) => {
          warn!(
            attempt = attempt,
            endpoint = %endpoint,
            model = %model,
            error = %err,
            "translation request failed"
          );
          last_error = Some(err);
          if attempt < self.config.max_retries {
            sleep(Duration::from_millis(250)).await;
          }
        }
      }
    }

    Err(TransnetError::Llm(
      last_error
        .unwrap_or_else(|| anyhow!("unknown translation error"))
        .to_string(),
    ))
  }

  fn provider_for(&self, source_lang: &str, target_lang: &str) -> (&str, &str) {
    if is_normal_language(source_lang, target_lang) {
      (
        self
          .config
          .normal_lang_base_url
          .as_ref()
          .unwrap_or(&self.config.base_url)
          .as_str(),
        self
          .config
          .normal_lang_model
          .as_ref()
          .unwrap_or(&self.config.model)
          .as_str(),
      )
    } else {
      (&self.config.base_url, &self.config.model)
    }
  }

  async fn send_request(
    &self,
    endpoint: &str,
    body: &ChatCompletionRequest,
    input_type: InputType,
    mode: TranslationMode,
  ) -> Result<ModelTranslationResponse> {
    let response = self
      .client
      .post(endpoint)
      .bearer_auth(&self.config.api_key)
      .json(body)
      .send()
      .await
      .with_context(|| format!("provider request to {endpoint} failed"))?;

    let status = response.status();
    let response_text = response
      .text()
      .await
      .with_context(|| format!("failed to read provider response from {endpoint}"))?;

    if !status.is_success() {
      return Err(anyhow!(
        "provider returned HTTP {} from {endpoint}",
        status.as_u16()
      ));
    }

    let payload: ChatCompletionResponse =
      serde_json::from_str(&response_text).context("failed to parse provider response envelope")?;

    let content = payload
      .choices
      .into_iter()
      .next()
      .and_then(|choice| choice.message.content)
      .ok_or_else(|| anyhow!("provider returned no message content in choices"))?;

    let parsed_json = parse_llm_response(&content)?;
    validate_translation_structure(&parsed_json, &input_type, &mode)?;

    info!(endpoint = endpoint, model = %body.model, "translation response validated");

    Ok(ModelTranslationResponse {
      translation: parsed_json,
    })
  }
}

fn validate_request(request: &TranslateRequest) -> Result<(), TransnetError> {
  if request.text.trim().is_empty() {
    return Err(TransnetError::Validation(
      "text must not be empty".to_string(),
    ));
  }
  if request.source_lang.trim().is_empty() {
    return Err(TransnetError::Validation(
      "source_lang must not be empty".to_string(),
    ));
  }
  if request.target_lang.trim().is_empty() {
    return Err(TransnetError::Validation(
      "target_lang must not be empty".to_string(),
    ));
  }

  Ok(())
}

fn validate_mode_combination(
  input_type: InputType,
  mode: TranslationMode,
) -> Result<(), TransnetError> {
  match (input_type, mode) {
    (InputType::Sentence, TranslationMode::FullAnalysis) => Err(TransnetError::Validation(
      "mode 'full_analysis' is not supported for input_type 'sentence'".to_string(),
    )),
    (
      InputType::Paragraph | InputType::Essay,
      TranslationMode::Explain | TranslationMode::FullAnalysis,
    ) => Err(TransnetError::Validation(format!(
      "modes 'explain' and 'full_analysis' are not supported for input_type '{input_type:?}'"
    ))),
    _ => Ok(()),
  }
}

fn is_normal_language(source_lang: &str, target_lang: &str) -> bool {
  let normal_languages = [
    "english",
    "en",
    "chinese",
    "zh",
    "chinese (simplified)",
    "chinese (traditional)",
    "spanish",
    "es",
    "french",
    "fr",
    "japanese",
    "ja",
    "korean",
    "ko",
    "german",
    "de",
  ];

  let source_lower = source_lang.to_lowercase();
  let target_lower = target_lang.to_lowercase();

  normal_languages.iter().any(|lang| source_lower == *lang)
    && normal_languages.iter().any(|lang| target_lower == *lang)
}

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionRequest {
  model: String,
  messages: Vec<ChatMessage>,
  temperature: f32,
  #[serde(skip_serializing_if = "Option::is_none")]
  chat_template_kwargs: Option<ChatTemplateKwargs>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatTemplateKwargs {
  enable_thinking: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ChatMessage {
  role: &'static str,
  content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
  choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
  message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
  content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelTranslationResponse {
  translation: Value,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_validate_request_rejects_blank_language() {
    let request = TranslateRequest {
      text: "hello".to_string(),
      source_lang: " ".to_string(),
      target_lang: "es".to_string(),
      mode: None,
      input_type: None,
    };

    assert!(matches!(
      validate_request(&request),
      Err(TransnetError::Validation(message)) if message == "source_lang must not be empty"
    ));
  }

  #[test]
  fn test_validate_mode_combination_rejects_paragraph_explanation() {
    assert!(matches!(
      validate_mode_combination(InputType::Paragraph, TranslationMode::Explain),
      Err(TransnetError::Validation(_))
    ));
  }

  #[test]
  fn test_is_normal_language_requires_both_languages_in_allowlist() {
    assert!(is_normal_language("EN", "Spanish"));
    assert!(!is_normal_language("en", "Esperanto"));
  }

  #[test]
  fn test_provider_for_uses_common_language_override() {
    let service = TranslationService::new(LlmConfig {
      api_key: "test".to_string(),
      base_url: "http://default.invalid/v1".to_string(),
      model: "default".to_string(),
      timeout_seconds: 1,
      max_retries: 0,
      normal_lang_base_url: Some("http://common.invalid/v1".to_string()),
      normal_lang_model: Some("common".to_string()),
    })
    .expect("test client should build");

    let (base_url, model) = service.provider_for("en", "es");

    assert_eq!(base_url, "http://common.invalid/v1");
    assert_eq!(model, "common");
  }
}
