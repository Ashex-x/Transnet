//! Strict canonical-data-v1 client for the atomically active knowledge release tuple.

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
  adapters::island_port::{IslandPortClientError, IslandPortTransport, ISLAND_PORT_SCHEMA_VERSION},
  domain::{
    canonical::{CanonicalId, CanonicalReleasePin},
    retrieval_data::NeighborProjectionExecutionExpectation,
  },
  ports::{
    active_knowledge_release::{ActiveKnowledgeReleaseError, ActiveKnowledgeReleasePort},
    canonical_read::CanonicalReadContext,
  },
};

const ACTIVE_KNOWLEDGE_RELEASE_PATH: &str = "/api/v1/knowledge-releases/active";
const MAX_BODY_BYTES: usize = 1_048_576;

/// Read-only Island-port adapter for one atomic active-pointer selection.
pub struct IslandPortActiveKnowledgeReleaseClient {
  transport: Arc<dyn IslandPortTransport>,
}

impl IslandPortActiveKnowledgeReleaseClient {
  /// Creates the client over the same bounded transport used by other Island-port adapters.
  pub fn new(transport: Arc<dyn IslandPortTransport>) -> Self {
    Self { transport }
  }
}

impl fmt::Debug for IslandPortActiveKnowledgeReleaseClient {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortActiveKnowledgeReleaseClient([redacted])")
  }
}

#[async_trait]
impl ActiveKnowledgeReleasePort for IslandPortActiveKnowledgeReleaseClient {
  async fn active_knowledge_release(
    &self,
    context: &CanonicalReadContext,
  ) -> Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError> {
    if context.request_id.trim() != context.request_id
      || context.request_id.is_empty()
      || context.request_id.len() > 128
      || context.request_id.chars().any(char::is_control)
      || context.deadline_at.trim() != context.deadline_at
      || context.deadline_at.is_empty()
      || context.timeout.is_zero()
    {
      return Err(ActiveKnowledgeReleaseError::InvalidRequest);
    }
    let request = RequestEnvelope {
      context: RequestContextDto {
        request_id: &context.request_id,
        deadline_at: &context.deadline_at,
        schema_version: ISLAND_PORT_SCHEMA_VERSION,
      },
      input: EmptyInput {},
    };
    let body =
      serde_json::to_vec(&request).map_err(|_| ActiveKnowledgeReleaseError::InvalidRequest)?;
    if body.len() > MAX_BODY_BYTES {
      return Err(ActiveKnowledgeReleaseError::InvalidRequest);
    }
    let response = self
      .transport
      .post_json(ACTIVE_KNOWLEDGE_RELEASE_PATH, body, context.timeout)
      .await
      .map_err(map_transport_error)?;
    if response.len() > MAX_BODY_BYTES {
      return Err(ActiveKnowledgeReleaseError::InconsistentData);
    }
    let response: ResponseEnvelope = serde_json::from_slice(&response)
      .map_err(|_| ActiveKnowledgeReleaseError::InconsistentData)?;
    response.into_domain(context)
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope<'a> {
  context: RequestContextDto<'a>,
  input: EmptyInput,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestContextDto<'a> {
  request_id: &'a str,
  deadline_at: &'a str,
  schema_version: &'static str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseEnvelope {
  request_id: String,
  schema_version: String,
  outcome: Outcome,
  value: Option<ActiveKnowledgeReleaseDto>,
  error: Option<ErrorDto>,
}

impl ResponseEnvelope {
  fn into_domain(
    self,
    context: &CanonicalReadContext,
  ) -> Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError> {
    if self.request_id != context.request_id {
      return Err(ActiveKnowledgeReleaseError::InconsistentData);
    }
    if self.schema_version != ISLAND_PORT_SCHEMA_VERSION {
      return Err(ActiveKnowledgeReleaseError::SchemaIncompatible);
    }
    match self.outcome {
      Outcome::Ok => {
        if self.error.is_some() {
          return Err(ActiveKnowledgeReleaseError::InconsistentData);
        }
        let value = self
          .value
          .ok_or(ActiveKnowledgeReleaseError::InconsistentData)?;
        value.into_domain().map(Some)
      }
      Outcome::NotFound => self.closed("not_found", Ok(None)),
      Outcome::VersionMismatch => self.closed(
        "schema_incompatible",
        Err(ActiveKnowledgeReleaseError::SchemaIncompatible),
      ),
      Outcome::Unavailable => self.closed(
        "dependency_unavailable",
        Err(ActiveKnowledgeReleaseError::Unavailable),
      ),
      Outcome::Timeout => self.closed("timeout", Err(ActiveKnowledgeReleaseError::Timeout)),
    }
  }

