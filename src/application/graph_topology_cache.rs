//! Public graph-topology snapshots cached by versioned, learner-neutral inputs.
//!
//! This module caches only bounded [`crate::domain::graph::GraphReadResult`] values from multi-hop
//! graph reads. Its key carries the immutable public content, ranker, and community-aggregate tuple
//! together with every typed traversal input. It deliberately accepts neither learner state nor
//! entity tags, and it does not make authorization decisions, expose an HTTP route, or claim a
//! production cache.

use std::{
  cmp::Ordering,
  collections::BTreeSet,
  hash::{Hash, Hasher},
  sync::Arc,
  time::Duration,
};

use thiserror::Error;

use crate::{
  application::graph::{GraphReadError, GraphService},
  domain::graph::{
    compare_graph_edges, GraphContentVersion, GraphFilter, GraphNodeKey, GraphReadRequest,
    GraphReadResult, GraphValidationError,
  },
  ports::{
    cache::Cache,
    clock::{Clock, UtcTimestamp},
  },
};

/// Validation failure for a public graph-topology cache key or cached snapshot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphTopologySnapshotValidationError {
  /// The source graph request was outside the bounded public graph contract.
  #[error(transparent)]
  Request(#[from] GraphValidationError),
  /// A cached result was pinned to different public graph content.
  #[error("graph topology snapshot content does not match its cache key")]
  ContentMismatch,
  /// A cached result was rooted at a different typed node.
  #[error("graph topology snapshot root does not match its cache key")]
  RootMismatch,
  /// A cached result carried a neighbor-page cursor instead of a complete topology read.
  #[error("graph topology snapshot must not contain a pagination cursor")]
  PaginationCursor,
  /// A cached result contained more nodes than its cache key permits.
  #[error("graph topology snapshot exceeds its cache-key node limit")]
  NodeLimitExceeded,
  /// A cached result contained more edges than its cache key permits.
  #[error("graph topology snapshot exceeds its cache-key edge limit")]
  EdgeLimitExceeded,
  /// The typed root node was not present in the cached topology.
  #[error("graph topology snapshot does not contain its root node")]
  MissingRootNode,
  /// A cached result repeated one typed graph node key.
  #[error("graph topology snapshot has duplicate typed graph nodes")]
  DuplicateNode,
  /// Cached graph nodes were not in deterministic typed-key order.
  #[error("graph topology snapshot has non-deterministic node order")]
  UnorderedNodes,
  /// A cached result repeated one public graph edge identifier.
  #[error("graph topology snapshot has duplicate graph edges")]
  DuplicateEdge,
  /// A cached edge endpoint was not present in the cached node set.
  #[error("graph topology snapshot has an edge without both endpoint nodes")]
  MissingEdgeEndpoint,
  /// A cached edge was not allowed by the relation-type filter in the cache key.
  #[error("graph topology snapshot has an edge excluded by its cache-key filter")]
  FilterMismatch,
  /// A cached edge used a ranker version different from the cache-key content tuple.
  #[error("graph topology snapshot has an edge with a different ranking version")]
  RankingVersionMismatch,
  /// Cached graph edges were not in the public deterministic order.
  #[error("graph topology snapshot has non-deterministic edge order")]
  UnorderedEdges,
}

/// Stable public identity for one rebuildable bounded graph-topology snapshot.
///
/// The key pins the immutable graph release, ranking version, and public community-aggregate
/// version. It also includes the typed root, relation filter, depth, node limit, and edge limit.
/// It has no field for learner identity, history, feedback, personalization, authorization, or
/// entity tags, so such input cannot select or populate this shared topology cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTopologySnapshotKey {
  content: GraphContentVersion,
  root: GraphNodeKey,
  filter: GraphFilter,
  depth: u8,
  node_limit: usize,
  edge_limit: usize,
}

impl GraphTopologySnapshotKey {
  /// Builds a cache key from a validated bounded graph request and active public content tuple.
  ///
  /// # Errors
  ///
  /// Returns an error when the request's depth or node or edge limits are outside the graph
  /// contract. This repeats request validation because public request fields can be constructed
  /// directly by Rust callers.
  pub fn new(
    request: &GraphReadRequest,
    content: GraphContentVersion,
  ) -> Result<Self, GraphTopologySnapshotValidationError> {
    validate_request(request)?;
    Ok(Self {
      content,
      root: request.root.clone(),
      filter: request.filter.clone(),
      depth: request.depth,
      node_limit: request.node_limit,
      edge_limit: request.edge_limit,
    })
  }

