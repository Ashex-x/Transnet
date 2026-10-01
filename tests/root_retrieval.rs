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
    canonical::{CanonicalId, LanguageTag},
    model_runtime::{CancellationSignal, EphemeralEmbedding, ModelVersion},
    request_context::{RequestContext, RequestId},
    retrieval_data::{
      DenseQueryVector, EdgeSearchRequest, EdgeSearchResult, NeighborSearchRequest,
      NeighborSearchResult, NodeCandidate, NodeCandidatePayload, NodeSearchRequest,
      NodeSearchResult, RetrievalDataScore, RetrievalNodeType, RetrievalVerificationState,
      ScaleSearchRequest, ScaleSearchResult, SparseQueryVector, DENSE_QUERY_VECTOR_DIMENSIONS,
    },
    root_retrieval::{
      CanonicalRoot, CanonicalRootResolution, HydratedRootNode, RootQuery, RootRetrievalCoverage,
      RootRetrievalRequest, RootRetrievalResult,
    },
  },
  ports::{
    model_runtime::{EmbeddingPort, EmbeddingRequest, ModelOperationContext, ModelOperationError},
    retrieval_data::{RetrievalDataError, RetrievalDataPort},
    root_retrieval::{
      CanonicalRootError, CanonicalRootPort, QueryLexicalEncoderError, QueryLexicalEncoderPort,
    },
  },
};

#[derive(Clone)]
enum Resolution {
  Missing,
  Ambiguous,
  Resolved,
}

struct FakeCanonical {
  resolution: Resolution,
  hydrate: Vec<HydratedRootNode>,
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
    Ok(match self.resolution {
      Resolution::Missing => CanonicalRootResolution::NotFound,
      Resolution::Ambiguous => {
        CanonicalRootResolution::Ambiguous(vec![root("root-a"), root("root-b")])
      }
      Resolution::Resolved => CanonicalRootResolution::Resolved(root("root")),
    })
  }

  async fn hydrate_nodes(
    &self,
    _context: &RequestContext,
    _release_id: &CanonicalId,
    _node_ids: &[CanonicalId],
  ) -> Result<Vec<HydratedRootNode>, CanonicalRootError> {
    self.hydrate_calls.fetch_add(1, Ordering::Relaxed);
    Ok(self.hydrate.clone())
  }
}

struct FakeEmbedding {
  unavailable: bool,
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
      ModelVersion::new("qwen-test-r1").unwrap(),
    )
    .map_err(|_| ModelOperationError::InvalidOutput)
  }
}

struct FakeLexical;

#[async_trait]
impl QueryLexicalEncoderPort for FakeLexical {
  async fn encode(
    &self,
    _context: &RequestContext,
    _release_id: &CanonicalId,
    _query: &RootQuery,
  ) -> Result<SparseQueryVector, QueryLexicalEncoderError> {
    SparseQueryVector::new(vec![1], vec![1.0])
      .map_err(|_| QueryLexicalEncoderError::InvalidEncoding)
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

fn root(node: &str) -> CanonicalRoot {
  CanonicalRoot {
    release_id: id("release-1"),
    node_id: id(node),
    sense_id: id(&format!("sense-{node}")),
    canonical_label: format!("label-{node}"),
  }
}

fn hydrated(node: &str) -> HydratedRootNode {
  HydratedRootNode {
    release_id: id("release-1"),
    node_id: id(node),
    sense_id: Some(id(&format!("sense-{node}"))),
    canonical_label: format!("label-{node}"),
  }
}

fn candidate(node: &str, score: f32, matched_by: &[&str]) -> NodeCandidate {
  NodeCandidate {
    node_id: id(node),
    score: RetrievalDataScore::new(score).unwrap(),
    matched_by: matched_by
      .iter()
      .map(|value| (*value).to_string())
      .collect(),
    payload: NodeCandidatePayload {
      node_type: RetrievalNodeType::LexicalSense,
      sense_id: Some(id(&format!("sense-{node}"))),
      canonical_label: format!("label-{node}"),
      verification_state: RetrievalVerificationState::Verified,
    },
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
  hydrate: Vec<HydratedRootNode>,
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
    calls: AtomicUsize::new(0),
  });
  let retrieval = Arc::new(FakeRetrieval {
    result: Mutex::new(result),
    calls: AtomicUsize::new(0),
  });
  let composed = BoundedRootRetrievalService::new(
    canonical.clone(),
    embedding.clone(),
    Arc::new(FakeLexical),
    retrieval.clone(),
  );
  (composed, canonical, embedding, retrieval)
}

#[tokio::test]
async fn no_root_never_creates_vectors_or_queries_retrieval_data() {
  let (service, _, embedding, retrieval) = service(
    Resolution::Missing,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
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
    Resolution::Ambiguous,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
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
async fn missing_hydration_is_reported_as_partial_publication() {
  let candidates = vec![
    candidate("node-a", 0.8, &["hybrid"]),
    candidate("node-b", 0.7, &["sparse"]),
  ];
  let (service, _, _, _) = service(
    Resolution::Resolved,
    vec![hydrated("node-a")],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
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
async fn cross_release_hydration_fails_closed() {
  let mut wrong_release = hydrated("node-a");
  wrong_release.release_id = id("release-0");
  let (service, _, _, _) = service(
    Resolution::Resolved,
    vec![wrong_release],
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
      candidates: vec![candidate("node-a", 0.8, &["hybrid"])],
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
async fn caller_pin_cannot_be_switched_by_active_resolution() {
  let (service, _, embedding, retrieval) = service(
    Resolution::Resolved,
    Vec::new(),
    false,
    Ok(NodeSearchResult {
      release_id: id("release-1"),
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
