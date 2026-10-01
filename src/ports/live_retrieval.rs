//! Injectable live search, public fetch, and DNS boundaries.

use crate::domain::{
  live_retrieval::{LiveFetchedPage, LiveRetrievalError, LiveSearchQuery, LiveSearchResult},
  model_runtime::CancellationSignal,
  request_context::RequestContext,
};
use async_trait::async_trait;
use std::net::IpAddr;

/// Executes the sole bounded search round.
#[async_trait]
pub trait LiveSearchPort: Send + Sync {
  /// Searches once under the caller deadline.
  async fn search(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    query: &LiveSearchQuery,
  ) -> Result<Vec<LiveSearchResult>, LiveRetrievalError>;
}
/// Fetches one already nominated public page safely.
#[async_trait]
pub trait LiveFetchPort: Send + Sync {
  /// Fetches and extracts one textual page.
  async fn fetch(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    result: LiveSearchResult,
  ) -> Result<LiveFetchedPage, LiveRetrievalError>;
}
/// Resolves one host for public-address validation and connection pinning.
#[async_trait]
pub trait LiveDnsPort: Send + Sync {
  /// Resolves all addresses without returning private diagnostics.
  async fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, LiveRetrievalError>;
}
