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
  assert_eq!(fixture.license, "CC0-1.0");
  assert!(!fixture.provenance.trim().is_empty());

  let required = BTreeSet::from([
    "routing",
    "translation_fidelity",
    "terminology",
    "formatting",
    "sense_resolution",
    "domain_resolution",
    "relationship_semantics",
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
    assert!(case.id.bytes().all(|byte| {
      byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
    }));
    assert!(matches!(case.source_language.as_str(), "en" | "zh-CN"));
    assert!(matches!(case.target_language.as_str(), "en" | "zh-CN"));
    assert_ne!(case.source_language, case.target_language);
    assert!(!case.input.trim().is_empty() && case.input.len() <= 512);
    assert!(!case.expected_codes.is_empty());
    assert!(case.expected_codes.iter().all(|code| {
      !code.is_empty()
        && code.len() <= 64
        && code
          .bytes()
          .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    }));
  }
}
