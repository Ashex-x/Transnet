//! Reusable success envelope metadata for target HTTP operations.

use serde::Serialize;

use crate::domain::request_context::RequestContext;

/// Standard target success envelope.
#[derive(Clone, Debug, Serialize)]
pub struct SuccessEnvelope<T> {
  /// Route-specific response value.
  pub data: T,
  /// Request-safe metadata for the completed operation.
  pub meta: SuccessMeta,
}

impl<T> SuccessEnvelope<T> {
  /// Wraps route data with metadata derived from the admitted request context.
  pub fn new(data: T, context: &RequestContext, result_schema_version: impl Into<String>) -> Self {
    Self {
      data,
      meta: SuccessMeta::from_context(context, result_schema_version),
    }
  }
}

/// Common metadata returned by successful target operations.
#[derive(Clone, Debug, Serialize)]
pub struct SuccessMeta {
  /// Correlation identifier also returned in the response header.
  pub request_id: String,
  /// Schema used to interpret the route result.
  pub schema_version: String,
  /// Immutable canonical release used by the operation, when selected.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content_release: Option<String>,
}

impl SuccessMeta {
  /// Builds metadata without exposing request content or internal dependency details.
  pub fn from_context(context: &RequestContext, result_schema_version: impl Into<String>) -> Self {
    Self {
      request_id: context.request_id().as_str().to_string(),
      schema_version: result_schema_version.into(),
      content_release: context.content_release().map(ToString::to_string),
    }
  }
}
