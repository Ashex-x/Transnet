//! Injectable readiness checks for process orchestration.

use async_trait::async_trait;

/// Checks whether dependencies required to accept traffic are available.
#[async_trait]
pub trait Readiness: Send + Sync {
  /// Returns `true` when the service may receive traffic.
  async fn is_ready(&self) -> bool;
}

/// Readiness implementation for a process without mandatory external dependencies.
#[derive(Debug, Default)]
pub struct AlwaysReady;

#[async_trait]
impl Readiness for AlwaysReady {
  async fn is_ready(&self) -> bool {
    true
  }
}
