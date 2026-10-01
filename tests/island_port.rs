//! Contract tests for the outbound island-port canonical-read adapter.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use async_trait::async_trait;
use serde_json::{json, Value};
use transnet::{
  adapters::island_port::{
    BasicCardResolveInput, DomainResolveInput, IslandPortCallContext, IslandPortCanonicalClient,
    IslandPortClientError, IslandPortTransport, KnowledgeFactsGetInput, KnowledgeNodesGetInput,
    LookupFormInput, SemanticScalesGetInput, SenseGetInput, TranslationResolveInput,
  },
  domain::{
    canonical::{CanonicalId, EvidenceUse, LanguageTag},
    canonical_translation::{SourceFingerprint, SOURCE_FINGERPRINT_VERSION},
    knowledge_hydration::CanonicalFactRef,
    retrieval::LexicalMatchKind,
    retrieval_data::RetrievalVerificationState,
  },
};

struct FakeTransport {
  response: Vec<u8>,
  request: Mutex<Option<(&'static str, Value, Duration)>>,
}

#[tokio::test]
async fn exact_fact_hydration_validates_wire_echo_revision_and_support() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
    "content_release":"knowledge-2026-09", "value":{"facts":[{
      "fact_id":"fact-1", "revision":2, "statement":"Scorching is hotter than sweltering.",
      "subject_node_id":"node-scorching", "predicate":"higher_degree_than",
      "relation_registry_version":1, "object_node_id":"node-sweltering",
      "domain_ids":["domain_weather"], "applicable_sense_ids":["sense-hot"],
      "conditions":[{"condition_id":"condition-weather","condition_type":"usage_context","parameter_ids":["context-weather"]}],
      "evidence_ids":["evidence-1"], "provenance":["source-1"],
      "verification_state":"verified"
    }]}
  })));
  let facts = IslandPortCanonicalClient::new(transport.clone())
    .get_knowledge_facts(
      &context(),
      &id("knowledge-2026-09"),
      KnowledgeFactsGetInput {
        facts: vec![CanonicalFactRef {
          fact_id: id("fact-1"),
          revision: 2,
        }],
        verification_states: vec![RetrievalVerificationState::Verified],
        limit: 20,
      },
    )
    .await
    .unwrap();
  assert_eq!(facts[0].revision, 2);
  let request = transport.request.lock().unwrap();
  let (path, body, _) = request.as_ref().unwrap();
  assert_eq!(*path, "/api/v1/knowledge-facts/get");
  assert_eq!(body["input"]["content_release"], "knowledge-2026-09");
  assert_eq!(body["input"]["facts"][0]["revision"], 2);
}

#[tokio::test]
async fn fact_hydration_rejects_wrong_revision_and_registry() {
  for (revision, registry) in [(3, 1), (2, 2)] {
    let response = json!({
      "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
      "content_release":"knowledge-2026-09", "value":{"facts":[{
        "fact_id":"fact-1", "revision":revision, "statement":"Reviewed statement.",
        "subject_node_id":"node-a", "predicate":"associated_with",
        "relation_registry_version":registry, "object_node_id":"node-b",
        "domain_ids":[], "applicable_sense_ids":[], "conditions":[],
        "evidence_ids":["evidence-1"], "provenance":["source-1"],
        "verification_state":"verified"
      }]}
    });
    let result = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .get_knowledge_facts(
        &context(),
        &id("knowledge-2026-09"),
        KnowledgeFactsGetInput {
          facts: vec![CanonicalFactRef {
            fact_id: id("fact-1"),
            revision: 2,
          }],
          verification_states: vec![RetrievalVerificationState::Verified],
          limit: 20,
        },
      )
      .await;
    assert_eq!(result, Err(IslandPortClientError::InconsistentData));
  }
}

