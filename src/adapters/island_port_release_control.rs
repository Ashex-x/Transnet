//! Strict outbound island-port client for offline activation submission and rollback selection.

use std::{fmt, sync::Arc, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
  adapters::island_port::{IslandPortClientError, IslandPortTransport},
  domain::{
    canonical::ReleaseId,
    knowledge_publication::PublicationManifestHash,
    release_control::{
      validate_canonical_content_hash, AuditSequence, ReleaseSelectionReceipt, RollbackSelection,
    },
  },
  ports::{
    knowledge_publication::KnowledgePublicationContext,
    release_control::{ReleaseControlError, ReleaseControlPort, SubmitActivationCandidate},
  },
};

/// Wire schema implemented by the offline release-control client.
pub const RELEASE_CONTROL_SCHEMA_VERSION: &str = "release-control-v1";
const ACTIVATION_PATH: &str = "/api/v1/releases/activation-candidates/submit";
const ROLLBACK_PATH: &str = "/api/v1/releases/rollback/select";
const MAX_CONTROL_BYTES: usize = 262_144;

/// Storage-neutral island-port release-control client with no database credentials.
pub struct IslandPortReleaseControlClient {
  transport: Arc<dyn IslandPortTransport>,
}

impl IslandPortReleaseControlClient {
  /// Creates a strict client over an injected bounded transport.
  pub fn new(transport: Arc<dyn IslandPortTransport>) -> Self {
    Self { transport }
  }

  async fn call<I: Serialize>(
    &self,
    path: &'static str,
    context: &KnowledgePublicationContext,
    release: &ReleaseId,
    input: I,
  ) -> Result<ResponseDto, ReleaseControlError> {
    let timeout = validate_context(context)?;
    let body = serde_json::to_vec(&RequestEnvelope {
      context: ContextDto {
        request_id: &context.request_id,
        deadline_at: &context.deadline_at,
        schema_version: RELEASE_CONTROL_SCHEMA_VERSION,
        content_release: release.as_str(),
      },
      input,
    })
    .map_err(|_| ReleaseControlError::InvalidRequest)?;
    if body.len() > MAX_CONTROL_BYTES {
      return Err(ReleaseControlError::InvalidRequest);
    }
    let response = self
      .transport
      .post_json(path, body, timeout)
      .await
      .map_err(map_transport_error)?;
    if response.len() > MAX_CONTROL_BYTES {
      return Err(ReleaseControlError::InconsistentData);
    }
    let response: ResponseDto =
      serde_json::from_slice(&response).map_err(|_| ReleaseControlError::InconsistentData)?;
    response.validate_context(context, release)?;
    Ok(response)
  }
}

impl fmt::Debug for IslandPortReleaseControlClient {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortReleaseControlClient([redacted])")
  }
}

#[async_trait]
impl ReleaseControlPort for IslandPortReleaseControlClient {
  async fn submit_activation_candidate(
    &self,
    context: &KnowledgePublicationContext,
    request: &SubmitActivationCandidate<'_>,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    let candidate = request.candidate;
    request
      .prior_audit_sequence
      .require_next(request.audit_sequence)
      .map_err(|_| ReleaseControlError::InvalidRequest)?;
    validate_canonical_content_hash(&candidate.canonical_content_hash)
      .map_err(|_| ReleaseControlError::InvalidRequest)?;
    let trio = &candidate.trio;
    let release = &trio.canonical().release_id;
    if &request.expected_active_release == release {
      return Err(ReleaseControlError::InvalidRequest);
    }
    let response = self
      .call(
        ACTIVATION_PATH,
        context,
        release,
        ActivationInputDto {
          build_id: candidate.build_id.as_str(),
          reconcile_id: candidate.reconcile_id.as_str(),
          release_id: release.as_str(),
          canonical_schema_version: &trio.canonical().canonical_schema_version,
          expected_active_release: request.expected_active_release.as_str(),
          canonical_content_hash: &candidate.canonical_content_hash,
          publication_manifest_hash: candidate.manifest_hash.as_str(),
          node_collection_id: trio.nodes().collection_id.as_str(),
          node_projection_hash: &trio.nodes().content_hash,
          node_persisted_hash: candidate.node_persisted_hash.as_str(),
          node_point_count: trio.nodes().point_count,
          edge_collection_id: trio.edges().collection_id.as_str(),
          edge_projection_hash: &trio.edges().content_hash,
          edge_persisted_hash: candidate.edge_persisted_hash.as_str(),
          edge_point_count: trio.edges().point_count,
          edge_verified_node_projection_hash: &trio.edges().verified_node_content_hash,
          expected_endpoint_count: trio.edges().expected_endpoint_count,
          resolved_endpoint_count: trio.edges().resolved_endpoint_count,
          dense_artifact_revision: &trio.dense_embedding().artifact_revision,
          sparse_encoder_revision: &trio.sparse_embedding().encoder_revision,
          idempotency_key: request.idempotency_key.as_str(),
          prior_audit_sequence: request.prior_audit_sequence.get(),
          audit_sequence: request.audit_sequence.get(),
        },
      )
      .await?;
    response.into_receipt(
      release,
      &request.expected_active_release,
      &candidate.manifest_hash,
      request.audit_sequence,
    )
  }

