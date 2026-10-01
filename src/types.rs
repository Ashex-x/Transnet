//! Minimal JSON contracts exposed by the translation service.

/// Returns whether `value` has a conservative BCP-47 language-tag shape.
pub fn is_language_code(value: &str) -> bool {
  if value.is_empty() || value.len() > 35 {
    return false;
  }

  let mut subtags = value.split('-');
  let Some(language) = subtags.next() else {
    return false;
  };
  if !(2..=8).contains(&language.len()) || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
  {
    return false;
  }

  subtags.all(|subtag| {
    !subtag.is_empty()
      && subtag.len() <= 8
      && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_language_code_accepts_common_bcp47_shapes() {
    for value in ["en", "zh-CN", "sr-Latn-RS"] {
      assert!(is_language_code(value), "expected {value} to be valid");
    }
  }

  #[test]
  fn test_language_code_rejects_names_and_malformed_tags() {
    for value in ["", "e", "toolongtag", "zh_CN", "en--US", "en-*"] {
      assert!(!is_language_code(value), "expected {value} to be invalid");
    }
  }
}
