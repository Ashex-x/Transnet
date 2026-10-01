//! Bounded request-local live retrieval values with content-free diagnostics.

use std::{fmt, time::Duration};

use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use url::Url;

/// Maximum results admitted from the sole search round.
pub const MAX_LIVE_SEARCH_RESULTS: usize = 5;
/// Maximum pages fetched for one request.
pub const MAX_LIVE_FETCHES: usize = 3;
/// Maximum redirects followed for one page.
pub const MAX_LIVE_REDIRECTS: usize = 3;
/// Maximum bytes accepted from one page.
pub const MAX_LIVE_PAGE_BYTES: usize = 512 * 1024;
/// Maximum bytes accepted across fetched pages.
pub const MAX_LIVE_AGGREGATE_BYTES: usize = 1024 * 1024;
/// Maximum Unicode scalars retained from one page.
pub const MAX_LIVE_FRAGMENT_SCALARS: usize = 8_192;
/// Default wall-clock budget shared by search and fetches.
pub const DEFAULT_LIVE_SUBDEADLINE: Duration = Duration::from_secs(5);
/// Largest configurable wall-clock budget for one live retrieval round.
pub const MAX_LIVE_SUBDEADLINE: Duration = Duration::from_secs(15);

/// Closed live-retrieval failure without query, URL, or page content.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum LiveRetrievalError {
  /// A bounded value or dependency result was invalid.
  #[error("live retrieval data is invalid")]
  Invalid,
  /// A dependency could not safely complete.
  #[error("live retrieval is unavailable")]
  Unavailable,
  /// The immutable deadline was exhausted.
  #[error("live retrieval deadline exceeded")]
  DeadlineExceeded,
  /// The request was cancelled.
  #[error("live retrieval request cancelled")]
  Cancelled,
}

/// Redacted bounded query sent only to the configured search provider.
#[derive(Clone, PartialEq, Eq)]
pub struct LiveSearchQuery(String);

impl LiveSearchQuery {
  /// Creates a nonblank query of at most 2,048 UTF-8 bytes.
  pub fn new(value: impl Into<String>) -> Result<Self, LiveRetrievalError> {
    let value = value.into();
    if value.trim().is_empty() || value.len() > 2_048 || value.chars().any(char::is_control) {
      return Err(LiveRetrievalError::Invalid);
    }
    Ok(Self(value))
  }
  /// Borrows the query solely for the search adapter call.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for LiveSearchQuery {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("LiveSearchQuery(REDACTED)")
  }
}

/// One bounded search nomination.
#[derive(Clone, PartialEq, Eq)]
pub struct LiveSearchResult {
  url: Url,
  title: String,
  publisher: Option<String>,
  published_at: Option<String>,
}

impl LiveSearchResult {
  /// Validates one HTTP(S) search result and bounded display metadata.
  pub fn new(
    url: Url,
    title: String,
    publisher: Option<String>,
    published_at: Option<String>,
  ) -> Result<Self, LiveRetrievalError> {
    if !matches!(url.scheme(), "http" | "https")
      || !url.username().is_empty()
      || url.password().is_some()
      || title.trim().is_empty()
      || title.chars().count() > 256
      || publisher
        .as_ref()
        .is_some_and(|v| v.trim().is_empty() || v.chars().count() > 256)
    {
      return Err(LiveRetrievalError::Invalid);
    }
    Ok(Self {
      url,
      title,
      publisher,
      published_at,
    })
  }
  /// Returns the nominated URL for the safe fetch boundary.
  pub fn url(&self) -> &Url {
    &self.url
  }
}

impl fmt::Debug for LiveSearchResult {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("LiveSearchResult(REDACTED)")
  }
}

/// One safely fetched request-local textual fragment.
#[derive(Clone, PartialEq, Eq)]
pub struct LiveFetchedPage {
  /// Search nomination whose display metadata identifies this page.
  pub result: LiveSearchResult,
  fragment: String,
  /// RFC 3339 instant at which retrieval completed.
  pub retrieved_at: String,
  /// Decompressed response byte count before text extraction.
  pub byte_count: usize,
}

impl LiveFetchedPage {
  /// Validates bounded usable fetched text.
  pub fn new(
    result: LiveSearchResult,
    fragment: String,
    retrieved_at: String,
    byte_count: usize,
  ) -> Result<Self, LiveRetrievalError> {
    let retrieved_instant =
      OffsetDateTime::parse(&retrieved_at, &Rfc3339).map_err(|_| LiveRetrievalError::Invalid)?;
    if fragment.trim().is_empty()
      || fragment.chars().count() > MAX_LIVE_FRAGMENT_SCALARS
      || byte_count == 0
      || byte_count > MAX_LIVE_PAGE_BYTES
      || retrieved_instant.offset() != UtcOffset::UTC
      || !retrieved_instant.nanosecond().is_multiple_of(1_000)
    {
      return Err(LiveRetrievalError::Invalid);
    }
    Ok(Self {
      result,
      fragment,
      retrieved_at,
      byte_count,
    })
  }
  /// Borrows untrusted text only for request-local composition.
  pub fn fragment(&self) -> &str {
    &self.fragment
  }
}

impl fmt::Debug for LiveFetchedPage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("LiveFetchedPage(REDACTED)")
  }
}

/// Response-local live citation and the claims it supports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveCitation {
  /// Response-local source identifier in deterministic retrieval order.
  pub source_id: String,
  /// Source identifiers supporting the response claim.
  pub source_ids: Vec<String>,
  /// Closed non-canonical evidence state, always `live_external`.
  pub evidence_state: &'static str,
}

/// Completed request-local retrieval material.
#[derive(Clone)]
pub struct LiveRetrievalMaterial {
  /// Safely fetched pages retained only for the request lifetime.
  pub pages: Vec<LiveFetchedPage>,
  /// Response-local citations corresponding one-for-one with retained pages.
  pub citations: Vec<LiveCitation>,
}

impl fmt::Debug for LiveRetrievalMaterial {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LiveRetrievalMaterial")
      .field("page_count", &self.pages.len())
      .field("citation_count", &self.citations.len())
      .finish()
  }
}
