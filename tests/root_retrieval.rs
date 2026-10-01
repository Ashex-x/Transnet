//! Acceptance tests for root-first bounded retrieval and explicit degradation.

use std::{
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
  time::Duration,
};

use async_trait::async_trait;
use time::OffsetDateTime;
use transnet::{
  application::root_retrieval::{BoundedRootRetrievalService, RootRetrievalError},
  domain::{
    canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
    knowledge_hydration::HydratedKnowledgeNode,
    model_runtime::{CancellationSignal, EphemeralEmbedding, ModelVersion},
    request_context::{RequestContext, RequestId},
    retrieval_data::{
      DenseQueryVector, EdgeSearchRequest, EdgeSearchResult, NeighborSearchRequest,
      NeighborSearchResult, NodeCandidate, NodeCandidatePayload, NodeMatchMechanism,
      NodeProjectionExecutionExpectation, NodeProjectionExecutionProof, NodeSearchRequest,
      NodeSearchResult, RetrievalDataScore, RetrievalNodeType, RetrievalVerificationState,
      ScaleSearchRequest, ScaleSearchResult, SparseQueryVector, DENSE_QUERY_VECTOR_DIMENSIONS,
    },
    root_retrieval::{
      CanonicalRoot, CanonicalRootResolution, RootQuery, RootRetrievalCoverage,
      RootRetrievalExecutionSpec, RootRetrievalRequest, RootRetrievalResult,
      SparseQueryExecutionReceipt, QUERY_LEXICAL_ENCODER_IDENTITY, QUERY_LEXICAL_ENCODER_REVISION,
      QUERY_LEXICAL_INPUT_VERSION,
    },
  },
  ports::{
    canonical_read::{
      CanonicalCandidateQuery, CanonicalKnowledgeNodeQuery, CanonicalReadContext,
      CanonicalReadError, CanonicalReadPort, CanonicalSenseQuery, CanonicalTranslationQuery,
    },
    model_runtime::{EmbeddingPort, EmbeddingRequest, ModelOperationContext, ModelOperationError},
    retrieval_data::{RetrievalDataError, RetrievalDataPort},
    root_retrieval::{
      CanonicalRootError, CanonicalRootPort, QueryLexicalEncoderError, QueryLexicalEncoderPort,
      QueryLexicalEncoding,
    },
  },
};

#[derive(Clone)]
enum Resolution {
  Missing,
  Ambiguous(Vec<CanonicalRoot>),
  Resolved,
}

struct FakeCanonical {
  resolution: Resolution,
  hydrate: Vec<HydratedKnowledgeNode>,
  resolve_calls: AtomicUsize,
  hydrate_calls: AtomicUsize,
}

#[async_trait]
impl CanonicalRootPort for FakeCanonical {
  async fn resolve_root(
    &self,
    _context: &RequestContext,
    _query: &RootQuery,
    _source_language: &LanguageTag,
    _explanation_language: &LanguageTag,
  ) -> Result<CanonicalRootResolution, CanonicalRootError> {
    self.resolve_calls.fetch_add(1, Ordering::Relaxed);
    Ok(match &self.resolution {
      Resolution::Missing => CanonicalRootResolution::NotFound,
      Resolution::Ambiguous(values) => CanonicalRootResolution::Ambiguous(values.clone()),
      Resolution::Resolved => CanonicalRootResolution::Resolved(root("root")),
    })
  }
}

