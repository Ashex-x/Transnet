//! HTTP composition coverage for public graph-topology snapshot caching.

use std::{
  collections::{BTreeSet, HashMap},
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
  },
  time::{Duration, SystemTime},
};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{header, HeaderMap, Request, StatusCode},
  response::Response,
  Router,
};
use tower::ServiceExt;
use transnet::{
  adapters::{clock::FixedClock, in_memory::InMemoryGraphRepository},
  app_router,
  application::{
    graph::GraphService,
    graph_topology_cache::{
      GraphTopologySnapshot, GraphTopologySnapshotCacheService, GraphTopologySnapshotKey,
    },
  },
  domain::{
    canonical::{CanonicalId, EvidenceConfidence, LanguageTag, LexicalPartOfSpeech},
    graph::{
      GraphContentVersion, GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphNode,
      GraphNodeKey, GraphNodeKind, GraphRanking, GraphRelationType, GraphScope, GraphScore,
      GraphScoreComponents, RelationVersion, StoredGraphRelation,
    },
  },
  ports::{
    cache::{Cache, CacheEntry, CacheError},
    clock::{Clock, UtcTimestamp},
    graph_repository::{
      GraphAdjacency, GraphAdjacencyRequest, GraphNeighborPage, GraphNeighborPageRequest,
      GraphRepository, GraphRepositoryError,
    },
  },
  AppState, ProviderConfig, TranslationConfig, TranslationService,
};

type TopologyCacheEntry = CacheEntry<GraphTopologySnapshot>;

fn translation_service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string().into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap()
}

fn id(value: &str) -> CanonicalId {
  CanonicalId::new(value).unwrap()
}

fn key(value: &str) -> GraphNodeKey {
  GraphNodeKey::new(GraphNodeKind::Sense, id(value))
}

fn content(community_aggregate_version: &str) -> GraphContentVersion {
  GraphContentVersion {
    release_id: id("release-2026-09"),
    ranking_version: "graph-rank-v1".to_string(),
    community_aggregate_version: community_aggregate_version.to_string(),
  }
}

fn node(value: &str) -> GraphNode {
  GraphNode::new(
    key(value),
    value,
    Some(LanguageTag::parse("en").unwrap()),
    Some(LexicalPartOfSpeech::Adjective),
    Some(format!("{value} definition")),
    true,
  )
  .unwrap()
}

fn ranking(score: u16) -> GraphRanking {
  GraphRanking {
    display_rank: GraphScore::new(score).unwrap(),
    components: GraphScoreComponents {
      evidence: GraphScore::new(9_000).unwrap(),
      community: Some(GraphScore::new(750).unwrap()),
      pedagogical: Some(GraphScore::new(300).unwrap()),
    },
    ranking_version: "graph-rank-v1".to_string(),
  }
}

fn relation() -> StoredGraphRelation {
  StoredGraphRelation {
    edge_id: GraphEdgeId::stored(id("edge-hot-warm")),
    relation_version: RelationVersion::new(3).unwrap(),
    source: key("hot"),
    target: key("warm"),
    relation_type: GraphRelationType::Hypernym,
    evidence: GraphEvidence::new(vec![id("evidence-heat")], EvidenceConfidence::High).unwrap(),
    scope: GraphScope {
      dialect: Some(LanguageTag::parse("en-US").unwrap()),
      domain: Some("weather".to_string()),
      register: Some("neutral".to_string()),
      note: Some("temperature only".to_string()),
    },
    feedback_capabilities: BTreeSet::from([
      GraphFeedbackCapability::Accuracy,
      GraphFeedbackCapability::Usefulness,
    ]),
    ranking: ranking(9_900),
  }
}

fn source_repository(source_content: GraphContentVersion) -> InMemoryGraphRepository {
  InMemoryGraphRepository::new(source_content)
    .with_node(node("hot"))
    .with_node(node("warm"))
    .with_relation(relation())
}

