//! Domain model for an English learning translation.

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::types::is_language_code;

/// Maximum number of Unicode scalar values in a lookup query.
pub const MAX_QUERY_CHARS: usize = 100;
/// Maximum number of Unicode scalar values in optional disambiguating context.
pub const MAX_CONTEXT_CHARS: usize = 1_000;

/// Validated input to the learning translation pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationInput {
  /// NFC-normalized word or short expression supplied by the learner.
  pub query: String,
  /// BCP-47 source language or `auto`.
  pub source_language: String,
  /// Optional NFC-normalized sentence or phrase used to rank meanings.
  pub context: Option<String>,
  /// Language used for supporting explanations and localized glosses.
  pub explanation_language: String,
  /// Requested English dialect.
  pub english_dialect: EnglishDialect,
  /// Optional CEFR level used to simplify explanations.
  pub learner_level: Option<CefrLevel>,
}

impl TranslationInput {
  /// Validates and normalizes learner input.
  ///
  /// # Errors
  ///
  /// Returns a field-specific validation error for blank, oversized, or malformed input.
  pub fn new(
    query: &str,
    source_language: &str,
    context: Option<&str>,
    explanation_language: &str,
    english_dialect: EnglishDialect,
    learner_level: Option<CefrLevel>,
  ) -> Result<Self, TranslationValidationError> {
    let query = normalize_text(query);
    if query.is_empty() {
      return Err(TranslationValidationError::new(
        "query",
        "must not be blank",
      ));
    }
    if query.chars().count() > MAX_QUERY_CHARS {
      return Err(TranslationValidationError::new(
        "query",
        format!("must contain at most {MAX_QUERY_CHARS} characters"),
      ));
    }

    if source_language != "auto" && !is_language_code(source_language) {
      return Err(TranslationValidationError::new(
        "source_language",
        "must be `auto` or a BCP-47 language tag",
      ));
    }
    if !is_language_code(explanation_language) {
      return Err(TranslationValidationError::new(
        "explanation_language",
        "must be a BCP-47 language tag",
      ));
    }

    let context = context
      .map(normalize_text)
      .filter(|normalized| !normalized.is_empty());
    if context
      .as_ref()
      .is_some_and(|value| value.chars().count() > MAX_CONTEXT_CHARS)
    {
      return Err(TranslationValidationError::new(
        "context",
        format!("must contain at most {MAX_CONTEXT_CHARS} characters"),
      ));
    }

    Ok(Self {
      query,
      source_language: normalize_language_tag(source_language),
      context,
      explanation_language: normalize_language_tag(explanation_language),
      english_dialect,
      learner_level,
    })
  }
}

/// English dialect used for spelling and usage guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnglishDialect {
  /// United States English.
  American,
  /// United Kingdom English.
  British,
}

impl EnglishDialect {
  /// Returns the dialect's canonical BCP-47 tag.
  pub const fn as_tag(self) -> &'static str {
    match self {
      Self::American => "en-US",
      Self::British => "en-GB",
    }
  }
}

/// Common European Framework of Reference learner level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CefrLevel {
  /// Beginner.
  A1,
  /// Elementary.
  A2,
  /// Intermediate.
  B1,
  /// Upper intermediate.
  B2,
  /// Advanced.
  C1,
  /// Proficient.
  C2,
}

impl CefrLevel {
  /// Returns the standard CEFR label.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::A1 => "A1",
      Self::A2 => "A2",
      Self::B1 => "B1",
      Self::B2 => "B2",
      Self::C1 => "C1",
      Self::C2 => "C2",
    }
  }
}

/// Structured model result for one lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslationResult {
  /// Resolved BCP-47 source language.
  pub source_language: String,
  /// Confidence in automatic language detection, when detection was requested.
  pub language_confidence: Confidence,
  /// Ranked possible English meanings.
  pub entries: Vec<EnglishEntry>,
  /// Non-fatal limitations visible to the learner.
  pub warnings: Vec<String>,
}

/// One English meaning and its learning annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct EnglishEntry {
  /// English dictionary headword.
  pub lemma: String,
  /// Part of speech for this meaning.
  pub part_of_speech: PartOfSpeech,
  /// Short English definition.
  pub definition: String,
  /// Optional explanation in the configured explanation language.
  pub localized_gloss: Option<String>,
  /// Confidence that this meaning matches the query.
  pub confidence: Confidence,
  /// Position in results after optional context reranking.
  pub rank: u16,
  /// Pronunciation guidance, when available.
  pub pronunciations: Vec<Pronunciation>,
  /// Important inflected or derived forms.
  pub forms: Vec<WordForm>,
  /// Register, dialect, grammar, and learner-habit guidance.
  pub usage_notes: Vec<UsageNote>,
  /// Contextual examples.
  pub examples: Vec<UsageExample>,
  /// Concise word-origin summary, distinct from lookup history.
  pub etymology: Option<String>,
  /// Suggested relationships that are not canonical graph assertions.
  pub related_words: Vec<RelatedWord>,
}

/// Coarse confidence bucket that does not imply sourced lexical truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
  /// Strong model confidence.
  High,
  /// Plausible result with meaningful uncertainty.
  Medium,
  /// Weak result that should be treated as a suggestion.
  Low,
}

