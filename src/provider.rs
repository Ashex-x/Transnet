//! OpenAI-compatible provider clients and text-length routing.

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
  config::{ProviderConfig, TranslationConfig},
  types::{is_language_code, TranslateRequest, TranslateResponse},
};

/// Failure returned by translation validation or model communication.
#[derive(Debug, Error)]
pub enum TranslationError {
  /// The caller supplied an invalid request.
  #[error("{0}")]
  Validation(String),
  /// The selected model could not produce a translation.
  #[error("translation provider unavailable")]
  Provider,
}

/// Translation service backed by Gemma 4 and TranslateGemma.
#[derive(Clone)]
pub struct TranslationService {
  client: Client,
  translation: TranslationConfig,
  gemma4: ProviderConfig,
  translate_gemma: ProviderConfig,
}

impl TranslationService {
  /// Creates a reusable translation service.
  ///
  /// # Errors
  ///
  /// Returns an error when the configured HTTP client cannot be built.
  pub fn new(
    translation: TranslationConfig,
    gemma4: ProviderConfig,
    translate_gemma: ProviderConfig,
  ) -> anyhow::Result<Self> {
    let client = Client::builder()
      .timeout(Duration::from_secs(translation.timeout_seconds))
      .build()?;
    Ok(Self {
      client,
      translation,
      gemma4,
      translate_gemma,
    })
  }

  /// Validates and translates one request with the provider selected by text length.
  ///
  /// # Errors
  ///
  /// Returns [`TranslationError::Validation`] for invalid caller input and
  /// [`TranslationError::Provider`] after all provider attempts fail.
  pub async fn translate(
    &self,
    request: TranslateRequest,
  ) -> Result<TranslateResponse, TranslationError> {
    validate_request(&request)?;

    let use_translate_gemma = request.text.chars().count() > self.translation.long_text_chars;
    let (provider, body) = if use_translate_gemma {
      (
        &self.translate_gemma,
        translate_gemma_body(&self.translate_gemma.model, &request),
      )
    } else {
      (&self.gemma4, gemma4_body(&self.gemma4.model, &request))
    };
    let endpoint = format!(
      "{}/chat/completions",
      provider.base_url.trim_end_matches('/')
    );

    for attempt in 0..=self.translation.max_retries {
      match self.send(&endpoint, &provider.api_key, &body).await {
        Ok(translation) => {
          info!(model = %provider.model, long_text = use_translate_gemma, "translation completed");
          return Ok(TranslateResponse { translation });
        }
        Err(error) => {
          warn!(
            attempt,
            model = %provider.model,
            endpoint = %endpoint,
            error = %error,
            "translation attempt failed"
          );
          if attempt < self.translation.max_retries {
            sleep(Duration::from_millis(self.translation.retry_delay_ms)).await;
          }
        }
      }
    }

    Err(TranslationError::Provider)
  }

  async fn send(
    &self,
    endpoint: &str,
    api_key: &str,
    body: &ChatCompletionRequest,
  ) -> anyhow::Result<String> {
    let response = self
      .client
      .post(endpoint)
      .bearer_auth(api_key)
      .json(body)
      .send()
      .await?;
    let response = response.error_for_status()?;
    let payload: ChatCompletionResponse = response.json().await?;
    let content = payload
      .choices
      .into_iter()
      .next()
      .and_then(|choice| choice.message.content)
      .map(|content| content.trim().to_string())
      .filter(|content| !content.is_empty())
      .ok_or_else(|| anyhow::anyhow!("provider returned no translation"))?;
    Ok(content)
  }
}

fn validate_request(request: &TranslateRequest) -> Result<(), TranslationError> {
  if request.text.trim().is_empty() {
    return Err(TranslationError::Validation(
      "text must not be blank".to_string(),
    ));
  }
  if !is_language_code(&request.source_lang) {
    return Err(TranslationError::Validation(
      "source_lang must be a BCP-47 language code".to_string(),
    ));
  }
  if !is_language_code(&request.target_lang) {
    return Err(TranslationError::Validation(
      "target_lang must be a BCP-47 language code".to_string(),
    ));
  }
  Ok(())
}

fn gemma4_body(model: &str, request: &TranslateRequest) -> ChatCompletionRequest {
  ChatCompletionRequest {
    model: model.to_string(),
    messages: vec![
      ChatMessage {
        role: "system",
        content: MessageContent::Text(
          "Translate accurately and return only the translated text.".to_string(),
        ),
      },
      ChatMessage {
        role: "user",
        content: MessageContent::Text(format!(
          "Translate from {} to {}:\n{}",
          request.source_lang, request.target_lang, request.text
        )),
      },
    ],
    temperature: 0.0,
  }
}

fn translate_gemma_body(model: &str, request: &TranslateRequest) -> ChatCompletionRequest {
  ChatCompletionRequest {
    model: model.to_string(),
    messages: vec![ChatMessage {
      role: "user",
      content: MessageContent::Structured(vec![TranslateGemmaContent {
        content_type: "text",
        source_lang_code: request.source_lang.clone(),
        target_lang_code: request.target_lang.clone(),
        text: request.text.clone(),
      }]),
    }],
    temperature: 0.0,
  }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
  model: String,
  messages: Vec<ChatMessage>,
  temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
  role: &'static str,
  content: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum MessageContent {
  Text(String),
  Structured(Vec<TranslateGemmaContent>),
}

#[derive(Debug, Serialize)]
struct TranslateGemmaContent {
  #[serde(rename = "type")]
  content_type: &'static str,
  source_lang_code: String,
  target_lang_code: String,
  text: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
  choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
  message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
  content: Option<String>,
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn request(text: String) -> TranslateRequest {
    TranslateRequest {
      text,
      source_lang: "en".to_string(),
      target_lang: "zh-CN".to_string(),
    }
  }

  #[test]
  fn test_gemma4_body_uses_standard_chat_messages() {
    let value = serde_json::to_value(gemma4_body("Gemma4", &request("hello".to_string()))).unwrap();
    assert_eq!(value["model"], "Gemma4");
    assert!(value["messages"][1]["content"]
      .as_str()
      .unwrap()
      .contains("hello"));
  }

  #[test]
  fn test_translate_gemma_body_uses_structured_content() {
    let value = serde_json::to_value(translate_gemma_body(
      "TranslateGemma",
      &request("long text".to_string()),
    ))
    .unwrap();
    assert_eq!(value["model"], "TranslateGemma");
    assert_eq!(
      value["messages"][0]["content"][0],
      json!({
        "type": "text",
        "source_lang_code": "en",
        "target_lang_code": "zh-CN",
        "text": "long text"
      })
    );
  }
}
