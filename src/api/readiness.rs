//! Injectable readiness checks for process orchestration.

use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

use crate::ports::canonical_read::{CanonicalReadContext, CanonicalReadPort};

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

/// Read-only active-release probe for an explicitly enabled canonical dependency.
pub struct CanonicalDependencyReadiness {
  authority: Arc<dyn CanonicalReadPort>,
  timeout: Duration,
}

impl CanonicalDependencyReadiness {
  /// Uses the same strict canonical authority as the request-local read service.
  pub fn new(authority: Arc<dyn CanonicalReadPort>, timeout: Duration) -> Self {
    Self { authority, timeout }
  }
}

#[async_trait]
impl Readiness for CanonicalDependencyReadiness {
  async fn is_ready(&self) -> bool {
    let Ok(deadline) = time::Duration::try_from(self.timeout) else {
      return false;
    };
    let Ok(deadline_at) = (time::OffsetDateTime::now_utc() + deadline)
      .format(&time::format_description::well_known::Rfc3339)
    else {
      return false;
    };
    let context = CanonicalReadContext {
      request_id: ulid::Ulid::new().to_string(),
      deadline_at,
      timeout: self.timeout,
    };
    matches!(
      tokio::time::timeout(self.timeout, self.authority.active_release(&context)).await,
      Ok(Ok(Some(_)))
    )
  }
}