  /// Returns the immutable release, ranking, and public community-aggregate tuple.
  pub fn content(&self) -> &GraphContentVersion {
    &self.content
  }

  /// Returns the typed canonical root node.
  pub fn root(&self) -> &GraphNodeKey {
    &self.root
  }

  /// Returns the public relation-type filter.
  pub fn filter(&self) -> &GraphFilter {
    &self.filter
  }

  /// Returns the bounded breadth-first traversal depth.
  pub fn depth(&self) -> u8 {
    self.depth
  }

  /// Returns the maximum cached node count, including the root and all edge endpoints.
  pub fn node_limit(&self) -> usize {
    self.node_limit
  }

  /// Returns the maximum cached edge count.
  pub fn edge_limit(&self) -> usize {
    self.edge_limit
  }

  /// Validates a full graph-read result before it is retained as a shared snapshot.
  fn validate_result(
    &self,
    result: &GraphReadResult,
  ) -> Result<(), GraphTopologySnapshotValidationError> {
    if result.content != self.content {
      return Err(GraphTopologySnapshotValidationError::ContentMismatch);
    }
    if result.root != self.root {
      return Err(GraphTopologySnapshotValidationError::RootMismatch);
    }
    if result.next_cursor.is_some() {
      return Err(GraphTopologySnapshotValidationError::PaginationCursor);
    }
    if result.nodes.len() > self.node_limit {
      return Err(GraphTopologySnapshotValidationError::NodeLimitExceeded);
    }
    if result.edges.len() > self.edge_limit {
      return Err(GraphTopologySnapshotValidationError::EdgeLimitExceeded);
    }

    let mut node_keys = BTreeSet::new();
    for node in &result.nodes {
      if !node_keys.insert(node.key.clone()) {
        return Err(GraphTopologySnapshotValidationError::DuplicateNode);
      }
    }
    if result
      .nodes
      .windows(2)
      .any(|nodes| nodes[0].key.cmp(&nodes[1].key).is_ge())
    {
      return Err(GraphTopologySnapshotValidationError::UnorderedNodes);
    }
    if !node_keys.contains(&self.root) {
      return Err(GraphTopologySnapshotValidationError::MissingRootNode);
    }

    let mut edge_ids = BTreeSet::new();
    for edge in &result.edges {
      if !edge_ids.insert(edge.id.clone()) {
        return Err(GraphTopologySnapshotValidationError::DuplicateEdge);
      }
      if !node_keys.contains(&edge.source) || !node_keys.contains(&edge.target) {
        return Err(GraphTopologySnapshotValidationError::MissingEdgeEndpoint);
      }
      if !self.filter.allows(edge.relation_type) {
        return Err(GraphTopologySnapshotValidationError::FilterMismatch);
      }
      if edge.ranking.ranking_version != self.content.ranking_version {
        return Err(GraphTopologySnapshotValidationError::RankingVersionMismatch);
      }
    }
    if result
      .edges
      .windows(2)
      .any(|edges| compare_graph_edges(&edges[0], &edges[1]) == Ordering::Greater)
    {
      return Err(GraphTopologySnapshotValidationError::UnorderedEdges);
    }
    Ok(())
  }
}

impl Hash for GraphTopologySnapshotKey {
  fn hash<State: Hasher>(&self, state: &mut State) {
    self.content.release_id.hash(state);
    self.content.ranking_version.hash(state);
    self.content.community_aggregate_version.hash(state);
    self.root.hash(state);
    self.filter.relation_types.len().hash(state);
    for relation_type in &self.filter.relation_types {
      relation_type.hash(state);
    }
    self.depth.hash(state);
    self.node_limit.hash(state);
    self.edge_limit.hash(state);
  }
}

