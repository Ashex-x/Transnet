//! Canonical `POST /v1/lookups` composition and fallback contract tests.

use std::{
  sync::Arc,
  time::{Duration, SystemTime},
};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  adapters::{
    clock::FixedClock,
    in_memory::InMemoryCache,
    in_memory_retrieval::{InMemoryRetrievalAdapter, InMemoryVectorAvailability},
  },
  app_router,
  application::{
    canonical_lookup::CanonicalLookupService,
    canonical_lookup_cache::CanonicalLookupSnapshotCacheService,
    retrieval::CanonicalRetrievalService,
  },
  domain::{
    canonical::{
      ActiveContentVersion, CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
      EvidenceKind, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech, Sense, SourcePermissions,
      WordForm,
    },
    canonical_lookup_cache::{CanonicalLookupSnapshot, CanonicalLookupSnapshotKey},
    retrieval::{CanonicalCandidate, RetrievalScore, VectorMatch, VectorPurpose, VectorTarget},
    translation::{Confidence, TranslationInput, TranslationResult},
  },
  ports::{
    cache::{Cache, CacheEntry, CacheError},
    canonical_repository::{CanonicalRepository, CanonicalRepositoryError},
    learning_model::{LearningModel, LearningModelError},
    vector_retriever::{VectorRetriever, VectorRetrieverError},
  },
  AppState, ProviderConfig, TranslationConfig, TranslationService,
};

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn language() -> LanguageTag {
  LanguageTag::parse("en").unwrap()
}

fn content() -> ActiveContentVersion {
  ActiveContentVersion {
    release_id: id("release-1"),
    vector_collection_id: id("vectors-1"),
    schema_version: "canonical-v1".to_string(),
    ranking_version: "rank-v1".to_string(),
  }
}

fn candidate() -> CanonicalCandidate {
  let release_id = id("release-1");
  let lexeme_id = id("lexeme-hot");
  let evidence_id = id("evidence-hot");
  CanonicalCandidate {
    lexeme: Lexeme {
      id: lexeme_id.clone(),
      release_id: release_id.clone(),
      language: language(),
      lemma: "hot".to_string(),
      normalized_lemma: "hot".to_string(),
      part_of_speech: LexicalPartOfSpeech::Adjective,
      status: CanonicalStatus::Active,
    },
    sense: Sense {
      id: id("sense-hot"),
      lexeme_id: lexeme_id.clone(),
      release_id: release_id.clone(),
      sense_key: "temperature".to_string(),
      definition: "having a high temperature".to_string(),
      definition_evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    },
    forms: vec![WordForm {
      id: id("form-hotter"),
      lexeme_id,
      release_id: release_id.clone(),
      form: "hotter".to_string(),
      normalized_form: "hotter".to_string(),
      kind: FormKind::Inflection,
      morphology: Some("comparative".to_string()),
      evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    }],
    evidence: vec![EvidenceFragment {
      id: evidence_id,
      source_id: id("source-licensed"),
      source_reference: "entry-42".to_string(),
      release_id,
      language: language(),
      kind: EvidenceKind::Definition,
      confidence: EvidenceConfidence::High,
      text: "having a high temperature".to_string(),
      content_hash: "hash-hot".to_string(),
      permissions: SourcePermissions {
        storage: true,
        display: true,
        embedding: true,
        model_processing: true,
        api_redistribution: true,
      },
      status: CanonicalStatus::Active,
    }],
  }
}

fn matching_vector() -> VectorMatch {
  VectorMatch {
    target: VectorTarget::Sense(id("sense-hot")),
    score: RetrievalScore::new(9_500).unwrap(),
    release_id: id("release-1"),
    vector_collection_id: id("vectors-1"),
    purpose: VectorPurpose::CanonicalEnglishSense,
    content_language: language(),
  }
}

fn translation_service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string(),
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

fn clock() -> Arc<FixedClock> {
  Arc::new(FixedClock::new(
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
  ))
}

fn canonical_service(
  adapter: InMemoryRetrievalAdapter,
  cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>>,
  clock: Arc<FixedClock>,
) -> Arc<CanonicalLookupService> {
  let adapter = Arc::new(adapter);
  let repository: Arc<dyn CanonicalRepository> = adapter.clone();
  let vectors: Arc<dyn VectorRetriever> = adapter;
  let retrieval = Arc::new(CanonicalRetrievalService::new(repository, vectors));
  let snapshots =
    CanonicalLookupSnapshotCacheService::new(retrieval, cache, clock, Duration::from_secs(30))
      .unwrap();
  Arc::new(CanonicalLookupService::new(Arc::new(snapshots)))
}

fn in_memory_canonical_service(adapter: InMemoryRetrievalAdapter) -> Arc<CanonicalLookupService> {
  let clock = clock();
  let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> =
    Arc::new(InMemoryCache::new(clock.clone()));
  canonical_service(adapter, cache, clock)
}