#[async_trait]
impl CanonicalReadPort for FakeCanonical {
  async fn active_release(
    &self,
    _context: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  async fn translations(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalTranslationQuery,
  ) -> Result<
    Vec<transnet::domain::canonical_translation::CanonicalTranslationRevision>,
    CanonicalReadError,
  > {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  async fn candidates(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalCandidateQuery,
  ) -> Result<Vec<transnet::domain::retrieval::RepositoryMatch>, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  async fn sense(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalSenseQuery,
  ) -> Result<transnet::domain::canonical_content::CanonicalSenseDetails, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  async fn knowledge_nodes(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalKnowledgeNodeQuery,
  ) -> Result<Vec<HydratedKnowledgeNode>, CanonicalReadError> {
    self.hydrate_calls.fetch_add(1, Ordering::Relaxed);
    Ok(self.hydrate.clone())
  }
}

struct FakeEmbedding {
  unavailable: bool,
  model_version: &'static str,
  calls: AtomicUsize,
}

#[async_trait]
impl EmbeddingPort for FakeEmbedding {
  async fn embed(
    &self,
    _context: ModelOperationContext<'_>,
    _request: EmbeddingRequest,
  ) -> Result<EphemeralEmbedding, ModelOperationError> {
    self.calls.fetch_add(1, Ordering::Relaxed);
    if self.unavailable {
      return Err(ModelOperationError::Unavailable);
    }
    EphemeralEmbedding::new(
      vec![0.25; DENSE_QUERY_VECTOR_DIMENSIONS],
      DENSE_QUERY_VECTOR_DIMENSIONS,
      ModelVersion::new(self.model_version).unwrap(),
    )
    .map_err(|_| ModelOperationError::InvalidOutput)
  }
}

struct FakeLexical {
  receipt_release: CanonicalId,
}

#[async_trait]
impl QueryLexicalEncoderPort for FakeLexical {
  async fn encode(
    &self,
    _context: &RequestContext,
    _release_id: &CanonicalId,
    _query: &RootQuery,
  ) -> Result<QueryLexicalEncoding, QueryLexicalEncoderError> {
    Ok(QueryLexicalEncoding {
      vector: SparseQueryVector::new(vec![1], vec![1.0])
        .map_err(|_| QueryLexicalEncoderError::InvalidEncoding)?,
      receipt: SparseQueryExecutionReceipt {
        release_id: self.receipt_release.clone(),
        encoder_identity: QUERY_LEXICAL_ENCODER_IDENTITY.to_string(),
        encoder_revision: QUERY_LEXICAL_ENCODER_REVISION.to_string(),
        input_version: QUERY_LEXICAL_INPUT_VERSION.to_string(),
      },
    })
  }
}

struct FakeRetrieval {
  result: Mutex<Result<NodeSearchResult, RetrievalDataError>>,
  calls: AtomicUsize,
}

#[async_trait]
impl RetrievalDataPort for FakeRetrieval {
  async fn search_nodes(
    &self,
    _context: &RequestContext,
    _request: NodeSearchRequest,
  ) -> Result<NodeSearchResult, RetrievalDataError> {
    self.calls.fetch_add(1, Ordering::Relaxed);
    self.result.lock().unwrap().clone()
  }

  async fn search_scales(
    &self,
    _context: &RequestContext,
    _request: ScaleSearchRequest,
  ) -> Result<ScaleSearchResult, RetrievalDataError> {
    unreachable!("root retrieval does not search scales")
  }

  async fn search_edges(
    &self,
    _context: &RequestContext,
    _request: EdgeSearchRequest,
  ) -> Result<EdgeSearchResult, RetrievalDataError> {
    unreachable!("root retrieval does not search edges")
  }

  async fn search_neighbors(
    &self,
    _context: &RequestContext,
    _request: NeighborSearchRequest,
  ) -> Result<NeighborSearchResult, RetrievalDataError> {
    unreachable!("root retrieval does not search neighbors")
  }
}

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn pin(release: &str) -> CanonicalReleasePin {
  CanonicalReleasePin::new(id(release), "canonical-v1".to_owned()).unwrap()
}

fn root(node: &str) -> CanonicalRoot {
  CanonicalRoot {
    content: pin("release-1"),
    node_id: id(node),
    revision: 1,
    node_family: RetrievalNodeType::LexicalSense,
    sense_id: id(&format!("sense-{node}")),
    canonical_label: format!("label-{node}"),
    evidence_ids: vec![id(&format!("evidence-{node}"))],
  }
}

fn hydrated(node: &str) -> HydratedKnowledgeNode {
  HydratedKnowledgeNode {
    node_id: id(node),
    revision: 1,
    node_type: RetrievalNodeType::LexicalSense,
    sense_id: Some(id(&format!("sense-{node}"))),
    canonical_label: format!("label-{node}"),
    language: Some(LanguageTag::parse("en").unwrap()),
    domain_ids: Vec::new(),
    evidence_ids: vec![id(&format!("evidence-{node}"))],
  }
}

fn candidate(node: &str, score: f32, matched_by: &[&str]) -> NodeCandidate {
  NodeCandidate {
    node_id: id(node),
    score: RetrievalDataScore::new(score).unwrap(),
    matched_by: matched_by
      .iter()
      .map(|value| NodeMatchMechanism::from_wire_name(value).unwrap())
      .collect(),
    payload: NodeCandidatePayload {
      node_type: RetrievalNodeType::LexicalSense,
      sense_id: Some(id(&format!("sense-{node}"))),
      canonical_label: format!("label-{node}"),
      verification_state: RetrievalVerificationState::Verified,
    },
  }
}

fn projection_expectation() -> NodeProjectionExecutionExpectation {
  NodeProjectionExecutionExpectation::v1("qwen-test-r1").unwrap()
}

fn retrieval_echo() -> NodeProjectionExecutionProof {
  let expected = projection_expectation();
  NodeProjectionExecutionProof {
    collection_id: id("nodes-release-1"),
    collection_content_hash:
      "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
    dense_artifact_revision: expected.dense_artifact_revision,
    dense_input_specification: expected.dense_input_specification,
    lexical_encoder_identity: expected.lexical_encoder_identity,
    lexical_encoder_revision: expected.lexical_encoder_revision,
    lexical_input_specification: expected.lexical_input_specification,
  }
}

fn context(pin: Option<CanonicalId>) -> RequestContext {
  let deadline = OffsetDateTime::now_utc() + Duration::from_secs(30);
  let deadline = deadline.replace_nanosecond(0).unwrap();
  RequestContext::new(
    RequestId::new("root-test").unwrap(),
    deadline,
    "transnet-v1",
    pin,
  )
  .unwrap()
}

fn request() -> RootRetrievalRequest {
  RootRetrievalRequest::new(
    RootQuery::new("private query").unwrap(),
    LanguageTag::parse("en").unwrap(),
    LanguageTag::parse("en").unwrap(),
    12,
  )
  .unwrap()
}

fn service(
  resolution: Resolution,
  hydrate: Vec<HydratedKnowledgeNode>,
  embedding_unavailable: bool,
  result: Result<NodeSearchResult, RetrievalDataError>,
) -> (
  BoundedRootRetrievalService,
  Arc<FakeCanonical>,
  Arc<FakeEmbedding>,
  Arc<FakeRetrieval>,
) {
  let canonical = Arc::new(FakeCanonical {
    resolution,
    hydrate,
    resolve_calls: AtomicUsize::new(0),
    hydrate_calls: AtomicUsize::new(0),
  });
  let embedding = Arc::new(FakeEmbedding {
    unavailable: embedding_unavailable,
    model_version: "qwen-test-r1",
    calls: AtomicUsize::new(0),
  });
  let retrieval = Arc::new(FakeRetrieval {
    result: Mutex::new(result),
    calls: AtomicUsize::new(0),
  });
  let composed = BoundedRootRetrievalService::new(
    canonical.clone(),
    canonical.clone(),
    embedding.clone(),
    Arc::new(FakeLexical {
      receipt_release: id("release-1"),
    }),
    retrieval.clone(),
    execution_spec(),
  );
  (composed, canonical, embedding, retrieval)
}

fn execution_spec() -> RootRetrievalExecutionSpec {
  RootRetrievalExecutionSpec::new(
    pin("release-1"),
    id("nodes-release-1"),
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    projection_expectation(),
  )
  .unwrap()
}

#[tokio::test]
async fn no_root_never_creates_vectors_or_queries_retrieval_data() {
  let (service, _, embedding, retrieval) = service(
    Resolution::Missing,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  let result = service
    .retrieve(&context(None), &CancellationSignal::default(), request())
    .await
    .unwrap();
  assert_eq!(result, RootRetrievalResult::NotFound);
  assert_eq!(embedding.calls.load(Ordering::Relaxed), 0);
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn ambiguity_is_preserved_without_semantic_guessing() {
  let (service, _, embedding, retrieval) = service(
    Resolution::Ambiguous(vec![root("root-a"), root("root-b")]),
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  let result = service
    .retrieve(&context(None), &CancellationSignal::default(), request())
    .await
    .unwrap();
  assert!(matches!(result, RootRetrievalResult::Ambiguous(values) if values.len() == 2));
  assert_eq!(embedding.calls.load(Ordering::Relaxed), 0);
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn ambiguous_candidates_must_be_unique_and_share_the_execution_pin() {
  let duplicate = root("root-a");
  let (duplicate_service, _, embedding, retrieval) = service(
    Resolution::Ambiguous(vec![duplicate.clone(), duplicate]),
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  assert_eq!(
    duplicate_service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::Canonical)
  );
  assert_eq!(embedding.calls.load(Ordering::Relaxed), 0);
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);

  let mut stale = root("root-b");
  stale.content = pin("release-0");
  let (service, _, _, _) = service(
    Resolution::Ambiguous(vec![root("root-a"), stale]),
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  assert_eq!(
    service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::ReleaseMismatch)
  );
}

#[tokio::test]
async fn qdrant_unavailability_returns_explicit_mysql_only_degradation() {
  let (service, canonical, _, retrieval) = service(
    Resolution::Resolved,
    Vec::new(),
    false,
    Err(RetrievalDataError::Unavailable),
  );
  let result = service
    .retrieve(&context(None), &CancellationSignal::default(), request())
    .await
    .unwrap();
  let RootRetrievalResult::Resolved(outcome) = result else {
    panic!("expected resolution")
  };
  assert_eq!(outcome.coverage, RootRetrievalCoverage::CanonicalOnly);
  assert!(outcome.exploratory.is_empty());
  assert_eq!(canonical.hydrate_calls.load(Ordering::Relaxed), 0);
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn deterministic_ties_use_canonical_identity_and_stay_exploratory() {
  let candidates = vec![
    candidate("node-z", 0.5, &["dense"]),
    candidate("node-a", 0.5, &["dense"]),
  ];
  let (service, _, _, _) = service(
    Resolution::Resolved,
    vec![hydrated("node-a"), hydrated("node-z")],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates,
    }),
  );
  let result = service
    .retrieve(&context(None), &CancellationSignal::default(), request())
    .await
    .unwrap();
  let RootRetrievalResult::Resolved(outcome) = result else {
    panic!("expected resolution")
  };
  assert_eq!(outcome.exploratory[0].node_id, id("node-a"));
  assert_eq!(outcome.exploratory[1].node_id, id("node-z"));
  assert!(outcome.inferred.is_empty());
  assert!(outcome
    .exploratory
    .iter()
    .all(|item| item.evidence_state
      == transnet::domain::root_retrieval::RootEvidenceState::Exploratory));
}

#[tokio::test]
async fn duplicate_nomination_mechanisms_fail_before_scoring() {
  let mut duplicated = candidate("node-a", 0.9, &["dense"]);
  duplicated.matched_by.push(NodeMatchMechanism::Dense);
  let (service, canonical, _, _) = service(
    Resolution::Resolved,
    vec![hydrated("node-a")],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: vec![duplicated],
    }),
  );
  assert_eq!(
    service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InconsistentProjection)
  );
  assert_eq!(canonical.hydrate_calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn hydrated_family_label_and_sense_must_match_the_pointer() {
  let mut wrong_family = hydrated("node-a");
  wrong_family.node_type = RetrievalNodeType::Concept;
  let (label_service, _, _, _) = service(
    Resolution::Resolved,
    vec![wrong_family],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: vec![candidate("node-a", 0.8, &["dense"])],
    }),
  );
  assert_eq!(
    label_service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InconsistentProjection)
  );

