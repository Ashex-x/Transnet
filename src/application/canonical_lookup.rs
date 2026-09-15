//! Request-scoped canonical lookup without query-derived caches or persistence ports.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  application::{
    canonical_lookup_card::CanonicalLookupCardMapper,
    observability::ClosedMetricsDispatcher,
    retrieval::{CanonicalRetrievalError, CanonicalRetrievalService, RetrievalPath},
  },
  domain::{
    lookup_card::CanonicalLookupCard,
    observability::{LookupStage, MetricEvent, MetricOutcome},
    retrieval::RetrievalRequest,
  },
  ports::canonical_repository::CanonicalRepositoryError,
};

/// Failure to obtain one consistent evidence-backed canonical card.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupError {
  /// The active immutable content version could not be selected.
  #[error(transparent)]
  ContentResolution(CanonicalRetrievalError),
  /// Authoritative candidate retrieval failed.
  #[error(transparent)]
  Retrieval(CanonicalRetrievalError),
  /// The active content metadata is incomplete.
  #[error("invalid canonical content version")]
  InvalidContentVersion,
}

impl CanonicalLookupError {
  /// Returns whether a transient authoritative dependency outage permits a later retry.
  pub fn is_retryable(&self) -> bool {
    matches!(
      self,
      Self::ContentResolution(CanonicalRetrievalError::Repository(
        CanonicalRepositoryError::Unavailable
      )) | Self::Retrieval(CanonicalRetrievalError::Repository(
        CanonicalRepositoryError::Unavailable
      ))
    )
  }
}

/// Reads and assembles canonical cards using only request-local query state.
///
/// No cache, queue, or mutation port is accepted. Even a query without identity or context remains
/// request content: neither its text nor its fingerprint may be retained between calls.
#[derive(Clone)]
pub struct CanonicalLookupService {
  retrieval: Arc<CanonicalRetrievalService>,
  metrics: Option<Arc<ClosedMetricsDispatcher>>,
}

impl CanonicalLookupService {
  /// Creates a service from read-only canonical retrieval dependencies.
  pub fn new(retrieval: Arc<CanonicalRetrievalService>) -> Self {
    Self {
      retrieval,
      metrics: None,
    }
  }

  /// Adds bounded, response-neutral delivery of content-free lookup stage outcomes.
  pub fn with_metrics_dispatcher(mut self, dispatcher: Arc<ClosedMetricsDispatcher>) -> Self {
    self.metrics = Some(dispatcher);
    self
  }

  /// Pins one content version, retrieves candidates, and assembles a card without retaining input.
  ///
  /// # Errors
  ///
  /// Returns a typed error for an unavailable or inconsistent authoritative repository. Vector
  /// unavailability retains the explicit lexical-only fallback behavior.
  pub async fn lookup(
    &self,
    request: RetrievalRequest,
  ) -> Result<CanonicalLookupCard, CanonicalLookupError> {
    self.record(LookupStage::RequestValidation, MetricOutcome::Succeeded);
    let content = self
      .retrieval
      .active_content_version()
      .await
      .map_err(|error| {
        self.record(LookupStage::ContentResolution, MetricOutcome::Failed);
        CanonicalLookupError::ContentResolution(error)
      })?;
    if content.schema_version.trim().is_empty() || content.ranking_version.trim().is_empty() {
      return Err(CanonicalLookupError::InvalidContentVersion);
    }
    self.record(LookupStage::ContentResolution, MetricOutcome::Succeeded);
    let outcome = self
      .retrieval
      .retrieve_with_content(request.clone(), content)
      .await
      .map_err(|error| {
        self.record(LookupStage::CandidateRetrieval, MetricOutcome::Failed);
        CanonicalLookupError::Retrieval(error)
      })?;
    self.record(
      LookupStage::CandidateRetrieval,
      match outcome.path {
        RetrievalPath::Hybrid => MetricOutcome::Succeeded,
        RetrievalPath::LexicalFallback => MetricOutcome::Degraded,
      },
    );
    let card = CanonicalLookupCardMapper::assemble(&request, outcome);
    self.record(LookupStage::ResponseAssembly, MetricOutcome::Succeeded);
    Ok(card)
  }

  fn record(&self, stage: LookupStage, outcome: MetricOutcome) {
    if let Some(metrics) = &self.metrics {
      metrics.dispatch(MetricEvent::LookupStage { stage, outcome });
    }
  }
}
