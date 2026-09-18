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
  assert_eq!(error, IslandPortClientError::VersionMismatch);
}

#[tokio::test]
async fn candidate_response_maps_authoritative_data_without_accepting_rank() {
  let transport = Arc::new(FakeTransport::new(json!({
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
          "evidence": [{"id":"evidence_dictionary_1042","source_id":"source_dictionary","source_reference":"entry:1","language":"en","kind":"definition","confidence":"high","text":"uncomfortably hot","content_hash":"sha256:abc","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"}]
        }
      }],
      "alternatives": [],
      "truncated": false
    }
  })));
  let matches = IslandPortCanonicalClient::new(transport)
    .resolve_basic_card_candidates(
      &context(),
      &id("knowledge-2026-09"),
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
      },
    )
    .await
    .unwrap();
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0].kind, LexicalMatchKind::ExactCanonical);
  assert_eq!(matches[0].score.basis_points(), 10_000);
}

#[tokio::test]
async fn sense_response_constructs_target_independently() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "mysql-adapter-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
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
      &id("knowledge-2026-09"),
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
      &id("knowledge-2026-09"),
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