/// One validated cache value containing a bounded, internally complete public graph result.
///
/// The value repeats its key so an adapter that returns a value for the wrong key cannot cause a
/// different root, filter, depth, or bound to become a cache hit. The snapshot constructor is
/// intentionally module-private: only [`GraphTopologySnapshotCacheService`] creates one after an
/// authoritative [`GraphService::read`] call. A structurally similar terminal neighbor page is not
/// sufficient provenance for a shared topology snapshot.
///
/// ```compile_fail
/// use transnet::{
///   application::graph_topology_cache::{GraphTopologySnapshot, GraphTopologySnapshotKey},
///   domain::graph::GraphReadResult,
/// };
///
/// fn cannot_store_a_terminal_neighbor_page(
///   key: GraphTopologySnapshotKey,
///   page: GraphReadResult,
/// ) {
///   let _ = GraphTopologySnapshot::new(key, page);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTopologySnapshot {
  key: GraphTopologySnapshotKey,
  result: GraphReadResult,
}

impl GraphTopologySnapshot {
  /// Creates a cache value after this module has obtained an authoritative full graph read.
  fn new(
    key: GraphTopologySnapshotKey,
    result: GraphReadResult,
  ) -> Result<Self, GraphTopologySnapshotValidationError> {
    key.validate_result(&result)?;
    Ok(Self { key, result })
  }

  /// Returns whether this internally validated value still matches every field of `key`.
  fn matches_key(&self, key: &GraphTopologySnapshotKey) -> bool {
    self.key == *key && self.key.validate_result(&self.result).is_ok()
  }

  /// Returns the cache key carried by this snapshot.
  pub fn key(&self) -> &GraphTopologySnapshotKey {
    &self.key
  }

  /// Returns the bounded internally complete public topology.
  pub fn result(&self) -> &GraphReadResult {
    &self.result
  }

  /// Consumes the snapshot and returns its graph topology.
  pub fn into_result(self) -> GraphReadResult {
    self.result
  }
}

/// Outcome of reading a public graph topology through the shared snapshot cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphTopologySnapshotCacheResult {
  /// A compatible unexpired public snapshot was served from the cache.
  Hit(GraphReadResult),
  /// A cache miss, expiry, or incompatible entry rebuilt the public topology.
  Miss(GraphReadResult),
  /// The cache dependency could not safely complete an operation, so topology was rebuilt uncached.
  Unavailable(GraphReadResult),
}

impl GraphTopologySnapshotCacheResult {
  /// Returns the public graph topology obtained or rebuilt by this cache operation.
  pub fn result(&self) -> &GraphReadResult {
    match self {
      Self::Hit(result) | Self::Miss(result) | Self::Unavailable(result) => result,
    }
  }
}

