//! Request-local unified translation values, normalization, and deterministic response projection.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Hard bound for serialized input accepted through either HTTP or the public service API.
pub const MAX_TURN_BYTES: usize = 1_048_576;
/// Version of the request-local translation lookup normalization rules.
pub const NORMALIZER_VERSION: &str = "translation-lookup-nfc-v1";
/// Version of deterministic response breadth rules.
pub const PROJECTION_VERSION: &str = "translation-projection-v1";
/// Long input is conservatively translated as connected text, not classified as one lexical unit.
pub const MAX_LEXICAL_CHARS: usize = 128;
/// Maximum number of deterministic lookup forms emitted for one input.
pub const MAX_DERIVED_FORMS: usize = 4;

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

/// Source-language selector supported by the initial translation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
  /// Detect English or Simplified Chinese for the current request.
  Auto,
  /// Use the caller-declared supported language.
  Known(TurnLanguage),
}

impl Serialize for SourceLanguage {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.as_str())
  }
}

impl SourceLanguage {
  /// Returns the exact wire selector.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Known(language) => language.as_str(),
    }
  }

  fn parse(value: &str) -> Option<Self> {
    match value {
      "auto" => Some(Self::Auto),
      _ => TurnLanguage::parse(value).map(Self::Known),
    }
  }
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
    match self {
      Self::English => "en",
      Self::Chinese => "zh-CN",
    }
  }

  /// Parses one exact language tag from the initial closed language set.
  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "en" => Some(Self::English),
      "zh-CN" => Some(Self::Chinese),
      _ => None,
    }
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

impl ResponseLevel {
  /// Parses one exact response level from the closed wire set.
  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "brief" => Some(Self::Brief),
      "standard" => Some(Self::Standard),
      "full" => Some(Self::Full),
      _ => None,
    }
  }
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
  source_language: SourceLanguage,
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
    if request.text.trim().is_empty() {
      return Err(TurnValidationError::Field("text"));
    }
    let source_language = SourceLanguage::parse(&request.source_language)
      .ok_or(TurnValidationError::Field("source_language"))?;
    let target_language = TurnLanguage::parse(&request.target_language)
      .ok_or(TurnValidationError::Field("target_language"))?;
    let response_level = ResponseLevel::parse(&request.response_level)
      .ok_or(TurnValidationError::Field("response_level"))?;
    // Check raw lengths before serialization to avoid allocating another oversized payload.
    let raw_bytes = request
      .history
      .iter()
      .try_fold(request.text.len(), |bytes, turn| {
        bytes
          .checked_add(turn.source_text.len())?
          .checked_add(turn.translated_text.len())?
          .checked_add(turn.source_language.len())?
          .checked_add(turn.target_language.len())
      })
      .ok_or(TurnValidationError::TooLarge)?;
    if raw_bytes > MAX_TURN_BYTES {
      return Err(TurnValidationError::TooLarge);
    }
    for turn in &request.history {
      if turn.source_text.trim().is_empty()
        || turn.translated_text.trim().is_empty()
        || TurnLanguage::parse(&turn.source_language).is_none()
        || TurnLanguage::parse(&turn.target_language).is_none()
      {
        return Err(TurnValidationError::Field("history"));
      }
    }
    if serde_json::to_vec(&request)
      .map_err(|_| TurnValidationError::TooLarge)?
      .len()
      > MAX_TURN_BYTES
    {
      return Err(TurnValidationError::TooLarge);
    }
    Ok(Self {
      text: request.text,
      source_language,
      target_language,
      history: request.history,
      response_level,
    })
  }

  /// Returns the original text without changing paragraph or formatting boundaries.
  pub fn text(&self) -> &str {
    &self.text
  }
  /// Returns the source-language selector.
  pub const fn source_language(&self) -> SourceLanguage {
    self.source_language
  }
  /// Returns the requested target language.
  pub fn target_language(&self) -> TurnLanguage {
    self.target_language
  }
  /// Returns chronological request-local context.
  pub fn history(&self) -> &[TranslationHistory] {
    &self.history
  }
  /// Returns the projection level, never passed to a generation prompt.
  pub fn response_level(&self) -> ResponseLevel {
    self.response_level
  }
  /// Derives one request-local lookup form while preserving significant symbols such as + and #.
  pub fn lookup_form(&self) -> String {
    TranslationNormalizer::new()
      .normalize(&self.text, self.source_language)
      .primary
  }
  /// Returns whether input is too long or structured to be one lexical unit.
  pub fn requires_passage(&self) -> bool {
    self.text.chars().count() > MAX_LEXICAL_CHARS || self.text.contains(['\n', '\r'])
  }
}