/// Mutable public-content fixture that exposes only the graph port used by the application layer.
struct FixtureGraphRepository {
  source: InMemoryGraphRepository,
  source_content: GraphContentVersion,
  active_content: RwLock<GraphContentVersion>,
  adjacency_calls: AtomicUsize,
  neighbor_page_calls: AtomicUsize,
}

impl FixtureGraphRepository {
  fn new() -> Self {
    let source_content = content("community-v1");
    Self {
      source: source_repository(source_content.clone()),
      source_content: source_content.clone(),
      active_content: RwLock::new(source_content),
      adjacency_calls: AtomicUsize::new(0),
      neighbor_page_calls: AtomicUsize::new(0),
    }
  }

  fn active_content(&self) -> GraphContentVersion {
    read_lock(&self.active_content).clone()
  }

  fn set_active_content(&self, content: GraphContentVersion) {
    *write_lock(&self.active_content) = content;
  }

  fn adjacency_calls(&self) -> usize {
    self.adjacency_calls.load(Ordering::SeqCst)
  }

  fn neighbor_page_calls(&self) -> usize {
    self.neighbor_page_calls.load(Ordering::SeqCst)
  }
}

#[async_trait]
impl GraphRepository for FixtureGraphRepository {
  async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
    Ok(self.active_content())
  }

  async fn load_nodes(
    &self,
    _content: &GraphContentVersion,
    keys: &[GraphNodeKey],
  ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
    self.source.load_nodes(&self.source_content, keys).await
  }

  async fn adjacency(
    &self,
    request: &GraphAdjacencyRequest,
  ) -> Result<GraphAdjacency, GraphRepositoryError> {
    self.adjacency_calls.fetch_add(1, Ordering::SeqCst);
    if request.content != self.active_content() {
      return Err(GraphRepositoryError::InconsistentData);
    }
    let mut source_request = request.clone();
    source_request.content = self.source_content.clone();
    self.source.adjacency(&source_request).await
  }

  async fn neighbor_page(
    &self,
    request: &GraphNeighborPageRequest,
  ) -> Result<GraphNeighborPage, GraphRepositoryError> {
    self.neighbor_page_calls.fetch_add(1, Ordering::SeqCst);
    if request.content != self.active_content() {
      return Err(GraphRepositoryError::InconsistentData);
    }
    let mut source_request = request.clone();
    source_request.content = self.source_content.clone();
    self.source.neighbor_page(&source_request).await
  }
}

/// Graph dependency that must never be selected after a cache-backed graph arrangement is added.
struct UnavailableGraphRepository;

#[async_trait]
impl GraphRepository for UnavailableGraphRepository {
  async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }

  async fn load_nodes(
    &self,
    _content: &GraphContentVersion,
    _keys: &[GraphNodeKey],
  ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }

  async fn adjacency(
    &self,
    _request: &GraphAdjacencyRequest,
  ) -> Result<GraphAdjacency, GraphRepositoryError> {
    Err(GraphRepositoryError::Unavailable)
  }
}

#[derive(Default)]
struct CacheCounters {
  gets: AtomicUsize,
  puts: AtomicUsize,
  removes: AtomicUsize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CacheCounts {
  gets: usize,
  puts: usize,
  removes: usize,
}

#[derive(Clone)]
enum TestCacheMode {
  Available,
  Unavailable,
  FixedEntry(Box<TopologyCacheEntry>),
}

/// Instrumented cache fixture that can return an incompatible snapshot regardless of the request.
struct TestTopologyCache {
  mode: Mutex<TestCacheMode>,
  entries: Mutex<HashMap<GraphTopologySnapshotKey, TopologyCacheEntry>>,
  counters: CacheCounters,
}

impl TestTopologyCache {
  fn available() -> Self {
    Self {
      mode: Mutex::new(TestCacheMode::Available),
      entries: Mutex::new(HashMap::new()),
      counters: CacheCounters::default(),
    }
  }

  fn unavailable() -> Self {
    Self {
      mode: Mutex::new(TestCacheMode::Unavailable),
      entries: Mutex::new(HashMap::new()),
      counters: CacheCounters::default(),
    }
  }

