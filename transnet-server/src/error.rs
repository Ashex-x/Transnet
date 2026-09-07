//! Gateway error categories shared by transport and HTTP layers.

use thiserror::Error;

/// Error returned while validating or forwarding a gateway request.
#[derive(Debug, Error)]
pub enum TransnetError {
  /// The caller supplied a request that the backend rejected as invalid.
  #[error("{0}")]
  Validation(String),
  /// The backend could not be reached or returned an invalid response.
  #[error("backend request failed: {0}")]
  Backend(String),
  /// Gateway configuration is missing or invalid.
  #[error("configuration error: {0}")]
  Config(String),
  /// An unexpected gateway operation failed.
  #[error("internal error: {0}")]
  Internal(String),
}