#[tokio::test]
async fn scale_and_node_hydration_validate_membership_order_and_evidence() {
  let scale_transport = Arc::new(FakeTransport::new(json!({
    "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
    "content_release":"knowledge-2026-09", "value":{"scales":[{
      "scale_id":"scale-heat", "revision":1, "dimension":"heat_intensity",
      "direction":"increasing", "domain_ids":["domain_weather"], "conditions":[],
      "members":[{"node_id":"node-warm","position":10},{"node_id":"node-hot","position":20}],
      "evidence_ids":["evidence-1"], "verification_state":"verified"
    }]}
  })));
  let scales = IslandPortCanonicalClient::new(scale_transport)
    .get_semantic_scales(
      &context(),
      &id("knowledge-2026-09"),
      SemanticScalesGetInput {
        scale_ids: vec![id("scale-heat")],
        for_node_id: id("node-hot"),
        verification_states: vec![RetrievalVerificationState::Verified],
        limit: 5,
      },
    )
    .await
    .unwrap();
  assert_eq!(scales[0].members.len(), 2);

  let node_transport = Arc::new(FakeTransport::new(json!({
    "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
    "content_release":"knowledge-2026-09", "value":{"nodes":[{
      "node_id":"node-hot", "revision":4, "node_type":"concept",
      "canonical_label":"heat", "language":"en", "domain_ids":["domain_weather"],
      "evidence_ids":["evidence-1"], "verification_state":"verified"
    }]}
  })));
  let nodes = IslandPortCanonicalClient::new(node_transport.clone())
    .get_knowledge_nodes(
      &context(),
      &id("knowledge-2026-09"),
      KnowledgeNodesGetInput {
        node_ids: vec![id("node-hot")],
        evidence_use: EvidenceUse::ApiRedistribution,
        limit: 10,
      },
    )
    .await
    .unwrap();
  assert_eq!(nodes[0].canonical_label, "heat");
  let request = node_transport.request.lock().unwrap();
  assert_eq!(request.as_ref().unwrap().0, "/api/v1/knowledge-nodes/get");
}

#[tokio::test]
async fn domain_inventory_is_strict_bounded_and_release_pinned() {
  let transport = Arc::new(FakeTransport::new(json!({
    "request_id": "req_stage_3",
    "schema_version": "canonical-data-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "catalog_complete": true,
      "candidates": [{
        "domain_id": "domain_weather",
        "revision": 4,
        "labels": [
          {"language": "en", "text": "weather"},
          {"language": "zh-CN", "text": "天气"}
        ],
        "aliases": [{"language": "en", "text": "meteorology weather"}],
        "definitions": [
          {"language": "en", "text": "Conditions of the atmosphere."},
          {"language": "zh-CN", "text": "大气状态。"}
        ],
        "inclusion_scope": ["meteorology", "temperature"],
        "exclusion_scope": ["long-term climate"],
        "broader_domain_ids": ["domain_earth_science"],
        "knowledge_profile": {
          "available_fact_families": ["definition", "taxonomy"],
          "languages": ["en", "zh-CN"],
          "verified_fact_count": 184,
          "coverage_state": "partial"
        }
      }]
    }
  })));
  let inventory = IslandPortCanonicalClient::new(transport.clone())
    .resolve_domains(
      &context(),
      &id("knowledge-2026-09"),
      DomainResolveInput {
        normalized_labels: vec!["weather".to_string()],
        scope_key: Some("earth-atmosphere-weather".to_string()),
        languages: vec![language("en"), language("zh-CN")],
        limit: 5,
      },
    )
    .await
    .unwrap();
  assert!(inventory.catalog_complete());
  assert_eq!(inventory.domains().len(), 1);
  assert_eq!(
    inventory.domains()[0].domain_id().as_str(),
    "domain_weather"
  );
  assert_eq!(inventory.domains()[0].labels().len(), 2);
  assert_eq!(inventory.domains()[0].definitions().len(), 2);
  assert_eq!(
    inventory.domains()[0]
      .knowledge_profile()
      .verified_fact_count(),
    184
  );

  let request = transport.request.lock().unwrap();
  let (path, body, _) = request.as_ref().unwrap();
  assert_eq!(*path, "/api/v1/domains/resolve");
  assert_eq!(body["context"]["content_release"], "knowledge-2026-09");
  assert_eq!(body["input"]["languages"], json!(["en", "zh-CN"]));
  assert_eq!(body["input"]["limit"], 5);
}

