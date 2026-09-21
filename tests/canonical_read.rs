//! Request-local canonical-only composition tests without a live island-port server.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use async_trait::async_trait;
use transnet::{
  application::canonical_read::CanonicalReadService,
  domain::{
    canonical::{
      CanonicalId, CanonicalReleasePin, CanonicalStatus, EvidenceUse, LanguageTag, Lexeme,
      LexicalPartOfSpeech, Sense,
    },
    canonical_content::{CanonicalSenseDetails, CanonicalSenseDetailsInput, SenseContentTarget},
    canonical_translation::{
      CanonicalRevision, CanonicalTranslationId, CanonicalTranslationRevision,
      CanonicalTranslationUnit, SourceFingerprint,
    },
    retrieval::{
      CanonicalCandidate, LexicalMatchKind, RepositoryMatch, RetrievalRequest, RetrievalScore,
    },
  },
  ports::canonical_read::{
    CanonicalCandidateQuery, CanonicalReadContext, CanonicalReadError, CanonicalReadPort,
    CanonicalSenseQuery, CanonicalTranslationQuery,
  },
};

#[derive(Default)]
struct SwitchingAuthority {
  active_calls: Mutex<usize>,
  downstream_releases: Mutex<Vec<String>>,
  wrong_release: bool,
  foreign_source: bool,
  has_candidate: bool,
  no_active: bool,
  sense_calls: Mutex<usize>,
  sent_lookup_forms: Mutex<Vec<String>>,
}

fn candidate(pin: &CanonicalReleasePin) -> CanonicalCandidate {
  let lexeme = Lexeme {
    id: CanonicalId::new("lexeme-hello").unwrap(),
    release_id: pin.release_id.clone(),
    language: language("en"),
    lemma: "hello".into(),
    normalized_lemma: "hello".into(),
    part_of_speech: LexicalPartOfSpeech::Interjection,
    status: CanonicalStatus::Active,
  };
  let sense = Sense {
    id: CanonicalId::new("sense-hello").unwrap(),
    lexeme_id: lexeme.id.clone(),
    release_id: pin.release_id.clone(),
    sense_key: "greeting".into(),
    definition: "a greeting".into(),
    definition_evidence_ids: Vec::new(),
    status: CanonicalStatus::Active,
  };
  CanonicalCandidate {
    lexeme,
    sense,
    forms: Vec::new(),
    evidence: Vec::new(),
    sources: Vec::new(),
  }
}

fn release(value: &str) -> CanonicalReleasePin {
  CanonicalReleasePin::new(CanonicalId::new(value).unwrap(), "canonical-v1".into()).unwrap()
}

fn language(value: &str) -> LanguageTag {
  LanguageTag::parse(value).unwrap()
}

#[async_trait]
impl CanonicalReadPort for SwitchingAuthority {
  async fn active_release(
    &self,
    _: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
    let mut calls = self.active_calls.lock().unwrap();
    *calls += 1;
    if self.no_active {
      return Ok(None);
    }
    Ok(Some(release(if *calls == 1 {
      "release-r1"
    } else {
      "release-r2"
    })))
  }

