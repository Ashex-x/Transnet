//! HTTP contract coverage for conditional active-release-pinned canonical sense details.

use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
  Router,
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  adapters::in_memory::InMemoryCanonicalSenseDetailsRepository,
  app_router,
  application::canonical_sense_details::CanonicalSenseDetailsService,
  domain::{
    canonical::{
      ActiveContentVersion, CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
      EvidenceKind, LanguageTag, Lexeme, LexicalPartOfSpeech, LexicalSource, Sense,
      SourcePermissions,
    },
    canonical_content::{
      CanonicalDetailKind, CanonicalEvidenceLineage, CanonicalEvidenceOrigin,
      CanonicalFactualAssertion, CanonicalPronunciation, CanonicalSenseDetails,
      CanonicalSenseDetailsInput, Collocation, CollocationConstruction, CollocationRole,
      CollocationTerm, EtymologyAssertion, EtymologyKind, EtymologyScope, GeneratedEvidenceReview,
      GrammarPattern, GrammarPatternKind, HistoricalRange, LearnerPitfall, LearnerPitfallKind,
      LocalizedGloss, PronunciationNotation, PronunciationScope, SenseContentTarget,
      SenseHistoryAssertion, SenseHistoryEventKind, UsageLabel, UsageLabelKind,
    },
  },
  ports::{
    active_content_reader::{ActiveContentReader, ActiveContentReaderError},
    canonical_sense_details_repository::{
      CanonicalSenseDetailsReadRequest, CanonicalSenseDetailsRepository,
      CanonicalSenseDetailsRepositoryError,
    },
  },
  AppState, ProviderConfig, TranslationConfig, TranslationService,
};

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn language(value: &str) -> LanguageTag {
  LanguageTag::parse(value).unwrap()
}

fn active_content(release_id: &str) -> ActiveContentVersion {
  ActiveContentVersion {
    release_id: id(release_id),
    vector_collection_id: id("vectors-for-tests"),
    schema_version: "canonical-v1".to_string(),
    ranking_version: "rank-v1".to_string(),
  }
}

fn permissions(api_redistribution: bool) -> SourcePermissions {
  SourcePermissions {
    storage: true,
    display: true,
    embedding: false,
    model_processing: false,
    api_redistribution,
  }
}

fn translation_service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string().into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap()
}

fn target(release_id: &str, sense_id: &str, status: CanonicalStatus) -> SenseContentTarget {
  let release_id = id(release_id);
  let lexeme = Lexeme {
    id: id("lexeme-run"),
    release_id: release_id.clone(),
    language: language("en-US"),
    lemma: "run".to_string(),
    normalized_lemma: "run".to_string(),
    part_of_speech: LexicalPartOfSpeech::Verb,
    status,
  };
  let sense = Sense {
    id: id(sense_id),
    lexeme_id: lexeme.id.clone(),
    release_id,
    sense_key: "run-1".to_string(),
    definition: "move quickly".to_string(),
    definition_evidence_ids: vec![id("definition-evidence")],
    status,
  };
  SenseContentTarget::new(&lexeme, &sense).unwrap()
}

fn evidence_kind(detail_kind: CanonicalDetailKind) -> EvidenceKind {
  match detail_kind {
    CanonicalDetailKind::LocalizedGloss => EvidenceKind::LocalizedGloss,
    CanonicalDetailKind::Pronunciation => EvidenceKind::Pronunciation,
    CanonicalDetailKind::UsageLabel
    | CanonicalDetailKind::GrammarPattern
    | CanonicalDetailKind::Collocation
    | CanonicalDetailKind::Pitfall
    | CanonicalDetailKind::SenseEvolution => EvidenceKind::Usage,
    CanonicalDetailKind::Example => EvidenceKind::Example,
    CanonicalDetailKind::Etymology | CanonicalDetailKind::SenseHistory => EvidenceKind::Etymology,
  }
}

