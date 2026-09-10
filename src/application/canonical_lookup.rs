//! Evidence-backed canonical lookup composition for transport adapters.
//!
//! The service obtains a public-only snapshot through the cache foundation, then maps the pinned
//! retrieval outcome into the canonical-card contract. It never invokes the learning model and it
//! never applies identity, context, history, mature-content, or learner-specific policy.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  application::{
    canonical_lookup_cache::{
      CanonicalLookupSnapshotCacheError, CanonicalLookupSnapshotCacheResult,
      CanonicalLookupSnapshotCacheService,
    },
    canonical_lookup_card::CanonicalLookupCardMapper,
    observability::ClosedMetricsDispatcher,
    retrieval::{RetrievalOutcome, RetrievalPath},
  },
  domain::{
    canonical_lookup_cache::{CanonicalLookupCacheEligibility, PublicCanonicalLookupRequest},
    lookup_card::CanonicalLookupCard,
    observability::{LookupStage, MetricEvent, MetricOutcome},
  },
};

/// Internal composition result that retains how a canonical snapshot was obtained.
///
/// Cache-result provenance is needed only for closed metric attribution. It never crosses the
/// application boundary or changes the public canonical card contract.
struct CanonicalLookupComposition {
  card: CanonicalLookupCard,
  snapshot_result: CanonicalLookupSnapshotCacheResult,
}

/// Failure while obtaining a deterministic evidence-backed canonical lookup card.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupError {
  /// The public snapshot cache or authoritative canonical retrieval could not produce a card.
  #[error(transparent)]
  Snapshot(#[from] CanonicalLookupSnapshotCacheError),
}

impl CanonicalLookupError {
  /// Returns whether retrying the lookup can recover a transient canonical dependency failure.
  ///
  /// Integrity, cache-contract, and configuration failures remain safely typed but are not
  /// retryable because a caller cannot repair them by repeating the same request.
  pub fn is_retryable(&self) -> bool {
    match self {
      Self::Snapshot(error) => error.is_retryable(),
    }
  }
}

/// Coordinates public snapshot retrieval and canonical-card assembly.
///
/// The service accepts an explicit cache eligibility declaration so callers must classify private
/// influences before lookup. A bypass still returns an uncached public base card; private
/// projection remains outside this service.
#[derive(Clone)]
pub struct CanonicalLookupService {
  snapshots: Arc<CanonicalLookupSnapshotCacheService>,
  metrics: Option<Arc<ClosedMetricsDispatcher>>,
}

impl CanonicalLookupService {
  /// Creates a canonical lookup service around an explicit public snapshot-cache service.
  pub fn new(snapshots: Arc<CanonicalLookupSnapshotCacheService>) -> Self {
    Self {
      snapshots,
      metrics: None,
    }
  }

  /// Adds bounded, response-neutral delivery for canonical lookup metric events.
  ///
  /// The service emits only the closed lookup-stage catalog: typed request validation, selected
  /// content, candidate retrieval, and card assembly. A lexical fallback is reported as degraded;
  /// cache hit or miss details are intentionally not emitted because the catalog has no truthful
  /// cache dimension. Ambiguous cache-contract or configuration failures are deliberately not
  /// attributed to a lookup stage. This service never emits a learning-model validation event.
  pub fn with_metrics_dispatcher(mut self, dispatcher: Arc<ClosedMetricsDispatcher>) -> Self {
    self.metrics = Some(dispatcher);
    self
  }

  /// Obtains a pinned, evidence-backed canonical lookup card without model generation.
  ///
  /// # Errors
  ///
  /// Returns an error when authoritative canonical retrieval cannot produce a safe public
  /// snapshot. Cache adapter misses and unavailability retain their best-effort fallback behavior.
  pub async fn lookup(
    &self,
    request: PublicCanonicalLookupRequest,
    eligibility: CanonicalLookupCacheEligibility,
  ) -> Result<CanonicalLookupCard, CanonicalLookupError> {
    let result = self.lookup_inner(request, eligibility).await;
    self.record_lookup_outcome(&result);
    result.map(|composition| composition.card)
  }

