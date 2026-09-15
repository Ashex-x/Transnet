//! Model operations required by unified request-local translation orchestration.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::translation_turn::{
  LexicalTurnDraft, TranslationTurn, TranslationUnit, TurnLanguage,
};

/// Closed model-operation failure without provider or request content.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum TranslationModelError {
  /// The external model dependency could not complete the operation.
  #[error("translation model unavailable")]
  Unavailable,
  /// The dependency responded but did not produce a valid bounded result.
  #[error("translation model returned invalid output")]
  InvalidOutput,
}

/// Produces plain connected-text translations without exposing provider selection.
#[async_trait]
pub trait ConnectedTextModel: Send + Sync {
  /// Translates the current text using a source language already resolved by orchestration.
  async fn translate_connected_text(
    &self,
    turn: &TranslationTurn,
    source_language: TurnLanguage,
  ) -> Result<String, TranslationModelError>;
}

/// Produces bounded structured lexical candidates without asserting canonical facts.
#[async_trait]
pub trait LexicalDraftModel: Send + Sync {
  /// Generates a complete response-level-independent draft for a word or established phrase.
  async fn generate_lexical_draft(
    &self,
    turn: &TranslationTurn,
    unit: TranslationUnit,
    source_language: TurnLanguage,
  ) -> Result<LexicalTurnDraft, TranslationModelError>;
}