  fn with_fixed_entry(entry: TopologyCacheEntry) -> Self {
    Self {
      mode: Mutex::new(TestCacheMode::FixedEntry(Box::new(entry))),
      entries: Mutex::new(HashMap::new()),
      counters: CacheCounters::default(),
    }
  }

  fn counts(&self) -> CacheCounts {
    CacheCounts {
      gets: self.counters.gets.load(Ordering::SeqCst),
      puts: self.counters.puts.load(Ordering::SeqCst),
      removes: self.counters.removes.load(Ordering::SeqCst),
    }
  }

  fn only_entry(&self) -> TopologyCacheEntry {
    lock(&self.entries)
      .values()
      .next()
      .cloned()
      .expect("the test cache must hold one generated snapshot")
  }
}

#[async_trait]
impl Cache<GraphTopologySnapshotKey, GraphTopologySnapshot> for TestTopologyCache {
  async fn get(
    &self,
    key: &GraphTopologySnapshotKey,
  ) -> Result<Option<TopologyCacheEntry>, CacheError> {
    self.counters.gets.fetch_add(1, Ordering::SeqCst);
    match lock(&self.mode).clone() {
      TestCacheMode::Available => Ok(lock(&self.entries).get(key).cloned()),
      TestCacheMode::Unavailable => Err(CacheError::Unavailable),
      TestCacheMode::FixedEntry(entry) => Ok(Some(*entry)),
    }
  }

  async fn put(
    &self,
    key: GraphTopologySnapshotKey,
    value: GraphTopologySnapshot,
    expires_at: UtcTimestamp,
  ) -> Result<(), CacheError> {
    self.counters.puts.fetch_add(1, Ordering::SeqCst);
    if matches!(*lock(&self.mode), TestCacheMode::Unavailable) {
      return Err(CacheError::Unavailable);
    }
    lock(&self.entries).insert(key, CacheEntry::new(value, expires_at));
    Ok(())
  }

  async fn remove(&self, key: &GraphTopologySnapshotKey) -> Result<(), CacheError> {
    self.counters.removes.fetch_add(1, Ordering::SeqCst);
    if matches!(*lock(&self.mode), TestCacheMode::Unavailable) {
      return Err(CacheError::Unavailable);
    }
    lock(&self.entries).remove(key);
    Ok(())
  }
}

fn topology_cache_service(
  repository: Arc<FixtureGraphRepository>,
  cache: Arc<TestTopologyCache>,
) -> Arc<GraphTopologySnapshotCacheService> {
  let graph = Arc::new(GraphService::new(repository));
  let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
  ));
  let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> = cache;
  Arc::new(
    GraphTopologySnapshotCacheService::new(graph, cache, clock, Duration::from_secs(60)).unwrap(),
  )
}

fn cached_graph_app(cache: Arc<GraphTopologySnapshotCacheService>) -> Router {
  app_router(AppState::new(translation_service()).with_graph_topology_snapshot_cache(cache))
}

fn graph_request(root_id: &str) -> Request<Body> {
  Request::get(format!(
    "/v1/graph?root_kind=sense&root_id={root_id}&depth=1&node_limit=3&edge_limit=2"
  ))
  .body(Body::empty())
  .unwrap()
}

fn neighbor_request() -> Request<Body> {
  Request::get("/v1/graph/nodes/sense/hot/neighbors?node_limit=2&edge_limit=1")
    .body(Body::empty())
    .unwrap()
}

async fn wire(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
  let status = response.status();
  let headers = response.headers().clone();
  let body = to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap()
    .to_vec();
  (status, headers, body)
}

fn assert_no_cache_state_headers(headers: &HeaderMap) {
  assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-store");
  assert!(headers.get(header::ETAG).is_none());
  assert!(headers.get("x-cache").is_none());
  assert!(headers.get("x-cache-status").is_none());
}

fn read_lock<Value>(lock: &RwLock<Value>) -> RwLockReadGuard<'_, Value> {
  match lock.read() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

