//! Provider-neutral model profiles, bounded values, and request-local call policy.

use std::{
  fmt,
  sync::atomic::{AtomicBool, Ordering},
};

use serde::Serialize;
use thiserror::Error;

/// Maximum UTF-8 bytes accepted by one generation operation.
pub const MAX_GENERATION_INPUT_BYTES: usize = 65_536;
/// Maximum UTF-8 bytes accepted from one generation operation.
pub const MAX_GENERATION_OUTPUT_BYTES: usize = 262_144;
/// Maximum UTF-8 bytes accepted by one ephemeral embedding operation.
pub const MAX_EMBEDDING_INPUT_BYTES: usize = 16_384;
/// Maximum bytes accepted for a model or artifact version identifier.
pub const MAX_MODEL_VERSION_BYTES: usize = 128;

/// Closed invocation profile for the configured generation model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProfile {
  /// Default bounded inference for ordinary operations.
  Fast,
  /// At-most-once internal escalation for a qualifying ambiguity or invalid result.
  Reasoning,
}

/// A validated, bounded generation input that redacts its contents from debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct GenerationInput(String);

impl GenerationInput {
  /// Validates a nonblank generation input.
  pub fn new(value: impl Into<String>) -> Result<Self, ModelValueError> {
    let value = value.into();
    validate_text(
      &value,
      MAX_GENERATION_INPUT_BYTES,
      ModelValueError::InvalidGenerationInput,
    )?;
    Ok(Self(value))
  }

  /// Borrows the request-local input for an adapter invocation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for GenerationInput {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("GenerationInput(REDACTED)")
  }
}

/// A validated, bounded generation result that redacts its contents from debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct GenerationOutput(String);

impl GenerationOutput {
  /// Validates a nonblank generation result.
  pub fn new(value: impl Into<String>) -> Result<Self, ModelValueError> {
    let value = value.into();
    validate_text(
      &value,
      MAX_GENERATION_OUTPUT_BYTES,
      ModelValueError::InvalidGenerationOutput,
    )?;
    Ok(Self(value))
  }

  /// Borrows the result for immediate request-local validation or composition.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for GenerationOutput {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("GenerationOutput(REDACTED)")
  }
}

/// A bounded, non-secret model or embedding artifact version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelVersion(String);

impl ModelVersion {
  /// Validates a nonblank version without control characters.
  pub fn new(value: impl Into<String>) -> Result<Self, ModelValueError> {
    let value = value.into();
    if value.is_empty()
      || value.len() > MAX_MODEL_VERSION_BYTES
      || value.chars().any(char::is_control)
    {
      return Err(ModelValueError::InvalidModelVersion);
    }
    Ok(Self(value))
  }

  /// Returns the validated version identifier.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Request-local input eligible for ephemeral candidate nomination.
#[derive(Clone, PartialEq, Eq)]
pub struct EmbeddingInput(String);

impl EmbeddingInput {
  /// Validates a nonblank, bounded embedding input.
  pub fn new(value: impl Into<String>) -> Result<Self, ModelValueError> {
    let value = value.into();
    validate_text(
      &value,
      MAX_EMBEDDING_INPUT_BYTES,
      ModelValueError::InvalidEmbeddingInput,
    )?;
    Ok(Self(value))
  }

  /// Borrows the request-local material for an adapter invocation.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for EmbeddingInput {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("EmbeddingInput(REDACTED)")
  }
}

/// Validated ephemeral vector and its exact artifact version.
#[derive(Clone, PartialEq)]
pub struct EphemeralEmbedding {
  values: Vec<f32>,
  model_version: ModelVersion,
}

impl EphemeralEmbedding {
  /// Validates finite values against the configured fixed dimension.
  pub fn new(
    values: Vec<f32>,
    expected_dimension: usize,
    model_version: ModelVersion,
  ) -> Result<Self, ModelValueError> {
    if expected_dimension == 0 || values.len() != expected_dimension {
      return Err(ModelValueError::InvalidEmbeddingDimension);
    }
    if values.iter().any(|value| !value.is_finite()) {
      return Err(ModelValueError::InvalidEmbeddingValue);
    }
    Ok(Self {
      values,
      model_version,
    })
  }

  /// Borrows vector values for request-local ranking.
  pub fn values(&self) -> &[f32] {
    &self.values
  }
  /// Returns the exact embedding artifact version.
  pub fn model_version(&self) -> &ModelVersion {
    &self.model_version
  }
}

