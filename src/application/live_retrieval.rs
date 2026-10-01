//! One-shot bounded live retrieval over injectable search and fetch ports.

use crate::{
  domain::{
    live_retrieval::*, model_runtime::CancellationSignal, request_context::RequestContext,
    translation_turn::FreshnessPolicy,
  },
  ports::live_retrieval::{LiveFetchPort, LiveSearchPort},
};
use std::sync::Arc;
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
}

impl LiveRetrievalService {
  /// Creates a request-scoped service from isolated search and fetch boundaries.
  pub fn new(search: Arc<dyn LiveSearchPort>, fetch: Arc<dyn LiveFetchPort>) -> Self {
    Self { search, fetch }
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
    let should_run = match policy {
      FreshnessPolicy::Offline => false,
      FreshnessPolicy::Allowed => decision.freshness_sensitive && decision.canonical_insufficient,
      FreshnessPolicy::Required => true,
    };
    if !should_run {
      return Ok(None);
    }
    ensure_active(context, &cancellation)?;
    let mut results = self.search.search(context, &cancellation, &query).await?;
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
