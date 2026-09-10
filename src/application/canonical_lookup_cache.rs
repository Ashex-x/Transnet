//! Public canonical lookup snapshot caching with typed privacy bypasses.

use std::{sync::Arc, time::Duration};

use thiserror::Error;

use crate::{
  application::retrieval::{CanonicalRetrievalError, CanonicalRetrievalService, RetrievalPath},
  domain::canonical_lookup_cache::{
    CanonicalLookupCacheBypassReason, CanonicalLookupCacheEligibility,
    CanonicalLookupCacheValidationError, CanonicalLookupSnapshot, CanonicalLookupSnapshotKey,
    PublicCanonicalLookupRequest,
  },
  ports::{
    cache::Cache,
    clock::{Clock, UtcTimestamp},
  },
};

/// Outcome of trying to obtain a public canonical-card snapshot through the shared cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalLookupSnapshotCacheResult {
  /// A compatible public snapshot was available in the shared cache.
  Hit(CanonicalLookupSnapshot),
  /// The cache missed, so the public snapshot was rebuilt from canonical retrieval.
  Miss(CanonicalLookupSnapshot),
  /// The cache could not safely serve the operation, so the public snapshot was rebuilt uncached.
  Unavailable(CanonicalLookupSnapshot),
  /// Private or request-specific input bypassed the shared cache and received an uncached snapshot.
  Bypass {
    /// Private-input category that caused the shared-cache bypass.
    reason: CanonicalLookupCacheBypassReason,
    /// Canonical snapshot rebuilt without reading or writing shared cache state.
    snapshot: CanonicalLookupSnapshot,
  },
}

impl CanonicalLookupSnapshotCacheResult {
  /// Returns the public snapshot when this result obtained or rebuilt one.
  pub fn snapshot(&self) -> Option<&CanonicalLookupSnapshot> {
    match self {
      Self::Hit(snapshot) | Self::Miss(snapshot) | Self::Unavailable(snapshot) => Some(snapshot),
      Self::Bypass { snapshot, .. } => Some(snapshot),
    }
  }
}

