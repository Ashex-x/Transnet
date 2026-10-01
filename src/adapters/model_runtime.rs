//! OpenAI-compatible adapters for neutral generation and ephemeral embedding ports.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
  config::ProviderConfig,
  domain::model_runtime::{EphemeralEmbedding, GenerationOutput, ModelVersion},
  ports::model_runtime::{
    EmbeddingPort, EmbeddingRequest, GenerationPort, GenerationRequest, GenerationResponse,
    ModelOperationContext, ModelOperationError,
  },
  provider::TranslationService,
};

/// Prompt contract used by the compatibility generation adapter.
pub const NEUTRAL_GENERATION_PROMPT_VERSION: &str = "neutral-generation-v1";

/// Compatibility adapter exposing the existing Gemma endpoint through [`GenerationPort`].
#[derive(Clone)]
pub struct OpenAiGenerationAdapter {
  service: TranslationService,
}

impl OpenAiGenerationAdapter {
  /// Wraps the existing resilient provider service without changing legacy routing.
  pub fn new(service: TranslationService) -> Self {
    Self { service }
  }
}

#[async_trait]
impl GenerationPort for OpenAiGenerationAdapter {
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError> {
    context.ensure_active()?;
    let remaining = context.request.remaining_budget();
    let operation = self
      .service
      .generate_with_profile(request.profile, request.input.as_str());
    let (output, model) = tokio::select! {
      _ = context.cancellation.cancelled() => return Err(ModelOperationError::Cancelled),
      result = tokio::time::timeout(remaining, operation) => result
        .map_err(|_| ModelOperationError::DeadlineExceeded)?
        .map_err(|_| ModelOperationError::Unavailable)?,
    };
    context.ensure_active()?;
    Ok(GenerationResponse {
      output: GenerationOutput::new(output).map_err(|_| ModelOperationError::InvalidOutput)?,
      model_version: ModelVersion::new(model).map_err(|_| ModelOperationError::InvalidOutput)?,
      prompt_version: ModelVersion::new(NEUTRAL_GENERATION_PROMPT_VERSION)
        .map_err(|_| ModelOperationError::InvalidOutput)?,
    })
  }
}

/// OpenAI-compatible adapter for request-local candidate embeddings.
#[derive(Clone)]
pub struct OpenAiEmbeddingAdapter {
  client: Client,
  config: ProviderConfig,
  expected_dimension: usize,
  artifact_version: ModelVersion,
}

impl OpenAiEmbeddingAdapter {
  /// Creates an adapter with an immutable artifact version and fixed output dimension.
  pub fn new(
    config: ProviderConfig,
    expected_dimension: usize,
    artifact_version: ModelVersion,
  ) -> Result<Self, ModelOperationError> {
    if expected_dimension == 0 {
      return Err(ModelOperationError::InvalidOutput);
    }
    let client = Client::builder()
      .no_proxy()
      .build()
      .map_err(|_| ModelOperationError::Unavailable)?;
    Ok(Self {
      client,
      config,
      expected_dimension,
      artifact_version,
    })
  }
}

#[async_trait]
impl EmbeddingPort for OpenAiEmbeddingAdapter {
  async fn embed(
    &self,
    context: ModelOperationContext<'_>,
    request: EmbeddingRequest,
  ) -> Result<EphemeralEmbedding, ModelOperationError> {
    context.ensure_active()?;
    let endpoint = format!("{}/embeddings", self.config.base_url.trim_end_matches('/'));
    let operation = self
      .client
      .post(endpoint)
      .bearer_auth(self.config.api_key.bearer_token())
      .json(&EmbeddingBody {
        model: &self.config.model,
        input: request.input.as_str(),
      })
      .send();
    let response = tokio::select! {
      _ = context.cancellation.cancelled() => return Err(ModelOperationError::Cancelled),
      result = tokio::time::timeout(context.request.remaining_budget(), operation) => result
        .map_err(|_| ModelOperationError::DeadlineExceeded)?
        .map_err(|_| ModelOperationError::Unavailable)?,
    };
    if !response.status().is_success() {
      return Err(ModelOperationError::Unavailable);
    }
    let payload: EmbeddingEnvelope = response
      .json()
      .await
      .map_err(|_| ModelOperationError::InvalidOutput)?;
    context.ensure_active()?;
    map_embedding(
      payload,
      self.expected_dimension,
      self.artifact_version.clone(),
    )
  }
}

#[derive(Serialize)]
struct EmbeddingBody<'a> {
  model: &'a str,
  input: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmbeddingEnvelope {
  data: Vec<EmbeddingDatum>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmbeddingDatum {
  embedding: Vec<f32>,
}

fn map_embedding(
  payload: EmbeddingEnvelope,
  expected_dimension: usize,
  version: ModelVersion,
) -> Result<EphemeralEmbedding, ModelOperationError> {
  if payload.data.len() != 1 {
    return Err(ModelOperationError::InvalidOutput);
  }
  EphemeralEmbedding::new(
    payload
      .data
      .into_iter()
      .next()
      .ok_or(ModelOperationError::InvalidOutput)?
      .embedding,
    expected_dimension,
    version,
  )
  .map_err(|_| ModelOperationError::InvalidOutput)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn embedding_envelope_is_strict_and_dimension_checked() {
    let payload: EmbeddingEnvelope =
      serde_json::from_str(r#"{"data":[{"embedding":[0.25,0.5]}]}"#).unwrap();
    let vector = map_embedding(payload, 2, ModelVersion::new("artifact-r1").unwrap()).unwrap();
    assert_eq!(vector.values(), &[0.25, 0.5]);
    assert_eq!(vector.model_version().as_str(), "artifact-r1");
    assert!(serde_json::from_str::<EmbeddingEnvelope>(r#"{"data":[],"usage":{}}"#).is_err());
  }

  #[test]
  fn errors_and_debug_output_are_content_free() {
    let error = map_embedding(
      EmbeddingEnvelope { data: vec![] },
      2,
      ModelVersion::new("v1").unwrap(),
    )
    .unwrap_err();
    assert_eq!(
      error.to_string(),
      "model dependency returned invalid output"
    );
    assert!(!format!("{error:?}").contains("private"));
  }
}
