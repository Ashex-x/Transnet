//! Request-local unified translation inputs, model drafts, and deterministic response projection.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Hard bound for serialized input accepted through either HTTP or the public service API.
pub const MAX_TURN_BYTES: usize = 1_048_576;
/// Version of the request-local lexical normalization rules.
pub const NORMALIZER_VERSION: &str = "lookup-nfc-v1";
/// Version of deterministic response breadth rules.
pub const PROJECTION_VERSION: &str = "translation-projection-v1";
/// Long input is conservatively translated as connected text, not classified as one lexical unit.
pub const MAX_LEXICAL_CHARS: usize = 128;

/// One prior linguistic turn, without identity, timestamps, or persistence instructions.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationHistory {
  /// Previous source text, retained only during the current request.
  pub source_text: String,
  /// Previous translated text used only for linguistic continuity.
  pub translated_text: String,
  /// Known source language of this prior turn: en or zh-CN.
  pub source_language: String,
  /// Known target language of this prior turn: en or zh-CN.
  pub target_language: String,
}

/// Strict wire input for POST /api/v1/translations.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationTurnRequest {
  /// Text to translate; formatting is preserved in the provider input.
  pub text: String,
  /// auto, en, or zh-CN.
  pub source_language: String,
  /// en or zh-CN.
  pub target_language: String,
  /// brief, standard, or full; applied only after generation.
  pub response_level: String,
  /// Chronological minimal turns, with no independent item-count cap.
  #[serde(default)]
  pub history: Vec<TranslationHistory>,
}

/// Initial product languages; provider output cannot introduce another language tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnLanguage {
  /// English.
  #[serde(rename = "en")]
  English,
  /// Simplified Chinese.
  #[serde(rename = "zh-CN")]
  Chinese,
}

impl TurnLanguage {
  /// Returns the exact wire language tag.
  pub const fn as_str(self) -> &'static str {
    match self { Self::English => "en", Self::Chinese => "zh-CN" }
  }

  fn parse(value: &str) -> Option<Self> {
    match value { "en" => Some(Self::English), "zh-CN" => Some(Self::Chinese), _ => None }
  }
}

/// Deterministic response breadth selected by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseLevel {
  /// Translations and ambiguity labels only.
  Brief,
  /// Concise usage and one example per meaning.
  Standard,
  /// All bounded eligible model-generated lexical details.
  Full,
}

/// Closed input-validation error that never includes the supplied value.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum TurnValidationError {
  /// A required input value is invalid.
  #[error("invalid translation field: {0}")]
  Field(&'static str),
  /// The serialized request exceeds the service byte budget.
  #[error("translation request is too large")]
  TooLarge,
}

/// Validated linguistic input; callers cannot mutate it after validation.
#[derive(Clone, Serialize)]
pub struct TranslationTurn {
  text: String,
  source_language: String,
  target_language: TurnLanguage,
  history: Vec<TranslationHistory>,
  #[serde(skip)]
  response_level: ResponseLevel,
}

impl TranslationTurn {
  /// Validates closed selectors and every history item without imposing a history-count cap.
  ///
  /// # Errors
  /// Returns a closed field error or a request-size error before any provider is called.
  pub fn new(request: TranslationTurnRequest) -> Result<Self, TurnValidationError> {
    if request.text.trim().is_empty() { return Err(TurnValidationError::Field("text")); }
    if request.source_language != "auto" && TurnLanguage::parse(&request.source_language).is_none() {
      return Err(TurnValidationError::Field("source_language"));
    }
    let target_language = TurnLanguage::parse(&request.target_language)
      .ok_or(TurnValidationError::Field("target_language"))?;
    let response_level = match request.response_level.as_str() {
      "brief" => ResponseLevel::Brief, "standard" => ResponseLevel::Standard,
      "full" => ResponseLevel::Full, _ => return Err(TurnValidationError::Field("response_level")),
    };
    // Check raw lengths before serialization to avoid allocating another oversized payload.
    let raw_bytes = request.history.iter().try_fold(request.text.len(), |bytes, turn| {
      bytes.checked_add(turn.source_text.len())?.checked_add(turn.translated_text.len())
        ?.checked_add(turn.source_language.len())?.checked_add(turn.target_language.len())
    }).ok_or(TurnValidationError::TooLarge)?;
    if raw_bytes > MAX_TURN_BYTES { return Err(TurnValidationError::TooLarge); }
    for turn in &request.history {
      if turn.source_text.trim().is_empty() || turn.translated_text.trim().is_empty()
        || TurnLanguage::parse(&turn.source_language).is_none()
        || TurnLanguage::parse(&turn.target_language).is_none() {
        return Err(TurnValidationError::Field("history"));
      }
    }
    if serde_json::to_vec(&request).map_err(|_| TurnValidationError::TooLarge)?.len() > MAX_TURN_BYTES {
      return Err(TurnValidationError::TooLarge);
    }
    Ok(Self { text: request.text, source_language: request.source_language, target_language,
      history: request.history, response_level })
  }