fn write_lock<Value>(lock: &RwLock<Value>) -> RwLockWriteGuard<'_, Value> {
  match lock.write() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

fn lock<Value>(lock: &Mutex<Value>) -> MutexGuard<'_, Value> {
  match lock.lock() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

#[tokio::test]
async fn full_graph_cache_hit_miss_and_outage_rebuilds_keep_the_public_wire_contract() {
  let repository = Arc::new(FixtureGraphRepository::new());
  let cache = Arc::new(TestTopologyCache::available());
  let router = cached_graph_app(topology_cache_service(repository.clone(), cache.clone()));

  let (miss_status, miss_headers, miss_body) =
    wire(router.clone().oneshot(graph_request("hot")).await.unwrap()).await;
  let (hit_status, hit_headers, hit_body) =
    wire(router.oneshot(graph_request("hot")).await.unwrap()).await;

  assert_eq!(miss_status, StatusCode::OK);
  assert_eq!(hit_status, StatusCode::OK);
  assert_no_cache_state_headers(&miss_headers);
  assert_no_cache_state_headers(&hit_headers);
  assert_eq!(miss_body, hit_body);
  assert_eq!(repository.adjacency_calls(), 1);
  assert_eq!(
    cache.counts(),
    CacheCounts {
      gets: 2,
      puts: 1,
      removes: 0,
    }
  );

  let outage_repository = Arc::new(FixtureGraphRepository::new());
  let outage_router = cached_graph_app(topology_cache_service(
    outage_repository.clone(),
    Arc::new(TestTopologyCache::unavailable()),
  ));
  let (outage_status, outage_headers, outage_body) =
    wire(outage_router.oneshot(graph_request("hot")).await.unwrap()).await;

  assert_eq!(outage_status, StatusCode::OK);
  assert_no_cache_state_headers(&outage_headers);
  assert_eq!(outage_body, miss_body);
  assert_eq!(outage_repository.adjacency_calls(), 1);
}

#[tokio::test]
async fn full_graph_cache_invalidates_when_the_public_content_version_changes() {
  let repository = Arc::new(FixtureGraphRepository::new());
  let cache = Arc::new(TestTopologyCache::available());
  let router = cached_graph_app(topology_cache_service(repository.clone(), cache.clone()));

  let (_, _, first_body) = wire(router.clone().oneshot(graph_request("hot")).await.unwrap()).await;
  repository.set_active_content(content("community-v2"));
  let (_, _, second_body) = wire(router.clone().oneshot(graph_request("hot")).await.unwrap()).await;
  let (_, _, third_body) = wire(router.oneshot(graph_request("hot")).await.unwrap()).await;

  let first: serde_json::Value = serde_json::from_slice(&first_body).unwrap();
  let second: serde_json::Value = serde_json::from_slice(&second_body).unwrap();
  assert_eq!(
    first["content_version"]["community_aggregate_version"],
    "community-v1"
  );
  assert_eq!(
    second["content_version"]["community_aggregate_version"],
    "community-v2"
  );
  assert_eq!(second_body, third_body);
  assert_eq!(repository.adjacency_calls(), 2);
  assert_eq!(
    cache.counts(),
    CacheCounts {
      gets: 3,
      puts: 2,
      removes: 0,
    }
  );
}

#[tokio::test]
async fn incompatible_cached_topology_is_discarded_and_rebuilt_before_http_serialization() {
  let generated_repository = Arc::new(FixtureGraphRepository::new());
  let generated_cache = Arc::new(TestTopologyCache::available());
  let generated_router = cached_graph_app(topology_cache_service(
    generated_repository,
    generated_cache.clone(),
  ));
  let generated = generated_router
    .oneshot(graph_request("warm"))
    .await
    .unwrap();
  assert_eq!(generated.status(), StatusCode::OK);

  let incompatible_cache = Arc::new(TestTopologyCache::with_fixed_entry(
    generated_cache.only_entry(),
  ));
  let repository = Arc::new(FixtureGraphRepository::new());
  let router = cached_graph_app(topology_cache_service(
    repository.clone(),
    incompatible_cache.clone(),
  ));
  let (status, headers, body) = wire(router.oneshot(graph_request("hot")).await.unwrap()).await;

  assert_eq!(status, StatusCode::OK);
  assert_no_cache_state_headers(&headers);
  let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
  assert_eq!(
    body["root"],
    serde_json::json!({"kind": "sense", "id": "hot"})
  );
  assert_eq!(repository.adjacency_calls(), 1);
  assert_eq!(
    incompatible_cache.counts(),
    CacheCounts {
      gets: 1,
      puts: 1,
      removes: 1,
    }
  );
}