impl fmt::Debug for EphemeralEmbedding {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("EphemeralEmbedding")
      .field("dimension", &self.values.len())
      .field("model_version", &self.model_version)
      .field("values", &"[redacted]")
      .finish()
  }
}

/// Cooperative cancellation signal shared by request-local operations.
#[derive(Debug, Default)]
pub struct CancellationSignal {
  cancelled: AtomicBool,
  notify: tokio::sync::Notify,
}

impl CancellationSignal {
  /// Requests cancellation without blocking the caller.
  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::Release);
    self.notify.notify_waiters();
  }
  /// Reports whether cancellation was requested.
  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::Acquire)
  }
  /// Waits asynchronously until cancellation is requested.
  pub async fn cancelled(&self) {
    while !self.is_cancelled() {
      let notified = self.notify.notified();
      if self.is_cancelled() {
        break;
      }
      notified.await;
    }
  }
}

/// One-per-request guard for the closed reasoning escalation budget.
#[derive(Debug, Default)]
pub struct ReasoningBudget(AtomicBool);

impl ReasoningBudget {
  /// Claims the sole reasoning escalation, returning false when already consumed.
  pub fn try_claim(&self) -> bool {
    self
      .0
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_ok()
  }
  /// Reports whether the request already spent its reasoning escalation.
  pub fn is_spent(&self) -> bool {
    self.0.load(Ordering::Acquire)
  }
}

/// Invalid bounded model value.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ModelValueError {
  /// Generation input is blank or exceeds its byte bound.
  #[error("invalid generation input")]
  InvalidGenerationInput,
  /// Generation output is blank or exceeds its byte bound.
  #[error("invalid generation output")]
  InvalidGenerationOutput,
  /// Embedding input is blank or exceeds its byte bound.
  #[error("invalid embedding input")]
  InvalidEmbeddingInput,
  /// Model version is blank, oversized, or contains control characters.
  #[error("invalid model version")]
  InvalidModelVersion,
  /// Embedding dimension does not match the configured fixed dimension.
  #[error("invalid embedding dimension")]
  InvalidEmbeddingDimension,
  /// Embedding contains a non-finite value.
  #[error("invalid embedding value")]
  InvalidEmbeddingValue,
}

fn validate_text(
  value: &str,
  maximum: usize,
  error: ModelValueError,
) -> Result<(), ModelValueError> {
  if value.trim().is_empty() || value.len() > maximum {
    Err(error)
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn private_values_are_redacted() {
    let input = GenerationInput::new("private source").unwrap();
    let embedding = EmbeddingInput::new("private query").unwrap();
    assert_eq!(format!("{input:?}"), "GenerationInput(REDACTED)");
    assert_eq!(format!("{embedding:?}"), "EmbeddingInput(REDACTED)");
  }

  #[test]
  fn bounded_values_reject_blank_oversized_and_unsafe_versions() {
    assert_eq!(
      GenerationInput::new("   "),
      Err(ModelValueError::InvalidGenerationInput)
    );
    assert_eq!(
      GenerationOutput::new("x".repeat(MAX_GENERATION_OUTPUT_BYTES + 1)),
      Err(ModelValueError::InvalidGenerationOutput)
    );
    assert_eq!(
      EmbeddingInput::new("x".repeat(MAX_EMBEDDING_INPUT_BYTES + 1)),
      Err(ModelValueError::InvalidEmbeddingInput)
    );
    assert_eq!(
      ModelVersion::new("unsafe\nversion"),
      Err(ModelValueError::InvalidModelVersion)
    );
  }

  #[tokio::test]
  async fn cancellation_wakes_waiters() {
    let signal = std::sync::Arc::new(CancellationSignal::default());
    let waiter = tokio::spawn({
      let signal = signal.clone();
      async move { signal.cancelled().await }
    });
    signal.cancel();
    waiter.await.unwrap();
    assert!(signal.is_cancelled());
  }

  #[test]
  fn reasoning_budget_is_claimed_once() {
    let budget = ReasoningBudget::default();
    assert!(budget.try_claim());
    assert!(!budget.try_claim());
    assert!(budget.is_spent());
  }

  #[test]
  fn embeddings_require_fixed_finite_dimensions() {
    let version = ModelVersion::new("embed-v1").unwrap();
    assert_eq!(
      EphemeralEmbedding::new(vec![0.0], 2, version.clone()),
      Err(ModelValueError::InvalidEmbeddingDimension)
    );
    assert_eq!(
      EphemeralEmbedding::new(vec![f32::NAN], 1, version),
      Err(ModelValueError::InvalidEmbeddingValue)
    );
  }
}