  async fn select_rollback(
    &self,
    context: &KnowledgePublicationContext,
    request: &RollbackSelection,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    validate_canonical_content_hash(&request.target_canonical_content_hash)
      .map_err(|_| ReleaseControlError::InvalidRequest)?;
    request
      .prior_audit_sequence
      .require_next(request.audit_sequence)
      .map_err(|_| ReleaseControlError::InvalidRequest)?;
    if request.expected_active_release == request.target_release {
      return Err(ReleaseControlError::InvalidRequest);
    }
    let response = self
      .call(
        ROLLBACK_PATH,
        context,
        &request.target_release,
        RollbackInputDto {
          expected_active_release: request.expected_active_release.as_str(),
          target_release: request.target_release.as_str(),
          target_canonical_content_hash: &request.target_canonical_content_hash,
          target_publication_manifest_hash: request.target_manifest_hash.as_str(),
          reason_code: request.reason_code.as_str(),
          idempotency_key: request.idempotency_key.as_str(),
          prior_audit_sequence: request.prior_audit_sequence.get(),
          audit_sequence: request.audit_sequence.get(),
        },
      )
      .await?;
    response.into_receipt(
      &request.target_release,
      &request.expected_active_release,
      &request.target_manifest_hash,
      request.audit_sequence,
    )
  }
}

fn validate_context(
  context: &KnowledgePublicationContext,
) -> Result<Duration, ReleaseControlError> {
  if context.request_id.is_empty()
    || context.request_id.len() > 128
    || !context
      .request_id
      .bytes()
      .all(|byte| byte.is_ascii_graphic())
    || context.deadline_at.len() > 64
    || context.timeout.is_zero()
  {
    return Err(ReleaseControlError::InvalidRequest);
  }
  let deadline = time::OffsetDateTime::parse(
    &context.deadline_at,
    &time::format_description::well_known::Rfc3339,
  )
  .map_err(|_| ReleaseControlError::InvalidRequest)?;
  let remaining = Duration::try_from(deadline - time::OffsetDateTime::now_utc())
    .map_err(|_| ReleaseControlError::Timeout)?;
  if remaining.is_zero() {
    return Err(ReleaseControlError::Timeout);
  }
  Ok(context.timeout.min(remaining))
}

fn map_transport_error(error: IslandPortClientError) -> ReleaseControlError {
  match error {
    IslandPortClientError::InvalidRequest => ReleaseControlError::InvalidRequest,
    IslandPortClientError::SchemaIncompatible => ReleaseControlError::SchemaIncompatible,
    IslandPortClientError::Timeout => ReleaseControlError::Timeout,
    IslandPortClientError::NotFound | IslandPortClientError::ContentReleaseUnavailable => {
      ReleaseControlError::ImmutableReleaseUnavailable
    }
    IslandPortClientError::Unavailable => ReleaseControlError::Unavailable,
    IslandPortClientError::InconsistentData => ReleaseControlError::InconsistentData,
  }
}

