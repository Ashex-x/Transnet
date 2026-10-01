//! Grammar, licensing, provenance, and executable-evidence checks for the target matrix.

use std::{collections::BTreeSet, fs, path::Path};

use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/target_execution_matrix_v1.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionMatrix {
  schema_version: String,
  license: FixtureLicense,
  provenance: FixtureProvenance,
  cases: Vec<ExecutionCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureLicense {
  spdx: String,
  applies_to: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureProvenance {
  kind: String,
  reviewed: bool,
  contains_production_data: bool,
}

#[derive(Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum SuiteKind {
  Success,
  Failure,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionCase {
  id: String,
  category: String,
  suite: SuiteKind,
  seed: String,
  expected_codes: Vec<String>,
  evidence: Vec<ExecutableEvidence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutableEvidence {
  path: String,
  test: String,
}

#[test]
fn execution_matrix_is_closed_licensed_reviewed_and_synthetic() {
  let matrix = parse();
  assert_eq!(matrix.schema_version, "transnet-execution-matrix-v1");
  assert_eq!(matrix.license.spdx, "Apache-2.0");
  assert_eq!(matrix.license.applies_to, "fixture_and_expected_outcomes");
  assert!(valid_provenance(&matrix.provenance));
  assert!(FIXTURE.len() <= 65_536);
  assert!((20..=48).contains(&matrix.cases.len()));

  let required = BTreeSet::from([
    "segments",
    "image_regions",
    "guidance",
    "live_citations",
    "alternatives",
    "relationship_page",
    "canonical_failure",
    "retrieval_failure",
    "live_failure",
    "model_failure",
    "stale_release",
    "partial_publication",
    "rate_limit",
    "timeout",
    "cancellation",
    "rollback",
    "observability_drop",
    "uds_lifecycle",
    "non_persistence",
    "privacy",
    "prompt_injection",
  ]);
  let actual = matrix
    .cases
    .iter()
    .map(|case| case.category.as_str())
    .collect::<BTreeSet<_>>();
  assert_eq!(actual, required);
}

#[test]
fn execution_cases_have_bounded_closed_grammar() {
  let matrix = parse();
  let mut ids = BTreeSet::new();
  for case in matrix.cases {
    assert!(ids.insert(case.id.clone()));
    assert!(safe_kebab(&case.id, 64));
    assert!(safe_snake(&case.category, 64));
    assert!(!case.seed.trim().is_empty() && case.seed.chars().count() <= 256);
    assert!((1..=8).contains(&case.expected_codes.len()));
    assert!((1..=4).contains(&case.evidence.len()));
    assert_eq!(
      case.expected_codes.iter().collect::<BTreeSet<_>>().len(),
      case.expected_codes.len()
    );
    assert!(case
      .expected_codes
      .iter()
      .all(|value| safe_snake(value, 64)));
    match case.category.as_str() {
      "segments" | "image_regions" | "guidance" | "live_citations" | "alternatives"
      | "relationship_page" => assert!(case.suite == SuiteKind::Success),
      _ => assert!(case.suite == SuiteKind::Failure),
    }
  }
}

#[test]
fn every_status_claim_names_a_checked_in_executable_test() {
  let matrix = parse();
  let root = Path::new(env!("CARGO_MANIFEST_DIR"));
  let mut evidence = BTreeSet::new();
  for case in matrix.cases {
    let case_id = case.id;
    for item in case.evidence {
      assert!(item.path.starts_with("tests/") || item.path.starts_with("src/"));
      assert!(!item.path.contains("..") && !item.path.starts_with('/'));
      assert!(safe_snake(&item.test, 128));
      assert!(evidence.insert((case_id.clone(), item.path.clone(), item.test.clone())));
      let source = fs::read_to_string(root.join(&item.path)).unwrap();
      let declaration = format!("fn {}(", item.test);
      let position = source
        .find(&declaration)
        .unwrap_or_else(|| panic!("missing executable evidence {}::{}", item.path, item.test));
      let prefix = source[..position]
        .lines()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .join("\n");
      assert!(
        prefix.contains("#[test]") || prefix.contains("#[tokio::test"),
        "evidence is not a test {}::{}",
        item.path,
        item.test
      );
    }
  }
}

#[test]
fn matrix_rejects_unknown_fields_and_unreviewed_or_non_synthetic_provenance() {
  let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
  value["unexpected"] = serde_json::json!(true);
  assert!(serde_json::from_value::<ExecutionMatrix>(value).is_err());

  let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
  value["provenance"]["reviewed"] = serde_json::json!(false);
  let matrix: ExecutionMatrix = serde_json::from_value(value).unwrap();
  assert!(!valid_provenance(&matrix.provenance));

  let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
  value["provenance"]["contains_production_data"] = serde_json::json!(true);
  let matrix: ExecutionMatrix = serde_json::from_value(value).unwrap();
  assert!(!valid_provenance(&matrix.provenance));
}

#[test]
fn identifier_grammar_rejects_traversal_case_and_empty_segments() {
  for invalid in [
    "../tests/api.rs",
    "/tests/api.rs",
    "Tests/api",
    "two--parts",
  ] {
    assert!(!safe_kebab(invalid, 64));
  }
  for invalid in ["", "Upper", "two__parts", "ends_"] {
    assert!(!safe_snake(invalid, 64));
  }
}

fn parse() -> ExecutionMatrix {
  serde_json::from_str(FIXTURE).unwrap()
}

fn valid_provenance(value: &FixtureProvenance) -> bool {
  value.kind == "repository_authored_synthetic" && value.reviewed && !value.contains_production_data
}

fn safe_kebab(value: &str, max: usize) -> bool {
  (3..=max).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
    && value.as_bytes().last().is_some_and(u8::is_ascii_digit)
    && !value.contains("--")
}

fn safe_snake(value: &str, max: usize) -> bool {
  (1..=max).contains(&value.len())
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
