//! Contract tests for the outbound island-port canonical-read adapter.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use async_trait::async_trait;
use serde_json::{json, Value};
use transnet::{
  adapters::island_port::{
    BasicCardResolveInput, IslandPortCallContext, IslandPortCanonicalClient, IslandPortClientError,
    IslandPortTransport, LookupFormInput, SenseGetInput, TranslationResolveInput,
  },
  domain::{
    canonical::{CanonicalId, EvidenceUse, LanguageTag},
    canonical_translation::{SourceFingerprint, SOURCE_FINGERPRINT_VERSION},
    retrieval::LexicalMatchKind,
  },
};

struct FakeTransport {
  response: Vec<u8>,
  request: Mutex<Option<(&'static str, Value, Duration)>>,
}

impl FakeTransport {
  fn new(response: Value) -> Self {
    Self {
      response: serde_json::to_vec(&response).unwrap(),
      request: Mutex::new(None),
    }
  }
}

#[async_trait]
impl IslandPortTransport for FakeTransport {
  async fn post_json(
    &self,
    path: &'static str,
    body: Vec<u8>,
    timeout: Duration,
  ) -> Result<Vec<u8>, IslandPortClientError> {
    *self.request.lock().unwrap() = Some((path, serde_json::from_slice(&body).unwrap(), timeout));
    Ok(self.response.clone())
  }
}

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn language(value: &str) -> LanguageTag {
  LanguageTag::parse(value).unwrap()
}

fn context() -> IslandPortCallContext {
  IslandPortCallContext::new(
    "req_stage_3",
    "2099-09-18T12:00:00.000000Z",
    Duration::from_secs(2),
  )
  .unwrap()
}

#[tokio::test]
async fn active_release_has_no_pin_in_request_and_maps_strict_authoritative_value() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "mysql-adapter-v1",
    "outcome": "ok",
    "value": {
      "content_release": "knowledge-2026-09",
      "canonical_schema_version": "canonical-v1"
    }
  })));
  let pin = IslandPortCanonicalClient::new(transport.clone())
    .active_release(&context())
    .await
    .unwrap()
    .unwrap();
  assert_eq!(pin.release_id.as_str(), "knowledge-2026-09");
  assert_eq!(pin.canonical_schema_version, "canonical-v1");
  let request = transport.request.lock().unwrap();
  let (path, body, timeout) = request.as_ref().unwrap();
  assert_eq!(*path, "/api/v1/releases/active");
  assert_eq!(body["context"]["request_id"], "req_stage_3");
  assert_eq!(body["context"]["schema_version"], "mysql-adapter-v1");
  assert!(body["context"].get("content_release").is_none());
  assert_eq!(body["input"], json!({}));
  assert_eq!(*timeout, Duration::from_secs(2));
}

#[tokio::test]
async fn active_release_missing_or_incompatible_response_fails_closed() {
  let cases = [
    (
      json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","outcome":"not_found","error":{"code":"no_active_release","message":"No active release.","retryable":false}}),
      None,
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","outcome":"version_mismatch","error":{"code":"schema_incompatible","message":"Incompatible schema.","retryable":false}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"old","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"other","schema_version":"mysql-adapter-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","outcome":"ok","value":{"content_release":"release-1"}}),
      Some(IslandPortClientError::InconsistentData),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1","vector_collection_id":"fake"}}),
      Some(IslandPortClientError::InconsistentData),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"},"content_release":"release-1"}),
      Some(IslandPortClientError::InconsistentData),
    ),
  ];
  for (response, expected_error) in cases {
    let result = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .active_release(&context())
      .await;
    match expected_error {
      Some(error) => assert_eq!(result.unwrap_err(), error),
      None => assert_eq!(result.unwrap(), None),
    }
  }
}