#[tokio::test]
async fn domain_inventory_rejects_unknown_or_incomplete_wire_data() {
  let cases = [
    json!({
      "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
      "content_release":"knowledge-2026-09",
      "value":{"catalog_complete":true,"candidates":[],"extra":"not-allowed"}
    }),
    json!({
      "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
      "content_release":"knowledge-2026-09", "value":{"candidates":[]}
    }),
    json!({
      "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
      "content_release":"other-release", "value":{"catalog_complete":true,"candidates":[]}
    }),
  ];
  for response in cases {
    let result = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .resolve_domains(
        &context(),
        &id("knowledge-2026-09"),
        DomainResolveInput {
          normalized_labels: vec!["weather".to_string()],
          scope_key: None,
          languages: vec![language("en")],
          limit: 5,
        },
      )
      .await;
    let Err(error) = result else {
      panic!("invalid domain wire data must fail closed");
    };
    assert_eq!(error, IslandPortClientError::InconsistentData);
  }
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
    "schema_version": "canonical-data-v1",
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
  assert_eq!(body["context"]["schema_version"], "canonical-data-v1");
  assert!(body["context"].get("content_release").is_none());
  assert_eq!(body["input"], json!({}));
  assert_eq!(*timeout, Duration::from_secs(2));
}

#[tokio::test]
async fn active_release_missing_or_incompatible_response_fails_closed() {
  let cases = [
    (
      json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","outcome":"not_found","error":{"code":"no_active_release","message":"No active release.","retryable":false}}),
      None,
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","outcome":"version_mismatch","error":{"code":"schema_incompatible","message":"Incompatible schema.","retryable":false}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"old","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"other","schema_version":"canonical-data-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"}}),
      Some(IslandPortClientError::SchemaIncompatible),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","outcome":"ok","value":{"content_release":"release-1"}}),
      Some(IslandPortClientError::InconsistentData),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1","vector_collection_id":"fake"}}),
      Some(IslandPortClientError::InconsistentData),
    ),
    (
      json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","outcome":"ok","value":{"content_release":"release-1","canonical_schema_version":"canonical-v1"},"content_release":"release-1"}),
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
    "schema_version":"canonical-data-v1",
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
    "schema_version": "canonical-data-v1",
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
  assert_eq!(body["context"]["schema_version"], "canonical-data-v1");
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
  assert_eq!(matches[0].matched_form, "sweltering");
  assert_eq!(
    matches[0].matched_form_id.as_ref().unwrap().as_str(),
    "form_sweltering"
  );
  assert_eq!(
    matches[0].candidate.sources[0].attribution.as_deref(),
    Some("Reviewed dictionary attribution")
  );
}

#[tokio::test]
async fn candidate_match_source_must_equal_the_authoritative_stored_form() {
  let mut cases = Vec::new();
  let mut changed_surface = candidate_response();
  *changed_surface
    .pointer_mut("/value/matches/0/matched_form")
    .unwrap() = json!("swelteringly");
  cases.push(changed_surface);
  let mut changed_id = candidate_response();
  *changed_id
    .pointer_mut("/value/matches/0/matched_form_id")
    .unwrap() = json!("form_other");
  cases.push(changed_id);
  let mut false_alias = candidate_response();
  *false_alias
    .pointer_mut("/value/matches/0/match_class")
    .unwrap() = json!("exact_alias");
  cases.push(false_alias);
  let mut unrelated_but_self_consistent = candidate_response();
  *unrelated_but_self_consistent
    .pointer_mut("/value/matches/0/matched_form")
    .unwrap() = json!("unrelated");
  *unrelated_but_self_consistent
    .pointer_mut("/value/matches/0/candidate/forms/0/form")
    .unwrap() = json!("unrelated");
  *unrelated_but_self_consistent
    .pointer_mut("/value/matches/0/candidate/forms/0/normalized_form")
    .unwrap() = json!("unrelated");
  cases.push(unrelated_but_self_consistent);

  for response in cases {
    let error = IslandPortCanonicalClient::new(Arc::new(FakeTransport::new(response)))
      .resolve_basic_card_candidates(&context(), &id("knowledge-2026-09"), candidate_input())
      .await
      .unwrap_err();
    assert_eq!(error, IslandPortClientError::InconsistentData);
  }
}