  let mut blank_label = hydrated("node-a");
  blank_label.canonical_label = " \n".to_string();
  let (service, _, _, _) = service(
    Resolution::Resolved,
    vec![blank_label],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: vec![candidate("node-a", 0.8, &["sparse"])],
    }),
  );
  assert_eq!(
    service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InconsistentProjection)
  );
}

#[tokio::test]
async fn dense_and_sparse_execution_receipts_must_match_the_release_spec() {
  let canonical = Arc::new(FakeCanonical {
    resolution: Resolution::Resolved,
    hydrate: Vec::new(),
    resolve_calls: AtomicUsize::new(0),
    hydrate_calls: AtomicUsize::new(0),
  });
  let retrieval = Arc::new(FakeRetrieval {
    result: Mutex::new(Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    })),
    calls: AtomicUsize::new(0),
  });
  let dense_drift = BoundedRootRetrievalService::new(
    canonical.clone(),
    canonical.clone(),
    Arc::new(FakeEmbedding {
      unavailable: false,
      model_version: "qwen-drift",
      calls: AtomicUsize::new(0),
    }),
    Arc::new(FakeLexical {
      receipt_release: id("release-1"),
    }),
    retrieval.clone(),
    execution_spec(),
  );
  assert_eq!(
    dense_drift
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InvalidSignal)
  );
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);

  let sparse_drift = BoundedRootRetrievalService::new(
    canonical.clone(),
    canonical,
    Arc::new(FakeEmbedding {
      unavailable: false,
      model_version: "qwen-test-r1",
      calls: AtomicUsize::new(0),
    }),
    Arc::new(FakeLexical {
      receipt_release: id("release-0"),
    }),
    retrieval.clone(),
    execution_spec(),
  );
  assert_eq!(
    sparse_drift
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InvalidSignal)
  );
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn missing_hydration_is_reported_as_partial_publication() {
  let candidates = vec![
    candidate("node-a", 0.8, &["dense", "sparse"]),
    candidate("node-b", 0.7, &["sparse"]),
  ];
  let (service, _, _, _) = service(
    Resolution::Resolved,
    vec![hydrated("node-a")],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates,
    }),
  );
  let result = service
    .retrieve(&context(None), &CancellationSignal::default(), request())
    .await
    .unwrap();
  let RootRetrievalResult::Resolved(outcome) = result else {
    panic!("expected resolution")
  };
  assert_eq!(outcome.coverage, RootRetrievalCoverage::PartialPublication);
  assert_eq!(outcome.exploratory.len(), 1);
}