#[tokio::test]
async fn expired_active_release_deadline_stops_before_transport() {
  let transport = Arc::new(FakeTransport::new(json!({})));
  let expired = IslandPortCallContext::new(
    "req_stage_3",
    "2000-01-01T00:00:00Z",
    Duration::from_secs(2),
  )
  .unwrap();
  let error = IslandPortCanonicalClient::new(transport.clone())
    .active_release(&expired)
    .await
    .unwrap_err();
  assert_eq!(error, IslandPortClientError::Timeout);
  assert!(transport.request.lock().unwrap().is_none());
}

#[tokio::test]
async fn active_release_errors_do_not_expose_peer_detail_or_credentials() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id":"req_stage_3",
    "schema_version":"mysql-adapter-v1",
    "outcome":"unavailable",
    "error":{"code":"internal_failure","message":"sensitive-source credential-secret","retryable":true}
  })));
  let error = IslandPortCanonicalClient::new(transport)
    .active_release(&context())
    .await
    .unwrap_err();
  assert_eq!(error, IslandPortClientError::Unavailable);
  assert!(!format!("{error:?} {error}").contains("sensitive-source"));
  assert!(!format!("{error:?} {error}").contains("credential-secret"));
}

#[tokio::test]
async fn translation_response_reconstructs_revision_and_propagates_context() {
  let fingerprint = SourceFingerprint::compute("sweltering", &language("en"));
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "mysql-adapter-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {"matches": [{
      "translation_id": "tr_sweltering_zh_01",
      "revision": 3,
      "unit": "word",
      "source_fingerprint": fingerprint.to_storage_key(),
      "source": {"text": "sweltering", "language": "en"},
      "target": {"text": "酷热的", "language": "zh-CN"},
      "scope": {
        "lexeme_id": "lexeme_sweltering",
        "sense_id": "sense_sweltering_hot",
        "part_of_speech": "adjective",
        "composition": "compositional",
        "domain_ids": ["domain_weather"]
      },
      "evidence_ids": ["evidence_dictionary_1042"]
    }]}
  })));
  let client = IslandPortCanonicalClient::new(transport.clone());
  let revisions = client
    .resolve_translations(
      &context(),
      &id("knowledge-2026-09"),
      TranslationResolveInput {
        source_fingerprint: fingerprint,
        source_language: language("en"),
        target_language: language("zh-CN"),
        sense_id: Some(id("sense_sweltering_hot")),
        domain_ids: vec![
          transnet::domain::canonical_translation::DomainId::new("domain_weather").unwrap(),
        ],
        dialect: None,
        register: None,
        limit: 5,
      },
    )
    .await
    .unwrap();

  assert_eq!(revisions.len(), 1);
  assert!(revisions[0].verifies_source("sweltering"));
  let request = transport.request.lock().unwrap();
  let (path, body, timeout) = request.as_ref().unwrap();
  assert_eq!(*path, "/api/v1/translations/resolve");
  assert_eq!(body["context"]["request_id"], "req_stage_3");
  assert_eq!(body["context"]["schema_version"], "mysql-adapter-v1");
  assert_eq!(body["context"]["content_release"], "knowledge-2026-09");
  assert_eq!(
    body["input"]["normalizer_version"],
    SOURCE_FINGERPRINT_VERSION
  );
  assert_eq!(*timeout, Duration::from_secs(2));
}

#[tokio::test]
async fn incompatible_or_incomplete_translation_response_fails_closed() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "old-adapter-v0",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {"matches": []}
  })));
  let error = IslandPortCanonicalClient::new(transport)
    .resolve_translations(
      &context(),
      &id("knowledge-2026-09"),
      TranslationResolveInput {
        source_fingerprint: SourceFingerprint::compute("safe", &language("en")),
        source_language: language("en"),
        target_language: language("zh-CN"),
        sense_id: None,
        domain_ids: Vec::new(),
        dialect: None,
        register: None,
        limit: 1,
      },
    )
    .await
    .unwrap_err();
  assert_eq!(error, IslandPortClientError::SchemaIncompatible);
}

