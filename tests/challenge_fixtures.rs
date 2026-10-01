//! Contract checks for the licensed, versioned target challenge-set foundation.

use std::collections::BTreeSet;

use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/target_challenges_v1.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengeSet {
  schema_version: String,
  license: String,
  provenance: String,
  cases: Vec<ChallengeCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengeCase {
  id: String,
  category: String,
  source_language: String,
  target_language: String,
  input: String,
  expected_codes: Vec<String>,
}

#[test]
fn target_challenge_fixture_is_versioned_licensed_and_complete_by_category() {
  let fixture: ChallengeSet = serde_json::from_str(FIXTURE).unwrap();
  assert_eq!(fixture.schema_version, "transnet-challenges-v1");
  assert_eq!(fixture.license, "Apache-2.0");
  assert_eq!(fixture.provenance, "synthetic");
  assert!(FIXTURE.len() <= 65_536);
  assert!(!fixture.cases.is_empty() && fixture.cases.len() <= 64);

  let required = BTreeSet::from([
    "routing",
    "translation_fidelity",
    "terminology",
    "register",
    "formatting",
    "sense_resolution",
    "concept_resolution",
    "domain_resolution",
    "domain_assessment",
    "relationship_semantics",
    "relationship_selection",
    "path_validity",
    "omission",
    "fabrication",
    "culture",
    "prompt_injection",
  ]);
  let actual = fixture
    .cases
    .iter()
    .map(|case| case.category.as_str())
    .collect::<BTreeSet<_>>();
  assert_eq!(actual, required);
}

#[test]
fn target_challenges_have_unique_safe_ids_and_bounded_synthetic_content() {
  let fixture: ChallengeSet = serde_json::from_str(FIXTURE).unwrap();
  let mut ids = BTreeSet::new();
  for case in fixture.cases {
    assert!(ids.insert(case.id.clone()));
    assert!(safe_case_id(&case.id));
    assert!(matches!(case.source_language.as_str(), "en" | "zh-CN"));
    assert!(matches!(case.target_language.as_str(), "en" | "zh-CN"));
    assert_ne!(case.source_language, case.target_language);
    assert!(!case.input.trim().is_empty() && case.input.len() <= 512);
    assert!(!case.expected_codes.is_empty() && case.expected_codes.len() <= 8);
    let unique_codes = case.expected_codes.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique_codes.len(), case.expected_codes.len());
    assert!(case
      .expected_codes
      .iter()
      .all(|code| safe_expected_code(code)));
  }
}

fn safe_case_id(value: &str) -> bool {
  (3..=64).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
    && value.as_bytes().last().is_some_and(u8::is_ascii_digit)
    && !value.contains("--")
}

fn safe_expected_code(value: &str) -> bool {
  (1..=64).contains(&value.len())
    && value.split('_').all(|segment| {
      !segment.is_empty()
        && segment
          .as_bytes()
          .first()
          .is_some_and(u8::is_ascii_lowercase)
        && segment
          .bytes()
          .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

#[test]
fn challenge_identifiers_reject_empty_separator_and_duplicate_shapes() {
  for invalid in ["", "---", "-case-001", "case--001", "case-"] {
    assert!(!safe_case_id(invalid));
  }
  assert!(!safe_case_id(&format!("case-{}", "1".repeat(65))));
  for invalid in ["", "_", "__code", "code_", "two__parts", "Upper"] {
    assert!(!safe_expected_code(invalid));
  }
  assert!(safe_case_id("routing-word-001"));
  assert!(safe_expected_code("preserve_lexical_ambiguity"));
}