/// Deterministic bounded lookup forms derived only for the current request.
#[derive(Clone, Serialize)]
pub struct NormalizedTranslationInput {
  /// NFC, case, whitespace, and punctuation-equivalent normalized primary form.
  pub primary: String,
  /// Deduplicated bounded alternatives ordered from strongest to weakest.
  pub derived_forms: Vec<String>,
}

/// Stateless versioned normalizer for translation lookup and intent classification.
#[derive(Debug, Clone, Copy, Default)]
pub struct TranslationNormalizer;

impl TranslationNormalizer {
  /// Creates the stateless normalizer.
  pub const fn new() -> Self {
    Self
  }

  /// Returns the stable ruleset version for response metadata and release compatibility.
  pub const fn version(self) -> &'static str {
    NORMALIZER_VERSION
  }

  /// Derives bounded lookup forms without altering the original request text.
  pub fn normalize(self, value: &str, language: SourceLanguage) -> NormalizedTranslationInput {
    let punctuation_normalized = value.nfc().map(normalize_punctuation).collect::<String>();
    let cased = match language {
      SourceLanguage::Auto | SourceLanguage::Known(TurnLanguage::English) => punctuation_normalized
        .chars()
        .flat_map(char::to_lowercase)
        .collect(),
      SourceLanguage::Known(TurnLanguage::Chinese) => punctuation_normalized,
    };
    let primary = collapse_whitespace(&cased);
    let mut derived_forms = Vec::with_capacity(MAX_DERIVED_FORMS);
    push_distinct(&mut derived_forms, primary.clone());

    let unquoted = primary.trim_matches(['\'', '"']).trim().to_string();
    push_distinct(&mut derived_forms, unquoted);

    let lexical_punctuation = primary
      .chars()
      .map(|character| match character {
        ',' | '.' | ':' | ';' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' => ' ',
        _ => character,
      })
      .collect::<String>();
    push_distinct(
      &mut derived_forms,
      collapse_whitespace(&lexical_punctuation),
    );
    derived_forms.truncate(MAX_DERIVED_FORMS);

    NormalizedTranslationInput {
      primary,
      derived_forms,
    }
  }
}

fn normalize_punctuation(character: char) -> char {
  match character {
    '\u{2018}' | '\u{2019}' | '\u{02bc}' | '\u{ff07}' => '\'',
    '\u{201c}' | '\u{201d}' | '\u{ff02}' => '"',
    '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2212}' | '\u{ff0d}' => '-',
    '\u{ff0b}' => '+',
    '\u{ff03}' => '#',
    '\u{3000}' => ' ',
    other => other,
  }
}