  /// Returns the original text without changing paragraph or formatting boundaries.
  pub fn text(&self) -> &str { &self.text }
  /// Returns the declared source language, or None for automatic detection.
  pub fn source_language(&self) -> Option<TurnLanguage> { TurnLanguage::parse(&self.source_language) }
  /// Returns the requested target language.
  pub fn target_language(&self) -> TurnLanguage { self.target_language }
  /// Returns chronological request-local context.
  pub fn history(&self) -> &[TranslationHistory] { &self.history }
  /// Returns the projection level, never passed to a generation prompt.
  pub fn response_level(&self) -> ResponseLevel { self.response_level }
  /// Derives one request-local lookup form while preserving significant symbols such as + and #.
  pub fn lookup_form(&self) -> String {
    self.text.nfc().flat_map(char::to_lowercase).collect::<String>()
      .split_whitespace().collect::<Vec<_>>().join(" ")
  }
  /// Returns whether input is too long or structured to be one lexical unit.
  pub fn requires_passage(&self) -> bool {
    self.text.chars().count() > MAX_LEXICAL_CHARS || self.text.contains(['\n', '\r'])
  }
}

/// Resolved unit, chosen by orchestration rather than the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationUnit {
  /// One lexical word or technical symbol.
  Word,
  /// An established multiword expression or term.
  Phrase,
  /// Connected text, including uncertain short fragments.
  Passage,
}

/// A model classification is used for lexical routing only at high confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingConfidence {
  /// Confident lexical classification.
  High,
  /// Insufficient evidence: use connected-text translation.
  Uncertain,
}

/// Closed classification result; None explicitly means an unsupported source language.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnClassification {
  /// Suggested unit family, subject to deterministic conservative routing.
  pub unit: TranslationUnit,
  /// Detected supported source language, or None when unsupported.
  pub detected_source_language: Option<TurnLanguage>,
  /// Confidence in lexical routing.
  pub confidence: RoutingConfidence,
}

/// One generated bilingual example, never canonical evidence.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnExample {
  /// Short example in the source language.
  pub source_text: String,
  /// Its translation in the target language.
  pub translated_text: String,
}

/// One generated meaning in a complete, response-level-independent lexical draft.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalMeaningDraft {
  /// Translation of this meaning.
  pub text: String,
  /// Concise distinction from other plausible meanings.
  pub meaning: String,
  /// Word class; an empty value is valid only for phrases.
  pub part_of_speech: String,
  /// Expression type such as idiom or technical term; empty for words.
  pub phrase_type: String,
  /// Bounded alternative lexical forms, not asserted canonical aliases.
  pub aliases: Vec<String>,
  /// Generated bilingual examples.
  pub examples: Vec<TurnExample>,
  /// Concise usage guidance.
  pub usage_notes: Vec<String>,
}

/// Generated meaning candidates without stable IDs, graph claims, or publication metadata.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalTurnDraft {
  /// Ordered materially distinct meanings.
  pub translations: Vec<LexicalMeaningDraft>,
}

impl LexicalTurnDraft {
  /// Rejects oversized, blank, duplicate, or structurally ineligible generated content.
  pub fn is_valid(&self, unit: TranslationUnit) -> bool {
    if self.translations.is_empty() || self.translations.len() > 8 || unit == TranslationUnit::Passage { return false; }
    let mut seen = std::collections::BTreeSet::new();
    self.translations.iter().all(|item| {
      bounded(&item.text, 512) && bounded(&item.meaning, 512)
        && seen.insert((item.text.trim(), item.meaning.trim()))
        && match unit { TranslationUnit::Word => bounded(&item.part_of_speech,64),
          TranslationUnit::Phrase => bounded(&item.phrase_type,64), TranslationUnit::Passage => false }
        && item.part_of_speech.len() <= 128 && item.phrase_type.len() <= 128
        && item.aliases.len() <= 6 && item.aliases.iter().all(|v| bounded(v,128))
        && item.examples.len() <= 4 && item.examples.iter().all(|v| bounded(&v.source_text,512) && bounded(&v.translated_text,512))
        && item.usage_notes.len() <= 6 && item.usage_notes.iter().all(|v| bounded(v,512))
    })
  }
}

