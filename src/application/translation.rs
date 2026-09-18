//! Unified request-local translation orchestration independent of transport and providers.

use std::sync::Arc;
use std::{
  collections::{BTreeMap, BTreeSet},
  ops::Range,
};

use thiserror::Error;

use crate::{
  domain::translation_turn::{
    ProjectedTranslationResult, RoutingConfidence, TranslationIntentClassifier,
    TranslationNormalizer, TranslationTurn, TranslationTurnResult, TranslationUnit,
    TranslationVersionMetadata, NORMALIZER_VERSION, PROJECTION_VERSION,
    TRANSLATION_RESULT_SCHEMA_VERSION,
  },
  ports::translation_model::{
    ConnectedTextModel, ConnectedTextRequest, LexicalDraftModel, ModelOperationVersions,
    TranslationModelError,
  },
};

/// Maximum Unicode scalar count sent in one application-planned segment.
pub const MAX_CONNECTED_CHUNK_CHARS: usize = 8_192;
/// Maximum number of segments produced for one accepted request.
pub const MAX_CONNECTED_CHUNKS: usize = 128;
/// Maximum number of repeated source terms retained in one request-local ledger.
pub const MAX_TERMINOLOGY_ENTRIES: usize = 64;
/// Maximum Unicode scalar count retained for one terminology hint.
pub const MAX_TERMINOLOGY_CHARS: usize = 64;
/// Maximum accepted translated scalar count for one segment.
pub const MAX_TRANSLATED_CHUNK_CHARS: usize = 32_768;

/// Closed orchestration failure without provider identity or private request content.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum TranslationOrchestrationError {
  /// Automatic source-language detection did not resolve to an initially supported language.
  #[error("unsupported translation source language")]
  UnsupportedSourceLanguage,
  /// A model dependency could not complete the selected operation.
  #[error("translation model unavailable")]
  ModelUnavailable,
  /// A model dependency returned an invalid bounded result.
  #[error("translation model returned invalid output")]
  InvalidModelOutput,
  /// The text cannot be split safely within the bounded natural-boundary plan.
  #[error("connected text exceeds bounded chunk planning limits")]
  ChunkPlanLimit,
}

impl From<TranslationModelError> for TranslationOrchestrationError {
  fn from(error: TranslationModelError) -> Self {
    match error {
      TranslationModelError::Unavailable => Self::ModelUnavailable,
      TranslationModelError::InvalidOutput => Self::InvalidModelOutput,
    }
  }
}

/// Chooses one translation workflow and assembles one response-level-independent result.
#[derive(Clone)]
pub struct TranslationOrchestrator {
  connected_text: Arc<dyn ConnectedTextModel>,
  lexical_draft: Arc<dyn LexicalDraftModel>,
  normalizer: TranslationNormalizer,
  classifier: TranslationIntentClassifier,
}

impl TranslationOrchestrator {
  /// Creates an orchestrator from operation-focused model ports.
  pub fn new(
    connected_text: Arc<dyn ConnectedTextModel>,
    lexical_draft: Arc<dyn LexicalDraftModel>,
  ) -> Self {
    Self {
      connected_text,
      lexical_draft,
      normalizer: TranslationNormalizer::new(),
      classifier: TranslationIntentClassifier::new(),
    }
  }

  /// Normalizes and classifies one validated turn, calls exactly one model port, and returns its
  /// complete unprojected result.
  ///
  /// # Errors
  /// Returns a closed language or model failure without embedding request content.
  pub async fn translate(
    &self,
    turn: &TranslationTurn,
  ) -> Result<ProjectedTranslationResult, TranslationOrchestrationError> {
    let normalized = self
      .normalizer
      .normalize(turn.text(), turn.source_language());
    let classification = self.classifier.classify(turn, &normalized);
    let source_language = classification
      .detected_source_language
      .ok_or(TranslationOrchestrationError::UnsupportedSourceLanguage)?;

    if classification.confidence == RoutingConfidence::High
      && matches!(
        classification.unit,
        TranslationUnit::Word | TranslationUnit::Phrase
      )
    {
      let output = self
        .lexical_draft
        .generate_lexical_draft(turn, classification.unit, source_language)
        .await?;
      if !output.draft.is_valid(classification.unit) {
        return Err(TranslationOrchestrationError::InvalidModelOutput);
      }
      let superset = TranslationTurnResult::lexical(
        output.draft,
        classification.unit,
        source_language,
        turn.target_language(),
      );
      return Ok(project_outcome(
        superset,
        turn.response_level(),
        std::iter::once(output.versions),
      ));
    }

    let (translated, versions) = self.translate_connected(turn, source_language).await?;
    let superset =
      TranslationTurnResult::passage(translated, source_language, turn.target_language());
    Ok(project_outcome(superset, turn.response_level(), versions))
  }

  async fn translate_connected(
    &self,
    turn: &TranslationTurn,
    source_language: crate::domain::translation_turn::TurnLanguage,
  ) -> Result<(String, Vec<ModelOperationVersions>), TranslationOrchestrationError> {
    if turn.text().chars().count() <= MAX_CONNECTED_CHUNK_CHARS {
      let output = self
        .translate_segment(turn, turn.text(), source_language, &[], None)
        .await?;
      return Ok((output.translation, vec![output.versions]));
    }

    let chunks = plan_chunks(turn.text())?;
    let terminology = build_terminology_ledger(turn.text());
    let mut assembled = String::new();
    let mut versions = Vec::new();
    let mut preceding_translation: Option<String> = None;
    for chunk in chunks {
      let output = self
        .translate_segment(
          turn,
          &turn.text()[chunk.text],
          source_language,
          &terminology,
          preceding_translation.as_deref(),
        )
        .await?;
      assembled.push_str(&output.translation);
      assembled.push_str(&turn.text()[chunk.separator]);
      preceding_translation = Some(output.translation);
      versions.push(output.versions);
    }
    Ok((assembled, versions))
  }