#[tokio::test]
async fn stale_projection_release_fails_closed() {
  let (service, _, _, _) = service(
    Resolution::Resolved,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("stale-release"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  assert_eq!(
    service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InconsistentProjection)
  );
}

#[tokio::test]
async fn projection_execution_echo_must_match_before_hydration() {
  let mut execution = retrieval_echo();
  execution.collection_content_hash = "sha256:stale-projection".to_owned();
  let (service, canonical, _, _) = service(
    Resolution::Resolved,
    vec![hydrated("node-a")],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution,
      candidates: vec![candidate("node-a", 0.8, &["dense", "sparse"])],
    }),
  );
  assert_eq!(
    service
      .retrieve(&context(None), &CancellationSignal::default(), request())
      .await,
    Err(RootRetrievalError::InconsistentProjection)
  );
  assert_eq!(canonical.hydrate_calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn caller_pin_cannot_be_switched_by_active_resolution() {
  let (service, _, embedding, retrieval) = service(
    Resolution::Resolved,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      execution: retrieval_echo(),
      candidates: Vec::new(),
    }),
  );
  assert_eq!(
    service
      .retrieve(
        &context(Some(id("release-0"))),
        &CancellationSignal::default(),
        request()
      )
      .await,
    Err(RootRetrievalError::ReleaseMismatch)
  );
  assert_eq!(embedding.calls.load(Ordering::Relaxed), 0);
  assert_eq!(retrieval.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn request_debug_redacts_private_query() {
  let debug = format!("{:?}", request());
  assert!(!debug.contains("private query"));
  assert!(debug.contains("[redacted]"));
  let dense = DenseQueryVector::new(vec![0.0; DENSE_QUERY_VECTOR_DIMENSIONS]).unwrap();
  assert!(!format!("{dense:?}").contains("0.0"));
}