#[tokio::test]
async fn candidate_response_maps_authoritative_data_without_accepting_rank() {
  let transport = Arc::new(FakeTransport::new(candidate_response()));
  let matches = IslandPortCanonicalClient::new(transport)
    .resolve_basic_card_candidates(&context(), &id("knowledge-2026-09"), candidate_input())
    .await
    .unwrap();
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0].kind, LexicalMatchKind::ExactCanonical);
  assert_eq!(matches[0].score.basis_points(), 10_000);
  assert_eq!(
    matches[0].candidate.sources[0].attribution.as_deref(),
    Some("Reviewed dictionary attribution")
  );
}

fn candidate_response() -> Value {
  json!({
    "request_id": "req_stage_3",
    "schema_version": "mysql-adapter-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "matches": [{
        "matched_form_id": "form_sweltering",
        "matched_form": "sweltering",
        "match_class": "exact_canonical",
        "lexical_score_basis_points": 10000,
        "candidate": {
          "lexeme": {"id":"lexeme_sweltering","language":"en","lemma":"sweltering","normalized_lemma":"sweltering","part_of_speech":"adjective","status":"active"},
          "sense": {"id":"sense_sweltering_hot","lexeme_id":"lexeme_sweltering","sense_key":"weather-hot","definition":"uncomfortably hot","definition_evidence_ids":["evidence_dictionary_1042"],"status":"active"},
          "forms": [{"id":"form_sweltering","lexeme_id":"lexeme_sweltering","form":"sweltering","normalized_form":"sweltering","kind":"lemma","morphology":null,"evidence_ids":["evidence_dictionary_1042"],"status":"active"}],
          "sources": [{"release_id":"knowledge-2026-09","source":{"id":"source_dictionary","name":"Reviewed dictionary","version":"2026-09","license":"internal-reviewed","attribution":"Reviewed dictionary attribution","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true}}}],
          "evidence": [{"id":"evidence_dictionary_1042","source_id":"source_dictionary","source_reference":"entry:1","language":"en","kind":"definition","confidence":"high","text":"uncomfortably hot","content_hash":"sha256:abc","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"}]
        }
      }],
      "alternatives": [],
      "truncated": false
    }
  })
}

fn candidate_input() -> BasicCardResolveInput {
  BasicCardResolveInput {
    lookup_forms: vec![LookupFormInput {
      form: "sweltering".into(),
      match_class: LexicalMatchKind::ExactCanonical,
      rank: 0,
    }],
    normalizer_version: "unicode-nfkc-v2".into(),
    source_language: language("en"),
    explanation_language: language("zh-CN"),
    dialect: None,
    evidence_use: EvidenceUse::ApiRedistribution,
    limit: 5,
  }
}

#[tokio::test]
async fn candidate_attribution_and_source_policy_fail_closed() {
  let mut cases = Vec::new();
  let source = "/value/matches/0/candidate/sources/0";
  let evidence = "/value/matches/0/candidate/evidence/0";
  let mut missing = candidate_response();
  missing
    .pointer_mut(&format!("{source}/source"))
    .unwrap()
    .as_object_mut()
    .unwrap()
    .remove("attribution");
  cases.push(missing);
  let mut empty = candidate_response();
  *empty
    .pointer_mut(&format!("{source}/source/attribution"))
    .unwrap() = json!("  ");
  cases.push(empty);
  let mut conflict = candidate_response();
  *conflict
    .pointer_mut(&format!("{source}/source/id"))
    .unwrap() = json!("another_source");
  cases.push(conflict);
  let mut wrong_release = candidate_response();
  *wrong_release
    .pointer_mut(&format!("{source}/release_id"))
    .unwrap() = json!("release-r2");
  cases.push(wrong_release);
  let mut permission = candidate_response();
  *permission
    .pointer_mut(&format!("{source}/source/permissions/api_redistribution"))
    .unwrap() = json!(false);
  cases.push(permission);
  let mut evidence_permission = candidate_response();
  *evidence_permission
    .pointer_mut(&format!("{evidence}/permissions/api_redistribution"))
    .unwrap() = json!(false);
  cases.push(evidence_permission);
  let mut duplicate = candidate_response();
  let copied = duplicate.pointer(source).unwrap().clone();
  duplicate
    .pointer_mut("/value/matches/0/candidate/sources")
    .unwrap()
    .as_array_mut()
    .unwrap()
    .push(copied);
  cases.push(duplicate);
  for response in cases {
    let error = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .resolve_basic_card_candidates(&context(), &id("knowledge-2026-09"), candidate_input())
      .await
      .unwrap_err();
    assert_eq!(error, IslandPortClientError::InconsistentData);
    let output = format!("{error:?} {error}");
    assert!(!output.contains("Reviewed dictionary attribution"));
    assert!(!output.contains("uncomfortably hot"));
  }
}

