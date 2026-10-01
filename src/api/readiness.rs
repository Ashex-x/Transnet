//! Injectable readiness checks for process orchestration.

use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

use crate::ports::active_knowledge_release::ActiveKnowledgeReleasePort;
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

/// Readiness check for one complete atomically active canonical/node/edge projection tuple.
pub struct KnowledgeProjectionReadiness {
  authority: Arc<dyn ActiveKnowledgeReleasePort>,
  timeout: Duration,
}

impl KnowledgeProjectionReadiness {
  /// Creates a bounded read-only active-tuple probe.
  pub fn new(authority: Arc<dyn ActiveKnowledgeReleasePort>, timeout: Duration) -> Self {
    Self { authority, timeout }
  }
}

#[async_trait]
impl Readiness for KnowledgeProjectionReadiness {
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
      tokio::time::timeout(
        self.timeout,
        self.authority.active_knowledge_release(&context)
      )
      .await,
      Ok(Ok(Some(execution))) if execution.validate().is_ok()
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalId, CanonicalReleasePin},
      embedding_input::{EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION},
      retrieval_data::{NeighborProjectionExecutionExpectation, RELATION_REGISTRY_VERSION},
    },
    ports::active_knowledge_release::ActiveKnowledgeReleaseError,
  };

  struct StubAuthority {
    result: Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError>,
  }

  #[async_trait]
  impl ActiveKnowledgeReleasePort for StubAuthority {
    async fn active_knowledge_release(
      &self,
      _context: &CanonicalReadContext,
    ) -> Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError> {
      self.result.clone()
    }
  }

  fn execution() -> NeighborProjectionExecutionExpectation {
    NeighborProjectionExecutionExpectation {
      content: CanonicalReleasePin::new(
        CanonicalId::new("release-1").unwrap(),
        "canonical-v1".into(),
      )
      .unwrap(),
      node_collection_id: CanonicalId::new("nodes-1").unwrap(),
      node_collection_content_hash: format!("sha256:{}", "a".repeat(64)),
      edge_collection_id: CanonicalId::new("edges-1").unwrap(),
      edge_collection_content_hash: format!("sha256:{}", "b".repeat(64)),
      relationship_registry_version: RELATION_REGISTRY_VERSION,
      edge_dense_input_version: EDGE_DENSE_INPUT_VERSION.into(),
      edge_lexical_input_version: EDGE_LEXICAL_INPUT_VERSION.into(),
    }
  }

  #[tokio::test]
  async fn knowledge_projection_is_ready_only_for_a_complete_valid_tuple() {
    let readiness = KnowledgeProjectionReadiness::new(
      Arc::new(StubAuthority {
        result: Ok(Some(execution())),
      }),
      Duration::from_secs(1),
    );

    assert!(readiness.is_ready().await);
  }

  #[tokio::test]
  async fn knowledge_projection_fails_closed_for_missing_invalid_or_failed_authority() {
    let mut invalid = execution();
    invalid.edge_collection_content_hash = "not-a-hash".into();
    let results = [
      Ok(None),
      Ok(Some(invalid)),
      Err(ActiveKnowledgeReleaseError::Unavailable),
    ];

    for result in results {
      let readiness = KnowledgeProjectionReadiness::new(
        Arc::new(StubAuthority { result }),
        Duration::from_secs(1),
      );
      assert!(!readiness.is_ready().await);
    }
  }
}
