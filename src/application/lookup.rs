//! Structured multilingual-to-English lookup use case.

use std::sync::Arc;

use crate::{
  domain::translation::{TranslationInput, TranslationResult},
  ports::learning_model::{LearningModel, LearningModelError},
};

/// Coordinates the core model-backed translation use case.
#[derive(Clone)]
pub struct LookupService {
  model: Arc<dyn LearningModel>,
}

impl LookupService {
  /// Creates a lookup service backed by the supplied model port.
  pub fn new(model: Arc<dyn LearningModel>) -> Self {
    Self { model }
  }

  /// Produces a structured English learning translation.
  ///
  /// # Errors
  ///
  /// Returns the model-port failure without hiding whether output was unavailable or invalid.
  pub async fn lookup(
    &self,
    input: &TranslationInput,
  ) -> Result<TranslationResult, LearningModelError> {
    self.model.generate(input).await
  }
}

#[cfg(test)]
mod tests {
  use async_trait::async_trait;

  use super::*;
  use crate::domain::translation::{Confidence, EnglishDialect};

  struct StubModel;

  #[async_trait]
  impl LearningModel for StubModel {
    async fn generate(
      &self,
      input: &TranslationInput,
    ) -> Result<TranslationResult, LearningModelError> {
      Ok(TranslationResult {
        source_language: input.source_language.clone(),
        language_confidence: Confidence::High,
        entries: Vec::new(),
        warnings: Vec::new(),
      })
    }
  }

  #[tokio::test]
  async fn delegates_validated_input_to_model() {
    let service = LookupService::new(Arc::new(StubModel));
    let input =
      TranslationInput::new("hola", "es", None, "en", EnglishDialect::American, None).unwrap();

    let result = service.lookup(&input).await.unwrap();

    assert_eq!(result.source_language, "es");
  }
}