/// Failure that prevents public canonical retrieval from safely producing a snapshot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupSnapshotCacheError {
  /// The authoritative canonical retrieval path could not produce a public fallback.
  #[error(transparent)]
  Retrieval(#[from] CanonicalRetrievalError),
  /// Content or retrieved candidates violated the public cache contract.
  #[error(transparent)]
  Contract(#[from] CanonicalLookupCacheValidationError),
  /// A cache lifetime of zero cannot store a meaningful snapshot.
  #[error("canonical lookup snapshot cache lifetime must be greater than zero")]
  ZeroTtl,
}

/// Caches rebuildable public canonical-card snapshots while refusing private lookup influences.
///
/// Cache reads and writes are strictly best effort. A miss or cache-adapter unavailability invokes
/// the canonical retrieval service; only a retrieval or public-contract failure is returned as an
/// error. A lexical-only fallback is intentionally not cached, so vector recovery can improve the
/// next lookup. When eligibility is a bypass, this service still performs the same pinned public
/// canonical retrieval but does not touch shared cache state; callers may apply their private or
/// personalized projection only after receiving that uncached base snapshot.
#[derive(Clone)]
pub struct CanonicalLookupSnapshotCacheService {
  retrieval: Arc<CanonicalRetrievalService>,
  cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>>,
  clock: Arc<dyn Clock>,
  ttl: Duration,
}

impl CanonicalLookupSnapshotCacheService {
  /// Creates a public snapshot cache service with an explicit positive lifetime.
  ///
  /// # Errors
  ///
  /// Returns an error when `ttl` is zero.
  pub fn new(
    retrieval: Arc<CanonicalRetrievalService>,
    cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>>,
    clock: Arc<dyn Clock>,
    ttl: Duration,
  ) -> Result<Self, CanonicalLookupSnapshotCacheError> {
    if ttl.is_zero() {
      return Err(CanonicalLookupSnapshotCacheError::ZeroTtl);
    }
    Ok(Self {
      retrieval,
      cache,
      clock,
      ttl,
    })
  }

  /// Gets or rebuilds a public canonical-card snapshot when its inputs are eligible for sharing.
  ///
  /// `eligibility` is intentionally separate from `request`: the public request has no fields for
  /// private data, while a private influence must produce [`CanonicalLookupSnapshotCacheResult::Bypass`]
  /// after rebuilding a public base snapshot without reading or writing cache state.
  ///
  /// # Errors
  ///
  /// Returns an error only when the authoritative canonical retrieval path or public snapshot
  /// contract cannot produce a safe result. Cache misses and cache adapter failures degrade to
  /// retrieval rather than becoming lookup failures.
  pub async fn lookup(
    &self,
    request: &PublicCanonicalLookupRequest,
    eligibility: CanonicalLookupCacheEligibility,
  ) -> Result<CanonicalLookupSnapshotCacheResult, CanonicalLookupSnapshotCacheError> {
    let content = self.retrieval.active_content_version().await?;
    let key = CanonicalLookupSnapshotKey::new(request, content)?;
    if let CanonicalLookupCacheEligibility::Bypass(reason) = eligibility {
      let (snapshot, _) = self.rebuild_snapshot(request, &key).await?;
      tracing::debug!(?reason, "bypassed shared canonical lookup snapshot cache");
      return Ok(CanonicalLookupSnapshotCacheResult::Bypass { reason, snapshot });
    }

    let cache_usable = match self.cache.get(&key).await {
      Ok(Some(entry)) if entry.value().matches_key(&key) => {
        tracing::debug!("served public canonical lookup snapshot cache hit");
        return Ok(CanonicalLookupSnapshotCacheResult::Hit(entry.into_value()));
      }
      Ok(Some(_)) => {
        tracing::warn!("discarded incompatible public canonical lookup snapshot cache entry");
        self.cache.remove(&key).await.is_ok()
      }
      Ok(None) => true,
      Err(_) => false,
    };

    let (snapshot, retrieval_path) = self.rebuild_snapshot(request, &key).await?;
    if !cache_usable {
      tracing::warn!("public canonical lookup snapshot cache unavailable; served rebuilt snapshot");
      return Ok(CanonicalLookupSnapshotCacheResult::Unavailable(snapshot));
    }
    if retrieval_path == RetrievalPath::LexicalFallback {
      tracing::debug!("kept lexical-only canonical lookup fallback out of shared cache");
      return Ok(CanonicalLookupSnapshotCacheResult::Miss(snapshot));
    }

    let Some(expires_at) = self.expires_at() else {
      tracing::warn!(
        "could not represent canonical lookup snapshot cache expiry; served rebuilt snapshot"
      );
      return Ok(CanonicalLookupSnapshotCacheResult::Unavailable(snapshot));
    };
    match self.cache.put(key, snapshot.clone(), expires_at).await {
      Ok(()) => {
        tracing::debug!("rebuilt and stored public canonical lookup snapshot cache miss");
        Ok(CanonicalLookupSnapshotCacheResult::Miss(snapshot))
      }
      Err(_) => {
        tracing::warn!(
          "public canonical lookup snapshot cache unavailable; served rebuilt snapshot"
        );
        Ok(CanonicalLookupSnapshotCacheResult::Unavailable(snapshot))
      }
    }
  }

  async fn rebuild_snapshot(
    &self,
    request: &PublicCanonicalLookupRequest,
    key: &CanonicalLookupSnapshotKey,
  ) -> Result<(CanonicalLookupSnapshot, RetrievalPath), CanonicalLookupSnapshotCacheError> {
    let outcome = self
      .retrieval
      .retrieve_with_content(request.retrieval().clone(), key.content().clone())
      .await?;
    let snapshot = CanonicalLookupSnapshot::new(key, outcome.candidates)?;
    Ok((snapshot, outcome.path))
  }

  fn expires_at(&self) -> Option<UtcTimestamp> {
    self.clock.now().checked_add(self.ttl)
  }
}

#[cfg(test)]
mod tests {
  use std::{
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use async_trait::async_trait;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock,
      in_memory::InMemoryCache,
      in_memory_retrieval::{InMemoryRetrievalAdapter, InMemoryVectorAvailability},
    },
    domain::{
      canonical::{
        ActiveContentVersion, CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
        EvidenceKind, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech, Sense, SourcePermissions,
        WordForm,
      },
      canonical_lookup_cache::{
        CanonicalCardPolicyVersions, CanonicalLookupCacheBypassReason, CanonicalLookupSnapshotKey,
      },
      retrieval::{
        CandidateFeatures, CanonicalCandidate, RankedCandidate, RetrievalScore, VectorMatch,
        VectorPurpose, VectorTarget,
      },
    },
    ports::{
      cache::{Cache, CacheEntry, CacheError},
      canonical_repository::{CanonicalRepository, CanonicalRepositoryError},
      vector_retriever::{VectorRetriever, VectorRetrieverError},
    },
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
      ranking_version: "lookup-rank-v1".to_string(),
    }
  }

  fn request() -> PublicCanonicalLookupRequest {
    PublicCanonicalLookupRequest::new(
      "hotter",
      language(),
      4,
      CanonicalCardPolicyVersions::new("retrieval-v1", "presentation-v1").unwrap(),
    )
    .unwrap()
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
        source_id: id("source-1"),
        source_reference: "definition-1".to_string(),
        release_id,
        language: language(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "having a high temperature".to_string(),
        content_hash: "hash-1".to_string(),
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
      score: RetrievalScore::new(9_200).unwrap(),
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      purpose: VectorPurpose::CanonicalEnglishSense,
      content_language: language(),
    }
  }

  fn retrieval_service(adapter: InMemoryRetrievalAdapter) -> Arc<CanonicalRetrievalService> {
    let adapter = Arc::new(adapter);
    let repository: Arc<dyn CanonicalRepository> = adapter.clone();
    let vector_retriever: Arc<dyn VectorRetriever> = adapter;
    Arc::new(CanonicalRetrievalService::new(repository, vector_retriever))
  }

  fn cache_service(
    adapter: InMemoryRetrievalAdapter,
    cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>>,
    clock: Arc<FixedClock>,
  ) -> CanonicalLookupSnapshotCacheService {
    CanonicalLookupSnapshotCacheService::new(
      retrieval_service(adapter),
      cache,
      clock,
      Duration::from_secs(30),
    )
    .unwrap()
  }

  fn clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::new(
      SystemTime::UNIX_EPOCH + Duration::from_secs(100),
    ))
  }

  #[tokio::test]
  async fn cache_miss_rebuilds_then_a_second_lookup_hits() {
    let clock = clock();
    let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> =
      Arc::new(InMemoryCache::new(clock.clone()));
    let service = cache_service(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_match(matching_vector()),
      cache,
      clock.clone(),
    );

    let first = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();
    let second = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();
    assert_eq!(
      clock.advance(Duration::from_secs(30)),
      Some(SystemTime::UNIX_EPOCH + Duration::from_secs(130))
    );
    let third = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();

    assert!(matches!(first, CanonicalLookupSnapshotCacheResult::Miss(_)));
    assert!(matches!(second, CanonicalLookupSnapshotCacheResult::Hit(_)));
    assert!(matches!(third, CanonicalLookupSnapshotCacheResult::Miss(_)));
  }

  #[tokio::test]
  async fn cache_unavailability_serves_rebuilt_canonical_snapshot() {
    let service = cache_service(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_match(matching_vector()),
      Arc::new(UnavailableCache),
      clock(),
    );

    let result = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();

    assert!(matches!(
      result,
      CanonicalLookupSnapshotCacheResult::Unavailable(snapshot) if snapshot.candidates().len() == 1
    ));
  }

  #[tokio::test]
  async fn lexical_fallback_is_returned_without_populating_the_shared_cache() {
    let clock = clock();
    let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> =
      Arc::new(InMemoryCache::new(clock.clone()));
    let service = cache_service(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_availability(InMemoryVectorAvailability::Unavailable),
      cache,
      clock,
    );

    let first = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();
    let second = service
      .lookup(&request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();

    assert!(matches!(first, CanonicalLookupSnapshotCacheResult::Miss(_)));
    assert!(matches!(
      second,
      CanonicalLookupSnapshotCacheResult::Miss(_)
    ));
  }

  #[tokio::test]
  async fn every_private_influence_bypasses_cache_but_rebuilds_canonical_snapshot() {
    let service = CanonicalLookupSnapshotCacheService::new(
      retrieval_service(
        InMemoryRetrievalAdapter::new(content())
          .with_candidate(candidate())
          .with_vector_match(matching_vector()),
      ),
      Arc::new(ForbiddenCache),
      clock(),
      Duration::from_secs(30),
    )
    .unwrap();
    let bypasses = [
      (
        CanonicalLookupCacheEligibility::with_context(),
        CanonicalLookupCacheBypassReason::Context,
      ),
      (
        CanonicalLookupCacheEligibility::with_identity(),
        CanonicalLookupCacheBypassReason::Identity,
      ),
      (
        CanonicalLookupCacheEligibility::with_history(),
        CanonicalLookupCacheBypassReason::History,
      ),
      (
        CanonicalLookupCacheEligibility::incognito(),
        CanonicalLookupCacheBypassReason::Incognito,
      ),
      (
        CanonicalLookupCacheEligibility::with_mature_content_choice(),
        CanonicalLookupCacheBypassReason::MatureContentChoice,
      ),
      (
        CanonicalLookupCacheEligibility::with_private_feedback(),
        CanonicalLookupCacheBypassReason::PrivateFeedback,
      ),
      (
        CanonicalLookupCacheEligibility::with_personal_projection(),
        CanonicalLookupCacheBypassReason::PersonalProjection,
      ),
    ];

    for (eligibility, reason) in bypasses {
      let result = service.lookup(&request(), eligibility).await.unwrap();
      assert!(matches!(
        result,
        CanonicalLookupSnapshotCacheResult::Bypass { reason: actual_reason, snapshot }
          if actual_reason == reason && snapshot.candidates().len() == 1
      ));
    }
  }

  #[tokio::test]
  async fn compatible_preloaded_snapshot_hits_without_retrieval() {
    let request = request();
    let key = CanonicalLookupSnapshotKey::new(&request, content()).unwrap();
    let snapshot = CanonicalLookupSnapshot::new(
      &key,
      vec![RankedCandidate {
        candidate: candidate(),
        features: CandidateFeatures::default(),
        rank: 1,
        fusion_score: 0,
      }],
    )
    .unwrap();
    let clock = clock();
    let cache = Arc::new(InMemoryCache::new(clock.clone()));
    cache
      .put(key, snapshot.clone(), clock.now() + Duration::from_secs(30))
      .await
      .unwrap();
    let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> = cache;
    let service = CanonicalLookupSnapshotCacheService::new(
      Arc::new(CanonicalRetrievalService::new(
        Arc::new(ActiveOnlyRepository),
        Arc::new(UnavailableVectorRetriever),
      )),
      cache,
      clock,
      Duration::from_secs(30),
    )
    .unwrap();

    let result = service
      .lookup(&request, CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();

    assert_eq!(result, CanonicalLookupSnapshotCacheResult::Hit(snapshot));
  }

  struct UnavailableCache;

  #[async_trait]
  impl Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot> for UnavailableCache {
    async fn get(
      &self,
      _key: &CanonicalLookupSnapshotKey,
    ) -> Result<Option<CacheEntry<CanonicalLookupSnapshot>>, CacheError> {
      Err(CacheError::Unavailable)
    }

    async fn put(
      &self,
      _key: CanonicalLookupSnapshotKey,
      _value: CanonicalLookupSnapshot,
      _expires_at: UtcTimestamp,
    ) -> Result<(), CacheError> {
      Err(CacheError::Unavailable)
    }

    async fn remove(&self, _key: &CanonicalLookupSnapshotKey) -> Result<(), CacheError> {
      Err(CacheError::Unavailable)
    }
  }

  struct ForbiddenCache;

  #[async_trait]
  impl Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot> for ForbiddenCache {
    async fn get(
      &self,
      _key: &CanonicalLookupSnapshotKey,
    ) -> Result<Option<CacheEntry<CanonicalLookupSnapshot>>, CacheError> {
      panic!("a private-influence bypass must not read the shared cache");
    }

    async fn put(
      &self,
      _key: CanonicalLookupSnapshotKey,
      _value: CanonicalLookupSnapshot,
      _expires_at: UtcTimestamp,
    ) -> Result<(), CacheError> {
      panic!("a private-influence bypass must not write the shared cache");
    }

    async fn remove(&self, _key: &CanonicalLookupSnapshotKey) -> Result<(), CacheError> {
      panic!("a private-influence bypass must not remove shared cache state");
    }
  }

  struct ActiveOnlyRepository;

  #[async_trait]
  impl CanonicalRepository for ActiveOnlyRepository {
    async fn active_content_version(
      &self,
    ) -> Result<ActiveContentVersion, CanonicalRepositoryError> {
      Ok(content())
    }

    async fn search_lexical(
      &self,
      _request: &crate::domain::retrieval::LexicalSearchRequest,
    ) -> Result<Vec<crate::domain::retrieval::RepositoryMatch>, CanonicalRepositoryError> {
      Err(CanonicalRepositoryError::Unavailable)
    }

    async fn load_candidates(
      &self,
      _request: &crate::domain::retrieval::CandidateLoadRequest,
    ) -> Result<Vec<CanonicalCandidate>, CanonicalRepositoryError> {
      Err(CanonicalRepositoryError::Unavailable)
    }
  }

  struct UnavailableVectorRetriever;

  #[async_trait]
  impl VectorRetriever for UnavailableVectorRetriever {
    async fn search(
      &self,
      _request: &crate::domain::retrieval::VectorSearchRequest,
    ) -> Result<Vec<VectorMatch>, VectorRetrieverError> {
      Err(VectorRetrieverError::Unavailable)
    }
  }
}
