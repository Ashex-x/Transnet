//! Parser and schema checks for JSON returned by translation models.
//!
//! This module validates the stable top-level response shapes. It intentionally
//! does not interpret the translated prose or enforce provider-specific output.

use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};

use crate::types::{InputType, TranslationMode};

/// Parses a JSON model response, accepting a surrounding Markdown code fence.
///
/// # Errors
///
/// Returns an error when no JSON object can be found or the extracted content is
/// not valid JSON.
pub fn parse_llm_response(content: &str) -> Result<Value> {
  let trimmed = content.trim();

  if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
    return Ok(parsed);
  }

  let json_str = extract_json_from_markdown(trimmed)?;

  serde_json::from_str(json_str).context("failed to parse model JSON payload")
}

fn extract_json_from_markdown(content: &str) -> Result<&str> {
  if content.starts_with("```") {
    let content = content.trim_start_matches("```");
    let content = content.trim_start_matches("json");
    let content = content.trim();

    if let Some(end_idx) = content.find("\n```") {
      return Ok(content[..end_idx].trim());
    }

    if let Some(end_idx) = content.find("```") {
      return Ok(content[..end_idx].trim());
    }

    return Ok(content.trim());
  }

  let start = content
    .find('{')
    .ok_or_else(|| anyhow!("response did not contain JSON"))?;

  let end = content
    .rfind('}')
    .ok_or_else(|| anyhow!("response did not contain closing JSON"))?;

  if end <= start {
    return Err(anyhow!(
      "invalid JSON structure: closing brace before opening brace"
    ));
  }

  Ok(content[start..=end].trim())
}

/// Validates the required response fields for an input type and translation mode.
///
/// The validation is intentionally structural: it checks object, array, and
/// string fields without constraining the provider's translated values.
///
/// # Errors
///
/// Returns an error for missing or wrongly typed fields and for unsupported
/// input-type/mode combinations.
pub fn validate_translation_structure(
  json: &Value,
  input_type: &InputType,
  mode: &TranslationMode,
) -> Result<()> {
  match (input_type, mode) {
    (InputType::Word, TranslationMode::Basic) => {
      validate_word_basic(json)?;
    }
    (InputType::Word, TranslationMode::Explain) => {
      validate_word_explain(json)?;
    }
    (InputType::Word, TranslationMode::FullAnalysis) => {
      validate_word_full_analysis(json)?;
    }

    (InputType::Phrase, TranslationMode::Basic) => {
      validate_phrase_basic(json)?;
    }
    (InputType::Phrase, TranslationMode::Explain) => {
      validate_phrase_explain(json)?;
    }
    (InputType::Phrase, TranslationMode::FullAnalysis) => {
      validate_phrase_full_analysis(json)?;
    }

    (InputType::Sentence, TranslationMode::Basic) => {
      validate_sentence_basic(json)?;
    }
    (InputType::Sentence, TranslationMode::Explain) => {
      validate_sentence_explain(json)?;
    }

    (InputType::Paragraph | InputType::Essay, TranslationMode::Basic) => {
      validate_paragraph_essay_basic(json)?;
    }

    (InputType::Sentence, TranslationMode::FullAnalysis)
    | (
      InputType::Paragraph | InputType::Essay,
      TranslationMode::Explain | TranslationMode::FullAnalysis,
    ) => {
      return Err(anyhow!(
        "unsupported combination: input_type={:?}, mode={:?}",
        input_type,
        mode
      ));
    }

    _ => {
      if !json.is_object() {
        return Err(anyhow!("expected JSON object, got {}", json));
      }
    }
  }

  Ok(())
}

fn validate_word_basic(json: &Value) -> Result<()> {
  ensure_string_field(json, "headword")?;
  ensure_string_field(json, "part_of_speech")?;
  ensure_string_field(json, "phonetic")?;
  ensure_array_field(json, "translations")?;
  ensure_array_field(json, "synonyms")?;
  ensure_array_field(json, "antonyms")?;
  ensure_array_field(json, "examples")?;
  Ok(())
}

fn validate_word_explain(json: &Value) -> Result<()> {
  validate_word_basic(json)?;
  ensure_object_field(json, "explain")?;

  let explain = json
    .get("explain")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow!("explain must be an object"))?;

  ensure_string_field_in_obj(explain, "meaning")?;
  ensure_string_field_in_obj(explain, "story")?;
  ensure_string_field_in_obj(explain, "when_to_use")?;
  ensure_string_field_in_obj(explain, "how_to_use")?;
  ensure_string_field_in_obj(explain, "context")?;
  ensure_object_field_in_obj(explain, "lexical_analysis")?;

  Ok(())
}

fn validate_word_full_analysis(json: &Value) -> Result<()> {
  validate_word_explain(json)?;
  ensure_object_field(json, "relationships")?;

  let relationships = json
    .get("relationships")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow!("relationships must be an object"))?;

  ensure_array_field_in_obj(relationships, "related_words")?;
  ensure_object_field_in_obj(relationships, "by_pos")?;

  Ok(())
}

fn validate_phrase_basic(json: &Value) -> Result<()> {
  ensure_string_field(json, "phrase")?;
  ensure_string_field(json, "headword")?;
  ensure_string_field(json, "part_of_speech")?;
  ensure_array_field(json, "translations")?;
  ensure_array_field(json, "examples")?;
  Ok(())
}

