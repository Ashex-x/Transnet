//! Narrow read interface for the active immutable canonical-content tuple.

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::canonical::ActiveContentVersion;

/// Typed failure while resolving the active canonical-content tuple.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ActiveContentReaderError {
  /// The active-content source could not complete its bounded read.
  #[error("active content reader unavailable")]
  Unavailable,
  /// The active-content source returned a contradictory or unsafe pointer.
  #[error("active content reader returned inconsistent data")]
  InconsistentData,
}

/// Resolves the one immutable canonical-content tuple selected for new public reads.
///
/// This intentionally narrow interface does not expose content-release staging, publication,
/// rollback, source quarantine, storage, or vector operations. Implementations must return the
/// single active tuple only when it is safe to serve. `Ok(None)` means no canonical release is
/// active; callers must not substitute a retained, staging, or separately selected release.
#[async_trait]
pub trait ActiveContentReader: Send + Sync {
  /// Returns the active immutable content tuple, when canonical content is safely servable.
  ///
  /// # Errors
  ///
  /// Returns an error when the active-content source is unavailable or violates its singleton
  /// pointer and source-safety contract.
  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ActiveContentReaderError>;
}