  async fn translate_segment(
    &self,
    turn: &TranslationTurn,
    text: &str,
    source_language: crate::domain::translation_turn::TurnLanguage,
    terminology: &[String],
    preceding_translation: Option<&str>,
  ) -> Result<crate::ports::translation_model::ConnectedTextOutput, TranslationOrchestrationError>
  {
    let output = self
      .connected_text
      .translate_connected_text(
        ConnectedTextRequest {
          turn,
          text,
          terminology,
          preceding_translation,
        },
        source_language,
      )
      .await?;
    if output.translation.trim().is_empty()
      || output.translation.chars().count() > MAX_TRANSLATED_CHUNK_CHARS
    {
      return Err(TranslationOrchestrationError::InvalidModelOutput);
    }
    Ok(output)
  }
}

fn project_outcome(
  superset: TranslationTurnResult,
  response_level: crate::domain::translation_turn::ResponseLevel,
  versions: impl IntoIterator<Item = ModelOperationVersions>,
) -> ProjectedTranslationResult {
  let mut model_versions = BTreeSet::new();
  let mut prompt_versions = BTreeSet::new();
  for version in versions {
    model_versions.insert(version.model_version);
    prompt_versions.insert(version.prompt_version);
  }
  ProjectedTranslationResult {
    translation: superset.project(response_level),
    metadata: TranslationVersionMetadata {
      schema_version: TRANSLATION_RESULT_SCHEMA_VERSION,
      normalizer_version: NORMALIZER_VERSION,
      projection_version: PROJECTION_VERSION,
      response_level,
      model_versions: model_versions.into_iter().collect(),
      prompt_versions: prompt_versions.into_iter().collect(),
      retrieval_version: None,
      content_release: None,
    },
  }
}

struct TextChunk {
  text: Range<usize>,
  separator: Range<usize>,
}

fn plan_chunks(text: &str) -> Result<Vec<TextChunk>, TranslationOrchestrationError> {
  let mut chunks = Vec::new();
  let mut start = 0;
  while start < text.len() {
    if chunks.len() == MAX_CONNECTED_CHUNKS {
      return Err(TranslationOrchestrationError::ChunkPlanLimit);
    }
    let remaining = &text[start..];
    if remaining.chars().count() <= MAX_CONNECTED_CHUNK_CHARS {
      chunks.push(TextChunk {
        text: start..text.len(),
        separator: text.len()..text.len(),
      });
      break;
    }

    let limit = byte_after_chars(remaining, MAX_CONNECTED_CHUNK_CHARS);
    let split =
      natural_split(&remaining[..limit]).ok_or(TranslationOrchestrationError::ChunkPlanLimit)?;
    let text_end = start + split;
    let separator_end = text_end
      + text[text_end..]
        .char_indices()
        .take_while(|(_, character)| character.is_whitespace())
        .map(|(_, character)| character.len_utf8())
        .sum::<usize>();
    chunks.push(TextChunk {
      text: start..text_end,
      separator: text_end..separator_end,
    });
    start = separator_end;
  }
  Ok(chunks)
}

fn byte_after_chars(value: &str, chars: usize) -> usize {
  value
    .char_indices()
    .nth(chars)
    .map_or(value.len(), |(index, _)| index)
}

fn natural_split(value: &str) -> Option<usize> {
  let minimum = byte_after_chars(value, MAX_CONNECTED_CHUNK_CHARS / 2);
  let candidates = [
    value.rfind("\n\n"),
    value
      .char_indices()
      .filter(|(_, character)| matches!(character, '.' | '!' | '?' | '。' | '！' | '？'))
      .map(|(index, character)| index + character.len_utf8())
      .next_back(),
    value
      .char_indices()
      .filter(|(_, character)| matches!(character, ',' | ';' | ':' | '，' | '；' | '：'))
      .map(|(index, character)| index + character.len_utf8())
      .next_back(),
    value
      .char_indices()
      .filter(|(_, character)| character.is_whitespace())
      .map(|(index, _)| index)
      .next_back(),
  ];
  candidates
    .into_iter()
    .flatten()
    .find(|index| *index >= minimum)
}

fn build_terminology_ledger(text: &str) -> Vec<String> {
  let mut occurrences: BTreeMap<String, (usize, String)> = BTreeMap::new();
  for candidate in text.split(|character: char| {
    !(character.is_alphanumeric() || matches!(character, '+' | '#' | '.' | '_' | '-'))
  }) {
    let chars = candidate.chars().count();
    if chars == 0 || chars > MAX_TERMINOLOGY_CHARS || !is_term_candidate(candidate) {
      continue;
    }
    let key = candidate.to_lowercase();
    let entry = occurrences
      .entry(key)
      .or_insert_with(|| (0, candidate.to_string()));
    entry.0 += 1;
  }
  occurrences
    .into_values()
    .filter(|(count, _)| *count > 1)
    .map(|(_, candidate)| candidate)
    .take(MAX_TERMINOLOGY_ENTRIES)
    .collect()
}

fn is_term_candidate(candidate: &str) -> bool {
  candidate.chars().count() >= 4
    || candidate
      .chars()
      .any(|character| matches!(character, '+' | '#' | '.' | '_' | '-'))
    || (candidate.chars().count() > 1
      && candidate
        .chars()
        .filter(|character| character.is_alphabetic())
        .all(char::is_uppercase))
}