/// Supported word classes for a generated English meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartOfSpeech {
  /// Noun.
  Noun,
  /// Verb.
  Verb,
  /// Adjective.
  Adjective,
  /// Adverb.
  Adverb,
  /// Pronoun.
  Pronoun,
  /// Preposition.
  Preposition,
  /// Conjunction.
  Conjunction,
  /// Determiner.
  Determiner,
  /// Interjection.
  Interjection,
  /// Numeral.
  Numeral,
  /// Multi-word expression or other lexical category.
  Other,
}

/// One pronunciation spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pronunciation {
  /// IPA or another explicitly named notation.
  pub value: String,
  /// Notation name, normally `ipa`.
  pub notation: String,
  /// Optional dialect tag.
  pub dialect: Option<String>,
}

/// One English form connected to the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordForm {
  /// Surface form.
  pub form: String,
  /// Learner-readable grammatical label.
  pub label: String,
}

/// Usage guidance attached to this meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageNote {
  /// Note category such as `register`, `grammar`, `collocation`, or `pitfall`.
  pub kind: UsageNoteKind,
  /// Concise guidance.
  pub text: String,
}

/// Supported usage-note categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageNoteKind {
  /// Formality, tone, or social setting.
  Register,
  /// Dialect or regional restriction.
  Dialect,
  /// Grammatical construction.
  Grammar,
  /// Common word combination.
  Collocation,
  /// Common learner error or misleading transfer.
  Pitfall,
  /// Frequency or habitual-use guidance.
  Habit,
}

/// Example sentence showing the entry in use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageExample {
  /// English example sentence.
  pub english: String,
  /// Optional translation in the explanation language.
  pub localized: Option<String>,
}

/// Suggested non-canonical relation to another English word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedWord {
  /// Related English lemma.
  pub lemma: String,
  /// Type of relationship suggested by the model.
  pub relation: RelationKind,
  /// Optional context restriction or distinction.
  pub note: Option<String>,
}

/// Relationship categories useful to a learner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
  /// Similar meaning in some contexts.
  Synonym,
  /// Opposing meaning in some contexts.
  Antonym,
  /// Broader semantic category.
  Broader,
  /// Narrower semantic category.
  Narrower,
  /// Morphologically related lexeme.
  WordFamily,
  /// Lower member of a contextual intensity scale.
  LowerDegree,
  /// Higher member of a contextual intensity scale.
  HigherDegree,
  /// Commonly confused term.
  Confusable,
  /// Loosely associated term.
  Related,
}

/// Field-specific validation failure.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{field} {message}")]
pub struct TranslationValidationError {
  /// Invalid input field.
  pub field: &'static str,
  /// Human-readable constraint violation.
  pub message: String,
}

impl TranslationValidationError {
  fn new(field: &'static str, message: impl Into<String>) -> Self {
    Self {
      field,
      message: message.into(),
    }
  }
}

fn normalize_text(value: &str) -> String {
  value.trim().nfc().collect()
}

fn normalize_language_tag(value: &str) -> String {
  if value == "auto" {
    return value.to_string();
  }

  value
    .split('-')
    .enumerate()
    .map(|(index, subtag)| {
      if index == 0 {
        subtag.to_ascii_lowercase()
      } else if subtag.len() == 2 {
        subtag.to_ascii_uppercase()
      } else if subtag.len() == 4 {
        let mut chars = subtag.chars();
        let first = chars
          .next()
          .map(|character| character.to_ascii_uppercase())
          .into_iter();
        first
          .chain(chars.map(|character| character.to_ascii_lowercase()))
          .collect()
      } else {
        subtag.to_ascii_lowercase()
      }
    })
    .collect::<Vec<_>>()
    .join("-")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn input_normalizes_unicode_whitespace_and_language_tags() {
    let input = TranslationInput::new(
      "  cafe\u{301}  ",
      "FR",
      Some("  un cafe\u{301} chaud  "),
      "zh-cn",
      EnglishDialect::American,
      Some(CefrLevel::B1),
    )
    .unwrap();

    assert_eq!(input.query, "café");
    assert_eq!(input.context.as_deref(), Some("un café chaud"));
    assert_eq!(input.source_language, "fr");
    assert_eq!(input.explanation_language, "zh-CN");
  }

  #[test]
  fn blank_context_is_removed() {
    let input = TranslationInput::new(
      "caliente",
      "es",
      Some("  "),
      "en",
      EnglishDialect::British,
      None,
    )
    .unwrap();

    assert_eq!(input.context, None);
  }

  #[test]
  fn input_rejects_invalid_or_oversized_fields() {
    let cases = [
      TranslationInput::new(" ", "es", None, "en", EnglishDialect::American, None),
      TranslationInput::new(
        &"a".repeat(MAX_QUERY_CHARS + 1),
        "es",
        None,
        "en",
        EnglishDialect::American,
        None,
      ),
      TranslationInput::new("hola", "es_ES", None, "en", EnglishDialect::American, None),
      TranslationInput::new(
        "hola",
        "es",
        Some(&"a".repeat(MAX_CONTEXT_CHARS + 1)),
        "en",
        EnglishDialect::American,
        None,
      ),
    ];

    assert!(cases.into_iter().all(|result| result.is_err()));
  }
}
