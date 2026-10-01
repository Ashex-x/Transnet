//! Frozen public BasicCard and release-pinned sense HTTP contract tests.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{Request, StatusCode},
  response::Response,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use transnet::{
  app_router,
  application::canonical_read::CanonicalReadService,
  domain::{
    canonical::{
      CanonicalId, CanonicalReleasePin, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
      EvidenceKind, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech, LexicalSource, Sense,
      SourcePermissions, WordForm,
    },
    canonical_content::{CanonicalSenseDetails, CanonicalSenseDetailsInput, SenseContentTarget},
    canonical_translation::CanonicalTranslationRevision,
    retrieval::{CanonicalCandidate, LexicalMatchKind, RepositoryMatch, RetrievalScore},
  },
  ports::canonical_read::{
    CanonicalCandidateQuery, CanonicalReadContext, CanonicalReadError, CanonicalReadPort,
    CanonicalSenseQuery, CanonicalTranslationQuery,
  },
  AppState,
};

#[derive(Clone, Copy)]
enum Mode {
  Resolved,
  Ambiguous,
  Missing,
  Oversized,
  Error(CanonicalReadError),
}
struct Authority {
  mode: Mode,
  active: Mutex<usize>,
  sensed: Mutex<Vec<String>>,
}

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}
fn language(value: &str) -> LanguageTag {
  LanguageTag::parse(value).unwrap()
}
fn pin(value: &str) -> CanonicalReleasePin {
  CanonicalReleasePin::new(id(value), "canonical-v1".into()).unwrap()
}
fn permissions() -> SourcePermissions {
  SourcePermissions {
    storage: true,
    display: true,
    embedding: true,
    model_processing: true,
    api_redistribution: true,
  }
}

fn candidate(release: &CanonicalReleasePin, suffix: &str) -> CanonicalCandidate {
  let lexeme_id = id(&format!("lexeme-{suffix}"));
  let sense_id = id(&format!("sense-{suffix}"));
  let evidence_id = id(&format!("evidence-{suffix}"));
  let lemma_form_id = id(&format!("form-{suffix}-lemma"));
  CanonicalCandidate {
    lexeme: Lexeme {
      id: lexeme_id.clone(),
      release_id: release.release_id.clone(),
      language: language("en"),
      lemma: "sweltering".into(),
      lemma_evidence_ids: vec![evidence_id.clone()],
      normalized_lemma: "sweltering".into(),
      part_of_speech: LexicalPartOfSpeech::Adjective,
      status: CanonicalStatus::Active,
    },
    sense: Sense {
      id: sense_id,
      lexeme_id: lexeme_id.clone(),
      release_id: release.release_id.clone(),
      sense_key: suffix.into(),
      definition: "uncomfortably hot".into(),
      definition_evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    },
    forms: vec![WordForm {
      id: lemma_form_id,
      lexeme_id: lexeme_id.clone(),
      release_id: release.release_id.clone(),
      form: "sweltering".into(),
      normalized_form: "sweltering".into(),
      kind: FormKind::Lemma,
      morphology: None,
      evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    }],
    evidence: vec![EvidenceFragment {
      id: evidence_id,
      source_id: id("source-reviewed"),
      source_reference: "entry:1".into(),
      release_id: release.release_id.clone(),
      language: language("en"),
      kind: EvidenceKind::Definition,
      confidence: EvidenceConfidence::High,
      text: "uncomfortably hot".into(),
      content_hash: "internal-hash".into(),
      permissions: permissions(),
      status: CanonicalStatus::Active,
    }],
    sources: vec![LexicalSource {
      id: id("source-reviewed"),
      name: "Dictionary".into(),
      version: "1".into(),
      license: "reviewed".into(),
      attribution: Some("Reviewed Dictionary (2026)".into()),
      permissions: permissions(),
    }],
  }
}