fn validate_phrase_explain(json: &Value) -> Result<()> {
  validate_phrase_basic(json)?;
  ensure_object_field(json, "explain")?;

  let explain = json
    .get("explain")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow!("explain must be an object"))?;

  ensure_string_field_in_obj(explain, "meaning")?;
  ensure_string_field_in_obj(explain, "story")?;
  ensure_string_field_in_obj(explain, "when_to_use")?;
  ensure_string_field_in_obj(explain, "how_to_use")?;
  ensure_string_field_in_obj(explain, "context")?;
  ensure_object_field_in_obj(explain, "lexical_analysis")?;

  Ok(())
}

fn validate_phrase_full_analysis(json: &Value) -> Result<()> {
  validate_phrase_explain(json)?;
  ensure_object_field(json, "relationships")?;

  let relationships = json
    .get("relationships")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow!("relationships must be an object"))?;

  ensure_array_field_in_obj(relationships, "related_phrases")?;
  ensure_array_field_in_obj(relationships, "related_concepts")?;

  Ok(())
}

fn validate_sentence_basic(json: &Value) -> Result<()> {
  ensure_string_field(json, "tone")?;
  ensure_string_field(json, "rephrasing")?;
  Ok(())
}

fn validate_sentence_explain(json: &Value) -> Result<()> {
  validate_sentence_basic(json)?;
  ensure_object_field(json, "explain")?;

  let explain = json
    .get("explain")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow!("explain must be an object"))?;

  ensure_string_field_in_obj(explain, "meaning")?;
  ensure_string_field_in_obj(explain, "usage")?;
  ensure_string_field_in_obj(explain, "context")?;

  Ok(())
}

fn validate_paragraph_essay_basic(json: &Value) -> Result<()> {
  ensure_string_field(json, "text")?;
  ensure_string_field(json, "translation")?;
  Ok(())
}

fn ensure_string_field(json: &Value, field: &str) -> Result<()> {
  ensure_field(json.get(field), field, Value::is_string, "string")
}

fn ensure_string_field_in_obj(obj: &Map<String, Value>, field: &str) -> Result<()> {
  ensure_field(obj.get(field), field, Value::is_string, "string")
}

fn ensure_array_field(json: &Value, field: &str) -> Result<()> {
  ensure_field(json.get(field), field, Value::is_array, "array")
}

fn ensure_array_field_in_obj(obj: &Map<String, Value>, field: &str) -> Result<()> {
  ensure_field(obj.get(field), field, Value::is_array, "array")
}

fn ensure_object_field(json: &Value, field: &str) -> Result<()> {
  ensure_field(json.get(field), field, Value::is_object, "object")
}

fn ensure_object_field_in_obj(obj: &Map<String, Value>, field: &str) -> Result<()> {
  ensure_field(obj.get(field), field, Value::is_object, "object")
}

fn ensure_field(
  value: Option<&Value>,
  field: &str,
  predicate: fn(&Value) -> bool,
  expected: &str,
) -> Result<()> {
  match value {
    Some(value) if predicate(value) => Ok(()),
    Some(value) => Err(anyhow!("field '{field}' must be {expected}, got {value}")),
    None => Err(anyhow!("missing required field '{field}'")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_llm_response_accepts_raw_json() {
    let json = r#"{"headword":"fox","part_of_speech":"noun"}"#;
    let parsed = parse_llm_response(json).unwrap();
    assert_eq!(parsed["headword"], "fox");
    assert_eq!(parsed["part_of_speech"], "noun");
  }

  #[test]
  fn test_parse_llm_response_accepts_json_in_markdown() {
    let markdown = "```json\n{\"headword\":\"fox\",\"part_of_speech\":\"noun\"}\n```";
    let parsed = parse_llm_response(markdown).unwrap();
    assert_eq!(parsed["headword"], "fox");
  }

  #[test]
  fn test_validate_translation_structure_accepts_word_basic() {
    let json = serde_json::json!({
      "headword": "fox",
      "part_of_speech": "noun",
      "phonetic": "/fɒks/",
      "translations": ["zorro"],
      "synonyms": [],
      "antonyms": [],
      "examples": []
    });
    assert!(
      validate_translation_structure(&json, &InputType::Word, &TranslationMode::Basic).is_ok()
    );
  }

  #[test]
  fn test_validate_translation_structure_rejects_missing_field() {
    let json = serde_json::json!({
      "headword": "fox",
      "part_of_speech": "noun"
    });
    assert!(
      validate_translation_structure(&json, &InputType::Word, &TranslationMode::Basic).is_err()
    );
  }

  #[test]
  fn test_validate_translation_structure_accepts_sentence_basic() {
    let json = serde_json::json!({
      "tone": "neutral",
      "rephrasing": "El zorro es astuto"
    });
    assert!(
      validate_translation_structure(&json, &InputType::Sentence, &TranslationMode::Basic).is_ok()
    );
  }

  #[test]
  fn test_validate_translation_structure_accepts_paragraph_basic() {
    let json = serde_json::json!({
      "text": "The fox is clever.",
      "translation": "El zorro es astuto."
    });
    assert!(
      validate_translation_structure(&json, &InputType::Paragraph, &TranslationMode::Basic).is_ok()
    );
  }

  #[test]
  fn test_validate_translation_structure_rejects_unsupported_combination() {
    let json = serde_json::json!({"translation": "test"});
    assert!(validate_translation_structure(
      &json,
      &InputType::Paragraph,
      &TranslationMode::Explain
    )
    .is_err());
  }
}