fn canonical_service_for_repository(
  repository: Arc<dyn CanonicalRepository>,
) -> Arc<CanonicalLookupService> {
  let retrieval = Arc::new(CanonicalRetrievalService::new(
    repository,
    Arc::new(UnavailableVector),
  ));
  let fixed_clock = clock();
  let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> =
    Arc::new(InMemoryCache::new(fixed_clock.clone()));
  let snapshots = CanonicalLookupSnapshotCacheService::new(
    retrieval,
    cache,
    fixed_clock,
    Duration::from_secs(30),
  )
  .unwrap();
  Arc::new(CanonicalLookupService::new(Arc::new(snapshots)))
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

struct StubModel;

#[async_trait]
impl LearningModel for StubModel {
  async fn generate(
    &self,
    _input: &TranslationInput,
  ) -> Result<TranslationResult, LearningModelError> {
    Ok(TranslationResult {
      source_language: "es".to_string(),
      language_confidence: Confidence::High,
      entries: Vec::new(),
      warnings: Vec::new(),
    })
  }
}

#[tokio::test]
async fn injected_canonical_lookup_returns_separated_evidence_backed_fields() {
  let canonical = in_memory_canonical_service(
    InMemoryRetrievalAdapter::new(content())
      .with_candidate(candidate())
      .with_vector_match(matching_vector()),
  );
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"  HOTTER  ","source_language":"EN","history_mode":"incognito","include":["relations","word_history"]}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json(response).await;
  assert_eq!(body["schema_version"], "1.0");
  assert_eq!(body["query"]["original"], "  HOTTER  ");
  assert_eq!(body["query"]["normalized"], "hotter");
  assert_eq!(body["query"]["language"], "en");
  assert_eq!(body["query"]["language_confidence"], "explicit");
  assert_eq!(body["query"]["evidence_use"], "api_redistribution");
  assert_eq!(body["matches"][0]["lexeme"]["id"], "lexeme-hot");
  assert_eq!(body["matches"][0]["lexeme"]["lemma"], "hot");
  assert_eq!(body["matches"][0]["part_of_speech"], "adjective");
  assert_eq!(body["matches"][0]["sense"]["id"], "sense-hot");
  assert_eq!(
    body["matches"][0]["sense"]["definition"]["kind"],
    "definition"
  );
  assert_eq!(body["matches"][0]["forms"][0]["kind"], "inflection");
  assert_eq!(
    body["matches"][0]["forms"][0]["assertion"]["evidence"][0]["provenance"]["source_id"],
    "source-licensed"
  );
  assert_eq!(
    body["matches"][0]["sense"]["definition"]["evidence"][0]["provenance"]["release_id"],
    "release-1"
  );
  assert_eq!(body["coverage"]["retrieval"]["state"], "available");
  assert_eq!(body["provenance"]["retrieval_path"], "hybrid");
  assert_eq!(body["provenance"]["evidence_backed"], true);
  assert_eq!(body["provenance"]["generation_contract"], "not_generated");
  assert!(body["warnings"].as_array().is_some_and(|warnings| {
    warnings.iter().any(|warning| {
      warning.as_str()
        == Some(
          "Relations and word history are not included by the current canonical lookup foundation.",
        )
    })
  }));
}

#[tokio::test]
async fn canonical_lookup_marks_lexical_fallback_as_vector_degraded() {
  let canonical = in_memory_canonical_service(
    InMemoryRetrievalAdapter::new(content())
      .with_candidate(candidate())
      .with_vector_availability(InMemoryVectorAvailability::Unavailable),
  );
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"hotter","source_language":"en","history_mode":"incognito"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let body = json(response).await;
  assert_eq!(body["matches"].as_array().map(Vec::len), Some(1));
  assert_eq!(body["coverage"]["retrieval"]["state"], "vector_degraded");
  assert_eq!(body["provenance"]["retrieval_path"], "lexical_fallback");
}

#[tokio::test]
async fn private_history_mode_bypasses_the_shared_cache() {
  for history_mode in ["incognito", "save"] {
    let canonical = canonical_service(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_match(matching_vector()),
      Arc::new(ForbiddenCache),
      clock(),
    );
    let response =
      app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
        .oneshot(
          Request::post("/v1/lookups")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(format!(
              r#"{{"query":"hotter","source_language":"en","history_mode":"{history_mode}"}}"#,
            )))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
      response.status(),
      StatusCode::OK,
      "history mode: {history_mode}"
    );
    assert_eq!(json(response).await["provenance"]["evidence_backed"], true);
  }
}

#[tokio::test]
async fn automatic_language_and_context_fall_back_to_the_model_without_cache_access() {
  for request in [
    r#"{"query":"caliente","source_language":"auto"}"#,
    r#"{"query":"hotter","source_language":"en","context":"The soup is hot."}"#,
  ] {
    let canonical = canonical_service(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_match(matching_vector()),
      Arc::new(ForbiddenCache),
      clock(),
    );
    let response = app_router(
      AppState::new(translation_service())
        .with_learning_model(Arc::new(StubModel))
        .with_canonical_lookup(canonical),
    )
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request))
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["provenance"]["evidence_backed"], false);
    assert_eq!(body["query"]["language"], "es");
  }
}