#[derive(Serialize)]
struct RequestEnvelope<'a, I> {
  context: ContextDto<'a>,
  input: I,
}

#[derive(Serialize)]
struct ContextDto<'a> {
  request_id: &'a str,
  deadline_at: &'a str,
  schema_version: &'static str,
  content_release: &'a str,
}

#[derive(Serialize)]
struct ActivationInputDto<'a> {
  build_id: &'a str,
  reconcile_id: &'a str,
  release_id: &'a str,
  canonical_schema_version: &'a str,
  expected_active_release: &'a str,
  canonical_content_hash: &'a str,
  publication_manifest_hash: &'a str,
  node_collection_id: &'a str,
  node_projection_hash: &'a str,
  node_persisted_hash: &'a str,
  node_point_count: u64,
  edge_collection_id: &'a str,
  edge_projection_hash: &'a str,
  edge_persisted_hash: &'a str,
  edge_point_count: u64,
  edge_verified_node_projection_hash: &'a str,
  expected_endpoint_count: u64,
  resolved_endpoint_count: u64,
  dense_artifact_revision: &'a str,
  sparse_encoder_revision: &'a str,
  idempotency_key: &'a str,
  prior_audit_sequence: u64,
  audit_sequence: u64,
}

#[derive(Serialize)]
struct RollbackInputDto<'a> {
  expected_active_release: &'a str,
  target_release: &'a str,
  target_canonical_content_hash: &'a str,
  target_publication_manifest_hash: &'a str,
  reason_code: &'a str,
  idempotency_key: &'a str,
  prior_audit_sequence: u64,
  audit_sequence: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseDto {
  request_id: String,
  schema_version: String,
  content_release: String,
  outcome: OutcomeDto,
  value: Option<ReceiptDto>,
  error: Option<ErrorDto>,
}

impl ResponseDto {
  fn validate_context(
    &self,
    context: &KnowledgePublicationContext,
    release: &ReleaseId,
  ) -> Result<(), ReleaseControlError> {
    if self.request_id != context.request_id
      || self.schema_version != RELEASE_CONTROL_SCHEMA_VERSION
      || self.content_release != release.as_str()
    {
      return Err(ReleaseControlError::InconsistentData);
    }
    Ok(())
  }