fn oversized_candidate(release: &CanonicalReleasePin, suffix: &str) -> CanonicalCandidate {
  let mut value = candidate(release, suffix);
  value.evidence[0].text = "x".repeat(4_096);
  value.forms.extend(
    (0..24)
      .map(|index| WordForm {
        id: id(&format!("form-{suffix}-{index}")),
        lexeme_id: value.lexeme.id.clone(),
        release_id: release.release_id.clone(),
        form: format!("form-{index}"),
        normalized_form: format!("form-{index}"),
        kind: FormKind::Inflection,
        morphology: None,
        evidence_ids: vec![value.evidence[0].id.clone()],
        status: CanonicalStatus::Active,
      })
      .collect::<Vec<_>>(),
  );
  value
}

#[async_trait]
impl CanonicalReadPort for Authority {
  async fn active_release(
    &self,
    _: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
    *self.active.lock().unwrap() += 1;
    match self.mode {
      Mode::Error(error) => Err(error),
      _ => Ok(Some(pin("release-r1"))),
    }
  }
  async fn translations(
    &self,
    _: &CanonicalReadContext,
    _: &CanonicalReleasePin,
    _: CanonicalTranslationQuery,
  ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError> {
    Ok(Vec::new())
  }
  async fn candidates(
    &self,
    _: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    _: CanonicalCandidateQuery,
  ) -> Result<Vec<RepositoryMatch>, CanonicalReadError> {
    let count = match self.mode {
      Mode::Resolved => 1,
      Mode::Ambiguous => 2,
      Mode::Oversized => 12,
      _ => 0,
    };
    Ok(
      (0..count)
        .map(|index| {
          let candidate = if matches!(self.mode, Mode::Oversized) {
            oversized_candidate(pin, &format!("s{index}"))
          } else {
            candidate(pin, &format!("s{index}"))
          };
          RepositoryMatch {
            matched_form_id: Some(candidate.forms[0].id.clone()),
            candidate,
            matched_form: "sweltering".into(),
            kind: LexicalMatchKind::ExactCanonical,
            score: RetrievalScore::exact(),
          }
        })
        .collect(),
    )
  }
  async fn sense(
    &self,
    _: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalSenseQuery,
  ) -> Result<CanonicalSenseDetails, CanonicalReadError> {
    if let Mode::Error(error) = self.mode {
      return Err(error);
    }
    self.sensed.lock().unwrap().push(pin.release_id.to_string());
    let candidate = candidate(pin, query.sense_id.as_str().trim_start_matches("sense-"));
    CanonicalSenseDetails::new(CanonicalSenseDetailsInput::empty(
      SenseContentTarget::new(&candidate.lexeme, &candidate.sense).unwrap(),
    ))
    .map_err(|_| CanonicalReadError::InconsistentData)
  }
}

fn router(mode: Mode) -> axum::Router {
  let authority = Arc::new(Authority {
    mode,
    active: Mutex::new(0),
    sensed: Mutex::new(Vec::new()),
  });
  app_router(AppState::new().with_canonical_read_service_timeout(
    Arc::new(CanonicalReadService::new(authority)),
    Duration::from_secs(1),
  ))
}
async fn body(response: Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
fn lookup_request(extra: &str) -> Request<Body> {
  Request::post("/api/v1/basic-cards/lookup")
    .header("content-type", "application/json")
    .body(Body::from(format!(
      r#"{{"query":"sweltering","source_language":"en","target_language":"zh-CN"{extra}}}"#
    )))
    .unwrap()
}

#[tokio::test]
async fn lookup_returns_frozen_three_state_contract_and_safe_attribution() {
  for (mode, resolution, count) in [
    (Mode::Resolved, "resolved", 1),
    (Mode::Ambiguous, "clarification_required", 2),
    (Mode::Missing, "not_found", 0),
  ] {
    let response = router(mode).oneshot(lookup_request("")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert_eq!(value["data"]["resolution"], resolution);
    assert_eq!(value["data"]["matches"].as_array().unwrap().len(), count);
    assert_eq!(value["meta"]["content_release"], "release-r1");
    assert_eq!(value["meta"]["canonical_schema_version"], "canonical-v1");
    assert!(value.to_string().find("internal-hash").is_none());
    if count > 0 {
      assert_eq!(
        value["data"]["matches"][0]["evidence"][0]["source"]["attribution"],
        "Reviewed Dictionary (2026)"
      );
    }
  }
}

#[tokio::test]
async fn lookup_rejects_unknown_invalid_and_oversized_inputs() {
  let response = router(Mode::Missing)
    .oneshot(lookup_request(",\"derived_forms\":[]"))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  let response = router(Mode::Missing)
    .oneshot(
      Request::post("/api/v1/basic-cards/lookup")
        .header("content-type", "application/json")
        .body(Body::from(
          json!({"query":"x".repeat(101),"source_language":"en","target_language":"zh-CN"})
            .to_string(),
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn lookup_rejects_a_projection_larger_than_one_mebibyte() {
  let response = router(Mode::Oversized)
    .oneshot(lookup_request(""))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
  assert_eq!(body(response).await["code"], "canonical_response_too_large");
}

#[tokio::test]
async fn canonical_failures_map_to_distinct_redacted_problems() {
  for (error, status, code) in [
    (
      CanonicalReadError::ContentReleaseUnavailable,
      StatusCode::CONFLICT,
      "content_release_unavailable",
    ),
    (
      CanonicalReadError::SchemaIncompatible,
      StatusCode::BAD_GATEWAY,
      "canonical_schema_incompatible",
    ),
    (
      CanonicalReadError::Timeout,
      StatusCode::SERVICE_UNAVAILABLE,
      "canonical_dependency_timeout",
    ),
    (
      CanonicalReadError::InconsistentData,
      StatusCode::BAD_GATEWAY,
      "invalid_canonical_response",
    ),
  ] {
    let response = router(Mode::Error(error))
      .oneshot(lookup_request(""))
      .await
      .unwrap();
    assert_eq!(response.status(), status);
    assert_eq!(body(response).await["code"], code);
  }
  let response = app_router(AppState::new())
    .oneshot(lookup_request(""))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    body(response).await["code"],
    "canonical_dependency_unavailable"
  );
}

#[tokio::test]
async fn pinned_sense_uses_caller_release_and_never_selects_active() {
  let authority = Arc::new(Authority {
    mode: Mode::Resolved,
    active: Mutex::new(0),
    sensed: Mutex::new(Vec::new()),
  });
  let router = app_router(
    AppState::new()
      .with_canonical_read_service(Arc::new(CanonicalReadService::new(authority.clone()))),
  );
  let response=router.oneshot(Request::post("/api/v1/senses/get").header("content-type","application/json").body(Body::from(r#"{"sense_id":"sense-s0","content_release":"release-r1","canonical_schema_version":"canonical-v1","target_language":"zh-CN"}"#)).unwrap()).await.unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let value = body(response).await;
  assert_eq!(value["meta"]["content_release"], "release-r1");
  assert_eq!(*authority.active.lock().unwrap(), 0);
  assert_eq!(*authority.sensed.lock().unwrap(), vec!["release-r1"]);
}

#[tokio::test]
async fn pinned_sense_maps_release_schema_validation_and_disabled_capability() {
  let request = |body: &'static str| {
    Request::post("/api/v1/senses/get")
      .header("content-type", "application/json")
      .body(Body::from(body))
      .unwrap()
  };
  for (error, status, code) in [
    (
      CanonicalReadError::ContentReleaseUnavailable,
      StatusCode::CONFLICT,
      "content_release_unavailable",
    ),
    (
      CanonicalReadError::SchemaIncompatible,
      StatusCode::BAD_GATEWAY,
      "canonical_schema_incompatible",
    ),
  ] {
    let response = router(Mode::Error(error))
      .oneshot(request(r#"{"sense_id":"sense-s0","content_release":"release-r1","canonical_schema_version":"canonical-v1","target_language":"zh-CN"}"#))
      .await
      .unwrap();
    assert_eq!(response.status(), status);
    assert_eq!(body(response).await["code"], code);
  }
  let response = router(Mode::Resolved)
    .oneshot(request(r#"{"sense_id":" ","content_release":"release-r1","canonical_schema_version":"canonical-v1","target_language":"zh-CN"}"#))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

  let response = app_router(AppState::new())
    .oneshot(request(r#"{"sense_id":"sense-s0","content_release":"release-r1","canonical_schema_version":"canonical-v1","target_language":"zh-CN"}"#))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    body(response).await["code"],
    "canonical_dependency_unavailable"
  );
}
