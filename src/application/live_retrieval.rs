//! One-shot bounded live retrieval over injectable search and fetch ports.

use crate::{
  domain::{
    live_retrieval::*, model_runtime::CancellationSignal, request_context::RequestContext,
    translation_turn::FreshnessPolicy,
  },
  ports::live_retrieval::{LiveFetchPort, LiveSearchPort},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

/// Deterministic facts used to decide whether policy permits the sole retrieval round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveRetrievalDecision {
  /// Whether the input contains a freshness-sensitive claim.
  pub freshness_sensitive: bool,
  /// Whether canonical material is insufficient for that claim.
  pub canonical_insufficient: bool,
}

/// Stateless service whose operation contains exactly one possible search round.
pub struct LiveRetrievalService {
  search: Arc<dyn LiveSearchPort>,
  fetch: Arc<dyn LiveFetchPort>,
  subdeadline: Duration,
}

impl LiveRetrievalService {
  /// Creates a request-scoped service from isolated search and fetch boundaries.
  pub fn new(search: Arc<dyn LiveSearchPort>, fetch: Arc<dyn LiveFetchPort>) -> Self {
    Self {
      search,
      fetch,
      subdeadline: DEFAULT_LIVE_SUBDEADLINE,
    }
  }

  /// Selects a shared nonzero subdeadline no greater than fifteen seconds.
  pub fn with_subdeadline(mut self, subdeadline: Duration) -> Result<Self, LiveRetrievalError> {
    if subdeadline.is_zero() || subdeadline > MAX_LIVE_SUBDEADLINE {
      return Err(LiveRetrievalError::Invalid);
    }
    self.subdeadline = subdeadline;
    Ok(self)
  }

  /// Executes policy, one search, and at most three concurrent fetches.
  pub async fn retrieve(
    &self,
    context: &RequestContext,
    cancellation: Arc<CancellationSignal>,
    policy: FreshnessPolicy,
    decision: LiveRetrievalDecision,
    query: LiveSearchQuery,
  ) -> Result<Option<LiveRetrievalMaterial>, LiveRetrievalError> {
    ensure_active(context, &cancellation)?;
    let budget = context.remaining_budget().min(self.subdeadline);
    tokio::select! {
      _ = cancellation.cancelled() => Err(LiveRetrievalError::Cancelled),
      _ = tokio::time::sleep(budget) => Err(LiveRetrievalError::DeadlineExceeded),
      result = self.retrieve_once(context, cancellation.clone(), policy, decision, query) => result,
    }
  }

  async fn retrieve_once(
    &self,
    context: &RequestContext,
    cancellation: Arc<CancellationSignal>,
    policy: FreshnessPolicy,
    decision: LiveRetrievalDecision,
    query: LiveSearchQuery,
  ) -> Result<Option<LiveRetrievalMaterial>, LiveRetrievalError> {
    let should_run = match policy {
      FreshnessPolicy::Offline => false,
      FreshnessPolicy::Allowed => decision.freshness_sensitive && decision.canonical_insufficient,
      FreshnessPolicy::Required => true,
    };
    if !should_run {
      return Ok(None);
    }
    ensure_active(context, &cancellation)?;
    let mut results = match self.search.search(context, &cancellation, &query).await {
      Ok(results) => results,
      Err(LiveRetrievalError::Cancelled) => return Err(LiveRetrievalError::Cancelled),
      Err(LiveRetrievalError::DeadlineExceeded) => {
        return Err(LiveRetrievalError::DeadlineExceeded)
      }
      Err(_) if policy == FreshnessPolicy::Allowed => return Ok(None),
      Err(_) => return Err(LiveRetrievalError::Unavailable),
    };
    ensure_active(context, &cancellation)?;
    results.truncate(MAX_LIVE_SEARCH_RESULTS);
    results.truncate(MAX_LIVE_FETCHES);
    let mut set = JoinSet::new();
    for (ordinal, result) in results.into_iter().enumerate() {
      let fetch = self.fetch.clone();
      let context = context.clone();
      let cancellation = cancellation.clone();
      set.spawn(async move { (ordinal, fetch.fetch(&context, &cancellation, result).await) });
    }
    let mut pages = Vec::new();
    while let Some(joined) = set.join_next().await {
      ensure_active(context, &cancellation)?;
      if let Ok((ordinal, Ok(page))) = joined {
        pages.push((ordinal, page));
      }
    }
    pages.sort_by_key(|(ordinal, _)| *ordinal);
    let pages = pages.into_iter().map(|(_, page)| page).collect::<Vec<_>>();
    if pages.is_empty() {
      return if policy == FreshnessPolicy::Required {
        Err(LiveRetrievalError::Unavailable)
      } else {
        Ok(None)
      };
    }
    if pages.iter().map(|page| page.byte_count).sum::<usize>() > MAX_LIVE_AGGREGATE_BYTES {
      return Err(LiveRetrievalError::Invalid);
    }
    let citations = (0..pages.len())
      .map(|index| {
        let id = format!("live_{}", index + 1);
        LiveCitation {
          source_id: id.clone(),
          source_ids: vec![id],
          evidence_state: "live_external",
        }
      })
      .collect();
    Ok(Some(LiveRetrievalMaterial { pages, citations }))
  }
}