#[tokio::test]
async fn release_unavailable_and_schema_incompatible_are_distinct_closed_outcomes() {
  for (outcome, code, expected) in [
    (
      "content_release_unavailable",
      "content_release_unavailable",
      IslandPortClientError::ContentReleaseUnavailable,
    ),
    (
      "version_mismatch",
      "schema_incompatible",
      IslandPortClientError::SchemaIncompatible,
    ),
  ] {
    let response = json!({
      "request_id":"req_stage_3", "schema_version":"mysql-adapter-v1",
      "content_release":"release-r1", "outcome":outcome,
      "error":{"code":code,"message":"secret response body and socket path","retryable":false}
    });
    let error = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .get_sense(
        &context(),
        &transnet::domain::canonical::CanonicalReleasePin::new(
          id("release-r1"),
          "canonical-v1".into(),
        )
        .unwrap(),
        SenseGetInput {
          sense_id: id("sense_sweltering_hot"),
          explanation_language: language("zh-CN"),
          dialect: None,
          evidence_use: EvidenceUse::ApiRedistribution,
        },
      )
      .await
      .unwrap_err();
    assert_eq!(error, expected);
    assert!(!format!("{error:?} {error}").contains("secret"));
  }
  let response = json!({"request_id":"req_stage_3","schema_version":"mysql-adapter-v1","content_release":"release-r1","outcome":"content_release_unavailable","error":{"code":"schema_incompatible","message":"secret","retryable":false}});
  let error = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
    .get_sense(
      &context(),
      &transnet::domain::canonical::CanonicalReleasePin::new(
        id("release-r1"),
        "canonical-v1".into(),
      )
      .unwrap(),
      SenseGetInput {
        sense_id: id("sense_sweltering_hot"),
        explanation_language: language("zh-CN"),
        dialect: None,
        evidence_use: EvidenceUse::ApiRedistribution,
      },
    )
    .await
    .unwrap_err();
  assert_eq!(error, IslandPortClientError::InconsistentData);
}

#[tokio::test]
async fn pinned_sense_rejects_canonical_schema_mismatch() {
  let response = json!({
    "request_id":"req_stage_3", "schema_version":"mysql-adapter-v1", "outcome":"ok",
    "content_release":"release-r1",
    "value": {"canonical_schema_version":"canonical-v2", "target":{
      "lexeme":{"id":"lexeme_x","language":"en","lemma":"x","normalized_lemma":"x","part_of_speech":"noun","status":"active"},
      "sense":{"id":"sense_x","lexeme_id":"lexeme_x","sense_key":"one","definition":"x","definition_evidence_ids":[],"status":"active"}
    }, "lineages":{},
      "localized_glosses":[],"pronunciations":[],"usage_labels":[],"grammar_patterns":[],
      "collocations":[],"examples":[],"pitfalls":[],"etymologies":[],"history":[]}
  });
  let mut missing = response.clone();
  missing["value"]
    .as_object_mut()
    .unwrap()
    .remove("canonical_schema_version");
  for response in [response, missing] {
    let error = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .get_sense(
        &context(),
        &transnet::domain::canonical::CanonicalReleasePin::new(
          id("release-r1"),
          "canonical-v1".into(),
        )
        .unwrap(),
        SenseGetInput {
          sense_id: id("sense_x"),
          explanation_language: language("zh-CN"),
          dialect: None,
          evidence_use: EvidenceUse::ApiRedistribution,
        },
      )
      .await
      .unwrap_err();
    assert_eq!(error, IslandPortClientError::SchemaIncompatible);
  }
}