fn bounded(value: &str, chars: usize) -> bool { !value.trim().is_empty() && value.chars().count() <= chars }

/// Shared inner translation result used by the unified HTTP response.
#[derive(Clone, Serialize)]
pub struct TranslationTurnResult {
  /// Word, established phrase, or connected passage.
  pub unit: TranslationUnit,
  /// Supported language resolved for the current text.
  pub detected_source_language: TurnLanguage,
  /// Ordered meanings preserved identically across all response levels.
  pub translations: Vec<TurnTranslation>,
}

/// One translation with optional meaning-specific generated detail.
#[derive(Clone, Serialize)]
pub struct TurnTranslation {
  /// Translated text, including preserved formatting for passages.
  pub text: String,
  /// Target language selected by the caller.
  pub language: TurnLanguage,
  /// Meaning label retained at every response level for lexical translations.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub meaning: Option<String>,
  /// Generated supporting material; omitted for brief responses and plain passages.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<TurnDetails>,
}

/// Model-only supporting material, explicitly separated from canonical knowledge.
#[derive(Clone, Serialize)]
pub struct TurnDetails {
  /// Meaning-specific unit type.
  #[serde(rename = "type")]
  pub unit: TranslationUnit,
  /// Word class, when applicable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub part_of_speech: Option<String>,
  /// Established expression type, when applicable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub phrase_type: Option<String>,
  /// Model-generated alternatives available at full level.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub aliases: Vec<String>,
  /// Generated examples; their provenance is declared by generated=true on this details object.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub examples: Vec<TurnExample>,
  /// Usage notes available at standard and full levels.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub usage_notes: Vec<String>,
  /// Always true for this model-only foundation.
  pub generated: bool,
  /// Always exploratory: a model response alone is not verified or evidence-grounded synthesis.
  pub evidence_state: &'static str,
}

impl TranslationTurnResult {
  /// Assembles the full lexical superset; callers must validate the model draft first.
  pub fn lexical(draft: LexicalTurnDraft, unit: TranslationUnit, source: TurnLanguage, target: TurnLanguage) -> Self {
    Self { unit, detected_source_language: source, translations: draft.translations.into_iter().map(|m| TurnTranslation {
      text: m.text, language: target, meaning: Some(m.meaning), details: Some(TurnDetails {
        unit, part_of_speech: (unit==TranslationUnit::Word).then_some(m.part_of_speech),
        phrase_type: (unit==TranslationUnit::Phrase).then_some(m.phrase_type),
        aliases: m.aliases, examples: m.examples, usage_notes: m.usage_notes,
        generated: true, evidence_state: "exploratory",
      }),
    }).collect() }
  }
  /// Produces a plain connected-text result without inventing optional tips or alternatives.
  pub fn passage(text: String, source: TurnLanguage, target: TurnLanguage) -> Self {
    Self { unit: TranslationUnit::Passage, detected_source_language: source,
      translations: vec![TurnTranslation { text, language: target, meaning: None, details: None }] }
  }
  /// Projects an existing superset without changing translation text, meaning count, or rank.
  pub fn project(mut self, level: ResponseLevel) -> Self {
    for translation in &mut self.translations {
      match level {
        ResponseLevel::Brief => translation.details=None,
        ResponseLevel::Standard => if let Some(details)=&mut translation.details {
          details.aliases.clear(); details.examples.truncate(1); details.usage_notes.truncate(2);
        },
        ResponseLevel::Full => {},
      }
    }
    self
  }
}

macro_rules! redacted_debug {
  ($($type:ty),+ $(,)?) => { $(impl std::fmt::Debug for $type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.write_str(concat!(stringify!($type), "(REDACTED)"))
    }
  })+ };
}
redacted_debug!(TranslationHistory, TranslationTurnRequest, TranslationTurn, TurnExample,
  LexicalMeaningDraft, LexicalTurnDraft, TranslationTurnResult, TurnTranslation, TurnDetails);
