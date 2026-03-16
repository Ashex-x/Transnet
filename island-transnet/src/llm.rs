use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing::warn;
use crate::types::{InputType, LlmConfig, TranslateRequest, TranslateResponse, TransnetError};

#[derive(Clone)]
pub struct TranslationService {
  client: reqwest::Client,
  config: LlmConfig,
}

impl TranslationService {
  pub fn new(config: LlmConfig) -> Result<Self> {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(config.timeout_seconds))
      .build()
      .context("failed to build http client")?;

    Ok(Self { client, config })
  }

  pub async fn translate(&self, request: TranslateRequest) -> Result<TranslateResponse, TransnetError> {
    validate_request(&request)?;

    let resolved_input_type = request.input_type.unwrap_or_default().resolve(&request.text);
    let user_prompt = build_user_prompt(
      &request.text,
      &request.source_lang,
      &request.target_lang,
      resolved_input_type,
    );

    let body = ChatCompletionRequest {
      model: self.config.model.clone(),
      messages: vec![
        ChatMessage {
          role: "system",
          content: SYSTEM_PROMPT.to_string(),
        },
        ChatMessage {
          role: "user",
          content: user_prompt,
        },
      ],
      temperature: 0.2,
    };

    let endpoint = format!(
      "{}/chat/completions",
      self.config.base_url.trim_end_matches('/')
    );

    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 0..=self.config.max_retries {
      match self.send_request(&endpoint, &body).await {
        Ok(model_response) => {
          return Ok(TranslateResponse {
            text: request.text,
            translation: model_response.translation,
            source_lang: request.source_lang,
            target_lang: request.target_lang,
            input_type: resolved_input_type,
            provider: "openai-compatible".to_string(),
            model: self.config.model.clone(),
          });
        }
        Err(err) => {
          warn!(attempt, error = %err, "translation request failed");
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

  async fn send_request(
    &self,
    endpoint: &str,
    body: &ChatCompletionRequest,
  ) -> Result<ModelTranslationResponse> {
    let response = self
      .client
      .post(endpoint)
      .bearer_auth(&self.config.api_key)
      .json(body)
      .send()
      .await
      .with_context(|| format!("request to {endpoint} failed"))?
      .error_for_status()
      .with_context(|| format!("provider returned error for {endpoint}"))?;

    let payload: ChatCompletionResponse = response
      .json()
      .await
      .context("failed to decode provider response")?;

    let content = payload
      .choices
      .into_iter()
      .next()
      .and_then(|choice| choice.message.content)
      .ok_or_else(|| anyhow!("provider returned no message content"))?;

    parse_model_response(&content)
  }
}

fn validate_request(request: &TranslateRequest) -> Result<(), TransnetError> {
  if request.text.trim().is_empty() {
    return Err(TransnetError::Validation("text must not be empty".to_string()));
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
  if let Some(mode) = &request.mode {
    if mode != "basic" {
      return Err(TransnetError::Validation(
        "mode must be 'basic' for the MVP".to_string(),
      ));
    }
  }

  Ok(())
}

fn build_user_prompt(
  text: &str,
  source_lang: &str,
  target_lang: &str,
  input_type: InputType,
) -> String {
  format!(
    "Translate the following text from {source_lang} to {target_lang}. \
Return only a compact JSON object with exactly one key named \"translation\". \
Do not include markdown, code fences, or explanations. \
The detected input type is {input_type:?}. \
Text: {text}"
  )
}

fn parse_model_response(content: &str) -> Result<ModelTranslationResponse> {
  if let Ok(parsed) = serde_json::from_str::<ModelTranslationResponse>(content) {
    return Ok(parsed);
  }

  let start = content.find('{').ok_or_else(|| anyhow!("response did not contain json"))?;
  let end = content.rfind('}').ok_or_else(|| anyhow!("response did not contain closing json"))?;
  serde_json::from_str(&content[start..=end]).context("failed to parse model json payload")
}

const SYSTEM_PROMPT: &str = "You are a translation engine. Return strict JSON only.";

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionRequest {
  model: String,
  messages: Vec<ChatMessage>,
  temperature: f32,
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
  translation: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_raw_json_payload() {
    let parsed = parse_model_response(r#"{"translation":"hola"}"#).unwrap();
    assert_eq!(parsed.translation, "hola");
  }

  #[test]
  fn parses_json_inside_markdown() {
    let parsed = parse_model_response("```json\n{\"translation\":\"hola\"}\n```").unwrap();
    assert_eq!(parsed.translation, "hola");
  }
}