#[tokio::test]
async fn sense_response_constructs_target_independently() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "mysql-adapter-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "canonical_schema_version": "canonical-v1",
      "target": {
        "lexeme": {"id":"lexeme_sweltering","language":"en","lemma":"sweltering","normalized_lemma":"sweltering","part_of_speech":"adjective","status":"active"},
        "sense": {"id":"sense_sweltering_hot","lexeme_id":"lexeme_sweltering","sense_key":"weather-hot","definition":"uncomfortably hot","definition_evidence_ids":[],"status":"active"}
      },
      "lineages": {
        "evidence_gloss": {
          "source": {"id":"source_dictionary","name":"Reviewed dictionary","version":"2026-09","license":"internal-reviewed","attribution":null,"permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true}},
          "fragment": {"id":"evidence_gloss","source_id":"source_dictionary","source_reference":"entry:gloss","language":"zh-CN","kind":"localized_gloss","confidence":"high","text":"酷热的","content_hash":"sha256:gloss","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"},
          "origin": {"kind":"licensed_source"}
        }
      },
      "localized_glosses": [{"id":"gloss_sweltering_zh","language":"zh-CN","assertion":{"text":"酷热的","status":"active","evidence_ids":["evidence_gloss"]}}], "pronunciations": [], "usage_labels": [],
      "grammar_patterns": [], "collocations": [], "examples": [], "pitfalls": [],
      "etymologies": [], "history": []
    }
  })));
  let details = IslandPortCanonicalClient::new(transport)
    .get_sense(
      &context(),
      &transnet::domain::canonical::CanonicalReleasePin::new(
        id("knowledge-2026-09"),
        "canonical-v1".into(),
      )
      .unwrap(),
      SenseGetInput {
        sense_id: id("sense_sweltering_hot"),
        explanation_language: language("zh-CN"),
        dialect: None,
        evidence_use: EvidenceUse::ApiRedistribution,
      },
    )
    .await
    .unwrap();
  assert_eq!(details.target().sense_id().as_str(), "sense_sweltering_hot");
  assert_eq!(details.localized_glosses().len(), 1);
}

#[tokio::test]
async fn dangling_lineage_reference_and_unknown_fields_fail_closed() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3", "schema_version": "mysql-adapter-v1", "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "canonical_schema_version": "canonical-v1",
      "target": {
        "lexeme": {"id":"lexeme_x","language":"en","lemma":"x","normalized_lemma":"x","part_of_speech":"noun","status":"active"},
        "sense": {"id":"sense_x","lexeme_id":"lexeme_x","sense_key":"one","definition":"x","definition_evidence_ids":[],"status":"active"}
      },
      "lineages": {},
      "localized_glosses": [{"id":"gloss_x","language":"zh-CN","assertion":{"text":"某物","status":"active","evidence_ids":["missing"]}}],
      "pronunciations": [], "usage_labels": [], "grammar_patterns": [], "collocations": [],
      "examples": [], "pitfalls": [], "etymologies": [], "history": []
    }
  })));
  let error = IslandPortCanonicalClient::new(transport)
    .get_sense(
      &context(),
      &transnet::domain::canonical::CanonicalReleasePin::new(
        id("knowledge-2026-09"),
        "canonical-v1".into(),
      )
      .unwrap(),
      SenseGetInput {
        sense_id: id("sense_x"),
        explanation_language: language("zh-CN"),
        dialect: None,
        evidence_use: EvidenceUse::ApiRedistribution,
      },
    )
    .await
    .unwrap_err();
  assert_eq!(error, IslandPortClientError::InconsistentData);
  assert!(!format!("{error:?}").contains("某物"));
}