#[tokio::test]
async fn model_fallback_reports_the_existing_problem_when_no_model_is_injected() {
  let canonical = in_memory_canonical_service(
    InMemoryRetrievalAdapter::new(content())
      .with_candidate(candidate())
      .with_vector_match(matching_vector()),
  );
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"caliente","source_language":"auto"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  assert_eq!(json(response).await["code"], "learning_model_unavailable");
}

#[tokio::test]
async fn canonical_failures_use_a_redacted_rfc_problem() {
  let canonical = canonical_service_for_repository(Arc::new(FailingRepository(
    CanonicalRepositoryError::Unavailable,
  )));
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "canonical-42")
        .body(Body::from(
          r#"{"query":"private-looking-query","source_language":"en","history_mode":"incognito"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["code"], "canonical_lookup_unavailable");
  assert_eq!(body["request_id"], "canonical-42");
  assert_eq!(body["retryable"], true);
  assert!(!serde_json::to_string(&body)
    .unwrap()
    .contains("private-looking-query"));
}

#[tokio::test]
async fn inconsistent_canonical_content_uses_a_nonretryable_problem() {
  let canonical = canonical_service_for_repository(Arc::new(FailingRepository(
    CanonicalRepositoryError::InconsistentData,
  )));
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"private-looking-query","source_language":"en","history_mode":"incognito"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["code"], "canonical_lookup_unavailable");
  assert_eq!(body["retryable"], false);
  assert!(!serde_json::to_string(&body)
    .unwrap()
    .contains("private-looking-query"));
}

#[tokio::test]
async fn invalid_canonical_cache_contract_uses_a_nonretryable_problem() {
  let canonical = canonical_service_for_repository(Arc::new(InvalidContentRepository));
  let response = app_router(AppState::new(translation_service()).with_canonical_lookup(canonical))
    .oneshot(
      Request::post("/v1/lookups")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
          r#"{"query":"private-looking-query","source_language":"en","history_mode":"incognito"}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  let body = json(response).await;
  assert_eq!(body["code"], "canonical_lookup_unavailable");
  assert_eq!(body["retryable"], false);
  assert!(!serde_json::to_string(&body)
    .unwrap()
    .contains("private-looking-query"));
}

struct ForbiddenCache;

#[async_trait]
impl Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot> for ForbiddenCache {
  async fn get(
    &self,
    _key: &CanonicalLookupSnapshotKey,
  ) -> Result<Option<CacheEntry<CanonicalLookupSnapshot>>, CacheError> {
    panic!("private and model-fallback paths must not read the shared canonical cache")
  }

  async fn put(
    &self,
    _key: CanonicalLookupSnapshotKey,
    _value: CanonicalLookupSnapshot,
    _expires_at: SystemTime,
  ) -> Result<(), CacheError> {
    panic!("private and model-fallback paths must not write the shared canonical cache")
  }

  async fn remove(&self, _key: &CanonicalLookupSnapshotKey) -> Result<(), CacheError> {
    panic!("private and model-fallback paths must not remove shared canonical cache state")
  }
}

#[derive(Clone)]
struct FailingRepository(CanonicalRepositoryError);

#[async_trait]
impl CanonicalRepository for FailingRepository {
  async fn active_content_version(&self) -> Result<ActiveContentVersion, CanonicalRepositoryError> {
    Err(self.0.clone())
  }

  async fn search_lexical(
    &self,
    _request: &transnet::domain::retrieval::LexicalSearchRequest,
  ) -> Result<Vec<transnet::domain::retrieval::RepositoryMatch>, CanonicalRepositoryError> {
    Err(self.0.clone())
  }

  async fn load_candidates(
    &self,
    _request: &transnet::domain::retrieval::CandidateLoadRequest,
  ) -> Result<Vec<CanonicalCandidate>, CanonicalRepositoryError> {
    Err(self.0.clone())
  }
}

struct InvalidContentRepository;

#[async_trait]
impl CanonicalRepository for InvalidContentRepository {
  async fn active_content_version(&self) -> Result<ActiveContentVersion, CanonicalRepositoryError> {
    let mut invalid = content();
    invalid.schema_version = " ".to_string();
    Ok(invalid)
  }

  async fn search_lexical(
    &self,
    _request: &transnet::domain::retrieval::LexicalSearchRequest,
  ) -> Result<Vec<transnet::domain::retrieval::RepositoryMatch>, CanonicalRepositoryError> {
    Err(CanonicalRepositoryError::InconsistentData)
  }

  async fn load_candidates(
    &self,
    _request: &transnet::domain::retrieval::CandidateLoadRequest,
  ) -> Result<Vec<CanonicalCandidate>, CanonicalRepositoryError> {
    Err(CanonicalRepositoryError::Unavailable)
  }
}

struct UnavailableVector;

#[async_trait]
impl VectorRetriever for UnavailableVector {
  async fn search(
    &self,
    _request: &transnet::domain::retrieval::VectorSearchRequest,
  ) -> Result<Vec<VectorMatch>, VectorRetrieverError> {
    Err(VectorRetrieverError::Unavailable)
  }
}