#[tokio::test]
async fn cache_backed_graph_routes_derive_neighbors_from_the_same_graph_without_cache_access() {
  let repository = Arc::new(FixtureGraphRepository::new());
  let cache = Arc::new(TestTopologyCache::available());
  let topology_cache = topology_cache_service(repository.clone(), cache.clone());
  let unrelated = Arc::new(GraphService::new(Arc::new(UnavailableGraphRepository)));
  let router = app_router(
    AppState::new(translation_service())
      .with_graph_service(unrelated)
      .with_graph_topology_snapshot_cache(topology_cache),
  );

  let (neighbor_status, neighbor_headers, neighbor_body) =
    wire(router.clone().oneshot(neighbor_request()).await.unwrap()).await;

  assert_eq!(neighbor_status, StatusCode::OK);
  assert_no_cache_state_headers(&neighbor_headers);
  let neighbor: serde_json::Value = serde_json::from_slice(&neighbor_body).unwrap();
  assert_eq!(neighbor["edges"][0]["id"], "edge-hot-warm");
  assert_eq!(repository.neighbor_page_calls(), 1);
  assert_eq!(cache.counts(), CacheCounts::default());

  let full_response = router.oneshot(graph_request("hot")).await.unwrap();
  assert_eq!(full_response.status(), StatusCode::OK);
  assert_eq!(repository.adjacency_calls(), 1);
}

#[tokio::test]
async fn full_graph_cache_rejects_private_or_entity_tag_query_inputs_and_never_exposes_entity_tags()
{
  let repository = Arc::new(FixtureGraphRepository::new());
  let cache = Arc::new(TestTopologyCache::available());
  let router = cached_graph_app(topology_cache_service(repository.clone(), cache.clone()));

  let (_, _, baseline_body) =
    wire(router.clone().oneshot(graph_request("hot")).await.unwrap()).await;
  let request =
    Request::get("/v1/graph?root_kind=sense&root_id=hot&depth=1&node_limit=3&edge_limit=2")
      .header(header::IF_NONE_MATCH, "\"private-entity-tag\"")
      .header("x-learner-id", "private-learner")
      .body(Body::empty())
      .unwrap();
  let (header_status, header_headers, header_body) =
    wire(router.clone().oneshot(request).await.unwrap()).await;

  assert_eq!(header_status, StatusCode::OK);
  assert_no_cache_state_headers(&header_headers);
  assert_eq!(header_body, baseline_body);
  assert_eq!(repository.adjacency_calls(), 1);
  assert_eq!(cache.counts().gets, 2);

  let rejected = router
    .oneshot(
      Request::get(
        "/v1/graph?root_kind=sense&root_id=hot&depth=1&node_limit=3&edge_limit=2&learner_id=private-learner&etag=private-entity-tag",
      )
      .body(Body::empty())
      .unwrap(),
    )
    .await
    .unwrap();
  let (rejected_status, rejected_headers, rejected_body) = wire(rejected).await;

  assert_eq!(rejected_status, StatusCode::BAD_REQUEST);
  assert_eq!(rejected_headers.get(header::ETAG), None);
  let rejected_body: serde_json::Value = serde_json::from_slice(&rejected_body).unwrap();
  assert_eq!(rejected_body["code"], "invalid_graph_request");
  assert!(!rejected_body.to_string().contains("private-learner"));
  assert!(!rejected_body.to_string().contains("private-entity-tag"));
  assert_eq!(cache.counts().gets, 2);
}