  fn closed(
    self,
    expected_code: &str,
    result: Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError>,
  ) -> Result<Option<NeighborProjectionExecutionExpectation>, ActiveKnowledgeReleaseError> {
    if self.value.is_some()
      || self
        .error
        .as_ref()
        .is_none_or(|error| error.code != expected_code)
    {
      Err(ActiveKnowledgeReleaseError::InconsistentData)
    } else {
      result
    }
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
  Ok,
  NotFound,
  VersionMismatch,
  Unavailable,
  Timeout,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorDto {
  code: String,
  #[allow(dead_code)]
  message: String,
  #[allow(dead_code)]
  retryable: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveKnowledgeReleaseDto {
  content_release: String,
  canonical_schema_version: String,
  node_collection_id: String,
  node_collection_content_hash: String,
  edge_collection_id: String,
  edge_collection_content_hash: String,
  relationship_registry_version: u32,
  edge_dense_input_version: String,
  edge_lexical_input_version: String,
}

impl ActiveKnowledgeReleaseDto {
  fn into_domain(
    self,
  ) -> Result<NeighborProjectionExecutionExpectation, ActiveKnowledgeReleaseError> {
    let expectation = NeighborProjectionExecutionExpectation {
      content: CanonicalReleasePin::new(
        parse_id(self.content_release)?,
        self.canonical_schema_version,
      )
      .ok_or(ActiveKnowledgeReleaseError::InconsistentData)?,
      node_collection_id: parse_id(self.node_collection_id)?,
      node_collection_content_hash: self.node_collection_content_hash,
      edge_collection_id: parse_id(self.edge_collection_id)?,
      edge_collection_content_hash: self.edge_collection_content_hash,
      relationship_registry_version: self.relationship_registry_version,
      edge_dense_input_version: self.edge_dense_input_version,
      edge_lexical_input_version: self.edge_lexical_input_version,
    };
    expectation
      .validate()
      .map_err(|_| ActiveKnowledgeReleaseError::InconsistentData)?;
    if expectation.node_collection_id == expectation.edge_collection_id {
      return Err(ActiveKnowledgeReleaseError::InconsistentData);
    }
    Ok(expectation)
  }
}

fn parse_id(value: String) -> Result<CanonicalId, ActiveKnowledgeReleaseError> {
  CanonicalId::new(value).map_err(|_| ActiveKnowledgeReleaseError::InconsistentData)
}

fn map_transport_error(error: IslandPortClientError) -> ActiveKnowledgeReleaseError {
  match error {
    IslandPortClientError::Timeout => ActiveKnowledgeReleaseError::Timeout,
    IslandPortClientError::SchemaIncompatible => ActiveKnowledgeReleaseError::SchemaIncompatible,
    IslandPortClientError::InvalidRequest => ActiveKnowledgeReleaseError::InvalidRequest,
    IslandPortClientError::Unavailable => ActiveKnowledgeReleaseError::Unavailable,
    IslandPortClientError::NotFound
    | IslandPortClientError::ContentReleaseUnavailable
    | IslandPortClientError::InconsistentData => ActiveKnowledgeReleaseError::InconsistentData,
  }
}

#[cfg(test)]
mod tests {
  use std::{sync::Mutex, time::Duration};

  use super::*;

  struct FakeTransport {
    response: Vec<u8>,
    calls: Mutex<Vec<(String, Vec<u8>, Duration)>>,
  }

  #[async_trait]
  impl IslandPortTransport for FakeTransport {
    async fn post_json(
      &self,
      path: &'static str,
      body: Vec<u8>,
      timeout: Duration,
    ) -> Result<Vec<u8>, IslandPortClientError> {
      self
        .calls
        .lock()
        .unwrap()
        .push((path.to_owned(), body, timeout));
      Ok(self.response.clone())
    }
  }

  fn context() -> CanonicalReadContext {
    CanonicalReadContext {
      request_id: "request-1".into(),
      deadline_at: "2099-01-01T00:00:00Z".into(),
      timeout: Duration::from_secs(2),
    }
  }

  fn response(overrides: &str) -> Vec<u8> {
    format!(
      r#"{{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"ok","value":{{"content_release":"release-1","canonical_schema_version":"canonical-v1","node_collection_id":"nodes-1","node_collection_content_hash":"sha256:{}","edge_collection_id":"edges-1","edge_collection_content_hash":"sha256:{}","relationship_registry_version":1,"edge_dense_input_version":"edge-dense-input-v1","edge_lexical_input_version":"edge-lexical-input-v1"}},"error":null{overrides}}}"#,
      "a".repeat(64),
      "b".repeat(64),
    )
    .into_bytes()
  }

  #[tokio::test]
  async fn returns_one_exact_validated_active_tuple_without_a_release_pin() {
    let transport = Arc::new(FakeTransport {
      response: response(""),
      calls: Mutex::new(Vec::new()),
    });
    let client = IslandPortActiveKnowledgeReleaseClient::new(transport.clone());
    let active = client
      .active_knowledge_release(&context())
      .await
      .unwrap()
      .unwrap();
    assert_eq!(active.content.release_id.as_str(), "release-1");
    assert_eq!(active.node_collection_id.as_str(), "nodes-1");
    assert_eq!(active.edge_collection_id.as_str(), "edges-1");
    assert_eq!(active.relationship_registry_version, 1);

    let calls = transport.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, ACTIVE_KNOWLEDGE_RELEASE_PATH);
    assert_eq!(calls[0].2, Duration::from_secs(2));
    let request: serde_json::Value = serde_json::from_slice(&calls[0].1).unwrap();
    assert_eq!(request["input"], serde_json::json!({}));
    assert_eq!(request["context"]["schema_version"], "canonical-data-v1");
    assert!(request["context"].get("content_release").is_none());
  }

  #[tokio::test]
  async fn closed_missing_outcome_is_the_only_successful_absence() {
    let client = IslandPortActiveKnowledgeReleaseClient::new(Arc::new(FakeTransport {
      response: br#"{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"not_found","value":null,"error":{"code":"not_found","message":"none","retryable":false}}"#.to_vec(),
      calls: Mutex::new(Vec::new()),
    }));
    assert_eq!(client.active_knowledge_release(&context()).await, Ok(None));
  }

  #[tokio::test]
  async fn rejects_echo_topology_unknown_fields_and_projection_drift() {
    let cases = [
      br#"{"request_id":"other","schema_version":"canonical-data-v1","outcome":"not_found","value":null,"error":{"code":"not_found","message":"none","retryable":false}}"#.to_vec(),
      br#"{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"ok","value":null,"error":null}"#.to_vec(),
      br#"{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"not_found","value":null,"error":{"code":"wrong","message":"none","retryable":false}}"#.to_vec(),
      br#"{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"ok","value":null,"error":null,"extra":true}"#.to_vec(),
      format!(r#"{{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"ok","value":{{"content_release":"release-1","canonical_schema_version":"canonical-v1","node_collection_id":"same","node_collection_content_hash":"sha256:{}","edge_collection_id":"same","edge_collection_content_hash":"sha256:{}","relationship_registry_version":1,"edge_dense_input_version":"edge-dense-input-v1","edge_lexical_input_version":"edge-lexical-input-v1"}},"error":null}}"#, "a".repeat(64), "b".repeat(64)).into_bytes(),
    ];
    for response in cases {
      let client = IslandPortActiveKnowledgeReleaseClient::new(Arc::new(FakeTransport {
        response,
        calls: Mutex::new(Vec::new()),
      }));
      assert_eq!(
        client.active_knowledge_release(&context()).await,
        Err(ActiveKnowledgeReleaseError::InconsistentData)
      );
    }
  }

  #[tokio::test]
  async fn maps_only_exact_closed_failure_pairs() {
    let cases = [
      (
        "version_mismatch",
        "schema_incompatible",
        ActiveKnowledgeReleaseError::SchemaIncompatible,
      ),
      (
        "unavailable",
        "dependency_unavailable",
        ActiveKnowledgeReleaseError::Unavailable,
      ),
      ("timeout", "timeout", ActiveKnowledgeReleaseError::Timeout),
    ];
    for (outcome, code, expected) in cases {
      let body = format!(r#"{{"request_id":"request-1","schema_version":"canonical-data-v1","outcome":"{outcome}","value":null,"error":{{"code":"{code}","message":"closed","retryable":false}}}}"#).into_bytes();
      let client = IslandPortActiveKnowledgeReleaseClient::new(Arc::new(FakeTransport {
        response: body,
        calls: Mutex::new(Vec::new()),
      }));
      assert_eq!(
        client.active_knowledge_release(&context()).await,
        Err(expected)
      );
    }
  }
}
