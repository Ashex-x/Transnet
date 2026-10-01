//! Injectable readiness checks for process orchestration.

use async_trait::async_trait;
use serde::Serialize;
use std::{sync::Arc, time::Duration};

use crate::domain::retrieval_data::NeighborProjectionExecutionExpectation;
use crate::ports::active_knowledge_release::ActiveKnowledgeReleasePort;
use crate::ports::canonical_read::{CanonicalReadContext, CanonicalReadPort};

/// Checks whether dependencies required to accept traffic are available.
#[async_trait]
pub trait Readiness: Send + Sync {
  /// Returns `true` when the service may receive traffic.
  async fn is_ready(&self) -> bool;

  /// Returns content-free component states for the target readiness response.
  async fn report(&self) -> ReadinessReport {
    ReadinessReport::without_knowledge(self.is_ready().await)
  }
}

/// Closed readiness state that never exposes dependency identities or endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessComponentState {
  /// The configured dependency safely serves the exact required release tuple.
  Available,
  /// The configured dependency cannot safely serve the required release tuple.
  Unavailable,
  /// The dependency and all capabilities that require it are not configured.
  Disabled,
}

/// Content-free component states for the canonical knowledge dependency bundle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct KnowledgeReadinessComponents {
  /// Authoritative canonical structured reads.
  pub canonical_data: ReadinessComponentState,
  /// Immutable retrieval reads.
  pub retrieval_data: ReadinessComponentState,
  /// Atomic canonical/node/edge projection selection.
  pub knowledge_projection: ReadinessComponentState,
}

/// One readiness decision and its safe closed component detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadinessReport {
  ready: bool,
  /// Knowledge dependency states safe to return to an internal caller.
  pub components: KnowledgeReadinessComponents,
}

impl ReadinessReport {
  /// Creates a content-free readiness result from an aggregate decision and closed components.
  pub const fn new(ready: bool, components: KnowledgeReadinessComponents) -> Self {
    Self { ready, components }
  }

  fn without_knowledge(ready: bool) -> Self {
    Self {
      ready,
      components: KnowledgeReadinessComponents::all(ReadinessComponentState::Disabled),
    }
  }

  /// Returns whether every configured required dependency is safely available.
  pub const fn is_ready(self) -> bool {
    self.ready
  }
}

impl KnowledgeReadinessComponents {
  const fn all(state: ReadinessComponentState) -> Self {
    Self {
      canonical_data: state,
      retrieval_data: state,
      knowledge_projection: state,
    }
  }
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

/// Composite readiness for one configured guided-knowledge bundle and exact immutable trio.
pub struct CompositeKnowledgeReadiness {
  authority: Option<Arc<dyn ActiveKnowledgeReleasePort>>,
  expected: Option<NeighborProjectionExecutionExpectation>,
  timeout: Duration,
}

impl CompositeKnowledgeReadiness {
  /// Creates a disabled knowledge bundle that reports all three components as disabled.
  pub fn disabled() -> Self {
    Self {
      authority: None,
      expected: None,
      timeout: Duration::from_secs(1),
    }
  }

  /// Creates one fully configured bundle pinned to the application service's exact trio.
  pub fn configured(
    authority: Arc<dyn ActiveKnowledgeReleasePort>,
    expected: NeighborProjectionExecutionExpectation,
    timeout: Duration,
  ) -> Self {
    Self {
      authority: Some(authority),
      expected: Some(expected),
      timeout,
    }
  }

  async fn probe(&self) -> ReadinessReport {
    let (Some(authority), Some(expected)) = (&self.authority, &self.expected) else {
      return ReadinessReport::without_knowledge(true);
    };
    if expected.validate().is_err() {
      return unavailable_report();
    }
    let Ok(deadline) = time::Duration::try_from(self.timeout) else {
      return unavailable_report();
    };
    let Ok(deadline_at) = (time::OffsetDateTime::now_utc() + deadline)
      .format(&time::format_description::well_known::Rfc3339)
    else {
      return unavailable_report();
    };
    let context = CanonicalReadContext {
      request_id: ulid::Ulid::new().to_string(),
      deadline_at,
      timeout: self.timeout,
    };
    let ready = matches!(
      tokio::time::timeout(self.timeout, authority.active_knowledge_release(&context)).await,
      Ok(Ok(Some(active))) if active.validate().is_ok() && active == *expected
    );
    if ready {
      ReadinessReport {
        ready: true,
        components: KnowledgeReadinessComponents::all(ReadinessComponentState::Available),
      }
    } else {
      unavailable_report()
    }
  }
}

fn unavailable_report() -> ReadinessReport {
  ReadinessReport {
    ready: false,
    components: KnowledgeReadinessComponents::all(ReadinessComponentState::Unavailable),
  }
}

#[async_trait]
impl Readiness for CompositeKnowledgeReadiness {
  async fn is_ready(&self) -> bool {
    self.probe().await.is_ready()
  }

  async fn report(&self) -> ReadinessReport {
    self.probe().await
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

  #[tokio::test]
  async fn composite_requires_the_exact_active_trio_and_reports_no_identifiers() {
    let expected = execution();
    let readiness = CompositeKnowledgeReadiness::configured(
      Arc::new(StubAuthority {
        result: Ok(Some(expected.clone())),
      }),
      expected.clone(),
      Duration::from_secs(1),
    );
    let report = readiness.report().await;
    assert!(report.is_ready());
    assert_eq!(
      report.components,
      KnowledgeReadinessComponents::all(ReadinessComponentState::Available)
    );
    let serialized = serde_json::to_string(&report.components).unwrap();
    for forbidden in ["release-1", "nodes-1", "edges-1", "sha256:"] {
      assert!(!serialized.contains(forbidden));
    }

    let mut drifted = expected.clone();
    drifted.edge_collection_content_hash = format!("sha256:{}", "c".repeat(64));
    let readiness = CompositeKnowledgeReadiness::configured(
      Arc::new(StubAuthority {
        result: Ok(Some(drifted)),
      }),
      expected,
      Duration::from_secs(1),
    );
    let report = readiness.report().await;
    assert!(!report.is_ready());
    assert_eq!(
      report.components,
      KnowledgeReadinessComponents::all(ReadinessComponentState::Unavailable)
    );
  }

  #[tokio::test]
  async fn disabled_composite_is_ready_with_only_disabled_states() {
    let report = CompositeKnowledgeReadiness::disabled().report().await;
    assert!(report.is_ready());
    assert_eq!(
      report.components,
      KnowledgeReadinessComponents::all(ReadinessComponentState::Disabled)
    );
  }
}