/// Failure that prevents authoritative graph reads from safely producing a cacheable snapshot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphTopologySnapshotCacheError {
  /// The authoritative graph service could not resolve or build public topology.
  #[error(transparent)]
  Graph(#[from] GraphReadError),
  /// The request or resulting topology violated the public snapshot contract.
  #[error(transparent)]
  Contract(#[from] GraphTopologySnapshotValidationError),
  /// A zero lifetime cannot store a meaningful public topology snapshot.
  #[error("graph topology snapshot cache lifetime must be greater than zero")]
  ZeroTtl,
}

/// Caches rebuildable public graph topology without accepting private graph influences.
///
/// Cache reads and writes are best effort. An unavailable cache, expired entry, or incompatible
/// value never becomes a topology response: the service rebuilds through [`GraphService`] instead.
/// The service only accepts [`GraphReadRequest`], not neighbor-page cursors, private feedback,
/// learner state, or entity tags. Callers remain responsible for authorization before invoking it.
#[derive(Clone)]
pub struct GraphTopologySnapshotCacheService {
  graph: Arc<GraphService>,
  cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>>,
  clock: Arc<dyn Clock>,
  ttl: Duration,
}

impl GraphTopologySnapshotCacheService {
  /// Creates a public topology cache service with an explicit nonzero snapshot lifetime.
  ///
  /// # Errors
  ///
  /// Returns an error when `ttl` is zero.
  pub fn new(
    graph: Arc<GraphService>,
    cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>>,
    clock: Arc<dyn Clock>,
    ttl: Duration,
  ) -> Result<Self, GraphTopologySnapshotCacheError> {
    if ttl.is_zero() {
      return Err(GraphTopologySnapshotCacheError::ZeroTtl);
    }
    Ok(Self {
      graph,
      cache,
      clock,
      ttl,
    })
  }

  /// Reads a bounded public graph topology through a version-pinned shared snapshot cache.
  ///
  /// An unexpired compatible value is a hit. A miss, expired value, or incompatible value rebuilds
  /// through the authoritative graph service. If any cache operation is unavailable or the expiry
  /// cannot be represented, the rebuilt result is returned uncached. The result is never derived
  /// from private learner state and never carries an entity tag.
  ///
  /// # Errors
  ///
  /// Returns an error when the graph request is invalid or authoritative graph reads cannot safely
  /// produce an internally complete result. Cache misses and cache failures do not become errors.
  pub async fn read(
    &self,
    request: GraphReadRequest,
  ) -> Result<GraphTopologySnapshotCacheResult, GraphTopologySnapshotCacheError> {
    let request = validate_request(&request)?;
    let initial_content = self.graph.active_content_version().await?;
    let initial_key = GraphTopologySnapshotKey::new(&request, initial_content)?;
    let cache_usable = match self.cache.get(&initial_key).await {
      Ok(Some(entry))
        if entry.expires_at() > self.clock.now() && entry.value().matches_key(&initial_key) =>
      {
        tracing::debug!("served public graph topology snapshot cache hit");
        return Ok(GraphTopologySnapshotCacheResult::Hit(
          entry.into_value().into_result(),
        ));
      }
      Ok(Some(_)) => {
        tracing::warn!(
          "discarded expired or incompatible public graph topology snapshot cache entry"
        );
        self.cache.remove(&initial_key).await.is_ok()
      }
      Ok(None) => true,
      Err(_) => false,
    };

    let result = self.graph.read(request.clone()).await?;
    let result_key = GraphTopologySnapshotKey::new(&request, result.content.clone())?;
    let snapshot = GraphTopologySnapshot::new(result_key.clone(), result)?;
    let result = snapshot.result().clone();

    if !cache_usable {
      tracing::warn!("public graph topology snapshot cache unavailable; served rebuilt topology");
      return Ok(GraphTopologySnapshotCacheResult::Unavailable(result));
    }
    let Some(expires_at) = self.expires_at() else {
      tracing::warn!(
        "could not represent graph topology snapshot cache expiry; served rebuilt topology"
      );
      return Ok(GraphTopologySnapshotCacheResult::Unavailable(result));
    };
    match self.cache.put(result_key, snapshot, expires_at).await {
      Ok(()) => {
        tracing::debug!("rebuilt and stored public graph topology snapshot cache miss");
        Ok(GraphTopologySnapshotCacheResult::Miss(result))
      }
      Err(_) => {
        tracing::warn!("public graph topology snapshot cache unavailable; served rebuilt topology");
        Ok(GraphTopologySnapshotCacheResult::Unavailable(result))
      }
    }
  }

  fn expires_at(&self) -> Option<UtcTimestamp> {
    self.clock.now().checked_add(self.ttl)
  }
}

fn validate_request(
  request: &GraphReadRequest,
) -> Result<GraphReadRequest, GraphTopologySnapshotValidationError> {
  Ok(GraphReadRequest::new(
    request.root.clone(),
    request.depth,
    request.node_limit,
    request.edge_limit,
    request.filter.clone(),
  )?)
}

#[cfg(test)]
mod tests {
  use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, SystemTime},
  };

  use async_trait::async_trait;

  use super::*;
  use crate::{
    adapters::{
      clock::FixedClock,
      in_memory::{InMemoryCache, InMemoryGraphRepository},
    },
    domain::{
      canonical::{CanonicalId, EvidenceConfidence, LanguageTag},
      graph::{
        GraphEdgeId, GraphEvidence, GraphNeighborRequest, GraphNode, GraphNodeKind, GraphRanking,
        GraphRelationType, GraphScope, GraphScore, GraphScoreComponents, RelationVersion,
        StoredGraphRelation,
      },
    },
    ports::{
      cache::{Cache, CacheEntry, CacheError},
      graph_repository::{
        GraphAdjacency, GraphAdjacencyRequest, GraphRepository, GraphRepositoryError,
      },
    },
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn key(value: &str) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Sense, id(value))
  }

  fn content() -> GraphContentVersion {
    GraphContentVersion {
      release_id: id("release-1"),
      ranking_version: "graph-rank-v1".to_string(),
      community_aggregate_version: "community-v1".to_string(),
    }
  }

  fn node(value: &str) -> GraphNode {
    GraphNode::new(
      key(value),
      value,
      Some(LanguageTag::parse("en").unwrap()),
      None,
      None,
      true,
    )
    .unwrap()
  }

  fn relation() -> StoredGraphRelation {
    StoredGraphRelation {
      edge_id: GraphEdgeId::stored(id("edge-root-target")),
      relation_version: RelationVersion::new(1).unwrap(),
      source: key("root"),
      target: key("target"),
      relation_type: GraphRelationType::Hypernym,
      evidence: GraphEvidence::new(vec![id("evidence-1")], EvidenceConfidence::High).unwrap(),
      scope: GraphScope::default(),
      feedback_capabilities: Default::default(),
      ranking: GraphRanking {
        display_rank: GraphScore::new(9_000).unwrap(),
        components: GraphScoreComponents {
          evidence: GraphScore::new(9_000).unwrap(),
          community: None,
          pedagogical: None,
        },
        ranking_version: "graph-rank-v1".to_string(),
      },
    }
  }

  fn repository() -> Arc<dyn GraphRepository> {
    Arc::new(
      InMemoryGraphRepository::new(content())
        .with_node(node("root"))
        .with_node(node("target"))
        .with_relation(relation()),
    )
  }

  fn request() -> GraphReadRequest {
    GraphReadRequest::new(key("root"), 1, 3, 2, GraphFilter::default()).unwrap()
  }

  fn clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::new(
      SystemTime::UNIX_EPOCH + Duration::from_secs(100),
    ))
  }

  fn service(
    repository: Arc<dyn GraphRepository>,
    cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>>,
    clock: Arc<FixedClock>,
  ) -> GraphTopologySnapshotCacheService {
    GraphTopologySnapshotCacheService::new(
      Arc::new(GraphService::new(repository)),
      cache,
      clock,
      Duration::from_secs(30),
    )
    .unwrap()
  }

  fn result() -> GraphReadResult {
    let edge = relation().project_from(&key("root")).unwrap();
    GraphReadResult {
      content: content(),
      root: key("root"),
      nodes: vec![node("root"), node("target")],
      edges: vec![edge],
      truncated: false,
      next_cursor: None,
    }
  }

  #[tokio::test]
  async fn cache_miss_rebuilds_then_a_second_read_hits_and_expiry_rebuilds() {
    let clock = clock();
    let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> =
      Arc::new(InMemoryCache::new(clock.clone()));
    let service = service(repository(), cache, clock.clone());

    let first = service.read(request()).await.unwrap();
    let second = service.read(request()).await.unwrap();
    assert_eq!(
      clock.advance(Duration::from_secs(30)),
      Some(SystemTime::UNIX_EPOCH + Duration::from_secs(130))
    );
    let third = service.read(request()).await.unwrap();

    assert!(matches!(first, GraphTopologySnapshotCacheResult::Miss(_)));
    assert!(matches!(second, GraphTopologySnapshotCacheResult::Hit(_)));
    assert!(matches!(third, GraphTopologySnapshotCacheResult::Miss(_)));
  }

  #[tokio::test]
  async fn preloaded_compatible_snapshot_hits_without_rebuilding_graph_topology() {
    let request = request();
    let key = GraphTopologySnapshotKey::new(&request, content()).unwrap();
    let snapshot = GraphTopologySnapshot::new(key.clone(), result()).unwrap();
    let clock = clock();
    let cache = Arc::new(InMemoryCache::new(clock.clone()));
    cache
      .put(key, snapshot.clone(), clock.now() + Duration::from_secs(30))
      .await
      .unwrap();
    let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> = cache;
    let service = service(Arc::new(ActiveOnlyGraphRepository), cache, clock);

    let actual = service.read(request).await.unwrap();

    assert_eq!(
      actual,
      GraphTopologySnapshotCacheResult::Hit(snapshot.into_result())
    );
  }

  #[tokio::test]
  async fn unavailable_cache_returns_rebuilt_topology_uncached() {
    let service = service(repository(), Arc::new(UnavailableCache), clock());

    let actual = service.read(request()).await.unwrap();

    assert!(matches!(
      actual,
      GraphTopologySnapshotCacheResult::Unavailable(result) if result.edges.len() == 1
    ));
  }

  #[tokio::test]
  async fn expired_entry_from_a_non_expiring_adapter_is_never_served() {
    let request = request();
    let key = GraphTopologySnapshotKey::new(&request, content()).unwrap();
    let mut stale_result = result();
    stale_result.edges.clear();
    stale_result.nodes.truncate(1);
    let stale = GraphTopologySnapshot::new(key, stale_result).unwrap();
    let clock = clock();
    let cache = Arc::new(FixedEntryCache::new(
      stale,
      clock.now() - Duration::from_secs(1),
    ));
    let service = service(repository(), cache, clock);

    let actual = service.read(request).await.unwrap();

    assert!(matches!(
      actual,
      GraphTopologySnapshotCacheResult::Miss(result) if result.edges.len() == 1
    ));
  }

  #[tokio::test]
  async fn wrong_key_cache_value_is_discarded_before_rebuilding() {
    let request = request();
    let wrong_request =
      GraphReadRequest::new(key("root"), 0, 3, 2, GraphFilter::default()).unwrap();
    let wrong_key = GraphTopologySnapshotKey::new(&wrong_request, content()).unwrap();
    let wrong_snapshot = GraphTopologySnapshot::new(wrong_key, result()).unwrap();
    let clock = clock();
    let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> = Arc::new(
      FixedEntryCache::new(wrong_snapshot, clock.now() + Duration::from_secs(30)),
    );
    let service = service(repository(), cache, clock);

    let actual = service.read(request).await.unwrap();

    assert!(matches!(
      actual,
      GraphTopologySnapshotCacheResult::Miss(result) if result.edges.len() == 1
    ));
  }

  #[tokio::test]
  async fn terminal_neighbor_page_cannot_enter_the_public_snapshot_cache() {
    let repository: Arc<dyn GraphRepository> = Arc::new(
      InMemoryGraphRepository::new(content())
        .with_node(node("root"))
        .with_node(node("target"))
        .with_node(node("leaf"))
        .with_relation(relation())
        .with_relation(StoredGraphRelation {
          edge_id: GraphEdgeId::stored(id("edge-target-leaf")),
          source: key("target"),
          target: key("leaf"),
          ..relation()
        }),
    );
    let graph = Arc::new(GraphService::new(repository));
    let neighbor_page = graph
      .neighbors(
        GraphNeighborRequest::new(key("root"), 3, 2, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();
    assert!(neighbor_page.next_cursor.is_none());
    assert!(!neighbor_page.truncated);
    assert_eq!(neighbor_page.edges.len(), 1);

    let full_request = GraphReadRequest::new(key("root"), 2, 3, 2, GraphFilter::default()).unwrap();
    let full_key =
      GraphTopologySnapshotKey::new(&full_request, neighbor_page.content.clone()).unwrap();
    // Structural validation cannot distinguish this terminal page from a depth-two read. The
    // module-private constructor and compile-fail boundary above therefore supply the provenance.
    assert!(full_key.validate_result(&neighbor_page).is_ok());
    let clock = clock();
    let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> =
      Arc::new(InMemoryCache::new(clock.clone()));
    let service =
      GraphTopologySnapshotCacheService::new(graph, cache, clock, Duration::from_secs(30)).unwrap();

    let cached = service.read(full_request).await.unwrap();

    assert!(matches!(&cached, GraphTopologySnapshotCacheResult::Miss(_)));
    assert_eq!(cached.result().edges.len(), 2);
    assert_eq!(cached.result().nodes.len(), 3);
  }

  #[test]
  fn snapshot_rejects_an_edge_without_every_endpoint_node() {
    let key = GraphTopologySnapshotKey::new(&request(), content()).unwrap();
    let mut incomplete = result();
    incomplete.nodes.truncate(1);

    assert_eq!(
      GraphTopologySnapshot::new(key, incomplete),
      Err(GraphTopologySnapshotValidationError::MissingEdgeEndpoint)
    );
  }

  #[test]
  fn key_hashes_every_public_content_and_traversal_input() {
    let baseline = GraphTopologySnapshotKey::new(&request(), content()).unwrap();
    let changed_release = GraphTopologySnapshotKey::new(
      &request(),
      GraphContentVersion {
        release_id: id("release-2"),
        ..content()
      },
    )
    .unwrap();
    let changed_ranking = GraphTopologySnapshotKey::new(
      &request(),
      GraphContentVersion {
        ranking_version: "graph-rank-v2".to_string(),
        ..content()
      },
    )
    .unwrap();
    let changed_community = GraphTopologySnapshotKey::new(
      &request(),
      GraphContentVersion {
        community_aggregate_version: "community-v2".to_string(),
        ..content()
      },
    )
    .unwrap();
    let changed_root = GraphTopologySnapshotKey::new(
      &GraphReadRequest::new(key("other-root"), 1, 3, 2, GraphFilter::default()).unwrap(),
      content(),
    )
    .unwrap();
    let changed_filter = GraphTopologySnapshotKey::new(
      &GraphReadRequest::new(
        key("root"),
        1,
        3,
        2,
        GraphFilter {
          relation_types: [GraphRelationType::Hypernym].into_iter().collect(),
        },
      )
      .unwrap(),
      content(),
    )
    .unwrap();
    let changed_depth = GraphTopologySnapshotKey::new(
      &GraphReadRequest::new(key("root"), 0, 3, 2, GraphFilter::default()).unwrap(),
      content(),
    )
    .unwrap();
    let changed_nodes = GraphTopologySnapshotKey::new(
      &GraphReadRequest::new(key("root"), 1, 2, 2, GraphFilter::default()).unwrap(),
      content(),
    )
    .unwrap();
    let changed_edges = GraphTopologySnapshotKey::new(
      &GraphReadRequest::new(key("root"), 1, 3, 1, GraphFilter::default()).unwrap(),
      content(),
    )
    .unwrap();

    for changed in [
      changed_release,
      changed_ranking,
      changed_community,
      changed_root,
      changed_filter,
      changed_depth,
      changed_nodes,
      changed_edges,
    ] {
      assert_ne!(baseline, changed);
      assert_ne!(hash(&baseline), hash(&changed));
    }
  }

  #[test]
  fn zero_ttl_is_rejected() {
    let cache: Arc<dyn Cache<GraphTopologySnapshotKey, GraphTopologySnapshot>> =
      Arc::new(UnavailableCache);

    assert!(matches!(
      GraphTopologySnapshotCacheService::new(
        Arc::new(GraphService::new(repository())),
        cache,
        clock(),
        Duration::ZERO,
      ),
      Err(GraphTopologySnapshotCacheError::ZeroTtl)
    ));
  }

  fn hash(value: &GraphTopologySnapshotKey) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
  }

  struct UnavailableCache;

  #[async_trait]
  impl Cache<GraphTopologySnapshotKey, GraphTopologySnapshot> for UnavailableCache {
    async fn get(
      &self,
      _key: &GraphTopologySnapshotKey,
    ) -> Result<Option<CacheEntry<GraphTopologySnapshot>>, CacheError> {
      Err(CacheError::Unavailable)
    }

    async fn put(
      &self,
      _key: GraphTopologySnapshotKey,
      _value: GraphTopologySnapshot,
      _expires_at: UtcTimestamp,
    ) -> Result<(), CacheError> {
      Err(CacheError::Unavailable)
    }

    async fn remove(&self, _key: &GraphTopologySnapshotKey) -> Result<(), CacheError> {
      Err(CacheError::Unavailable)
    }
  }

  struct FixedEntryCache {
    entry: CacheEntry<GraphTopologySnapshot>,
  }

  impl FixedEntryCache {
    fn new(value: GraphTopologySnapshot, expires_at: UtcTimestamp) -> Self {
      Self {
        entry: CacheEntry::new(value, expires_at),
      }
    }
  }

  #[async_trait]
  impl Cache<GraphTopologySnapshotKey, GraphTopologySnapshot> for FixedEntryCache {
    async fn get(
      &self,
      _key: &GraphTopologySnapshotKey,
    ) -> Result<Option<CacheEntry<GraphTopologySnapshot>>, CacheError> {
      Ok(Some(self.entry.clone()))
    }

    async fn put(
      &self,
      _key: GraphTopologySnapshotKey,
      _value: GraphTopologySnapshot,
      _expires_at: UtcTimestamp,
    ) -> Result<(), CacheError> {
      Ok(())
    }

    async fn remove(&self, _key: &GraphTopologySnapshotKey) -> Result<(), CacheError> {
      Ok(())
    }
  }

  struct ActiveOnlyGraphRepository;

  #[async_trait]
  impl GraphRepository for ActiveOnlyGraphRepository {
    async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
      Ok(content())
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
}