fn collapse_whitespace(value: &str) -> String {
  value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_distinct(forms: &mut Vec<String>, candidate: String) {
  if !candidate.is_empty() && !forms.contains(&candidate) && forms.len() < MAX_DERIVED_FORMS {
    forms.push(candidate);
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
    if self.translations.is_empty()
      || self.translations.len() > 8
      || unit == TranslationUnit::Passage
    {
      return false;
    }
    let mut seen = std::collections::BTreeSet::new();
    self.translations.iter().all(|item| {
      bounded(&item.text, 512)
        && bounded(&item.meaning, 512)
        && seen.insert((item.text.trim(), item.meaning.trim()))
        && match unit {
          TranslationUnit::Word => bounded(&item.part_of_speech, 64),
          TranslationUnit::Phrase => bounded(&item.phrase_type, 64),
          TranslationUnit::Passage => false,
        }
        && item.part_of_speech.len() <= 128
        && item.phrase_type.len() <= 128
        && item.aliases.len() <= 6
        && item.aliases.iter().all(|v| bounded(v, 128))
        && item.examples.len() <= 4
        && item
          .examples
          .iter()
          .all(|v| bounded(&v.source_text, 512) && bounded(&v.translated_text, 512))
        && item.usage_notes.len() <= 6
        && item.usage_notes.iter().all(|v| bounded(v, 512))
    })
  }
}

fn bounded(value: &str, chars: usize) -> bool {
  !value.trim().is_empty() && value.chars().count() <= chars
}

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
  pub fn lexical(
    draft: LexicalTurnDraft,
    unit: TranslationUnit,
    source: TurnLanguage,
    target: TurnLanguage,
  ) -> Self {
    Self {
      unit,
      detected_source_language: source,
      translations: draft
        .translations
        .into_iter()
        .map(|m| TurnTranslation {
          text: m.text,
          language: target,
          meaning: Some(m.meaning),
          details: Some(TurnDetails {
            unit,
            part_of_speech: (unit == TranslationUnit::Word).then_some(m.part_of_speech),
            phrase_type: (unit == TranslationUnit::Phrase).then_some(m.phrase_type),
            aliases: m.aliases,
            examples: m.examples,
            usage_notes: m.usage_notes,
            generated: true,
            evidence_state: "exploratory",
          }),
        })
        .collect(),
    }
  }
  /// Produces a plain connected-text result without inventing optional tips or alternatives.
  pub fn passage(text: String, source: TurnLanguage, target: TurnLanguage) -> Self {
    Self {
      unit: TranslationUnit::Passage,
      detected_source_language: source,
      translations: vec![TurnTranslation {
        text,
        language: target,
        meaning: None,
        details: None,
      }],
    }
  }
  /// Projects an existing superset without changing translation text, meaning count, or rank.
  pub fn project(mut self, level: ResponseLevel) -> Self {
    for translation in &mut self.translations {
      match level {
        ResponseLevel::Brief => translation.details = None,
        ResponseLevel::Standard => {
          if let Some(details) = &mut translation.details {
            details.aliases.clear();
            details.examples.truncate(1);
            details.usage_notes.truncate(2);
          }
        }
        ResponseLevel::Full => {}
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
redacted_debug!(
  TranslationHistory,
  TranslationTurnRequest,
  TranslationTurn,
  TurnExample,
  LexicalMeaningDraft,
  LexicalTurnDraft,
  TranslationTurnResult,
  TurnTranslation,
  TurnDetails
);

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn request() -> TranslationTurnRequest {
    TranslationTurnRequest {
      text: "C++".to_string(),
      source_language: "en".to_string(),
      target_language: "zh-CN".to_string(),
      response_level: "standard".to_string(),
      history: Vec::new(),
    }
  }

  fn history(source_text: impl Into<String>) -> TranslationHistory {
    TranslationHistory {
      source_text: source_text.into(),
      translated_text: "译文".to_string(),
      source_language: "en".to_string(),
      target_language: "zh-CN".to_string(),
    }
  }

  #[test]
  fn accepts_only_the_initial_closed_language_and_response_level_sets() {
    for source in ["auto", "en", "zh-CN"] {
      for target in ["en", "zh-CN"] {
        for level in ["brief", "standard", "full"] {
          let mut input = request();
          input.source_language = source.to_string();
          input.target_language = target.to_string();
          input.response_level = level.to_string();
          assert!(TranslationTurn::new(input).is_ok());
        }
      }
    }

    for source in ["EN", "zh-cn", "fr", "", "en-US"] {
      let mut input = request();
      input.source_language = source.to_string();
      assert_eq!(
        TranslationTurn::new(input).unwrap_err(),
        TurnValidationError::Field("source_language")
      );
    }

    let mut input = request();
    input.target_language = "auto".to_string();
    assert_eq!(
      TranslationTurn::new(input).unwrap_err(),
      TurnValidationError::Field("target_language")
    );
  }

  #[test]
  fn strict_request_and_history_shapes_reject_unknown_fields() {
    assert!(serde_json::from_value::<TranslationTurnRequest>(json!({
      "text": "hello",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "user_id": "private"
    }))
    .is_err());
    assert!(serde_json::from_value::<TranslationTurnRequest>(json!({
      "text": "hello",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "history": [{
        "source_text": "hello",
        "translated_text": "你好",
        "source_language": "en",
        "target_language": "zh-CN",
        "turn_id": "not-allowed"
      }]
    }))
    .is_err());
  }

  #[test]
  fn normalizer_is_versioned_nfc_and_language_aware() {
    let normalizer = TranslationNormalizer::new();
    let normalized = normalizer.normalize(
      "  CAFE\u{301}\tTEST  ",
      SourceLanguage::Known(TurnLanguage::English),
    );
    assert_eq!(normalizer.version(), "translation-lookup-nfc-v1");
    assert_eq!(normalized.primary, "caf\u{e9} test");

    let chinese = normalizer.normalize("术语 C++", SourceLanguage::Known(TurnLanguage::Chinese));
    assert_eq!(chinese.primary, "术语 C++");
  }

  #[test]
  fn normalizer_maps_equivalent_punctuation_and_collapses_whitespace() {
    let normalized = TranslationNormalizer::new().normalize(
      "  \u{201c}Up\u{2014}in\u{2014}the\u{2014}air\u{201d}\u{3000}test  ",
      SourceLanguage::Known(TurnLanguage::English),
    );
    assert_eq!(normalized.primary, "\"up-in-the-air\" test");
    assert_eq!(normalized.derived_forms[1], "up-in-the-air\" test");
    assert!(normalized.derived_forms.len() <= MAX_DERIVED_FORMS);
  }

  #[test]
  fn normalizer_preserves_meaningful_technical_symbols() {
    let normalizer = TranslationNormalizer::new();
    let c = normalizer.normalize("C", SourceLanguage::Known(TurnLanguage::English));
    let cpp = normalizer.normalize(
      "C\u{ff0b}\u{ff0b}",
      SourceLanguage::Known(TurnLanguage::English),
    );
    let csharp = normalizer.normalize("C\u{ff03}", SourceLanguage::Known(TurnLanguage::English));
    assert_eq!(c.primary, "c");
    assert_eq!(cpp.primary, "c++");
    assert_eq!(csharp.primary, "c#");
    assert_ne!(c.primary, cpp.primary);
    assert_ne!(c.primary, csharp.primary);
    assert_ne!(cpp.primary, csharp.primary);
  }

  #[test]
  fn derived_forms_are_bounded_deduplicated_and_keep_symbols() {
    let normalized = TranslationNormalizer::new()
      .normalize("\"C++?!\"", SourceLanguage::Known(TurnLanguage::English));
    assert!(normalized.derived_forms.len() <= MAX_DERIVED_FORMS);
    assert_eq!(normalized.derived_forms[0], "\"c++?!\"");
    assert!(normalized
      .derived_forms
      .iter()
      .any(|form| form.contains("c++")));
    let unique = normalized
      .derived_forms
      .iter()
      .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), normalized.derived_forms.len());
  }

  #[test]
  fn history_has_no_item_cap_inside_the_common_body_bound() {
    let mut input = request();
    input.history = (0..2_048)
      .map(|index| history(format!("turn-{index}")))
      .collect();
    let turn = TranslationTurn::new(input).unwrap();
    assert_eq!(turn.history().len(), 2_048);
  }

  #[test]
  fn history_and_current_text_share_the_one_mebibyte_bound() {
    let mut oversized_history = request();
    oversized_history.history = vec![history("x".repeat(MAX_TURN_BYTES))];
    assert_eq!(
      TranslationTurn::new(oversized_history).unwrap_err(),
      TurnValidationError::TooLarge
    );

    let mut oversized_text = request();
    oversized_text.text = "x".repeat(MAX_TURN_BYTES);
    assert_eq!(
      TranslationTurn::new(oversized_text).unwrap_err(),
      TurnValidationError::TooLarge
    );
  }

  #[test]
  fn history_rejects_blank_text_and_non_closed_language_tags() {
    let mut blank = request();
    blank.history = vec![history("  ")];
    assert_eq!(
      TranslationTurn::new(blank).unwrap_err(),
      TurnValidationError::Field("history")
    );

    let mut unsupported = request();
    let mut prior = history("hello");
    prior.source_language = "auto".to_string();
    unsupported.history = vec![prior];
    assert_eq!(
      TranslationTurn::new(unsupported).unwrap_err(),
      TurnValidationError::Field("history")
    );
  }

  #[test]
  fn projection_changes_only_supporting_breadth() {
    let draft = LexicalTurnDraft {
      translations: vec![
        LexicalMeaningDraft {
          text: "热的".to_string(),
          meaning: "high temperature".to_string(),
          part_of_speech: "adjective".to_string(),
          phrase_type: String::new(),
          aliases: vec!["heated".to_string()],
          examples: vec![
            TurnExample {
              source_text: "hot tea".to_string(),
              translated_text: "热茶".to_string(),
            },
            TurnExample {
              source_text: "hot day".to_string(),
              translated_text: "炎热的一天".to_string(),
            },
          ],
          usage_notes: vec![
            "temperature".to_string(),
            "literal".to_string(),
            "common".to_string(),
          ],
        },
        LexicalMeaningDraft {
          text: "热门的".to_string(),
          meaning: "popular".to_string(),
          part_of_speech: "adjective".to_string(),
          phrase_type: String::new(),
          aliases: Vec::new(),
          examples: Vec::new(),
          usage_notes: Vec::new(),
        },
      ],
    };
    assert!(draft.is_valid(TranslationUnit::Word));
    let full = TranslationTurnResult::lexical(
      draft,
      TranslationUnit::Word,
      TurnLanguage::English,
      TurnLanguage::Chinese,
    );
    let brief = serde_json::to_value(full.clone().project(ResponseLevel::Brief)).unwrap();
    let standard = serde_json::to_value(full.clone().project(ResponseLevel::Standard)).unwrap();
    let full = serde_json::to_value(full.project(ResponseLevel::Full)).unwrap();

    for result in [&brief, &standard, &full] {
      assert_eq!(result["translations"].as_array().unwrap().len(), 2);
      assert_eq!(result["translations"][0]["text"], "热的");
      assert_eq!(result["translations"][1]["meaning"], "popular");
    }
    assert!(brief["translations"][0].get("details").is_none());
    assert_eq!(
      standard["translations"][0]["details"]["examples"]
        .as_array()
        .unwrap()
        .len(),
      1
    );
    assert!(standard["translations"][0]["details"]
      .get("aliases")
      .is_none());
    assert_eq!(
      full["translations"][0]["details"]["examples"]
        .as_array()
        .unwrap()
        .len(),
      2
    );
  }

  #[test]
  fn debug_and_validation_errors_never_contain_request_or_history_text() {
    let secret_text = "current-secret-8172";
    let history_secret = "history-secret-4815";
    let mut input = request();
    input.text = secret_text.to_string();
    input.history = vec![history(history_secret)];
    assert_eq!(format!("{input:?}"), "TranslationTurnRequest(REDACTED)");
    let turn = TranslationTurn::new(input).unwrap();
    let rendered = format!("{turn:?}");
    assert!(!rendered.contains(secret_text));
    assert!(!rendered.contains(history_secret));

    let mut invalid = request();
    invalid.text = secret_text.to_string();
    invalid.history = vec![history("  ")];
    let error = TranslationTurn::new(invalid).unwrap_err().to_string();
    assert!(!error.contains(secret_text));
    assert!(!error.contains(history_secret));
  }
}
