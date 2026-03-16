use crate::types::{InputType, TranslationMode};

pub fn build_system_prompt(model_name: &str) -> &'static str {
  if model_name.to_lowercase().contains("qwen") {
    QWEN_SYSTEM_PROMPT
  } else {
    SYSTEM_PROMPT
  }
}

pub fn build_user_prompt(
  text: &str,
  source_lang: &str,
  target_lang: &str,
  input_type: InputType,
  mode: TranslationMode,
) -> String {
  match (input_type, mode) {
    (InputType::Word, TranslationMode::Basic) => build_word_basic_prompt(text, source_lang, target_lang),
    (InputType::Word, TranslationMode::Explain) => build_word_explain_prompt(text, source_lang, target_lang),
    (InputType::Word, TranslationMode::FullAnalysis) => build_word_full_analysis_prompt(text, source_lang, target_lang),

    (InputType::Phrase, TranslationMode::Basic) => build_phrase_basic_prompt(text, source_lang, target_lang),
    (InputType::Phrase, TranslationMode::Explain) => build_phrase_explain_prompt(text, source_lang, target_lang),
    (InputType::Phrase, TranslationMode::FullAnalysis) => build_phrase_full_analysis_prompt(text, source_lang, target_lang),

    (InputType::Sentence, TranslationMode::Basic) => build_sentence_basic_prompt(text, source_lang, target_lang),
    (InputType::Sentence, TranslationMode::Explain) => build_sentence_explain_prompt(text, source_lang, target_lang),

    (InputType::Paragraph, TranslationMode::Basic) | (InputType::Essay, TranslationMode::Basic) => {
      build_paragraph_essay_basic_prompt(text, source_lang, target_lang)
    }

    (InputType::Sentence, TranslationMode::FullAnalysis)
    | (InputType::Paragraph, TranslationMode::Explain | TranslationMode::FullAnalysis)
    | (InputType::Essay, TranslationMode::Explain | TranslationMode::FullAnalysis) => {
      build_unsupported_combination_prompt(input_type, mode)
    }

    _ => build_default_prompt(text, source_lang, target_lang, input_type),
  }
}

fn build_word_basic_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate the word '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"headword\": \"the word\",\n\
  \"part_of_speech\": \"noun|verb|adjective|etc\",\n\
  \"phonetic\": \"/ipa/pronunciation/\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"synonyms\": [\"synonym1\", \"synonym2\"],\n\
  \"antonyms\": [\"antonym1\", \"antonym2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ]\n\
}}\n\n\
Provide 1-3 primary translations, 2-5 synonyms, and 1-2 examples. Return strict JSON only."
  )
}

fn build_word_explain_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate and explain the word '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"headword\": \"the word\",\n\
  \"part_of_speech\": \"noun|verb|adjective|etc\",\n\
  \"phonetic\": \"/ipa/pronunciation/\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"synonyms\": [\"synonym1\", \"synonym2\"],\n\
  \"antonyms\": [\"antonym1\", \"antonym2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ],\n\
  \"explain\": {{\n\
    \"meaning\": \"concise definition and meaning\",\n\
    \"story\": \"etymology or origin story if available\",\n\
    \"when_to_use\": \"when and in what contexts to use this word\",\n\
    \"how_to_use\": \"practical usage example in {target_lang}\",\n\
    \"context\": \"linguistic or cultural context\",\n\
    \"lexical_analysis\": {{\n\
      \"root\": \"etymological root\",\n\
      \"related_phrases\": [\"phrase1\", \"phrase2\"]\n\
    }}\n\
  }}\n\
}}\n\n\
Return strict JSON only."
  )
}

fn build_word_full_analysis_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Provide a full analysis and translation of the word '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"headword\": \"the word\",\n\
  \"part_of_speech\": \"noun|verb|adjective|etc\",\n\
  \"phonetic\": \"/ipa/pronunciation/\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"synonyms\": [\"synonym1\", \"synonym2\"],\n\
  \"antonyms\": [\"antonym1\", \"antonym2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ],\n\
  \"explain\": {{\n\
    \"meaning\": \"concise definition and meaning\",\n\
    \"story\": \"etymology or origin story if available\",\n\
    \"when_to_use\": \"when and in what contexts to use this word\",\n\
    \"how_to_use\": \"practical usage example in {target_lang}\",\n\
    \"context\": \"linguistic or cultural context\",\n\
    \"lexical_analysis\": {{\n\
      \"root\": \"etymological root\",\n\
      \"related_phrases\": [\"phrase1\", \"phrase2\"]\n\
    }}\n\
  }},\n\
  \"relationships\": {{\n\
    \"related_words\": [\n\
      {{\n\
        \"word\": \"related word\",\n\
        \"type\": \"relationship type\",\n\
        \"similarity\": 0.0-1.0\n\
      }}\n\
    ],\n\
    \"by_pos\": {{\n\
      \"nouns\": [\"noun1\", \"noun2\"],\n\
      \"verbs\": [\"verb1\", \"verb2\"],\n\
      \"adjectives\": [\"adjective1\", \"adjective2\"]\n\
    }}\n\
  }}\n\
}}\n\n\
Include 3-5 related words with similarity scores. Return strict JSON only."
  )
}

