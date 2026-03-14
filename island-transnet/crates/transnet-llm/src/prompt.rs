// Language code to name mappings for the 8 major languages supported by Qwen
const MAJOR_LANGUAGES: &[(&str, &str)] = &[
  ("zh", "中文"),
  ("en", "English"),
  ("ja", "日本語"),
  ("ko", "한국어"),
  ("fr", "Français"),
  ("de", "Deutsch"),
  ("es", "Español"),
  ("ru", "Русский"),
];

/// Check if a language (by code or name) is one of the 8 major languages supported by Qwen
pub fn is_major_language(lang: &str) -> bool {
  let lang_lower = lang.to_lowercase();

  // Check by language code
  for (code, name) in MAJOR_LANGUAGES {
    if code.to_lowercase() == lang_lower || name.to_lowercase() == lang_lower {
      return true;
    }
  }
  false
}

/// Get the language name from a language code (for major languages)
pub fn get_language_name(code: &str) -> Option<&'static str> {
  for (lang_code, lang_name) in MAJOR_LANGUAGES {
    if lang_code.to_lowercase() == code.to_lowercase() {
      return Some(lang_name);
    }
  }
  None
}

// ============================================================================
// QWEN PROMPT FUNCTIONS (8 Major Languages with Structured Output)
// ============================================================================

// Word Translation Prompts

/// Qwen prompt for word basic translation
/// Returns: headword, part_of_speech, phonetic, translations, synonyms, antonyms, examples
pub fn qwen_word_basic_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator. Translate the word from {source} to {target}.

Word to translate: {text}

Provide the following information in plain text format:

headword: [the word itself]
part_of_speech: [noun/verb/adjective/etc.]
phonetic: [IPA pronunciation if available]
translations: [comma-separated translations in {target}]
synonyms: [comma-separated synonyms in source language]
antonyms: [comma-separated antonyms in source language]
examples: [1-3 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

/// Qwen prompt for word explain translation
/// Includes explain section with meaning, story, when_to_use, how_to_use, context, lexical_analysis
pub fn qwen_word_explain_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator and linguist. Translate and explain the word from {source} to {target}.

Word to translate: {text}

Provide the following information in plain text format:

headword: [the word itself]
part_of_speech: [noun/verb/adjective/etc.]
phonetic: [IPA pronunciation if available]
translations: [comma-separated translations in {target}]
synonyms: [comma-separated synonyms in source language]
antonyms: [comma-separated antonyms in source language]
examples: [1-3 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

explain:
meaning: [detailed definition in English]
story: [etymology or origin story]
when_to_use: [situations and contexts where this word is appropriate]
how_to_use: [practical usage example in {target}]
context: [formal/informal, technical, literary, etc.]
lexical_analysis:
root: [etymological root]
related_phrases: [comma-separated related phrases or idioms]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

/// Qwen prompt for word full analysis translation
/// Adds relationships section with related_words and by_pos
pub fn qwen_word_full_analysis_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator and linguist. Translate and provide full analysis of the word from {source} to {target}.

Word to translate: {text}

Provide the following information in plain text format:

headword: [the word itself]
part_of_speech: [noun/verb/adjective/etc.]
phonetic: [IPA pronunciation if available]
translations: [comma-separated translations in {target}]
synonyms: [comma-separated synonyms in source language]
antonyms: [comma-separated antonyms in source language]
examples: [1-3 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

explain:
meaning: [detailed definition in English]
story: [etymology or origin story]
when_to_use: [situations and contexts where this word is appropriate]
how_to_use: [practical usage example in {target}]
context: [formal/informal, technical, literary, etc.]
lexical_analysis:
root: [etymological root]
related_phrases: [comma-separated related phrases or idioms]

relationships:
related_words: [3-5 related words with similarity scores, format: \"word\" (similarity: 0.XX, type: category)]
by_pos:
nouns: [comma-separated related nouns]
verbs: [comma-separated related verbs]
adjectives: [comma-separated related adjectives]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

// Phrase Translation Prompts

/// Qwen prompt for phrase basic translation
/// Returns: phrase, headword, part_of_speech, translations, examples
pub fn qwen_phrase_basic_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator. Translate the phrase from {source} to {target}.

Phrase to translate: {text}

Provide the following information in plain text format:

phrase: [the phrase itself]
headword: [main keyword in the phrase]
part_of_speech: [phrase/idiom/collocation/etc.]
translations: [comma-separated translations in {target}]
examples: [1-2 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

/// Qwen prompt for phrase explain translation
/// Adds explain section with phrase-specific content
pub fn qwen_phrase_explain_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator and linguist. Translate and explain the phrase from {source} to {target}.

Phrase to translate: {text}

Provide the following information in plain text format:

phrase: [the phrase itself]
headword: [main keyword in the phrase]
part_of_speech: [phrase/idiom/collocation/etc.]
translations: [comma-separated translations in {target}]
examples: [1-2 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

explain:
meaning: [detailed definition in English]
story: [etymology or origin story]
when_to_use: [situations and contexts where this phrase is appropriate]
how_to_use: [practical usage example in {target}]
context: [formal/informal, idiomatic, literary, etc.]
lexical_analysis:
structure: [grammatical structure]
idiomatic: [true/false]
related_phrases: [comma-separated related phrases or idioms]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

/// Qwen prompt for phrase full analysis translation
/// Adds relationships section
pub fn qwen_phrase_full_analysis_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator and linguist. Translate and provide full analysis of the phrase from {source} to {target}.

Phrase to translate: {text}

Provide the following information in plain text format:

phrase: [the phrase itself]
headword: [main keyword in the phrase]
part_of_speech: [phrase/idiom/collocation/etc.]
translations: [comma-separated translations in {target}]
examples: [1-2 example sentences with their translations, format: \"Source sentence\" -> \"Target sentence\"\"]

explain:
meaning: [detailed definition in English]
story: [etymology or origin story]
when_to_use: [situations and contexts where this phrase is appropriate]
how_to_use: [practical usage example in {target}]
context: [formal/informal, idiomatic, literary, etc.]
lexical_analysis:
structure: [grammatical structure]
idiomatic: [true/false]
related_phrases: [comma-separated related phrases or idioms]

relationships:
related_phrases: [3-5 related phrases with similarity scores, format: \"phrase\" (similarity: 0.XX, type: category)]
related_concepts: [comma-separated related concepts]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

// Sentence Translation Prompts

/// Qwen prompt for sentence basic translation
/// Returns: tone, rephrasing
pub fn qwen_sentence_basic_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator. Translate the sentence from {source} to {target}.

Sentence to translate: {text}

Provide the following information in plain text format:

tone: [neutral/formal/informal/friendly/professional/etc.]
rephrasing: [natural translation in {target}]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

/// Qwen prompt for sentence explain translation
/// Adds explain section with meaning, usage, context
pub fn qwen_sentence_explain_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator and linguist. Translate and explain the sentence from {source} to {target}.

Sentence to translate: {text}

Provide the following information in plain text format:

tone: [neutral/formal/informal/friendly/professional/etc.]
rephrasing: [natural translation in {target}]

explain:
meaning: [detailed explanation of the sentence's meaning in English]
usage: [when and how this sentence is typically used]
context: [situational context, register, formality level]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

// Paragraph/Essay Translation Prompts

/// Qwen prompt for paragraph/essay basic translation
/// Returns: text, translation
pub fn qwen_paragraph_basic_prompt(source_lang: &str, target_lang: &str, text: &str) -> String {
  format!(
    "You are a professional translator. Translate the following text from {source} to {target}.

Text to translate: {text}

Provide the following information in plain text format:

text: [the original text]
translation: [natural translation in {target}]

Only provide the requested information, no conversational text.",
    source = source_lang,
    target = target_lang,
    text = text.trim()
  )
}

// ============================================================================
// GEMMA PROMPT FUNCTION (All other languages - Translation Only)
// ============================================================================

/// Gemma prompt for translation-only output
/// Returns ONLY the translation without any additional information
pub fn gemma_translation_prompt(source_code: &str, target_code: &str, text: &str) -> String {
  format!(
    "Translate the following text from {source} to {target}. Provide ONLY the translation, no explanations or additional information.

Text: {text}

Translation:",
    source = source_code,
    target = target_code,
    text = text.trim()
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_major_language() {
    assert!(is_major_language("en"));
    assert!(is_major_language("EN"));
    assert!(is_major_language("english"));
    assert!(is_major_language("English"));
    assert!(is_major_language("中文"));
    assert!(is_major_language("zh"));
    assert!(is_major_language("日本語"));
    assert!(is_major_language("ja"));
    assert!(is_major_language("한국어"));
    assert!(is_major_language("ko"));
    assert!(is_major_language("Français"));
    assert!(is_major_language("fr"));
    assert!(is_major_language("Deutsch"));
    assert!(is_major_language("de"));
    assert!(is_major_language("Español"));
    assert!(is_major_language("es"));
    assert!(is_major_language("Русский"));
    assert!(is_major_language("ru"));

    assert!(!is_major_language("cs"));
    assert!(!is_major_language("it"));
    assert!(!is_major_language("pt"));
    assert!(!is_major_language("arabic"));
  }

  #[test]
  fn test_get_language_name() {
    assert_eq!(get_language_name("zh"), Some("中文"));
    assert_eq!(get_language_name("en"), Some("English"));
    assert_eq!(get_language_name("ja"), Some("日本語"));
    assert_eq!(get_language_name("ko"), Some("한국어"));
    assert_eq!(get_language_name("fr"), Some("Français"));
    assert_eq!(get_language_name("de"), Some("Deutsch"));
    assert_eq!(get_language_name("es"), Some("Español"));
    assert_eq!(get_language_name("ru"), Some("Русский"));
    assert_eq!(get_language_name("cs"), None);
  }

  #[test]
  fn test_qwen_word_basic_prompt() {
    let prompt = qwen_word_basic_prompt("English", "中文", "hello");
    assert!(prompt.contains("English"));
    assert!(prompt.contains("中文"));
    assert!(prompt.contains("hello"));
    assert!(prompt.contains("headword"));
    assert!(prompt.contains("part_of_speech"));
    assert!(prompt.contains("phonetic"));
    assert!(prompt.contains("translations"));
  }

  #[test]
  fn test_gemma_translation_prompt() {
    let prompt = gemma_translation_prompt("cs", "de-DE", "Dobrý den");
    assert!(prompt.contains("cs"));
    assert!(prompt.contains("de-DE"));
    assert!(prompt.contains("Dobrý den"));
    assert!(prompt.contains("ONLY"));
    assert!(!prompt.contains("headword"));
    assert!(!prompt.contains("explain"));
  }
}