fn assertion(
  release_id: &str,
  status: CanonicalStatus,
  detail_kind: CanonicalDetailKind,
  assertion_id: &str,
  text: &str,
  source_api_redistribution: bool,
  asset_api_redistribution: bool,
) -> CanonicalFactualAssertion {
  let source_id = id(&format!("source-{assertion_id}"));
  let source_permissions = permissions(source_api_redistribution);
  let asset_permissions = permissions(asset_api_redistribution);
  let lineage = CanonicalEvidenceLineage::new(
    LexicalSource {
      id: source_id.clone(),
      name: "Test source".to_string(),
      version: "test-v1".to_string(),
      license: "LicenseRef-Test".to_string(),
      attribution: Some("Copyright 2026 Test Licensor".to_string()),
      permissions: source_permissions,
    },
    EvidenceFragment {
      id: id(&format!("evidence-{assertion_id}")),
      source_id,
      source_reference: format!("reference-{assertion_id}"),
      release_id: id(release_id),
      language: language("en-US"),
      kind: evidence_kind(detail_kind),
      confidence: EvidenceConfidence::High,
      text: text.to_string(),
      content_hash: format!("hash-{assertion_id}"),
      permissions: asset_permissions,
      status,
    },
    CanonicalEvidenceOrigin::LicensedSource,
  )
  .unwrap();
  CanonicalFactualAssertion::new(detail_kind, id(release_id), status, text, vec![lineage]).unwrap()
}

fn full_details(
  release_id: &str,
  sense_id: &str,
  status: CanonicalStatus,
  source_api_redistribution: bool,
  asset_api_redistribution: bool,
) -> CanonicalSenseDetails {
  let target = target(release_id, sense_id, status);
  let factual = |kind, assertion_id, text| {
    assertion(
      release_id,
      status,
      kind,
      assertion_id,
      text,
      source_api_redistribution,
      asset_api_redistribution,
    )
  };
  let localized_gloss = LocalizedGloss::new(
    id("gloss-run"),
    target.clone(),
    language("zh-CN"),
    factual(CanonicalDetailKind::LocalizedGloss, "gloss-run", "跑"),
  )
  .unwrap();
  let pronunciation = CanonicalPronunciation::new(
    id("pronunciation-run"),
    target.clone(),
    PronunciationScope::LexemeWide,
    language("en-GB"),
    PronunciationNotation::Ipa,
    factual(
      CanonicalDetailKind::Pronunciation,
      "pronunciation-run",
      "rʌn",
    ),
  )
  .unwrap();
  let usage_label = UsageLabel::new(
    id("usage-run"),
    target.clone(),
    UsageLabelKind::Register,
    "informal",
    factual(
      CanonicalDetailKind::UsageLabel,
      "usage-run",
      "informal register",
    ),
  )
  .unwrap();
  let grammar_pattern = GrammarPattern::new(
    id("grammar-run"),
    target.clone(),
    GrammarPatternKind::Valency,
    factual(
      CanonicalDetailKind::GrammarPattern,
      "grammar-run",
      "run + adverb",
    ),
  )
  .unwrap();
  let collocation = Collocation::new(
    id("collocation-run"),
    target.clone(),
    CollocationRole::Head,
    CollocationConstruction::VerbObject,
    CollocationTerm::new(
      "run",
      Some(target.lexeme_id().clone()),
      Some(target.sense_id().clone()),
    )
    .unwrap(),
    CollocationTerm::new("a race", None, None).unwrap(),
    factual(
      CanonicalDetailKind::Collocation,
      "collocation-run",
      "run a race",
    ),
  )
  .unwrap();
  let example = transnet::domain::canonical_content::CanonicalExample::new(
    id("example-run"),
    target.clone(),
    language("en-US"),
    factual(
      CanonicalDetailKind::Example,
      "example-run",
      "They run daily.",
    ),
  )
  .unwrap();
  let pitfall = LearnerPitfall::new(
    id("pitfall-run"),
    target.clone(),
    language("zh-CN"),
    LearnerPitfallKind::CommonError,
    factual(
      CanonicalDetailKind::Pitfall,
      "pitfall-mistake",
      "run quicklyly",
    ),
    factual(
      CanonicalDetailKind::Pitfall,
      "pitfall-correction",
      "run quickly",
    ),
  )
  .unwrap();
  let etymology = EtymologyAssertion::new(
    id("etymology-run"),
    target.clone(),
    EtymologyScope::LexemeWide,
    EtymologyKind::OriginSummary,
    language("en"),
    HistoricalRange::new(Some(900), Some(1_100)).unwrap(),
    factual(
      CanonicalDetailKind::Etymology,
      "etymology-run",
      "Source-qualified etymology.",
    ),
  )
  .unwrap();
  let history = SenseHistoryAssertion::new(
    id("history-run"),
    target.clone(),
    SenseHistoryEventKind::Attestation,
    HistoricalRange::new(Some(1_000), None).unwrap(),
    factual(
      CanonicalDetailKind::SenseHistory,
      "history-run",
      "Source-qualified attestation.",
    ),
  )
  .unwrap();

  CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
    target,
    localized_glosses: vec![localized_gloss],
    pronunciations: vec![pronunciation],
    usage_labels: vec![usage_label],
    grammar_patterns: vec![grammar_pattern],
    collocations: vec![collocation],
    examples: vec![example],
    pitfalls: vec![pitfall],
    etymologies: vec![etymology],
    history: vec![history],
  })
  .unwrap()
}

