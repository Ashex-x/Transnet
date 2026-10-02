//! Resilient OpenAI-compatible Gemma4-27B generation provider.

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
  config::ProviderConfig,
  resilience::{
    status_failure, transport_failure, ProviderAttemptError, ProviderMetricsSnapshot,
    ProviderPolicy, ProviderResilience,
  },
};

use crate::domain::model_runtime::GenerationProfile;

tokio::task_local! {
  static STREAM_EVENTS: tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>;
}

/// Provider deltas that may be forwarded by an explicitly streaming transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStreamEvent {
  /// A reasoning delta returned in llama.cpp's separate `reasoning_content` field.
  ReasoningDelta(String),
}

/// Runs one operation with a request-local provider event sink.
pub async fn with_stream_events<F: std::future::Future>(
  sender: tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>,
  future: F,
) -> F::Output {
  STREAM_EVENTS.scope(sender, future).await
}

/// Failure returned by translation validation or model communication.
#[derive(Debug, Error)]
pub enum GenerationProviderError {
  /// The selected model could not produce a translation.
  #[error("translation provider unavailable")]
  Provider,
}

/// Generation service backed by one Gemma4-27B endpoint.
#[derive(Clone)]
pub struct GemmaGenerationProvider {
  gemma4: TranslationProvider,
}

#[derive(Clone)]
struct TranslationProvider {
  client: Client,
  config: ProviderConfig,
  resilience: ProviderResilience,
}

/// Redacted provider counters for the generation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationProviderMetrics {
  /// Counters for requests routed to Gemma 4.
  pub gemma4: ProviderMetricsSnapshot,
}

impl GemmaGenerationProvider {
  /// Runs one provider-neutral operation against the transitional Gemma 4 endpoint.
  ///
  /// Both closed profiles use the same configured model identity.
  pub(crate) async fn generate_with_profile(
    &self,
    profile: GenerationProfile,
    input: &str,
  ) -> Result<(String, Option<String>, String), GenerationProviderError> {
    let instruction = match profile {
      GenerationProfile::Fast => "Complete the requested operation accurately and concisely.",
      GenerationProfile::Reasoning => {
        "Resolve the requested operation carefully. Return only the requested conclusion."
      }
    };
    let body = ChatCompletionRequest {
      model: self.gemma4.config.model.clone(),
      messages: vec![
        ChatMessage {
          role: "system",
          content: MessageContent::Text(instruction.to_string()),
        },
        ChatMessage {
          role: "user",
          content: MessageContent::Text(input.to_string()),
        },
      ],
      temperature: 0.0,
      stream: true,
      response_format: ResponseFormat {
        kind: "json_object",
      },
      chat_template_kwargs: ChatTemplateKwargs {
        enable_thinking: matches!(profile, GenerationProfile::Reasoning),
      },
      reasoning: match profile {
        GenerationProfile::Fast => "off",
        GenerationProfile::Reasoning => "on",
      },
      reasoning_format: match profile {
        GenerationProfile::Fast => "auto",
        GenerationProfile::Reasoning => "auto",
      },
      reasoning_effort: match profile {
        GenerationProfile::Fast => "none",
        GenerationProfile::Reasoning => "minimal",
      },
      reasoning_budget_tokens: match profile {
        GenerationProfile::Fast => 0,
        GenerationProfile::Reasoning => 128,
      },
      max_tokens: match profile {
        GenerationProfile::Fast | GenerationProfile::Reasoning => 256,
      },
    };
    let output = self
      .gemma4
      .resilience
      .execute("generate", || self.gemma4.send(&body))
      .await
      .map_err(|_| GenerationProviderError::Provider)?;
    Ok((
      output.content,
      output.reasoning,
      self.gemma4.config.model.clone(),
    ))
  }

