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

    // Select URL and model based on language types
    let is_normal_pair = crate::types::is_normal_language(&request.source_lang)
      && crate::types::is_normal_language(&request.target_lang);

    let (base_url, model) = if is_normal_pair {
      (
        self.config.normal_lang_base_url.as_ref().unwrap_or(&self.config.base_url),
        self.config.normal_lang_model.as_ref().unwrap_or(&self.config.model),
      )
    } else {
      (&self.config.base_url, &self.config.model)
    };

    // Use a specific prompt for Qwen models to disable thinking mode
    let system_prompt = if model.to_lowercase().contains("qwen") {
      QWEN_SYSTEM_PROMPT
    } else {
      SYSTEM_PROMPT
    };

    let body = ChatCompletionRequest {
      model: model.to_string(),
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
    };

    let endpoint = format!(
      "{}/chat/completions",
      base_url.trim_end_matches('/')
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
          // Log detailed error information with full context
          let error_msg = format!("{}", err);
          let mut error_details = format!("Error: {}", error_msg);
          
          // Add error chain context
          for (i, cause) in err.chain().skip(1).enumerate() {
            error_details.push_str(&format!("\n  Caused by {}: {}", i + 1, cause));
          }
          
          warn!(
            attempt = attempt,
            endpoint = %endpoint,
            model = %model,
            "translation request failed: {}",
            error_details
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

  async fn send_request(
    &self,
    endpoint: &str,
    body: &ChatCompletionRequest,
  ) -> Result<ModelTranslationResponse> {
    let response_result = self
      .client
      .post(endpoint)
      .bearer_auth(&self.config.api_key)
      .json(body)
      .send()
      .await;

    let response = match response_result {
      Ok(r) => r,
      Err(e) => {
        // Check if this is a timeout error
        if e.is_timeout() {
          return Err(anyhow::anyhow!(
            "Request timeout after {} seconds while connecting to {}",
            self.config.timeout_seconds,
            endpoint
          ))
          .context("Request timeout");
        }
        // Check if this is a connection error
        if e.is_connect() {
          return Err(anyhow::anyhow!(
            "Failed to connect to {}: {}",
            endpoint,
            e
          ))
          .context("Connection error");
        }
        // Other network errors
        return Err(anyhow::anyhow!(
          "Network error for {}: {}",
          endpoint,
          e
        ))
        .context("Network error");
      }
    };

    let status = response.status();
    
    // Read the response body as text first to capture error details
    let response_text_result = response.text().await;
    let response_text = match response_text_result {
      Ok(text) => text,
      Err(e) => {
        if e.is_timeout() {
          return Err(anyhow::anyhow!(
            "Request timeout after {} seconds while reading response from {}",
            self.config.timeout_seconds,
            endpoint
          ))
          .context("Response read timeout");
        }
        return Err(anyhow::anyhow!(
          "Failed to read response body from {}: {}",
          endpoint,
          e
        ))
        .context("Response read error");
      }
    };

    // Check if the status indicates an error
    if !status.is_success() {
      return Err(anyhow::anyhow!(
        "HTTP {} {} - Response body: {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown"),
        response_text
      ))
      .context(format!("Server returned error status for {}", endpoint));
    }

    // Try to parse as JSON
    let payload: ChatCompletionResponse = match serde_json::from_str(&response_text) {
      Ok(p) => p,
      Err(e) => {
        return Err(anyhow::anyhow!(
          "Failed to parse JSON response. Status: {}, Parse error: {}, Response body: {}",
          status.as_u16(),
          e,
          response_text
        ))
        .context("JSON parsing error");
      }
    };

    let content = payload
      .choices
      .into_iter()
      .next()
      .and_then(|choice| choice.message.content)
      .ok_or_else(|| anyhow!("provider returned no message content in choices"))?;

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

const QWEN_SYSTEM_PROMPT: &str = "You are a translation engine. Return strict JSON only. Do not think, reason, or explain - just translate directly without any additional processing.";

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
