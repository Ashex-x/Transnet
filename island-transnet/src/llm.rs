use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use tokio::time::{sleep, Duration};
use tracing::warn;
use crate::format::{parse_llm_response, validate_translation_structure};
use crate::prompt::{build_system_prompt, build_user_prompt};
use crate::types::{InputType, LlmConfig, TranslateRequest, TranslateResponse, TransnetError, TranslationMode, next_translation_id};

fn log_llm_response(timestamp: &str, endpoint: &str, model: &str, request: &str, response: &str) {
  // Log to terminal
  println!("\n=== Translation Response [{}] ===", timestamp);
  println!("Endpoint: {}", endpoint);
  println!("Model: {}", model);
  println!("\nRequest:\n{}", request);
  println!("\nResponse (sent to gateway):\n{}", response);
  println!("=== End Translation Response ===\n");

  // Write to log file
  let log_dir = Path::new("Transnet/logs/island-transnet");
  if let Err(e) = fs::create_dir_all(log_dir) {
    eprintln!("Failed to create log directory: {}", e);
    return;
  }

  let log_filename = format!("{}.log", timestamp);
  let log_path = log_dir.join(&log_filename);

  let log_content = format!(
    "=== Translation Response Log [{}] ===\nEndpoint: {}\nModel: {}\n\nRequest:\n{}\n\nResponse (sent to gateway):\n{}\n=== End Log ===\n\n",
    timestamp, endpoint, model, request, response
  );

  if let Err(e) = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&log_path)
    .and_then(|mut file| file.write_all(log_content.as_bytes()))
  {
    eprintln!("Failed to write to log file {}: {}", log_path.display(), e);
  }
}

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
    let mode = request.mode.unwrap_or_default();
    
    // Validate that the (input_type, mode) combination is supported
    validate_mode_combination(resolved_input_type, mode)?;
    
    let user_prompt = build_user_prompt(
      &request.text,
      &request.source_lang,
      &request.target_lang,
      resolved_input_type,
      mode,
    );

    // Determine if this is a normal language translation
    let is_normal_lang = is_normal_language(&request.source_lang, &request.target_lang);

    // Select URL and model based on language types
    let (base_url, model) = if is_normal_lang {
      (
        self.config.normal_lang_base_url.as_ref().unwrap_or(&self.config.base_url),
        self.config.normal_lang_model.as_ref().unwrap_or(&self.config.model),
      )
    } else {
      (&self.config.base_url, &self.config.model)
    };

    // Use a specific prompt for Qwen models to disable thinking mode
    let system_prompt: &str = build_system_prompt(&model);

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
      chat_template_kwargs: if model.to_lowercase().contains("qwen") {
        Some(ChatTemplateKwargs {
          enable_thinking: false,
        })
      } else {
        None
      },
    };

    let endpoint: String = format!(
      "{}/chat/completions",
      base_url.trim_end_matches('/')
    );

    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 0..=self.config.max_retries {
      match self.send_request(&endpoint, &body, resolved_input_type, mode).await {
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
    input_type: InputType,
    mode: TranslationMode,
  ) -> Result<ModelTranslationResponse> {
    // Generate timestamp for logging
    let timestamp = chrono::Utc::now().format("%Y-%m-%d:%H-%M-%S").to_string();

    // Serialize request body for logging
    let request_json = match serde_json::to_string_pretty(body) {
      Ok(json) => json,
      Err(e) => format!("Failed to serialize request: {}", e),
    };

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

    // Parse the LLM response using the format module
    let parsed_json = parse_llm_response(&content)?;
    
    // Validate the structure matches the expected type
    validate_translation_structure(&parsed_json, &input_type, &mode)?;
    
    // Log the parsed JSON response that will be sent to gateway
    let parsed_json_pretty = serde_json::to_string_pretty(&parsed_json)
      .unwrap_or_else(|_| "Failed to serialize parsed JSON".to_string());
    log_llm_response(&timestamp, endpoint, &body.model, &request_json, &parsed_json_pretty);
    
    Ok(ModelTranslationResponse {
      translation: parsed_json,
    })
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

  Ok(())
}

fn validate_mode_combination(input_type: InputType, mode: TranslationMode) -> Result<(), TransnetError> {
  match (input_type, mode) {
    (InputType::Sentence, TranslationMode::FullAnalysis) => {
      return Err(TransnetError::Validation(
        format!("mode 'full_analysis' is not supported for input_type 'sentence'")
      ));
    }
    (InputType::Paragraph | InputType::Essay, TranslationMode::Explain | TranslationMode::FullAnalysis) => {
      return Err(TransnetError::Validation(
        format!("modes 'explain' and 'full_analysis' are not supported for input_type '{:?}'", input_type)
      ));
    }
    _ => Ok(())
  }
}

fn is_normal_language(source_lang: &str, target_lang: &str) -> bool {
  let normal_languages = [
    "english", "en",
    "chinese", "zh", "chinese (simplified)", "chinese (traditional)",
    "spanish", "es",
    "french", "fr",
    "japanese", "ja",
    "korean", "ko",
    "german", "de",
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
  fn parses_raw_json_payload() {
    let parsed = parse_llm_response(r#"{"translation":"hola"}"#).unwrap();
    assert_eq!(parsed["translation"], "hola");
  }

  #[test]
  fn parses_json_inside_markdown() {
    let parsed = parse_llm_response("```json\n{\"translation\":\"hola\"}\n```").unwrap();
    assert_eq!(parsed["translation"], "hola");
  }
}
