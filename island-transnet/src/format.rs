use anyhow::{anyhow, Context, Result};
use serde_json::Value;

/// Parse LLM response content and extract JSON
/// Handles responses that may include markdown code fences
pub fn parse_llm_response(content: &str) -> Result<Value> {
  let trimmed = content.trim();

  // Try parsing as direct JSON first
  if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
    return Ok(parsed);
  }

  // Try to extract JSON from markdown code blocks
  let json_str = extract_json_from_markdown(trimmed)?;

  serde_json::from_str(&json_str)
    .context("failed to parse model JSON payload")
}

/// Extract JSON content from markdown code blocks
/// Handles ```json, ```, and plain JSON extraction
fn extract_json_from_markdown(content: &str) -> Result<String> {
  // Look for JSON in code blocks
  if content.starts_with("```") {
    let content = content.trim_start_matches("```");
    let content = content.trim_start_matches("json");
    let content = content.trim();

    if let Some(end_idx) = content.find("\n```") {
      return Ok(content[..end_idx].trim().to_string());
    }

    if let Some(end_idx) = content.find("```") {
      return Ok(content[..end_idx].trim().to_string());
    }

    // If no closing fence found, try to parse the entire content
    return Ok(content.trim().to_string());
  }

  // Fallback: try to find first { and last }
  let start = content
    .find('{')
    .ok_or_else(|| anyhow!("response did not contain JSON"))?;

  let end = content
    .rfind('}')
    .ok_or_else(|| anyhow!("response did not contain closing JSON"))?;

  if end <= start {
    return Err(anyhow!("invalid JSON structure: closing brace before opening brace"));
  }

  Ok(content[start..=end].trim().to_string())
}

/// Validate that parsed JSON matches expected structure for given input type and mode
/// Returns validated JSON or an error if validation fails
pub fn validate_translation_structure(
  json: &Value,
  input_type: &crate::types::InputType,
  mode: &crate::types::TranslationMode,
) -> Result<()> {
  match (input_type, mode) {
    (crate::types::InputType::Word, crate::types::TranslationMode::Basic) => {
      validate_word_basic(json)?;
    }
    (crate::types::InputType::Word, crate::types::TranslationMode::Explain) => {
      validate_word_explain(json)?;
    }
    (crate::types::InputType::Word, crate::types::TranslationMode::FullAnalysis) => {
      validate_word_full_analysis(json)?;
    }

    (crate::types::InputType::Phrase, crate::types::TranslationMode::Basic) => {
      validate_phrase_basic(json)?;
    }
    (crate::types::InputType::Phrase, crate::types::TranslationMode::Explain) => {
      validate_phrase_explain(json)?;
    }
    (crate::types::InputType::Phrase, crate::types::TranslationMode::FullAnalysis) => {
      validate_phrase_full_analysis(json)?;
    }

    (crate::types::InputType::Sentence, crate::types::TranslationMode::Basic) => {
      validate_sentence_basic(json)?;
    }
    (crate::types::InputType::Sentence, crate::types::TranslationMode::Explain) => {
      validate_sentence_explain(json)?;
    }

    (crate::types::InputType::Paragraph | crate::types::InputType::Essay, crate::types::TranslationMode::Basic) => {
      validate_paragraph_essay_basic(json)?;
    }

    (crate::types::InputType::Sentence, crate::types::TranslationMode::FullAnalysis)
    | (crate::types::InputType::Paragraph, crate::types::TranslationMode::Explain | crate::types::TranslationMode::FullAnalysis)
    | (crate::types::InputType::Essay, crate::types::TranslationMode::Explain | crate::types::TranslationMode::FullAnalysis) => {
      return Err(anyhow!(
        "unsupported combination: input_type={:?}, mode={:?}",
        input_type, mode
      ));
    }

    _ => {
      // For Auto or other combinations, just check that it's a valid JSON object
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

  let explain = json.get("explain").and_then(|v| v.as_object())
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

  let relationships = json.get("relationships").and_then(|v| v.as_object())
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

  let explain = json.get("explain").and_then(|v| v.as_object())
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

  let relationships = json.get("relationships").and_then(|v| v.as_object())
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

  let explain = json.get("explain").and_then(|v| v.as_object())
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

// Helper functions for validation

fn ensure_string_field(json: &Value, field: &str) -> Result<()> {
  match json.get(field) {
    Some(value) if value.is_string() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be a string, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

fn ensure_string_field_in_obj(obj: &serde_json::Map<String, Value>, field: &str) -> Result<()> {
  match obj.get(field) {
    Some(value) if value.is_string() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be a string, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

fn ensure_array_field(json: &Value, field: &str) -> Result<()> {
  match json.get(field) {
    Some(value) if value.is_array() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be an array, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

fn ensure_array_field_in_obj(obj: &serde_json::Map<String, Value>, field: &str) -> Result<()> {
  match obj.get(field) {
    Some(value) if value.is_array() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be an array, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

fn ensure_object_field(json: &Value, field: &str) -> Result<()> {
  match json.get(field) {
    Some(value) if value.is_object() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be an object, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

fn ensure_object_field_in_obj(obj: &serde_json::Map<String, Value>, field: &str) -> Result<()> {
  match obj.get(field) {
    Some(value) if value.is_object() => Ok(()),
    Some(value) => Err(anyhow!("field '{}' must be an object, got {}", field, value)),
    None => Err(anyhow!("missing required field '{}'", field)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{InputType, TranslationMode};

  #[test]
  fn parses_raw_json() {
    let json = r#"{"headword":"fox","part_of_speech":"noun"}"#;
    let parsed = parse_llm_response(json).unwrap();
    assert_eq!(parsed["headword"], "fox");
    assert_eq!(parsed["part_of_speech"], "noun");
  }

  #[test]
  fn parses_json_in_markdown() {
    let markdown = "```json\n{\"headword\":\"fox\",\"part_of_speech\":\"noun\"}\n```";
    let parsed = parse_llm_response(markdown).unwrap();
    assert_eq!(parsed["headword"], "fox");
  }

  #[test]
  fn validates_word_basic_structure() {
    let json = serde_json::json!({
      "headword": "fox",
      "part_of_speech": "noun",
      "phonetic": "/fɒks/",
      "translations": ["zorro"],
      "synonyms": [],
      "antonyms": [],
      "examples": []
    });
    assert!(validate_translation_structure(&json, &InputType::Word, &TranslationMode::Basic).is_ok());
  }

  #[test]
  fn rejects_missing_required_field() {
    let json = serde_json::json!({
      "headword": "fox",
      "part_of_speech": "noun"
    });
    assert!(validate_translation_structure(&json, &InputType::Word, &TranslationMode::Basic).is_err());
  }

  #[test]
  fn validates_sentence_basic_structure() {
    let json = serde_json::json!({
      "tone": "neutral",
      "rephrasing": "El zorro es astuto"
    });
    assert!(validate_translation_structure(&json, &InputType::Sentence, &TranslationMode::Basic).is_ok());
  }

  #[test]
  fn validates_paragraph_basic_structure() {
    let json = serde_json::json!({
      "text": "The fox is clever.",
      "translation": "El zorro es astuto."
    });
    assert!(validate_translation_structure(&json, &InputType::Paragraph, &TranslationMode::Basic).is_ok());
  }

  #[test]
  fn rejects_unsupported_combination() {
    let json = serde_json::json!({"translation": "test"});
    assert!(validate_translation_structure(&json, &InputType::Paragraph, &TranslationMode::Explain).is_err());
  }
}
