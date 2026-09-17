//! Model operations required by unified request-local translation orchestration.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::translation_turn::{
  LexicalTurnDraft, TranslationTurn, TranslationUnit, TurnLanguage,
};

/// Non-sensitive version identifiers for one completed model operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOperationVersions {
  /// Configured model identifier that actually served the operation.
  pub model_version: String,
  /// Version of the bounded prompt contract used for the operation.
  pub prompt_version: &'static str,
}

/// Result of one connected-text model operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedTextOutput {
  /// Translated text returned by the selected model adapter.
  pub translation: String,
  /// Actual model and prompt versions used for this operation.
  pub versions: ModelOperationVersions,
}

/// Result of one structured lexical-draft model operation.
#[derive(Clone, Debug)]
pub struct LexicalDraftOutput {
  /// Bounded, non-canonical lexical draft.
  pub draft: LexicalTurnDraft,
  /// Actual model and prompt versions used for this operation.
  pub versions: ModelOperationVersions,
}

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
  ) -> Result<ConnectedTextOutput, TranslationModelError>;
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
  ) -> Result<LexicalDraftOutput, TranslationModelError>;
}