fn minimal_details(
  release_id: &str,
  sense_id: &str,
  status: CanonicalStatus,
  source_api_redistribution: bool,
  asset_api_redistribution: bool,
) -> CanonicalSenseDetails {
  let target = target(release_id, sense_id, status);
  let localized_gloss = LocalizedGloss::new(
    id("gloss-minimal"),
    target.clone(),
    language("zh-CN"),
    assertion(
      release_id,
      status,
      CanonicalDetailKind::LocalizedGloss,
      "gloss-minimal",
      "跑",
      source_api_redistribution,
      asset_api_redistribution,
    ),
  )
  .unwrap();
  CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
    target,
    localized_glosses: vec![localized_gloss],
    pronunciations: Vec::new(),
    usage_labels: Vec::new(),
    grammar_patterns: Vec::new(),
    collocations: Vec::new(),
    examples: Vec::new(),
    pitfalls: Vec::new(),
    etymologies: Vec::new(),
    history: Vec::new(),
  })
  .unwrap()
}

fn reviewed_generated_details(release_id: &str, sense_id: &str) -> CanonicalSenseDetails {
  let target = target(release_id, sense_id, CanonicalStatus::Active);
  let source_id = id("source-generated-reviewed");
  let source_permissions = permissions(true);
  let lineage = CanonicalEvidenceLineage::new(
    LexicalSource {
      id: source_id.clone(),
      name: "Test source".to_string(),
      version: "test-v1".to_string(),
      license: "LicenseRef-Test".to_string(),
      attribution: None,
      permissions: source_permissions,
    },
    EvidenceFragment {
      id: id("evidence-generated-reviewed"),
      source_id,
      source_reference: "reference-generated-reviewed".to_string(),
      release_id: id(release_id),
      language: language("zh-CN"),
      kind: EvidenceKind::LocalizedGloss,
      confidence: EvidenceConfidence::Medium,
      text: "跑".to_string(),
      content_hash: "hash-generated-reviewed".to_string(),
      permissions: source_permissions,
      status: CanonicalStatus::Active,
    },
    CanonicalEvidenceOrigin::Generated {
      generation_id: id("generation-internal-only"),
      review: GeneratedEvidenceReview::ReviewedAndPromoted {
        review_id: id("review-internal-only"),
      },
    },
  )
  .unwrap();
  let localized_gloss = LocalizedGloss::new(
    id("gloss-generated-reviewed"),
    target.clone(),
    language("zh-CN"),
    CanonicalFactualAssertion::new(
      CanonicalDetailKind::LocalizedGloss,
      id(release_id),
      CanonicalStatus::Active,
      "跑",
      vec![lineage],
    )
    .unwrap(),
  )
  .unwrap();
  CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
    target,
    localized_glosses: vec![localized_gloss],
    pronunciations: Vec::new(),
    usage_labels: Vec::new(),
    grammar_patterns: Vec::new(),
    collocations: Vec::new(),
    examples: Vec::new(),
    pitfalls: Vec::new(),
    etymologies: Vec::new(),
    history: Vec::new(),
  })
  .unwrap()
}