  /// Runs one provider-neutral multimodal operation against the Gemma 4 endpoint.
  pub(crate) async fn generate_with_profile_images(
    &self,
    profile: GenerationProfile,
    input: &str,
    images: &[(String, String)],
  ) -> Result<(String, Option<String>, String), GenerationProviderError> {
    let instruction = match profile {
      GenerationProfile::Fast => "Complete the requested operation accurately and concisely.",
      GenerationProfile::Reasoning => {
        "Resolve the requested operation carefully. Return only the requested conclusion."
      }
    };
    let mut content = Vec::with_capacity(images.len() + 1);
    content.push(VlmContent::Text {
      text: input.to_string(),
    });
    content.extend(
      images
        .iter()
        .map(|(media_type, data)| VlmContent::ImageUrl {
          image_url: ImageUrl {
            url: format!("data:{media_type};base64,{data}"),
          },
        }),
    );
    let body = ChatCompletionRequest {
      model: self.gemma4.config.model.clone(),
      messages: vec![
        ChatMessage {
          role: "system",
          content: MessageContent::Text(instruction.to_string()),
        },
        ChatMessage {
          role: "user",
          content: MessageContent::Vlm(content),
        },
      ],
      temperature: 0.0,
      stream: true,
      response_format: ResponseFormat {
        kind: "json_object",
      },
      chat_template_kwargs: ChatTemplateKwargs {
        enable_thinking: matches!(profile, GenerationProfile::Reasoning),
      },
      reasoning: match profile {
        GenerationProfile::Fast => "off",
        GenerationProfile::Reasoning => "on",
      },
      reasoning_format: match profile {
        GenerationProfile::Fast => "auto",
        GenerationProfile::Reasoning => "auto",
      },
      reasoning_effort: match profile {
        GenerationProfile::Fast => "none",
        GenerationProfile::Reasoning => "minimal",
      },
      reasoning_budget_tokens: match profile {
        GenerationProfile::Fast => 0,
        GenerationProfile::Reasoning => 128,
      },
      max_tokens: match profile {
        GenerationProfile::Fast | GenerationProfile::Reasoning => 256,
      },
    };
    let output = self
      .gemma4
      .resilience
      .execute("generate", || self.gemma4.send(&body))
      .await
      .map_err(|_| GenerationProviderError::Provider)?;
    Ok((
      output.content,
      output.reasoning,
      self.gemma4.config.model.clone(),
    ))
  }

  /// Creates a reusable generation service with one resilience policy.
  ///
  /// # Errors
  ///
  /// Returns an error when a provider HTTP client cannot be built.
  pub fn new(gemma4: ProviderConfig, gemma4_policy: ProviderPolicy) -> anyhow::Result<Self> {
    Ok(Self {
      gemma4: TranslationProvider::new("gemma4", gemma4, gemma4_policy)?,
    })
  }

  /// Returns redacted counters for direct translation provider boundaries.
  pub fn provider_metrics(&self) -> GenerationProviderMetrics {
    GenerationProviderMetrics {
      gemma4: self.gemma4.resilience.metrics().snapshot(),
    }
  }
}

impl TranslationProvider {
  fn new(
    provider_name: &'static str,
    config: ProviderConfig,
    policy: ProviderPolicy,
  ) -> anyhow::Result<Self> {
    let client = Client::builder()
      .no_proxy()
      .timeout(policy.timeout())
      .build()?;
    Ok(Self {
      client,
      config,
      resilience: ProviderResilience::new(provider_name, policy),
    })
  }

  async fn send(
    &self,
    body: &ChatCompletionRequest,
  ) -> Result<StreamedGeneration, ProviderAttemptError> {
    let allow_reasoning = body.reasoning == "on";
    let endpoint = format!(
      "{}/chat/completions",
      self.config.base_url.trim_end_matches('/')
    );
    let response = self
      .client
      .post(endpoint)
      .bearer_auth(self.config.api_key.bearer_token())
      .json(body)
      .send()
      .await
      .map_err(|error| transport_failure(&error))?;
    if !response.status().is_success() {
      return Err(status_failure(response.status(), response.headers()));
    }
    let mut bytes = response.bytes_stream();
    let mut pending = String::new();
    let mut output = StreamedGeneration::default();
    while let Some(chunk) = bytes.next().await {
      let chunk = chunk.map_err(|error| transport_failure(&error))?;
      let text = std::str::from_utf8(&chunk).map_err(|_| ProviderAttemptError::InvalidEnvelope)?;
      pending.push_str(text);
      while let Some(end) = pending.find('\n') {
        let line = pending[..end].trim_end_matches('\r').to_string();
        pending.drain(..=end);
        consume_stream_line(&line, &mut output, allow_reasoning)?;
      }
    }
    if !pending.is_empty() {
      consume_stream_line(pending.trim_end_matches('\r'), &mut output, allow_reasoning)?;
    }
    output.content = output.content.trim().to_string();
    output.reasoning = output
      .reasoning
      .map(|value| value.trim().to_string())
      .filter(|value| !value.is_empty());
    if output.content.is_empty() {
      return Err(ProviderAttemptError::InvalidEnvelope);
    }
    Ok(output)
  }
}