fn ensure_active(
  context: &RequestContext,
  cancellation: &CancellationSignal,
) -> Result<(), LiveRetrievalError> {
  if cancellation.is_cancelled() {
    Err(LiveRetrievalError::Cancelled)
  } else if context.remaining_budget().is_zero() {
    Err(LiveRetrievalError::DeadlineExceeded)
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use async_trait::async_trait;
  use std::{
    sync::{
      atomic::{AtomicUsize, Ordering},
      Mutex,
    },
    time::Duration,
  };
  use time::OffsetDateTime;
  use url::Url;

  use crate::{
    domain::request_context::RequestId,
    ports::live_retrieval::{LiveFetchPort, LiveSearchPort},
  };

  struct FakeSearch {
    calls: AtomicUsize,
    result: Result<Vec<LiveSearchResult>, LiveRetrievalError>,
  }

  #[async_trait]
  impl LiveSearchPort for FakeSearch {
    async fn search(
      &self,
      _context: &RequestContext,
      _cancellation: &CancellationSignal,
      _query: &LiveSearchQuery,
    ) -> Result<Vec<LiveSearchResult>, LiveRetrievalError> {
      self.calls.fetch_add(1, Ordering::SeqCst);
      self.result.clone()
    }
  }

  struct FakeFetch {
    calls: AtomicUsize,
    active: AtomicUsize,
    max_active: AtomicUsize,
    failures: Mutex<Vec<usize>>,
    byte_count: usize,
  }

  #[async_trait]
  impl LiveFetchPort for FakeFetch {
    async fn fetch(
      &self,
      _context: &RequestContext,
      _cancellation: &CancellationSignal,
      result: LiveSearchResult,
    ) -> Result<LiveFetchedPage, LiveRetrievalError> {
      let call = self.calls.fetch_add(1, Ordering::SeqCst);
      let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
      self.max_active.fetch_max(active, Ordering::SeqCst);
      tokio::time::sleep(Duration::from_millis(10 - (call as u64 * 2))).await;
      self.active.fetch_sub(1, Ordering::SeqCst);
      if self.failures.lock().unwrap().contains(&call) {
        return Err(LiveRetrievalError::Unavailable);
      }
      LiveFetchedPage::new(
        result,
        format!("page-{call}"),
        "2026-10-02T00:00:00Z".into(),
        self.byte_count,
      )
    }
  }

  fn context() -> RequestContext {
    let now = OffsetDateTime::now_utc();
    let deadline = (now + time::Duration::seconds(5))
      .replace_nanosecond(now.nanosecond() / 1_000 * 1_000)
      .unwrap();
    RequestContext::new(
      RequestId::new("live-orchestration").unwrap(),
      deadline,
      "transnet-v1",
      None,
    )
    .unwrap()
  }

  fn search_results(count: usize) -> Vec<LiveSearchResult> {
    (0..count)
      .map(|index| {
        LiveSearchResult::new(
          Url::parse(&format!("https://example.com/{index}")).unwrap(),
          format!("title-{index}"),
          None,
          None,
        )
        .unwrap()
      })
      .collect()
  }

  fn service(
    search_result: Result<Vec<LiveSearchResult>, LiveRetrievalError>,
    failures: Vec<usize>,
    byte_count: usize,
  ) -> (Arc<FakeSearch>, Arc<FakeFetch>, LiveRetrievalService) {
    let search = Arc::new(FakeSearch {
      calls: AtomicUsize::new(0),
      result: search_result,
    });
    let fetch = Arc::new(FakeFetch {
      calls: AtomicUsize::new(0),
      active: AtomicUsize::new(0),
      max_active: AtomicUsize::new(0),
      failures: Mutex::new(failures),
      byte_count,
    });
    let service = LiveRetrievalService::new(search.clone(), fetch.clone());
    (search, fetch, service)
  }

  fn decision(freshness_sensitive: bool, canonical_insufficient: bool) -> LiveRetrievalDecision {
    LiveRetrievalDecision {
      freshness_sensitive,
      canonical_insufficient,
    }
  }

  #[tokio::test]
  async fn offline_and_ineligible_allowed_never_search() {
    for (policy, decision) in [
      (FreshnessPolicy::Offline, decision(true, true)),
      (FreshnessPolicy::Allowed, decision(false, true)),
      (FreshnessPolicy::Allowed, decision(true, false)),
    ] {
      let (search, fetch, service) = service(Ok(search_results(5)), vec![], 10);
      let material = service
        .retrieve(
          &context(),
          Arc::new(CancellationSignal::default()),
          policy,
          decision,
          LiveSearchQuery::new("private query").unwrap(),
        )
        .await
        .unwrap();
      assert!(material.is_none());
      assert_eq!(search.calls.load(Ordering::SeqCst), 0);
      assert_eq!(fetch.calls.load(Ordering::SeqCst), 0);
    }
  }

  #[tokio::test]
  async fn spends_one_search_and_fetches_three_concurrently_in_nomination_order() {
    let (search, fetch, service) = service(Ok(search_results(7)), vec![], 10);
    let material = service
      .retrieve(
        &context(),
        Arc::new(CancellationSignal::default()),
        FreshnessPolicy::Required,
        decision(false, false),
        LiveSearchQuery::new("private query").unwrap(),
      )
      .await
      .unwrap()
      .unwrap();
    assert_eq!(search.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fetch.calls.load(Ordering::SeqCst), 3);
    assert_eq!(fetch.max_active.load(Ordering::SeqCst), 3);
    assert_eq!(
      material
        .pages
        .iter()
        .map(|page| page.fragment())
        .collect::<Vec<_>>(),
      vec!["page-0", "page-1", "page-2"]
    );
    assert_eq!(
      material
        .citations
        .iter()
        .map(|citation| citation.source_id.as_str())
        .collect::<Vec<_>>(),
      vec!["live_1", "live_2", "live_3"]
    );
    assert!(material
      .citations
      .iter()
      .all(|citation| citation.evidence_state == "live_external"));
  }

  #[tokio::test]
  async fn keeps_partial_success_but_required_fails_when_every_fetch_fails() {
    let (_, _, partial_service) = service(Ok(search_results(3)), vec![1, 2], 10);
    let partial = partial_service
      .retrieve(
        &context(),
        Arc::new(CancellationSignal::default()),
        FreshnessPolicy::Required,
        decision(false, false),
        LiveSearchQuery::new("query").unwrap(),
      )
      .await
      .unwrap()
      .unwrap();
    assert_eq!(partial.pages.len(), 1);

    let (_, _, failed_service) = service(Ok(search_results(3)), vec![0, 1, 2], 10);
    assert_eq!(
      failed_service
        .retrieve(
          &context(),
          Arc::new(CancellationSignal::default()),
          FreshnessPolicy::Required,
          decision(false, false),
          LiveSearchQuery::new("query").unwrap()
        )
        .await
        .unwrap_err(),
      LiveRetrievalError::Unavailable
    );
  }

  #[tokio::test]
  async fn allowed_dependency_failure_degrades_while_required_fails() {
    for policy in [FreshnessPolicy::Allowed, FreshnessPolicy::Required] {
      let (_, _, service) = service(Err(LiveRetrievalError::Unavailable), vec![], 10);
      let actual = service
        .retrieve(
          &context(),
          Arc::new(CancellationSignal::default()),
          policy,
          decision(true, true),
          LiveSearchQuery::new("query").unwrap(),
        )
        .await;
      match policy {
        FreshnessPolicy::Allowed => assert!(matches!(actual, Ok(None))),
        FreshnessPolicy::Required => {
          assert!(matches!(actual, Err(LiveRetrievalError::Unavailable)))
        }
        FreshnessPolicy::Offline => unreachable!(),
      }
    }
  }

  #[tokio::test]
  async fn rejects_aggregate_larger_than_one_mebibyte() {
    let (_, _, service) = service(Ok(search_results(3)), vec![], 400 * 1024);
    assert_eq!(
      service
        .retrieve(
          &context(),
          Arc::new(CancellationSignal::default()),
          FreshnessPolicy::Required,
          decision(false, false),
          LiveSearchQuery::new("query").unwrap()
        )
        .await
        .unwrap_err(),
      LiveRetrievalError::Invalid
    );
  }

  #[test]
  fn request_local_values_redact_and_drop_without_persistence() {
    let query = LiveSearchQuery::new("seeded private query").unwrap();
    let result = search_results(1).remove(0);
    let page = LiveFetchedPage::new(
      result,
      "seeded private page".into(),
      "2026-10-02T00:00:00Z".into(),
      32,
    )
    .unwrap();
    let material = LiveRetrievalMaterial {
      pages: vec![page],
      citations: vec![],
    };
    for debug in [format!("{query:?}"), format!("{material:?}")] {
      assert!(!debug.contains("seeded private"));
      assert!(!debug.contains("example.com"));
    }
    drop(material);
  }

  #[test]
  fn validates_shared_live_subdeadline() {
    let (_, _, service_value) = service(Ok(vec![]), vec![], 10);
    assert!(service_value.with_subdeadline(Duration::ZERO).is_err());
    let (_, _, service_value) = service(Ok(vec![]), vec![], 10);
    assert!(service_value
      .with_subdeadline(Duration::from_secs(16))
      .is_err());
    let (_, _, service_value) = service(Ok(vec![]), vec![], 10);
    assert!(service_value
      .with_subdeadline(Duration::from_secs(15))
      .is_ok());
  }
}