  async fn lookup_inner(
    &self,
    request: PublicCanonicalLookupRequest,
    eligibility: CanonicalLookupCacheEligibility,
  ) -> Result<CanonicalLookupComposition, CanonicalLookupError> {
    let snapshot_result = self.snapshots.lookup(&request, eligibility).await?;
    let snapshot = match &snapshot_result {
      CanonicalLookupSnapshotCacheResult::Hit(snapshot)
      | CanonicalLookupSnapshotCacheResult::Miss(snapshot)
      | CanonicalLookupSnapshotCacheResult::Unavailable(snapshot)
      | CanonicalLookupSnapshotCacheResult::Bypass { snapshot, .. } => snapshot,
    };
    let outcome = RetrievalOutcome {
      content: snapshot.content().clone(),
      path: snapshot.retrieval_path(),
      candidates: snapshot.candidates().to_vec(),
    };
    Ok(CanonicalLookupComposition {
      card: CanonicalLookupCardMapper::assemble(request.retrieval(), outcome),
      snapshot_result,
    })
  }

  fn record_lookup_outcome(
    &self,
    result: &Result<CanonicalLookupComposition, CanonicalLookupError>,
  ) {
    let Some(metrics) = &self.metrics else {
      return;
    };

    metrics.dispatch(MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Succeeded,
    });
    match result {
      Ok(composition) => {
        metrics.dispatch(MetricEvent::LookupStage {
          stage: LookupStage::ContentResolution,
          outcome: MetricOutcome::Succeeded,
        });
        if !matches!(
          &composition.snapshot_result,
          CanonicalLookupSnapshotCacheResult::Hit(_)
        ) {
          metrics.dispatch(MetricEvent::LookupStage {
            stage: LookupStage::CandidateRetrieval,
            outcome: match composition.snapshot_result.retrieval_path() {
              RetrievalPath::Hybrid => MetricOutcome::Succeeded,
              RetrievalPath::LexicalFallback => MetricOutcome::Degraded,
            },
          });
        }
        metrics.dispatch(MetricEvent::LookupStage {
          stage: LookupStage::ResponseAssembly,
          outcome: MetricOutcome::Succeeded,
        });
      }
      Err(CanonicalLookupError::Snapshot(
        CanonicalLookupSnapshotCacheError::ContentResolution(_),
      )) => {
        metrics.dispatch(MetricEvent::LookupStage {
          stage: LookupStage::ContentResolution,
          outcome: MetricOutcome::Failed,
        });
      }
      Err(CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Retrieval(_))) => {
        metrics.dispatch(MetricEvent::LookupStage {
          stage: LookupStage::CandidateRetrieval,
          outcome: MetricOutcome::Failed,
        });
      }
      Err(CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Contract(_)))
      | Err(CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::ZeroTtl)) => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock,
      in_memory::{InMemoryCache, InMemoryMetricsRecorder},
      in_memory_retrieval::InMemoryRetrievalAdapter,
    },
    application::retrieval::{CanonicalRetrievalError, CanonicalRetrievalService},
    domain::{
      canonical::{ActiveContentVersion, CanonicalId, LanguageTag},
      canonical_lookup_cache::{
        CanonicalCardPolicyVersions, CanonicalLookupCacheValidationError, CanonicalLookupSnapshot,
        CanonicalLookupSnapshotKey,
      },
    },
    ports::{
      cache::Cache,
      canonical_repository::{CanonicalRepository, CanonicalRepositoryError},
      vector_retriever::VectorRetriever,
    },
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn content() -> ActiveContentVersion {
    ActiveContentVersion {
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "rank-v1".to_string(),
    }
  }

  fn request() -> PublicCanonicalLookupRequest {
    PublicCanonicalLookupRequest::new(
      "hot",
      LanguageTag::parse("en").unwrap(),
      4,
      CanonicalCardPolicyVersions::new("retrieval-v1", "presentation-v1").unwrap(),
    )
    .unwrap()
  }

  async fn recorded_events(
    recorder: &InMemoryMetricsRecorder,
    expected_count: usize,
  ) -> Vec<MetricEvent> {
    tokio::time::timeout(Duration::from_secs(1), async {
      loop {
        let events = recorder.events().await;
        if events.len() >= expected_count {
          return events;
        }
        tokio::task::yield_now().await;
      }
    })
    .await
    .expect("metrics recorder should receive the expected closed events")
  }

  async fn settled_events(recorder: &InMemoryMetricsRecorder) -> Vec<MetricEvent> {
    for _ in 0..4 {
      tokio::task::yield_now().await;
    }
    recorder.events().await
  }

  #[test]
  fn only_transient_canonical_repository_unavailability_is_retryable() {
    let unavailable_content =
      CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::ContentResolution(
        CanonicalRetrievalError::Repository(CanonicalRepositoryError::Unavailable),
      ));
    let unavailable = CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Retrieval(
      CanonicalRetrievalError::Repository(CanonicalRepositoryError::Unavailable),
    ));
    let inconsistent =
      CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Retrieval(
        CanonicalRetrievalError::Repository(CanonicalRepositoryError::InconsistentData),
      ));
    let contract = CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::Contract(
      CanonicalLookupCacheValidationError::CandidateLimitExceeded,
    ));
    let invalid_configuration =
      CanonicalLookupError::Snapshot(CanonicalLookupSnapshotCacheError::ZeroTtl);

    assert!(unavailable_content.is_retryable());
    assert!(unavailable.is_retryable());
    assert!(!inconsistent.is_retryable());
    assert!(!contract.is_retryable());
    assert!(!invalid_configuration.is_retryable());
  }

  #[tokio::test]
  async fn shared_snapshot_cache_hits_omit_candidate_retrieval_metrics() {
    let clock = Arc::new(FixedClock::new(
      SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
    ));
    let adapter = Arc::new(InMemoryRetrievalAdapter::new(content()));
    let repository: Arc<dyn CanonicalRepository> = adapter.clone();
    let vectors: Arc<dyn VectorRetriever> = adapter;
    let retrieval = Arc::new(CanonicalRetrievalService::new(repository, vectors));
    let cache: Arc<dyn Cache<CanonicalLookupSnapshotKey, CanonicalLookupSnapshot>> =
      Arc::new(InMemoryCache::new(clock.clone()));
    let snapshots = Arc::new(
      CanonicalLookupSnapshotCacheService::new(retrieval, cache, clock, Duration::from_secs(30))
        .unwrap(),
    );
    let recorder = InMemoryMetricsRecorder::new();
    let service = CanonicalLookupService::new(snapshots).with_metrics_dispatcher(Arc::new(
      ClosedMetricsDispatcher::new(Arc::new(recorder.clone())),
    ));

    service
      .lookup(request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();
    let _ = recorded_events(&recorder, 4).await;
    assert_eq!(settled_events(&recorder).await.len(), 4);

    recorder.clear().await;

    service
      .lookup(request(), CanonicalLookupCacheEligibility::public())
      .await
      .unwrap();
    let _ = recorded_events(&recorder, 3).await;
    assert_eq!(
      settled_events(&recorder).await,
      vec![
        MetricEvent::LookupStage {
          stage: LookupStage::RequestValidation,
          outcome: MetricOutcome::Succeeded,
        },
        MetricEvent::LookupStage {
          stage: LookupStage::ContentResolution,
          outcome: MetricOutcome::Succeeded,
        },
        MetricEvent::LookupStage {
          stage: LookupStage::ResponseAssembly,
          outcome: MetricOutcome::Succeeded,
        },
      ]
    );
  }
}