fn app(
  active_content: Arc<dyn ActiveContentReader>,
  details: Arc<dyn CanonicalSenseDetailsRepository>,
) -> Router {
  let details = Arc::new(CanonicalSenseDetailsService::new(details));
  app_router(
    AppState::new(translation_service()).with_canonical_sense_details(active_content, details),
  )
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[derive(Debug)]
struct FixedActiveContentReader(Result<Option<ActiveContentVersion>, ActiveContentReaderError>);

#[async_trait]
impl ActiveContentReader for FixedActiveContentReader {
  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ActiveContentReaderError> {
    self.0.clone()
  }
}

#[derive(Debug)]
struct CountingActiveContentReader {
  first: ActiveContentVersion,
  later: ActiveContentVersion,
  calls: AtomicUsize,
}

impl CountingActiveContentReader {
  fn new(first: ActiveContentVersion, later: ActiveContentVersion) -> Self {
    Self {
      first,
      later,
      calls: AtomicUsize::new(0),
    }
  }
}

#[async_trait]
impl ActiveContentReader for CountingActiveContentReader {
  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ActiveContentReaderError> {
    if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
      Ok(Some(self.first.clone()))
    } else {
      Ok(Some(self.later.clone()))
    }
  }
}

#[derive(Debug)]
struct FailingDetailsRepository(CanonicalSenseDetailsRepositoryError);