#[derive(Default)]
struct StreamedGeneration {
  content: String,
  reasoning: Option<String>,
}

fn consume_stream_line(
  line: &str,
  output: &mut StreamedGeneration,
  allow_reasoning: bool,
) -> Result<(), ProviderAttemptError> {
  let Some(data) = line.strip_prefix("data:").map(str::trim) else {
    return Ok(());
  };
  if data.is_empty() || data == "[DONE]" {
    return Ok(());
  }
  let payload: ChatCompletionChunk =
    serde_json::from_str(data).map_err(|_| ProviderAttemptError::InvalidEnvelope)?;
  for choice in payload.choices {
    if let Some(content) = choice.delta.content {
      output.content.push_str(&content);
    }
    if let Some(reasoning) = choice.delta.reasoning_content.filter(|_| allow_reasoning) {
      if !reasoning.is_empty() {
        let _ = STREAM_EVENTS
          .try_with(|sender| sender.send(ProviderStreamEvent::ReasoningDelta(reasoning.clone())));
      }
      output
        .reasoning
        .get_or_insert_with(String::new)
        .push_str(&reasoning);
    }
  }
  Ok(())
}

#[derive(Serialize)]
struct ChatCompletionRequest {
  model: String,
  messages: Vec<ChatMessage>,
  temperature: f32,
  stream: bool,
  response_format: ResponseFormat,
  chat_template_kwargs: ChatTemplateKwargs,
  reasoning: &'static str,
  reasoning_format: &'static str,
  reasoning_effort: &'static str,
  reasoning_budget_tokens: u32,
  max_tokens: u32,
}

#[derive(Serialize)]
struct ResponseFormat {
  #[serde(rename = "type")]
  kind: &'static str,
}

#[derive(Serialize)]
struct ChatTemplateKwargs {
  enable_thinking: bool,
}

#[derive(Serialize)]
struct ChatMessage {
  role: &'static str,
  content: MessageContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum MessageContent {
  Text(String),
  Vlm(Vec<VlmContent>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum VlmContent {
  Text { text: String },
  ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
  url: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
  choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
  delta: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
  content: Option<String>,
  #[serde(default)]
  reasoning_content: Option<String>,
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn vlm_content_uses_standard_text_and_image_url_parts() {
    let value = serde_json::to_value(MessageContent::Vlm(vec![
      VlmContent::Text {
        text: "bounded prompt".into(),
      },
      VlmContent::ImageUrl {
        image_url: ImageUrl {
          url: "data:image/png;base64,cHJpdmF0ZQ==".into(),
        },
      },
    ]))
    .unwrap();
    assert_eq!(value[0], json!({"type":"text","text":"bounded prompt"}));
    assert_eq!(value[1]["type"], "image_url");
    assert_eq!(
      value[1]["image_url"]["url"],
      "data:image/png;base64,cHJpdmF0ZQ=="
    );
  }

  #[test]
  fn streamed_answer_and_reasoning_stay_separate() {
    let mut output = StreamedGeneration::default();
    consume_stream_line(
      r#"data: {"choices":[{"delta":{"reasoning_content":"check "}}]}"#,
      &mut output,
      true,
    )
    .unwrap();
    consume_stream_line(
      r#"data: {"choices":[{"delta":{"content":"answer"}}]}"#,
      &mut output,
      true,
    )
    .unwrap();
    assert_eq!(output.reasoning.as_deref(), Some("check "));
    assert_eq!(output.content, "answer");
  }

  #[test]
  fn profiles_use_llama_cpp_reasoning_controls() {
    let request = ChatCompletionRequest {
      model: "model".into(),
      messages: Vec::new(),
      temperature: 0.0,
      stream: true,
      response_format: ResponseFormat {
        kind: "json_object",
      },
      chat_template_kwargs: ChatTemplateKwargs {
        enable_thinking: true,
      },
      reasoning: "on",
      reasoning_format: "auto",
      reasoning_effort: "minimal",
      reasoning_budget_tokens: 128,
      max_tokens: 256,
    };
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["stream"], true);
    assert_eq!(value["response_format"]["type"], "json_object");
    assert_eq!(value["chat_template_kwargs"]["enable_thinking"], true);
    assert_eq!(value["reasoning"], "on");
    assert_eq!(value["reasoning_format"], "auto");
    assert_eq!(value["reasoning_effort"], "minimal");
    assert_eq!(value["reasoning_budget_tokens"], 128);
    assert_eq!(value["max_tokens"], 256);
  }
}
