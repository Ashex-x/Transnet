//! Model operations required by unified request-local translation orchestration.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::translation_turn::{
  LexicalTurnDraft, TranslationTurn, TranslationUnit, TurnLanguage,
};

/// One connected-text operation with request-local consistency context.
pub struct ConnectedTextRequest<'a> {
  /// Validated parent turn supplying languages and minimal history.
  pub turn: &'a TranslationTurn,
  /// Current complete segment; it is never retained by the model port.
  pub text: &'a str,
  /// Bounded repeated source terms that should be rendered consistently.
  pub terminology: &'a [String],
  /// Immediately preceding translated segment, when chunking is active.
  pub preceding_translation: Option<&'a str>,
}

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
    request: ConnectedTextRequest<'_>,
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