  async fn translations(
    &self,
    _: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    _: CanonicalTranslationQuery,
  ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError> {
    self
      .downstream_releases
      .lock()
      .unwrap()
      .push(pin.release_id.to_string());
    if !self.wrong_release && !self.foreign_source {
      return Ok(Vec::new());
    }
    let source_language = language("en");
    let stored_source = if self.foreign_source {
      "other"
    } else {
      "hello"
    };
    Ok(vec![CanonicalTranslationRevision::new(
      CanonicalTranslationId::new("tr_example").unwrap(),
      CanonicalRevision::new(1).unwrap(),
      CanonicalId::new(if self.wrong_release {
        "release-r2"
      } else {
        "release-r1"
      })
      .unwrap(),
      CanonicalTranslationUnit::Passage,
      source_language.clone(),
      language("zh-CN"),
      stored_source,
      "你好",
      SourceFingerprint::compute(stored_source, &source_language),
      None,
      Vec::new(),
    )
    .unwrap()])
  }

  async fn candidates(
    &self,
    _: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalCandidateQuery,
  ) -> Result<Vec<RepositoryMatch>, CanonicalReadError> {
    *self.sent_lookup_forms.lock().unwrap() = query
      .lookup_forms
      .into_iter()
      .map(|form| form.form)
      .collect();
    self
      .downstream_releases
      .lock()
      .unwrap()
      .push(pin.release_id.to_string());
    Ok(if self.has_candidate {
      vec![RepositoryMatch {
        candidate: candidate(pin),
        kind: LexicalMatchKind::ExactCanonical,
        score: RetrievalScore::exact(),
      }]
    } else {
      Vec::new()
    })
  }

  async fn sense(
    &self,
    _: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalSenseQuery,
  ) -> Result<CanonicalSenseDetails, CanonicalReadError> {
    *self.sense_calls.lock().unwrap() += 1;
    let candidate = candidate(pin);
    assert_eq!(query.sense_id, candidate.sense.id);
    CanonicalSenseDetails::new(CanonicalSenseDetailsInput::empty(
      SenseContentTarget::new(&candidate.lexeme, &candidate.sense).unwrap(),
    ))
    .map_err(|_| CanonicalReadError::InconsistentData)
  }
}

#[tokio::test]
async fn request_local_lookup_forms_reach_authority_without_changing_identity() {
  let authority = Arc::new(SwitchingAuthority::default());
  let query = RetrievalRequest::new(
    "  UP  IN  THE  AIR!  ",
    language("en"),
    EvidenceUse::ApiRedistribution,
    5,
  )
  .unwrap();
  assert_eq!(query.query, "up  in  the  air!");
  let result = CanonicalReadService::new(authority.clone())
    .resolve(
      &context(),
      query,
      "  UP  IN  THE  AIR!  ",
      language("zh-CN"),
    )
    .await
    .unwrap();
  assert!(result.card.candidates.is_empty());
  assert_eq!(
    *authority.sent_lookup_forms.lock().unwrap(),
    ["up  in  the  air!", "up in the air!", "up  in  the  air"]
  );
}

fn context() -> CanonicalReadContext {
  CanonicalReadContext {
    request_id: "req_read".into(),
    deadline_at: "2099-01-01T00:00:00Z".into(),
    timeout: Duration::from_secs(2),
  }
}

fn request() -> RetrievalRequest {
  RetrievalRequest::new("hello", language("en"), EvidenceUse::ApiRedistribution, 5).unwrap()
}

#[tokio::test]
async fn active_switch_does_not_repin_within_one_request() {
  let authority = Arc::new(SwitchingAuthority::default());
  let outcome = CanonicalReadService::new(authority.clone())
    .resolve(&context(), request(), "hello", language("zh-CN"))
    .await
    .unwrap();
  assert_eq!(outcome.pin.release_id.as_str(), "release-r1");
  assert_eq!(outcome.card.content.release_id.as_str(), "release-r1");
  assert_eq!(*authority.active_calls.lock().unwrap(), 1);
  assert_eq!(
    *authority.downstream_releases.lock().unwrap(),
    ["release-r1", "release-r1"]
  );
}

#[tokio::test]
async fn caller_pinned_sense_remains_on_r1_after_active_switches_to_r2() {
  let authority = Arc::new(SwitchingAuthority::default());
  assert_eq!(
    authority
      .active_release(&context())
      .await
      .unwrap()
      .unwrap()
      .release_id
      .as_str(),
    "release-r1"
  );
  assert_eq!(
    authority
      .active_release(&context())
      .await
      .unwrap()
      .unwrap()
      .release_id
      .as_str(),
    "release-r2"
  );
  let details = CanonicalReadService::new(authority.clone())
    .read_pinned_sense(
      &context(),
      &release("release-r1"),
      CanonicalId::new("sense-hello").unwrap(),
      language("zh-CN"),
      EvidenceUse::ApiRedistribution,
    )
    .await
    .unwrap();
  assert_eq!(details.target().release_id().as_str(), "release-r1");
  assert_eq!(*authority.active_calls.lock().unwrap(), 2);
  assert_eq!(*authority.sense_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn contradictory_downstream_release_fails_closed() {
  let authority = Arc::new(SwitchingAuthority {
    wrong_release: true,
    ..Default::default()
  });
  let error = CanonicalReadService::new(authority.clone())
    .resolve(&context(), request(), "hello", language("zh-CN"))
    .await
    .err()
    .unwrap();
  assert_eq!(error, CanonicalReadError::InconsistentData);
  assert_eq!(*authority.active_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn fingerprint_candidates_require_exact_stored_source_verification() {
  let authority = Arc::new(SwitchingAuthority {
    foreign_source: true,
    ..Default::default()
  });
  let result = CanonicalReadService::new(authority)
    .resolve(&context(), request(), "hello", language("zh-CN"))
    .await
    .unwrap();
  assert!(result.translations.is_empty());
}

#[tokio::test]
async fn one_resolved_card_loads_sense_details_under_same_pin() {
  let authority = Arc::new(SwitchingAuthority {
    has_candidate: true,
    ..Default::default()
  });
  let result = CanonicalReadService::new(authority.clone())
    .resolve(&context(), request(), "hello", language("zh-CN"))
    .await
    .unwrap();
  assert_eq!(result.card.candidates.len(), 1);
  assert_eq!(result.card.candidates[0].rank, 1);
  assert_eq!(
    result.sense_details.unwrap().target().release_id().as_str(),
    "release-r1"
  );
  assert_eq!(*authority.sense_calls.lock().unwrap(), 1);
  assert_eq!(*authority.active_calls.lock().unwrap(), 1);
  assert_eq!(
    *authority.downstream_releases.lock().unwrap(),
    ["release-r1", "release-r1"]
  );
}

#[tokio::test]
async fn no_active_release_stops_before_downstream_reads() {
  let authority = Arc::new(SwitchingAuthority {
    no_active: true,
    ..Default::default()
  });
  let error = CanonicalReadService::new(authority.clone())
    .resolve(&context(), request(), "hello", language("zh-CN"))
    .await
    .err()
    .unwrap();
  assert_eq!(error, CanonicalReadError::NotFound);
  assert!(authority.downstream_releases.lock().unwrap().is_empty());
}

#[test]
fn canonical_pin_rejects_invalid_schema_without_vector_or_ranking() {
  assert!(CanonicalReleasePin::new(CanonicalId::new("release-r1").unwrap(), "".into()).is_none());
  assert!(CanonicalReleasePin::new(
    CanonicalId::new("release-r1").unwrap(),
    " canonical-v1 ".into()
  )
  .is_none());
  assert_eq!(
    release("release-r1").canonical_schema_version,
    "canonical-v1"
  );
}