fn candidate_response() -> Value {
  json!({
    "request_id": "req_stage_3",
    "schema_version": "canonical-data-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "matches": [{
        "matched_form_id": "form_sweltering",
        "matched_form": "sweltering",
        "match_class": "exact_canonical",
        "lexical_score_basis_points": 10000,
        "candidate": {
          "lexeme": {"id":"lexeme_sweltering","language":"en","lemma":"sweltering","lemma_evidence_ids":["evidence_lemma"],"normalized_lemma":"sweltering","part_of_speech":"adjective","status":"active"},
          "sense": {"id":"sense_sweltering_hot","lexeme_id":"lexeme_sweltering","sense_key":"weather-hot","definition":"uncomfortably hot","definition_evidence_ids":["evidence_dictionary_1042"],"status":"active"},
          "forms": [{"id":"form_sweltering","lexeme_id":"lexeme_sweltering","form":"sweltering","normalized_form":"sweltering","kind":"lemma","morphology":null,"evidence_ids":["evidence_dictionary_1042"],"status":"active"}],
          "sources": [{"release_id":"knowledge-2026-09","source":{"id":"source_dictionary","name":"Reviewed dictionary","version":"2026-09","license":"internal-reviewed","attribution":"Reviewed dictionary attribution","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true}}}],
          "evidence": [{"id":"evidence_dictionary_1042","source_id":"source_dictionary","source_reference":"entry:1","language":"en","kind":"definition","confidence":"high","text":"uncomfortably hot","content_hash":"sha256:abc","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"},{"id":"evidence_lemma","source_id":"source_dictionary","source_reference":"entry:lemma","language":"en","kind":"other","confidence":"high","text":"sweltering","content_hash":"sha256:lemma","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"}]
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
      "request_id":"req_stage_3", "schema_version":"canonical-data-v1",
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
  let response = json!({"request_id":"req_stage_3","schema_version":"canonical-data-v1","content_release":"release-r1","outcome":"content_release_unavailable","error":{"code":"schema_incompatible","message":"secret","retryable":false}});
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
    "request_id":"req_stage_3", "schema_version":"canonical-data-v1", "outcome":"ok",
    "content_release":"release-r1",
    "value": {"canonical_schema_version":"canonical-v2", "target":{
      "lexeme":{"id":"lexeme_x","language":"en","lemma":"x","lemma_evidence_ids":["evidence_x"],"normalized_lemma":"x","part_of_speech":"noun","status":"active"},
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
    "schema_version": "canonical-data-v1",
    "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "canonical_schema_version": "canonical-v1",
      "target": {
        "lexeme": {"id":"lexeme_sweltering","language":"en","lemma":"sweltering","lemma_evidence_ids":["evidence_definition"],"normalized_lemma":"sweltering","part_of_speech":"adjective","status":"active"},
        "sense": {"id":"sense_sweltering_hot","lexeme_id":"lexeme_sweltering","sense_key":"weather-hot","definition":"uncomfortably hot","definition_evidence_ids":[],"status":"active"}
      },
      "lineages": {
        "evidence_definition": {
          "source": {"id":"source_dictionary","name":"Reviewed dictionary","version":"2026-09","license":"internal-reviewed","attribution":null,"permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true}},
          "fragment": {"id":"evidence_definition","source_id":"source_dictionary","source_reference":"entry:lemma","language":"en","kind":"other","confidence":"high","text":"sweltering","content_hash":"sha256:lemma","permissions":{"storage":true,"display":true,"embedding":true,"model_processing":true,"api_redistribution":true},"status":"active"},
          "origin": {"kind":"licensed_source"}
        },
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
    "request_id": "req_stage_3", "schema_version": "canonical-data-v1", "outcome": "ok",
    "content_release": "knowledge-2026-09",
    "value": {
      "canonical_schema_version": "canonical-v1",
      "target": {
        "lexeme": {"id":"lexeme_x","language":"en","lemma":"x","lemma_evidence_ids":["evidence_x"],"normalized_lemma":"x","part_of_speech":"noun","status":"active"},
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