fn build_phrase_basic_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate the phrase '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"phrase\": \"the phrase\",\n\
  \"headword\": \"key word in phrase\",\n\
  \"part_of_speech\": \"phrase|idiom|expression\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ]\n\
}}\n\n\
Provide 1-3 primary translations and 1-2 examples. Return strict JSON only."
  )
}

fn build_phrase_explain_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate and explain the phrase '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"phrase\": \"the phrase\",\n\
  \"headword\": \"key word in phrase\",\n\
  \"part_of_speech\": \"phrase|idiom|expression\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ],\n\
  \"explain\": {{\n\
    \"meaning\": \"what the phrase means\",\n\
    \"story\": \"origin or background of the phrase\",\n\
    \"when_to_use\": \"appropriate contexts for use\",\n\
    \"how_to_use\": \"practical usage example in {target_lang}\",\n\
    \"context\": \"register (formal/informal) and usage notes\",\n\
    \"lexical_analysis\": {{\n\
      \"structure\": \"grammatical structure\",\n\
      \"idiomatic\": true/false,\n\
      \"related_phrases\": [\"phrase1\", \"phrase2\"]\n\
    }}\n\
  }}\n\
}}\n\n\
Return strict JSON only."
  )
}

fn build_phrase_full_analysis_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Provide a full analysis and translation of the phrase '{text}' from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"phrase\": \"the phrase\",\n\
  \"headword\": \"key word in phrase\",\n\
  \"part_of_speech\": \"phrase|idiom|expression\",\n\
  \"translations\": [\"translation1\", \"translation2\"],\n\
  \"examples\": [\n\
    {{\n\
      \"source\": \"example sentence in {source_lang}\",\n\
      \"translation\": \"example sentence in {target_lang}\"\n\
    }}\n\
  ],\n\
  \"explain\": {{\n\
    \"meaning\": \"what the phrase means\",\n\
    \"story\": \"origin or background of the phrase\",\n\
    \"when_to_use\": \"appropriate contexts for use\",\n\
    \"how_to_use\": \"practical usage example in {target_lang}\",\n\
    \"context\": \"register (formal/informal) and usage notes\",\n\
    \"lexical_analysis\": {{\n\
      \"structure\": \"grammatical structure\",\n\
      \"idiomatic\": true/false,\n\
      \"related_phrases\": [\"phrase1\", \"phrase2\"]\n\
    }}\n\
  }},\n\
  \"relationships\": {{\n\
    \"related_phrases\": [\n\
      {{\n\
        \"phrase\": \"related phrase\",\n\
        \"type\": \"relationship type\",\n\
        \"similarity\": 0.0-1.0\n\
      }}\n\
    ],\n\
    \"related_concepts\": [\"concept1\", \"concept2\"]\n\
  }}\n\
}}\n\n\
Include 3-5 related phrases with similarity scores. Return strict JSON only."
  )
}

fn build_sentence_basic_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate the following sentence from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"tone\": \"neutral|formal|informal|casual|academic\",\n\
  \"rephrasing\": \"natural {target_lang} translation\"\n\
}}\n\n\
Text: {text}\n\n\
Return strict JSON only."
  )
}

fn build_sentence_explain_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate and explain the following sentence from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"tone\": \"neutral|formal|informal|casual|academic\",\n\
  \"rephrasing\": \"natural {target_lang} translation\",\n\
  \"explain\": {{\n\
    \"meaning\": \"what the sentence means\",\n\
    \"usage\": \"how and when this sentence is typically used\",\n\
    \"context\": \"additional context about the sentence\"\n\
  }}\n\
}}\n\n\
Text: {text}\n\n\
Return strict JSON only."
  )
}

fn build_paragraph_essay_basic_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
  format!(
    "Translate the following text from {source_lang} to {target_lang}. \
Return a compact JSON object with this exact structure:\n\
{{\n\
  \"text\": \"original text\",\n\
  \"translation\": \"full translation in {target_lang}\"\n\
}}\n\n\
Text: {text}\n\n\
Return strict JSON only."
  )
}

fn build_default_prompt(text: &str, source_lang: &str, target_lang: &str, input_type: InputType) -> String {
  format!(
    "Translate the following text from {source_lang} to {target_lang}. \
Return a compact JSON object with exactly one key named \"translation\". \
Do not include markdown, code fences, or explanations. \
The detected input type is {input_type:?}. \
Text: {text}"
  )
}

fn build_unsupported_combination_prompt(input_type: InputType, mode: TranslationMode) -> String {
  format!(
    "Error: The combination of input_type '{:?}' and mode '{:?}' is not supported.",
    input_type, mode
  )
}

const SYSTEM_PROMPT: &str = "You are a translation engine. Return strict JSON only.";

const QWEN_SYSTEM_PROMPT: &str = "You are a translation engine. Return strict JSON only. Do not think, reason, or explain - just translate directly without any additional processing.";