#[async_trait]
impl CanonicalSenseDetailsRepository for FailingDetailsRepository {
  async fn load(
    &self,
    _request: &CanonicalSenseDetailsReadRequest,
  ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError> {
    Err(self.0.clone())
  }
}

#[derive(Debug)]
struct ForeignDetailsRepository(CanonicalSenseDetails);

#[async_trait]
impl CanonicalSenseDetailsRepository for ForeignDetailsRepository {
  async fn load(
    &self,
    _request: &CanonicalSenseDetailsReadRequest,
  ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError> {
    Ok(Some(self.0.clone()))
  }
}

#[tokio::test]
async fn default_runtime_does_not_register_canonical_sense_details() {
  let response = app_router(AppState::new(translation_service()))
    .oneshot(
      Request::get("/v1/senses/sense-private-looking")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(json(response).await["code"], "not_found");
}

#[tokio::test]
async fn injected_route_returns_full_typed_aggregate_with_assertion_provenance() {
  let repository: Arc<dyn CanonicalSenseDetailsRepository> = Arc::new(
    InMemoryCanonicalSenseDetailsRepository::new().with_details(full_details(
      "release-1",
      "sense-run",
      CanonicalStatus::Active,
      true,
      true,
    )),
  );
  let response = app(
    Arc::new(FixedActiveContentReader(Ok(Some(active_content(
      "release-1",
    ))))),
    repository,
  )
  .oneshot(
    Request::get("/v1/senses/sense-run")
      .header("x-request-id", "sense-details-42")
      .body(Body::empty())
      .unwrap(),
  )
  .await
  .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(response.headers()["x-request-id"], "sense-details-42");
  let body = json(response).await;
  assert_eq!(body["schema_version"], "1.0");
  assert_eq!(body["target"]["lexeme_id"], "lexeme-run");
  assert_eq!(body["target"]["sense_id"], "sense-run");
  assert_eq!(body["target"]["release_id"], "release-1");
  assert_eq!(body["target"]["language"], "en-US");
  assert_eq!(
    body["localized_glosses"][0]["assertion"]["kind"],
    "localized_gloss"
  );
  assert_eq!(body["pronunciations"][0]["scope"], "lexeme_wide");
  assert_eq!(body["pronunciations"][0]["notation"], "ipa");
  assert_eq!(body["usage_labels"][0]["code"], "informal");
  assert_eq!(body["grammar_patterns"][0]["kind"], "valency");
  assert_eq!(body["collocations"][0]["target_role"], "head");
  assert_eq!(body["collocations"][0]["head"]["sense_id"], "sense-run");
  assert_eq!(body["examples"][0]["assertion"]["kind"], "example");
  assert_eq!(body["pitfalls"][0]["mistake"]["kind"], "pitfall");
  assert_eq!(body["etymologies"][0]["period"]["first_year"], 900);
  assert_eq!(body["history"][0]["period"]["last_year"], Value::Null);
  let evidence = &body["localized_glosses"][0]["assertion"]["evidence"][0];
  assert_eq!(evidence["id"], "evidence-gloss-run");
  assert_eq!(evidence["kind"], "localized_gloss");
  assert_eq!(evidence["provenance"]["source_id"], "source-gloss-run");
  assert_eq!(evidence["provenance"]["release_id"], "release-1");
  assert_eq!(evidence["provenance"]["origin"], "licensed_source");
  assert_eq!(body["provenance"]["release_id"], "release-1");
  assert_eq!(body["provenance"]["evidence_use"], "api_redistribution");
  assert_eq!(body["provenance"]["evidence_backed"], true);
}

#[tokio::test]
async fn public_evidence_provenance_preserves_source_attribution() {
  let repository: Arc<dyn CanonicalSenseDetailsRepository> = Arc::new(
    InMemoryCanonicalSenseDetailsRepository::new().with_details(minimal_details(
      "release-1",
      "sense-run",
      CanonicalStatus::Active,
      true,
      true,
    )),
  );
  let response = app(
    Arc::new(FixedActiveContentReader(Ok(Some(active_content(
      "release-1",
    ))))),
    repository,
  )
  .oneshot(
    Request::get("/v1/senses/sense-run")
      .body(Body::empty())
      .unwrap(),
  )
  .await
  .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    json(response).await["localized_glosses"][0]["assertion"]["evidence"][0]["provenance"]
      ["attribution"],
    "Copyright 2026 Test Licensor"
  );
}

#[tokio::test]
async fn reviewed_generated_evidence_uses_a_safe_origin_label_without_audit_identifiers() {
  let repository: Arc<dyn CanonicalSenseDetailsRepository> = Arc::new(
    InMemoryCanonicalSenseDetailsRepository::new()
      .with_details(reviewed_generated_details("release-1", "sense-run")),
  );
  let response = app(
    Arc::new(FixedActiveContentReader(Ok(Some(active_content(
      "release-1",
    ))))),
    repository,
  )
  .oneshot(
    Request::get("/v1/senses/sense-run")
      .body(Body::empty())
      .unwrap(),
  )
  .await
  .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let body = json(response).await;
  assert_eq!(
    body["localized_glosses"][0]["assertion"]["evidence"][0]["provenance"]["origin"],
    "reviewed_generated"
  );
  let body = body.to_string();
  assert!(!body.contains("generation-internal-only"));
  assert!(!body.contains("review-internal-only"));
  assert!(!body.contains("generation_contract"));
}

#[tokio::test]
async fn route_pins_the_active_release_once_before_loading_details() {
  let reader = Arc::new(CountingActiveContentReader::new(
    active_content("release-1"),
    active_content("release-2"),
  ));
  let repository: Arc<dyn CanonicalSenseDetailsRepository> = Arc::new(
    InMemoryCanonicalSenseDetailsRepository::new().with_details(minimal_details(
      "release-1",
      "sense-run",
      CanonicalStatus::Active,
      true,
      true,
    )),
  );
  let response = app(reader.clone(), repository)
    .oneshot(
      Request::get("/v1/senses/sense-run")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
  assert_eq!(
    json(response).await["provenance"]["release_id"],
    "release-1"
  );
}

#[tokio::test]
async fn missing_inactive_and_permission_filtered_details_share_one_non_disclosing_not_found_shape()
{
  let active = Arc::new(FixedActiveContentReader(Ok(Some(active_content(
    "release-1",
  )))));
  let cases = [
    ("missing", InMemoryCanonicalSenseDetailsRepository::new()),
    (
      "inactive",
      InMemoryCanonicalSenseDetailsRepository::new().with_details(minimal_details(
        "release-1",
        "sense-run",
        CanonicalStatus::Draft,
        true,
        true,
      )),
    ),
    (
      "source-permission",
      InMemoryCanonicalSenseDetailsRepository::new().with_details(minimal_details(
        "release-1",
        "sense-run",
        CanonicalStatus::Active,
        false,
        false,
      )),
    ),
    (
      "asset-permission",
      InMemoryCanonicalSenseDetailsRepository::new().with_details(minimal_details(
        "release-1",
        "sense-run",
        CanonicalStatus::Active,
        true,
        false,
      )),
    ),
  ];

  for (case, repository) in cases {
    let response = app(active.clone(), Arc::new(repository))
      .oneshot(
        Request::get("/v1/senses/sense-run")
          .header("x-request-id", "details-policy-42")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "case: {case}");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["code"], "canonical_sense_not_found", "case: {case}");
    assert_eq!(body["retryable"], false, "case: {case}");
    assert!(!body.to_string().contains(case), "case: {case}");
    assert!(!body.to_string().contains("source-"), "case: {case}");
  }
}

#[tokio::test]
async fn foreign_like_details_and_dependency_failures_are_redacted_and_classified() {
  let active = Arc::new(FixedActiveContentReader(Ok(Some(active_content(
    "release-1",
  )))));
  let requested = "sense-private-request";
  let foreign = app(
    active.clone(),
    Arc::new(ForeignDetailsRepository(minimal_details(
      "release-1",
      "sense-private-returned",
      CanonicalStatus::Active,
      true,
      true,
    ))),
  )
  .oneshot(
    Request::get(format!("/v1/senses/{requested}"))
      .body(Body::empty())
      .unwrap(),
  )
  .await
  .unwrap();

  assert_eq!(foreign.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(foreign.headers()[header::CACHE_CONTROL], "no-store");
  let foreign_body = json(foreign).await;
  assert_eq!(foreign_body["code"], "canonical_sense_details_unavailable");
  assert_eq!(foreign_body["retryable"], false);
  assert!(!foreign_body.to_string().contains(requested));
  assert!(!foreign_body.to_string().contains("sense-private-returned"));

  for (error, retryable) in [
    (CanonicalSenseDetailsRepositoryError::Unavailable, true),
    (
      CanonicalSenseDetailsRepositoryError::InconsistentData,
      false,
    ),
  ] {
    let response = app(active.clone(), Arc::new(FailingDetailsRepository(error)))
      .oneshot(
        Request::get(format!("/v1/senses/{requested}"))
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json(response).await;
    assert_eq!(body["retryable"], retryable);
    assert!(!body.to_string().contains(requested));
  }

  for (error, retryable) in [
    (ActiveContentReaderError::Unavailable, true),
    (ActiveContentReaderError::InconsistentData, false),
  ] {
    let response = app(
      Arc::new(FixedActiveContentReader(Err(error))),
      Arc::new(InMemoryCanonicalSenseDetailsRepository::new()),
    )
    .oneshot(
      Request::get(format!("/v1/senses/{requested}"))
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json(response).await;
    assert_eq!(body["retryable"], retryable);
    assert!(!body.to_string().contains(requested));
  }
}

#[tokio::test]
async fn invalid_identifiers_are_typed_non_cacheable_problems_without_echoing_path_values() {
  let repository: Arc<dyn CanonicalSenseDetailsRepository> =
    Arc::new(InMemoryCanonicalSenseDetailsRepository::new());
  let app = app(
    Arc::new(FixedActiveContentReader(Ok(Some(active_content(
      "release-1",
    ))))),
    repository,
  );
  let long_id = "x".repeat(257);
  for path in [
    "/v1/senses/%20".to_string(),
    format!("/v1/senses/{long_id}"),
  ] {
    let response = app
      .clone()
      .oneshot(Request::get(&path).body(Body::empty()).unwrap())
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["code"], "invalid_sense_request");
    assert_eq!(body["errors"][0]["field"], "sense_id");
    assert_eq!(body["retryable"], false);
    assert!(!body.to_string().contains(&long_id));
  }
}
