//! Provider-neutral generation and ephemeral embedding operations.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  model_runtime::{
    CancellationSignal, EmbeddingInput, EphemeralEmbedding, GenerationImage, GenerationInput,
    GenerationOutput, GenerationProfile, ModelVersion,
  },
  request_context::RequestContext,
};

/// Deadline and cancellation hooks shared by one outbound model operation.
#[derive(Clone, Copy)]
pub struct ModelOperationContext<'a> {
  /// Immutable request identity, schema, release pin, and absolute deadline.
  pub request: &'a RequestContext,
  /// Cooperative request-local cancellation signal.
  pub cancellation: &'a CancellationSignal,
}

impl ModelOperationContext<'_> {
  /// Fails before an adapter call when the request is cancelled or its deadline is exhausted.
  pub fn ensure_active(&self) -> Result<(), ModelOperationError> {
    if self.cancellation.is_cancelled() {
      return Err(ModelOperationError::Cancelled);
    }
    if self.request.remaining_budget().is_zero() {
      return Err(ModelOperationError::DeadlineExceeded);
    }
    Ok(())
  }
}

/// One bounded provider-neutral generation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationRequest {
  /// Closed invocation policy selected by application orchestration.
  pub profile: GenerationProfile,
  /// Version of the application-owned operation prompt contract.
  pub prompt_version: ModelVersion,
  /// Validated request-local operation input.
  pub input: GenerationInput,
  /// Bounded decoded images available only during this operation.
  pub images: Vec<GenerationImage>,
}

/// One validated generation response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationResponse {
  /// Bounded generated material for immediate validation.
  pub output: GenerationOutput,
  /// Exact configured model version that served the operation.
  pub model_version: ModelVersion,
  /// Version of the operation-specific prompt contract.
  pub prompt_version: ModelVersion,
}

/// One bounded request-local embedding request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingRequest {
  /// Validated ephemeral input that must be dropped with the request.
  pub input: EmbeddingInput,
}

/// Closed model-operation failure without content, credentials, or provider response details.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ModelOperationError {
  /// The shared request deadline was exhausted.
  #[error("model operation deadline exceeded")]
  DeadlineExceeded,
  /// The request was cooperatively cancelled.
  #[error("model operation cancelled")]
  Cancelled,
  /// The dependency could not complete the operation.
  #[error("model dependency unavailable")]
  Unavailable,
  /// The dependency returned an invalid bounded result.
  #[error("model dependency returned invalid output")]
  InvalidOutput,
}

/// Single generation operation implemented by the configured VLM adapter.
#[async_trait]
pub trait GenerationPort: Send + Sync {
  /// Runs one bounded fast or reasoning operation under the caller context.
  async fn generate(
    &self,
    context: ModelOperationContext<'_>,
    request: GenerationRequest,
  ) -> Result<GenerationResponse, ModelOperationError>;
}

/// Request-local candidate-nomination embedding operation.
#[async_trait]
pub trait EmbeddingPort: Send + Sync {
  /// Produces one fixed-dimension ephemeral vector under the caller context.
  async fn embed(
    &self,
    context: ModelOperationContext<'_>,
    request: EmbeddingRequest,
  ) -> Result<EphemeralEmbedding, ModelOperationError>;
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use time::OffsetDateTime;

  use super::*;
  use crate::domain::request_context::RequestId;

  fn context(deadline: OffsetDateTime) -> RequestContext {
    RequestContext::new(
      RequestId::new("model-operation-test").unwrap(),
      deadline,
      "transnet-v1",
      None,
    )
    .unwrap()
  }

  #[test]
  fn operation_context_rejects_expiry_and_cancellation() {
    let cancellation = CancellationSignal::default();
    let now =
      OffsetDateTime::from_unix_timestamp(OffsetDateTime::now_utc().unix_timestamp()).unwrap();
    let expired = context(now - Duration::from_secs(1));
    assert_eq!(
      ModelOperationContext {
        request: &expired,
        cancellation: &cancellation,
      }
      .ensure_active(),
      Err(ModelOperationError::DeadlineExceeded)
    );

    let active = context(now + Duration::from_secs(30));
    cancellation.cancel();
    assert_eq!(
      ModelOperationContext {
        request: &active,
        cancellation: &cancellation,
      }
      .ensure_active(),
      Err(ModelOperationError::Cancelled)
    );
  }
}
