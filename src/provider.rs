//! Resilient OpenAI-compatible Gemma4-27B generation provider.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
  config::ProviderConfig,
  resilience::{
    response_failure, status_failure, transport_failure, ProviderAttemptError,
    ProviderMetricsSnapshot, ProviderPolicy, ProviderResilience,
  },
};

use crate::domain::model_runtime::GenerationProfile;

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
  ) -> Result<(String, String), GenerationProviderError> {
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
    };
    let output = self
      .gemma4
      .resilience
      .execute("generate", || self.gemma4.send(&body))
      .await
      .map_err(|_| GenerationProviderError::Provider)?;
    Ok((output, self.gemma4.config.model.clone()))
  }

  /// Runs one provider-neutral multimodal operation against the Gemma 4 endpoint.
  pub(crate) async fn generate_with_profile_images(
    &self,
    profile: GenerationProfile,
    input: &str,
    images: &[(String, String)],
  ) -> Result<(String, String), GenerationProviderError> {
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
    };
    let output = self
      .gemma4
      .resilience
      .execute("generate", || self.gemma4.send(&body))
      .await
      .map_err(|_| GenerationProviderError::Provider)?;
    Ok((output, self.gemma4.config.model.clone()))
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

  async fn send(&self, body: &ChatCompletionRequest) -> Result<String, ProviderAttemptError> {
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
    let payload: ChatCompletionResponse = response
      .json()
      .await
      .map_err(|error| response_failure(&error))?;
    payload
      .choices
      .into_iter()
      .next()
      .and_then(|choice| choice.message.content)
      .map(|content| content.trim().to_string())
      .filter(|content| !content.is_empty())
      .ok_or(ProviderAttemptError::InvalidEnvelope)
  }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
  model: String,
  messages: Vec<ChatMessage>,
  temperature: f32,
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
}