  fn into_receipt(
    self,
    active: &ReleaseId,
    previous: &ReleaseId,
    manifest: &PublicationManifestHash,
    audit: AuditSequence,
  ) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    if self.outcome != OutcomeDto::Ok {
      if self.value.is_some() {
        return Err(ReleaseControlError::InconsistentData);
      }
      return self
        .error
        .ok_or(ReleaseControlError::InconsistentData)?
        .into_error(self.outcome);
    }
    if self.error.is_some() {
      return Err(ReleaseControlError::InconsistentData);
    }
    let value = self.value.ok_or(ReleaseControlError::InconsistentData)?;
    if value.active_release != active.as_str()
      || value.previous_release != previous.as_str()
      || value.publication_manifest_hash != manifest.as_str()
      || value.audit_sequence != audit.get()
      || time::OffsetDateTime::parse(
        &value.selected_at,
        &time::format_description::well_known::Rfc3339,
      )
      .is_err()
    {
      return Err(ReleaseControlError::InconsistentData);
    }
    Ok(ReleaseSelectionReceipt {
      active_release: active.clone(),
      previous_release: previous.clone(),
      selected_at: value.selected_at,
      audit_sequence: audit,
      manifest_hash: manifest.clone(),
    })
  }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum OutcomeDto {
  Ok,
  InvalidPayload,
  VersionMismatch,
  Conflict,
  Missing,
  Unavailable,
  Timeout,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptDto {
  active_release: String,
  previous_release: String,
  selected_at: String,
  audit_sequence: u64,
  publication_manifest_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorDto {
  code: ErrorCodeDto,
  #[allow(dead_code)]
  message: String,
}

impl ErrorDto {
  fn into_error(self, outcome: OutcomeDto) -> Result<ReleaseSelectionReceipt, ReleaseControlError> {
    let error = match (outcome, self.code) {
      (OutcomeDto::InvalidPayload, ErrorCodeDto::InvalidProof) => {
        ReleaseControlError::InvalidRequest
      }
      (OutcomeDto::VersionMismatch, ErrorCodeDto::SchemaIncompatible) => {
        ReleaseControlError::SchemaIncompatible
      }
      (OutcomeDto::Conflict, ErrorCodeDto::ActivationConflict)
      | (OutcomeDto::Conflict, ErrorCodeDto::IdempotencyConflict)
      | (OutcomeDto::Conflict, ErrorCodeDto::AuditSequenceConflict) => {
        ReleaseControlError::Conflict
      }
      (OutcomeDto::Missing, ErrorCodeDto::ImmutableReleaseUnavailable) => {
        ReleaseControlError::ImmutableReleaseUnavailable
      }
      (OutcomeDto::Unavailable, ErrorCodeDto::DependencyUnavailable) => {
        ReleaseControlError::Unavailable
      }
      (OutcomeDto::Timeout, ErrorCodeDto::Timeout) => ReleaseControlError::Timeout,
      _ => return Err(ReleaseControlError::InconsistentData),
    };
    Err(error)
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCodeDto {
  InvalidProof,
  SchemaIncompatible,
  ActivationConflict,
  IdempotencyConflict,
  AuditSequenceConflict,
  ImmutableReleaseUnavailable,
  DependencyUnavailable,
  Timeout,
}

#[cfg(test)]
mod tests {
  use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
  };

  use serde_json::{json, Value};

  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalReleasePin, ReleaseId},
      knowledge_publication::{
        PersistedCollectionHash, PublicationBuildId, PublicationIdempotencyKey,
        PublicationReconcileIdentity,
      },
      knowledge_release::{
        DenseEmbeddingVersion, EdgeCollectionId, EdgeCollectionManifest, KnowledgeReleaseTrio,
        NodeCollectionId, NodeCollectionManifest, ProjectionCollectionState,
        SparseEmbeddingVersion,
      },
      release_control::RollbackReasonCode,
    },
    ports::knowledge_publication::PublicationActivationCandidate,
  };

  #[derive(Default)]
  struct FakeTransport {
    responses: Mutex<VecDeque<Vec<u8>>>,
    calls: Mutex<Vec<(&'static str, Value)>>,
  }

  impl FakeTransport {
    fn with_responses(responses: Vec<Value>) -> Self {
      Self {
        responses: Mutex::new(
          responses
            .into_iter()
            .map(|value| serde_json::to_vec(&value).unwrap())
            .collect(),
        ),
        calls: Mutex::new(Vec::new()),
      }
    }
  }

  #[async_trait]
  impl IslandPortTransport for FakeTransport {
    async fn post_json(
      &self,
      path: &'static str,
      body: Vec<u8>,
      _timeout: Duration,
    ) -> Result<Vec<u8>, IslandPortClientError> {
      let value = serde_json::from_slice(&body).unwrap();
      self.calls.lock().unwrap().push((path, value));
      self
        .responses
        .lock()
        .unwrap()
        .pop_front()
        .ok_or(IslandPortClientError::Unavailable)
    }
  }

  fn hash(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
  }

  fn release(value: &str) -> ReleaseId {
    ReleaseId::new(value).unwrap()
  }

  fn context() -> KnowledgePublicationContext {
    KnowledgePublicationContext {
      request_id: "req-release-control".into(),
      deadline_at: "2099-01-01T00:00:00Z".into(),
      timeout: Duration::from_secs(5),
    }
  }

  fn candidate() -> PublicationActivationCandidate {
    let canonical = CanonicalReleasePin::new(release("release-r2"), "canonical-v1".into()).unwrap();
    let nodes = NodeCollectionManifest {
      release_id: canonical.release_id.clone(),
      collection_id: NodeCollectionId::parse("nodes-r2").unwrap(),
      payload_schema_version: "knowledge-graph-v1".into(),
      content_hash: hash('b'),
      point_count: 2,
      state: ProjectionCollectionState::Verified,
    };
    let edges = EdgeCollectionManifest {
      release_id: canonical.release_id.clone(),
      collection_id: EdgeCollectionId::parse("edges-r2").unwrap(),
      payload_schema_version: "knowledge-graph-v1".into(),
      content_hash: hash('c'),
      point_count: 1,
      verified_node_content_hash: hash('b'),
      expected_endpoint_count: 2,
      resolved_endpoint_count: 2,
      state: ProjectionCollectionState::Verified,
    };
    let trio = KnowledgeReleaseTrio::try_from_parts(
      canonical,
      Some(nodes),
      Some(edges),
      DenseEmbeddingVersion {
        model_family: "Qwen/Qwen3-Embedding-0.6B".into(),
        artifact_revision: "immutable-r1".into(),
        dimensions: 1024,
      },
      SparseEmbeddingVersion {
        encoder_identity: "transnet-lexical-bm25".into(),
        encoder_revision: "v1".into(),
      },
    )
    .unwrap();
    let build_id = PublicationBuildId::parse(hash('a')).unwrap();
    let nodes_hash = PersistedCollectionHash::parse(hash('d')).unwrap();
    let edges_hash = PersistedCollectionHash::parse(hash('e')).unwrap();
    PublicationActivationCandidate {
      reconcile_id: PublicationReconcileIdentity::derive(
        &build_id,
        &hash('9'),
        &nodes_hash,
        &trio.nodes().collection_id,
        &edges_hash,
        &trio.edges().collection_id,
      )
      .unwrap(),
      build_id,
      trio,
      canonical_content_hash: hash('9'),
      manifest_hash: PublicationManifestHash::parse(hash('f')).unwrap(),
      node_persisted_hash: nodes_hash,
      edge_persisted_hash: edges_hash,
    }
  }

  fn receipt(active: &str, previous: &str, sequence: u64, manifest: &str) -> Value {
    json!({
      "request_id": "req-release-control",
      "schema_version": RELEASE_CONTROL_SCHEMA_VERSION,
      "content_release": active,
      "outcome": "ok",
      "value": {
        "active_release": active,
        "previous_release": previous,
        "selected_at": "2026-10-01T00:00:00Z",
        "audit_sequence": sequence,
        "publication_manifest_hash": manifest
      }
    })
  }

  #[tokio::test]
  async fn activation_submits_complete_reconciliation_proof_without_content() {
    let candidate = candidate();
    let transport = Arc::new(FakeTransport::with_responses(vec![receipt(
      "release-r2",
      "release-r1",
      41,
      candidate.manifest_hash.as_str(),
    )]));
    let client = IslandPortReleaseControlClient::new(transport.clone());
    let request = SubmitActivationCandidate {
      candidate: &candidate,
      expected_active_release: release("release-r1"),
      idempotency_key: PublicationIdempotencyKey::parse("activate-r2").unwrap(),
      prior_audit_sequence: AuditSequence::new(40).unwrap(),
      audit_sequence: AuditSequence::new(41).unwrap(),
    };

    let result = client
      .submit_activation_candidate(&context(), &request)
      .await
      .unwrap();

    assert_eq!(result.active_release, release("release-r2"));
    let calls = transport.calls.lock().unwrap();
    assert_eq!(calls[0].0, ACTIVATION_PATH);
    assert_eq!(calls[0].1["input"]["build_id"], hash('a'));
    assert_eq!(calls[0].1["input"]["canonical_content_hash"], hash('9'));
    let encoded = serde_json::to_string(&calls[0].1).unwrap();
    assert!(!encoded.contains("source_text"));
    assert!(!encoded.contains("evidence_text"));
    assert!(!encoded.contains("prompt"));
    assert_eq!(
      format!("{client:?}"),
      "IslandPortReleaseControlClient([redacted])"
    );
  }

  #[tokio::test]
  async fn rollback_uses_separate_endpoint_and_closed_reason() {
    let manifest = PublicationManifestHash::parse(hash('7')).unwrap();
    let transport = Arc::new(FakeTransport::with_responses(vec![receipt(
      "release-r1",
      "release-r2",
      42,
      manifest.as_str(),
    )]));
    let client = IslandPortReleaseControlClient::new(transport.clone());
    let request = RollbackSelection {
      expected_active_release: release("release-r2"),
      target_release: release("release-r1"),
      target_manifest_hash: manifest,
      target_canonical_content_hash: hash('8'),
      reason_code: RollbackReasonCode::VerificationFailure,
      idempotency_key: PublicationIdempotencyKey::parse("rollback-r1").unwrap(),
      prior_audit_sequence: AuditSequence::new(41).unwrap(),
      audit_sequence: AuditSequence::new(42).unwrap(),
    };

    client.select_rollback(&context(), &request).await.unwrap();

    let calls = transport.calls.lock().unwrap();
    assert_eq!(calls[0].0, ROLLBACK_PATH);
    assert_eq!(calls[0].1["input"]["reason_code"], "verification_failure");
  }

  #[tokio::test]
  async fn receipt_must_echo_release_manifest_and_audit_sequence() {
    let candidate = candidate();
    let bad = receipt(
      "release-r2",
      "release-r1",
      99,
      candidate.manifest_hash.as_str(),
    );
    let client =
      IslandPortReleaseControlClient::new(Arc::new(FakeTransport::with_responses(vec![bad])));
    let request = SubmitActivationCandidate {
      candidate: &candidate,
      expected_active_release: release("release-r1"),
      idempotency_key: PublicationIdempotencyKey::parse("activate-r2").unwrap(),
      prior_audit_sequence: AuditSequence::new(40).unwrap(),
      audit_sequence: AuditSequence::new(41).unwrap(),
    };

    assert_eq!(
      client
        .submit_activation_candidate(&context(), &request)
        .await,
      Err(ReleaseControlError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn audit_gap_is_rejected_before_transport() {
    let candidate = candidate();
    let transport = Arc::new(FakeTransport::default());
    let client = IslandPortReleaseControlClient::new(transport.clone());
    let request = SubmitActivationCandidate {
      candidate: &candidate,
      expected_active_release: release("release-r1"),
      idempotency_key: PublicationIdempotencyKey::parse("activate-r2").unwrap(),
      prior_audit_sequence: AuditSequence::new(39).unwrap(),
      audit_sequence: AuditSequence::new(41).unwrap(),
    };

    assert_eq!(
      client
        .submit_activation_candidate(&context(), &request)
        .await,
      Err(ReleaseControlError::InvalidRequest)
    );
    assert!(transport.calls.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn contradictory_outcome_topology_fails_closed() {
    let candidate = candidate();
    let response = json!({
      "request_id": "req-release-control",
      "schema_version": RELEASE_CONTROL_SCHEMA_VERSION,
      "content_release": "release-r2",
      "outcome": "conflict",
      "value": {
        "active_release": "release-r2",
        "previous_release": "release-r1",
        "selected_at": "2026-10-01T00:00:00Z",
        "audit_sequence": 41,
        "publication_manifest_hash": candidate.manifest_hash.as_str()
      },
      "error": {"code": "activation_conflict", "message": "ignored"}
    });
    let client =
      IslandPortReleaseControlClient::new(Arc::new(FakeTransport::with_responses(vec![response])));
    let request = SubmitActivationCandidate {
      candidate: &candidate,
      expected_active_release: release("release-r1"),
      idempotency_key: PublicationIdempotencyKey::parse("activate-r2").unwrap(),
      prior_audit_sequence: AuditSequence::new(40).unwrap(),
      audit_sequence: AuditSequence::new(41).unwrap(),
    };

    assert_eq!(
      client
        .submit_activation_candidate(&context(), &request)
        .await,
      Err(ReleaseControlError::InconsistentData)
    );
  }

  #[test]
  fn audit_sequences_are_positive_and_consecutive() {
    assert!(AuditSequence::new(0).is_err());
    let first = AuditSequence::new(41).unwrap();
    assert!(first.require_next(AuditSequence::new(42).unwrap()).is_ok());
    assert!(first.require_next(AuditSequence::new(43).unwrap()).is_err());
  }
}
