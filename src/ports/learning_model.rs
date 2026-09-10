//! Port for generating a structured English learning translation.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::translation::{TranslationInput, TranslationResult};

/// Failure returned by a structured learning model.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LearningModelError {
  /// No valid result was available after transport retries.
  #[error("learning model unavailable")]
  Unavailable,
  /// The provider responded, but neither the first response nor one repair matched the contract.
  #[error("learning model returned invalid structured output")]
  InvalidOutput,
}

/// Generates structured, explicitly non-canonical learning suggestions.
#[async_trait]
pub trait LearningModel: Send + Sync {
  /// Produces ranked English meanings for one validated input.
  async fn generate(
    &self,
    input: &TranslationInput,
  ) -> Result<TranslationResult, LearningModelError>;
}
