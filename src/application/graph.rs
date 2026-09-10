//! Bounded, deterministic typed graph reads and neighbor expansion.

use std::{
  collections::{BTreeMap, BTreeSet},
  sync::Arc,
};

use thiserror::Error;

use crate::{
  domain::graph::{
    compare_graph_edge_ordering_keys, compare_graph_edges, GraphContentVersion, GraphCursor,
    GraphEdge, GraphEdgeId, GraphEdgeOrderingKey, GraphEdgeOrigin, GraphFilter,
    GraphNeighborRequest, GraphNode, GraphNodeKey, GraphNodeKind, GraphReadRequest,
    GraphReadResult, GraphValidationError, MAX_GRAPH_EDGE_LIMIT,
  },
  ports::graph_repository::{
    GraphAdjacency, GraphAdjacencyRequest, GraphNeighborPage, GraphNeighborPageRequest,
    GraphRepository, GraphRepositoryError,
  },
};

/// Failure from the graph application service.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GraphReadError {
  /// The caller supplied a graph request outside the bounded contract.
  #[error(transparent)]
  Validation(#[from] GraphValidationError),
  /// The canonical graph store could not safely complete the read.
  #[error(transparent)]
  Repository(#[from] GraphRepositoryError),
  /// The requested typed root was absent from the pinned graph content version.
  #[error("graph root was not found")]
  RootNotFound,
}

/// Coordinates canonical graph reads without exposing storage topology or derived edge rules.
#[derive(Clone)]
pub struct GraphService {
  repository: Arc<dyn GraphRepository>,
}

impl GraphService {
  /// Creates a graph service from an explicit immutable-topology repository port.
  pub fn new(repository: Arc<dyn GraphRepository>) -> Self {
    Self { repository }
  }

  /// Reads a bounded breadth-first graph rooted at a typed canonical node.
  ///
  /// The service validates canonical records again after loading, derives inverse and adjacent-scale
  /// projections in memory, ranks every layer deterministically, and never returns an edge whose
  /// endpoints are missing from `nodes`.
  ///
  /// # Errors
  ///
  /// Returns an error when the root is absent or the graph repository cannot provide consistent
  /// immutable content.
  pub async fn read(&self, request: GraphReadRequest) -> Result<GraphReadResult, GraphReadError> {
    let content = self.repository.active_graph_content().await?;
    let root = self.load_root(&content, &request.root).await?;
    let mut nodes = BTreeMap::from([(root.key.clone(), root)]);
    let mut frontier = BTreeSet::from([request.root.clone()]);
    let mut visited = frontier.clone();
    let mut edges = BTreeMap::<GraphEdgeId, GraphEdge>::new();
    let mut truncated = false;

    for _ in 0..request.depth {
      if frontier.is_empty()
        || edges.len() >= request.edge_limit
        || nodes.len() >= request.node_limit
      {
        if !frontier.is_empty() {
          truncated = true;
        }
        break;
      }

      let mut layer_edges = Vec::new();
      let mut source_was_bounded = false;
      for node in &frontier {
        let (mut projected, bounded) = self
          .projected_adjacency(&content, node, &request.filter, MAX_GRAPH_EDGE_LIMIT)
          .await?;
        layer_edges.append(&mut projected);
        source_was_bounded |= bounded;
      }
      layer_edges.sort_by(compare_graph_edges);

      let remaining_edges = request.edge_limit.saturating_sub(edges.len());
      let mut selected = Vec::new();
      let mut selected_nodes = nodes.keys().cloned().collect::<BTreeSet<_>>();
      let mut saw_omitted_edge = false;
      for edge in layer_edges {
        if edges.contains_key(&edge.id)
          || selected
            .iter()
            .any(|selected: &GraphEdge| selected.id == edge.id)
        {
          continue;
        }
        if selected.len() >= remaining_edges {
          saw_omitted_edge = true;
          break;
        }

        let required_nodes = [&edge.source, &edge.target]
          .into_iter()
          .filter(|key| !selected_nodes.contains(*key))
          .count();
        if selected_nodes.len() + required_nodes > request.node_limit {
          saw_omitted_edge = true;
          continue;
        }
        selected_nodes.insert(edge.source.clone());
        selected_nodes.insert(edge.target.clone());
        selected.push(edge);
      }

      let selected_keys = selected
        .iter()
        .flat_map(|edge| [edge.source.clone(), edge.target.clone()])
        .filter(|key| !nodes.contains_key(key))
        .collect::<BTreeSet<_>>();
      let loaded = self.load_nodes(&content, selected_keys).await?;
      let mut next_frontier = BTreeSet::new();
      for edge in selected {
        let source_loaded = nodes.contains_key(&edge.source) || loaded.contains_key(&edge.source);
        let target_loaded = nodes.contains_key(&edge.target) || loaded.contains_key(&edge.target);
        if !source_loaded || !target_loaded {
          truncated = true;
          continue;
        }
        for key in [&edge.source, &edge.target] {
          if visited.insert(key.clone()) {
            next_frontier.insert(key.clone());
          }
        }
        edges.insert(edge.id.clone(), edge);
      }
      nodes.extend(loaded);
      frontier = next_frontier;
      truncated |= source_was_bounded || saw_omitted_edge;
    }

    let mut edges = edges.into_values().collect::<Vec<_>>();
    edges.sort_by(compare_graph_edges);
    Ok(GraphReadResult {
      content,
      root: request.root,
      nodes: nodes.into_values().collect(),
      edges,
      truncated,
      next_cursor: None,
    })
  }

  /// Returns one deterministic, internally complete page of direct neighbor projections.
  ///
  /// The cursor is tied to the typed root, pinned content version, and normalized relation filter.
  /// It resumes after the last safely processed ordering key, so storage order can never affect
  /// page boundaries or make an unavailable endpoint block a later complete edge.
  ///
  /// # Errors
  ///
  /// Returns an error for a foreign cursor, absent root, or inconsistent repository result.
  pub async fn neighbors(
    &self,
    request: GraphNeighborRequest,
  ) -> Result<GraphReadResult, GraphReadError> {
    let content = self.repository.active_graph_content().await?;
    if let Some(cursor) = &request.cursor {
      if cursor.root != request.root {
        return Err(GraphValidationError::CursorRootMismatch.into());
      }
      if cursor.content != content {
        return Err(GraphValidationError::CursorContentMismatch.into());
      }
      if cursor.filter != request.filter {
        return Err(GraphValidationError::CursorFilterMismatch.into());
      }
    }

    let root = self.load_root(&content, &request.root).await?;
    let page = self
      .repository
      .neighbor_page(&GraphNeighborPageRequest {
        content: content.clone(),
        node: request.root.clone(),
        filter: request.filter.clone(),
        after: request.cursor.as_ref().map(|cursor| cursor.after.clone()),
        edge_limit: MAX_GRAPH_EDGE_LIMIT,
      })
      .await?;
    let GraphNeighborPage { edges, has_more } = validate_neighbor_page(
      page,
      &content,
      &request.root,
      &request.filter,
      request.cursor.as_ref().map(|cursor| &cursor.after),
    )?;
    let candidate_keys = edges
      .iter()
      .flat_map(|edge| [edge.source.clone(), edge.target.clone()])
      .filter(|key| *key != root.key)
      .collect::<BTreeSet<_>>();
    let loaded = self.load_nodes(&content, candidate_keys).await?;

    let mut nodes = BTreeMap::from([(root.key.clone(), root)]);
    let mut selected = Vec::new();
    let mut selected_nodes = nodes.keys().cloned().collect::<BTreeSet<_>>();
    let mut omitted_endpoint = false;
    let mut stopped_before_candidate = false;
    let mut last_processed = None;
    for edge in edges {
      let source_loaded = nodes.contains_key(&edge.source) || loaded.contains_key(&edge.source);
      let target_loaded = nodes.contains_key(&edge.target) || loaded.contains_key(&edge.target);
      if !source_loaded || !target_loaded {
        omitted_endpoint = true;
        last_processed = Some(edge.ordering_key());
        continue;
      }
      if selected.len() >= request.edge_limit {
        stopped_before_candidate = true;
        break;
      }
      let required_nodes = [&edge.source, &edge.target]
        .into_iter()
        .filter(|key| !selected_nodes.contains(*key))
        .count();
      if selected_nodes.len() + required_nodes > request.node_limit {
        stopped_before_candidate = true;
        break;
      }
      selected_nodes.insert(edge.source.clone());
      selected_nodes.insert(edge.target.clone());
      last_processed = Some(edge.ordering_key());
      selected.push(edge);
    }

    for edge in &selected {
      for key in [&edge.source, &edge.target] {
        if let Some(node) = loaded.get(key) {
          nodes.insert(key.clone(), node.clone());
        }
      }
    }
    let needs_continuation = has_more || stopped_before_candidate || omitted_endpoint;
    let next_cursor = if needs_continuation {
      let after = last_processed.ok_or(GraphRepositoryError::InconsistentData)?;
      Some(GraphCursor {
        root: request.root.clone(),
        content: content.clone(),
        filter: request.filter.clone(),
        after,
      })
    } else {
      None
    };

    Ok(GraphReadResult {
      content,
      root: request.root,
      nodes: nodes.into_values().collect(),
      edges: selected,
      truncated: needs_continuation,
      next_cursor,
    })
  }

  async fn load_root(
    &self,
    content: &GraphContentVersion,
    root: &GraphNodeKey,
  ) -> Result<GraphNode, GraphReadError> {
    self
      .repository
      .load_nodes(content, std::slice::from_ref(root))
      .await?
      .into_iter()
      .find(|node| node.key == *root)
      .ok_or(GraphReadError::RootNotFound)
  }

  async fn load_nodes(
    &self,
    content: &GraphContentVersion,
    keys: BTreeSet<GraphNodeKey>,
  ) -> Result<BTreeMap<GraphNodeKey, GraphNode>, GraphReadError> {
    if keys.is_empty() {
      return Ok(BTreeMap::new());
    }
    Ok(
      self
        .repository
        .load_nodes(content, &keys.into_iter().collect::<Vec<_>>())
        .await?
        .into_iter()
        .map(|node| (node.key.clone(), node))
        .collect(),
    )
  }

  async fn projected_adjacency(
    &self,
    content: &GraphContentVersion,
    node: &GraphNodeKey,
    filter: &GraphFilter,
    record_limit: usize,
  ) -> Result<(Vec<GraphEdge>, bool), GraphReadError> {
    let adjacency = self
      .repository
      .adjacency(&GraphAdjacencyRequest {
        content: content.clone(),
        node: node.clone(),
        filter: filter.clone(),
        record_limit,
      })
      .await?;
    let raw_records = adjacency.relations.len() + adjacency.scales.len();
    let edges = project_adjacency(adjacency, content, node, filter)?;
    Ok((edges, raw_records >= record_limit))
  }
}

fn validate_neighbor_page(
  page: GraphNeighborPage,
  content: &GraphContentVersion,
  root: &GraphNodeKey,
  filter: &GraphFilter,
  after: Option<&GraphEdgeOrderingKey>,
) -> Result<GraphNeighborPage, GraphReadError> {
  if page.edges.len() > MAX_GRAPH_EDGE_LIMIT || (page.has_more && page.edges.is_empty()) {
    return Err(GraphRepositoryError::InconsistentData.into());
  }

  let mut previous = after.cloned();
  let mut edge_ids = BTreeSet::new();
  for edge in &page.edges {
    let ordering_key = edge.ordering_key();
    if edge.source != *root
      || edge.target == *root
      || !filter.allows(edge.relation_type)
      || edge.ranking.ranking_version != content.ranking_version
      || edge.evidence.evidence_ids.is_empty()
      || !edge_ids.insert(edge.id.clone())
      || !edge_origin_is_consistent(edge, content)
      || previous
        .as_ref()
        .is_some_and(|previous| !compare_graph_edge_ordering_keys(&ordering_key, previous).is_gt())
    {
      return Err(GraphRepositoryError::InconsistentData.into());
    }
    previous = Some(ordering_key);
  }
  Ok(page)
}

fn edge_origin_is_consistent(edge: &GraphEdge, content: &GraphContentVersion) -> bool {
  match &edge.origin {
    GraphEdgeOrigin::Canonical => {
      edge.relation_version.is_some()
        && !edge.relation_type.is_scale_projection()
        && edge.directed == !edge.relation_type.is_symmetric()
        && (!edge.relation_type.is_symmetric() || edge.source < edge.target)
    }
    GraphEdgeOrigin::InverseProjection => {
      edge.relation_version.is_some()
        && !edge.relation_type.is_scale_projection()
        && edge.directed == !edge.relation_type.is_symmetric()
        && (!edge.relation_type.is_symmetric() || edge.source > edge.target)
    }
    GraphEdgeOrigin::ScaleAdjacency { scale_id } => {
      if edge.relation_version.is_some() || !edge.feedback_capabilities.is_empty() || !edge.directed
      {
        return false;
      }
      let expected_id = match edge.relation_type {
        crate::domain::graph::GraphRelationType::LowerDegree => {
          GraphEdgeId::derived_scale(&content.release_id, scale_id, &edge.target, &edge.source)
        }
        crate::domain::graph::GraphRelationType::HigherDegree => {
          GraphEdgeId::derived_scale(&content.release_id, scale_id, &edge.source, &edge.target)
        }
        _ => return false,
      };
      edge.id == expected_id
    }
    GraphEdgeOrigin::ScaleMembership { scale_id, ordinal } => {
      if edge.relation_version.is_some() || !edge.feedback_capabilities.is_empty() || !edge.directed
      {
        return false;
      }
      let scale = GraphNodeKey::new(GraphNodeKind::Scale, scale_id.clone());
      let member = match edge.relation_type {
        crate::domain::graph::GraphRelationType::ScaleContains if edge.source == scale => {
          &edge.target
        }
        crate::domain::graph::GraphRelationType::MemberOfScale if edge.target == scale => {
          &edge.source
        }
        _ => return false,
      };
      edge.id
        == GraphEdgeId::derived_scale_membership(&content.release_id, scale_id, *ordinal, member)
    }
  }
}

fn project_adjacency(
  adjacency: GraphAdjacency,
  content: &GraphContentVersion,
  node: &GraphNodeKey,
  filter: &GraphFilter,
) -> Result<Vec<GraphEdge>, GraphReadError> {
  let mut edges = Vec::new();
  for relation in adjacency.relations {
    relation
      .validate()
      .map_err(|_| GraphRepositoryError::InconsistentData)?;
    if relation.ranking.ranking_version != content.ranking_version {
      return Err(GraphRepositoryError::InconsistentData.into());
    }
    let Some(edge) = relation.project_from(node) else {
      return Err(GraphRepositoryError::InconsistentData.into());
    };
    if filter.allows(edge.relation_type) {
      edges.push(edge);
    }
  }
  for scale in adjacency.scales {
    scale
      .validate()
      .map_err(|_| GraphRepositoryError::InconsistentData)?;
    if scale.release_id != content.release_id
      || scale.ranking.ranking_version != content.ranking_version
    {
      return Err(GraphRepositoryError::InconsistentData.into());
    }
    for edge in scale.project_from(node) {
      if filter.allows(edge.relation_type) {
        edges.push(edge);
      }
    }
  }
  edges.sort_by(compare_graph_edges);
  let mut seen = BTreeSet::new();
  edges.retain(|edge| seen.insert(edge.id.clone()));
  Ok(edges)
}

#[cfg(test)]
mod tests {
  use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
  };

  use async_trait::async_trait;

  use super::*;
  use crate::{
    adapters::in_memory::InMemoryGraphRepository,
    domain::{
      canonical::{CanonicalId, EvidenceConfidence, LanguageTag},
      graph::{
        GraphEdgeId, GraphEvidence, GraphFeedbackCapability, GraphNodeKind, GraphRanking,
        GraphRelationType, GraphScope, GraphScore, GraphScoreComponents, RelationVersion,
        SemanticScale, SemanticScaleMember, StoredGraphRelation,
      },
    },
    ports::graph_repository::{GraphNeighborPage, GraphNeighborPageRequest},
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn key(value: &str) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Sense, id(value))
  }

  fn scale_key(value: &str) -> GraphNodeKey {
    GraphNodeKey::new(GraphNodeKind::Scale, id(value))
  }

  fn content() -> GraphContentVersion {
    GraphContentVersion {
      release_id: id("release-1"),
      ranking_version: "graph-rank-v1".to_string(),
      community_aggregate_version: "community-v1".to_string(),
    }
  }

  fn evidence() -> GraphEvidence {
    GraphEvidence::new(vec![id("evidence-1")], EvidenceConfidence::High).unwrap()
  }

  fn ranking(score: u16) -> GraphRanking {
    GraphRanking {
      display_rank: GraphScore::new(score).unwrap(),
      components: GraphScoreComponents {
        evidence: GraphScore::new(score).unwrap(),
        community: None,
        pedagogical: None,
      },
      ranking_version: "graph-rank-v1".to_string(),
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

  fn page_nodes(values: &[&str]) -> BTreeMap<GraphNodeKey, GraphNode> {
    values
      .iter()
      .map(|value| (key(value), node(value)))
      .collect()
  }

  fn scale_node(value: &str) -> GraphNode {
    GraphNode::new(scale_key(value), value, None, None, None, true).unwrap()
  }

  fn relation(source: &str, target: &str, edge: &str, score: u16) -> StoredGraphRelation {
    StoredGraphRelation {
      edge_id: GraphEdgeId::stored(id(edge)),
      relation_version: RelationVersion::new(1).unwrap(),
      source: key(source),
      target: key(target),
      relation_type: GraphRelationType::Hypernym,
      evidence: evidence(),
      scope: GraphScope::default(),
      feedback_capabilities: BTreeSet::from([GraphFeedbackCapability::Accuracy]),
      ranking: ranking(score),
    }
  }

  fn service(repository: InMemoryGraphRepository) -> GraphService {
    GraphService::new(Arc::new(repository))
  }

  struct SourceBoundedNeighborRepository {
    content: GraphContentVersion,
    nodes: BTreeMap<GraphNodeKey, GraphNode>,
    first: GraphEdge,
    second: GraphEdge,
  }

  struct FixedNeighborPageRepository {
    content: GraphContentVersion,
    root: GraphNodeKey,
    nodes: BTreeMap<GraphNodeKey, GraphNode>,
    edges: Vec<GraphEdge>,
  }

  #[async_trait]
  impl GraphRepository for FixedNeighborPageRepository {
    async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
      Ok(self.content.clone())
    }

    async fn load_nodes(
      &self,
      _content: &GraphContentVersion,
      keys: &[GraphNodeKey],
    ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
      Ok(
        keys
          .iter()
          .filter_map(|key| self.nodes.get(key).cloned())
          .collect(),
      )
    }

    async fn adjacency(
      &self,
      _request: &GraphAdjacencyRequest,
    ) -> Result<GraphAdjacency, GraphRepositoryError> {
      Ok(GraphAdjacency::default())
    }

    async fn neighbor_page(
      &self,
      request: &GraphNeighborPageRequest,
    ) -> Result<GraphNeighborPage, GraphRepositoryError> {
      if request.content != self.content || request.node != self.root {
        return Err(GraphRepositoryError::InconsistentData);
      }
      if request.after.is_some() {
        return Ok(GraphNeighborPage::default());
      }
      Ok(GraphNeighborPage {
        edges: self.edges.clone(),
        has_more: false,
      })
    }
  }

  async fn fixed_neighbor_page_error(
    root: GraphNodeKey,
    nodes: BTreeMap<GraphNodeKey, GraphNode>,
    edges: Vec<GraphEdge>,
  ) -> GraphReadError {
    let service = GraphService::new(Arc::new(FixedNeighborPageRepository {
      content: content(),
      root: root.clone(),
      nodes,
      edges,
    }));
    service
      .neighbors(GraphNeighborRequest::new(root, 75, 200, GraphFilter::default(), None).unwrap())
      .await
      .unwrap_err()
  }

  #[async_trait]
  impl GraphRepository for SourceBoundedNeighborRepository {
    async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
      Ok(self.content.clone())
    }

    async fn load_nodes(
      &self,
      _content: &GraphContentVersion,
      keys: &[GraphNodeKey],
    ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
      Ok(
        keys
          .iter()
          .filter_map(|key| self.nodes.get(key).cloned())
          .collect(),
      )
    }

    async fn adjacency(
      &self,
      _request: &GraphAdjacencyRequest,
    ) -> Result<GraphAdjacency, GraphRepositoryError> {
      Ok(GraphAdjacency::default())
    }

    async fn neighbor_page(
      &self,
      request: &GraphNeighborPageRequest,
    ) -> Result<GraphNeighborPage, GraphRepositoryError> {
      if request.content != self.content || request.node != self.first.source {
        return Err(GraphRepositoryError::InconsistentData);
      }
      match &request.after {
        None => Ok(GraphNeighborPage {
          edges: vec![self.first.clone()],
          has_more: true,
        }),
        Some(after) if *after == self.first.ordering_key() => Ok(GraphNeighborPage {
          edges: vec![self.second.clone()],
          has_more: false,
        }),
        _ => Ok(GraphNeighborPage::default()),
      }
    }
  }

  #[tokio::test]
  async fn graph_read_respects_depth_and_keeps_every_edge_endpoint() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("middle"))
      .with_node(node("leaf"))
      .with_relation(relation("root", "middle", "edge-1", 9_000))
      .with_relation(relation("middle", "leaf", "edge-2", 8_000));
    let service = service(repository);
    let depth_one = service
      .read(GraphReadRequest::new(key("root"), 1, 75, 200, GraphFilter::default()).unwrap())
      .await
      .unwrap();
    let depth_two = service
      .read(GraphReadRequest::new(key("root"), 2, 75, 200, GraphFilter::default()).unwrap())
      .await
      .unwrap();

    assert_eq!(depth_one.edges.len(), 1);
    assert_eq!(depth_two.edges.len(), 2);
    for edge in &depth_two.edges {
      assert!(depth_two.nodes.iter().any(|node| node.key == edge.source));
      assert!(depth_two.nodes.iter().any(|node| node.key == edge.target));
    }
  }

  #[tokio::test]
  async fn graph_read_enforces_the_node_cap_by_rank() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("first"))
      .with_node(node("second"))
      .with_node(node("third"))
      .with_relation(relation("root", "first", "edge-1", 9_000))
      .with_relation(relation("root", "second", "edge-2", 8_000))
      .with_relation(relation("root", "third", "edge-3", 7_000));
    let result = service(repository)
      .read(GraphReadRequest::new(key("root"), 1, 2, 200, GraphFilter::default()).unwrap())
      .await
      .unwrap();

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.edges.len(), 1);
    assert_eq!(result.edges[0].id.as_str(), "edge-1");
    assert!(result.truncated);
  }

  #[tokio::test]
  async fn graph_read_enforces_the_edge_cap_by_rank() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("first"))
      .with_node(node("second"))
      .with_node(node("third"))
      .with_relation(relation("root", "first", "edge-1", 9_000))
      .with_relation(relation("root", "second", "edge-2", 8_000))
      .with_relation(relation("root", "third", "edge-3", 7_000));
    let result = service(repository)
      .read(GraphReadRequest::new(key("root"), 1, 75, 1, GraphFilter::default()).unwrap())
      .await
      .unwrap();

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.edges.len(), 1);
    assert_eq!(result.edges[0].id.as_str(), "edge-1");
    assert!(result.truncated);
  }

  #[tokio::test]
  async fn direct_neighbor_pages_are_ranked_stably_and_resumable() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("first"))
      .with_node(node("second"))
      .with_node(node("third"))
      .with_relation(relation("root", "first", "edge-z", 9_000))
      .with_relation(relation("root", "second", "edge-b", 8_000))
      .with_relation(relation("root", "third", "edge-a", 8_000));
    let service = service(repository);
    let first = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 4, 1, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();
    let second = service
      .neighbors(
        GraphNeighborRequest::new(
          key("root"),
          4,
          1,
          GraphFilter::default(),
          first.next_cursor.clone(),
        )
        .unwrap(),
      )
      .await
      .unwrap();
    let third = service
      .neighbors(
        GraphNeighborRequest::new(
          key("root"),
          4,
          1,
          GraphFilter::default(),
          second.next_cursor.clone(),
        )
        .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(first.edges[0].id.as_str(), "edge-z");
    assert_eq!(second.edges[0].id.as_str(), "edge-a");
    assert_eq!(third.edges[0].id.as_str(), "edge-b");
    assert!(third.next_cursor.is_none());
  }

  #[tokio::test]
  async fn direct_neighbor_page_rejects_stored_scale_projection_types() {
    let mut canonical = relation("root", "target", "edge-canonical", 9_000)
      .project_from(&key("root"))
      .unwrap();
    canonical.relation_type = GraphRelationType::LowerDegree;
    assert_eq!(
      fixed_neighbor_page_error(
        key("root"),
        page_nodes(&["root", "target"]),
        vec![canonical],
      )
      .await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );

    let mut inverse = relation("source", "root", "edge-inverse", 9_000)
      .project_from(&key("root"))
      .unwrap();
    inverse.relation_type = GraphRelationType::HigherDegree;
    assert_eq!(
      fixed_neighbor_page_error(key("root"), page_nodes(&["root", "source"]), vec![inverse],).await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn direct_neighbor_page_rejects_an_invalid_stored_direction() {
    let mut edge = relation("root", "target", "edge-direction", 9_000)
      .project_from(&key("root"))
      .unwrap();
    edge.directed = false;

    assert_eq!(
      fixed_neighbor_page_error(key("root"), page_nodes(&["root", "target"]), vec![edge],).await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn direct_neighbor_page_rejects_unordered_symmetric_projections() {
    let mut canonical = relation("z-root", "a-target", "edge-canonical", 9_000)
      .project_from(&key("z-root"))
      .unwrap();
    canonical.relation_type = GraphRelationType::Synonym;
    canonical.directed = false;
    assert_eq!(
      fixed_neighbor_page_error(
        key("z-root"),
        page_nodes(&["z-root", "a-target"]),
        vec![canonical],
      )
      .await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );

    let mut inverse = relation("z-source", "a-root", "edge-inverse", 9_000)
      .project_from(&key("a-root"))
      .unwrap();
    inverse.relation_type = GraphRelationType::Synonym;
    inverse.directed = false;
    assert_eq!(
      fixed_neighbor_page_error(
        key("a-root"),
        page_nodes(&["a-root", "z-source"]),
        vec![inverse],
      )
      .await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn direct_neighbor_page_rejects_duplicate_edge_ids() {
    let first = relation("root", "first", "edge-duplicate", 9_000)
      .project_from(&key("root"))
      .unwrap();
    let mut second = relation("root", "second", "edge-distinct", 8_000)
      .project_from(&key("root"))
      .unwrap();
    second.id = first.id.clone();

    assert_eq!(
      fixed_neighbor_page_error(
        key("root"),
        page_nodes(&["root", "first", "second"]),
        vec![first, second],
      )
      .await,
      GraphReadError::Repository(GraphRepositoryError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn direct_neighbor_node_cap_pages_without_skipping_ranked_edges() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("first"))
      .with_node(node("second"))
      .with_node(node("third"))
      .with_relation(relation("root", "first", "edge-1", 9_000))
      .with_relation(relation("root", "second", "edge-2", 8_000))
      .with_relation(relation("root", "third", "edge-3", 7_000));
    let service = service(repository);
    let first = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 200, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();
    let second = service
      .neighbors(
        GraphNeighborRequest::new(
          key("root"),
          2,
          200,
          GraphFilter::default(),
          first.next_cursor.clone(),
        )
        .unwrap(),
      )
      .await
      .unwrap();
    let third = service
      .neighbors(
        GraphNeighborRequest::new(
          key("root"),
          2,
          200,
          GraphFilter::default(),
          second.next_cursor.clone(),
        )
        .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(first.edges[0].id.as_str(), "edge-1");
    assert_eq!(second.edges[0].id.as_str(), "edge-2");
    assert_eq!(third.edges[0].id.as_str(), "edge-3");
    assert!(first.next_cursor.is_some());
    assert!(second.next_cursor.is_some());
    assert!(third.next_cursor.is_none());
  }

  #[tokio::test]
  async fn direct_neighbor_cursor_advances_past_an_unavailable_endpoint_to_a_later_complete_edge() {
    let missing = relation("root", "missing", "edge-missing", 9_000)
      .project_from(&key("root"))
      .unwrap();
    let complete = relation("root", "available", "edge-available", 8_000)
      .project_from(&key("root"))
      .unwrap();
    let repository = SourceBoundedNeighborRepository {
      content: content(),
      nodes: BTreeMap::from([
        (key("root"), node("root")),
        (key("available"), node("available")),
      ]),
      first: missing,
      second: complete,
    };
    let service = GraphService::new(Arc::new(repository));
    let first = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 1, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();

    assert!(first.edges.is_empty());
    assert_eq!(first.nodes.len(), 1);
    assert!(first.truncated);
    let cursor = first
      .next_cursor
      .clone()
      .expect("cursor after missing edge");
    assert_eq!(cursor.after.edge_id.as_str(), "edge-missing");

    let second = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 1, GraphFilter::default(), Some(cursor)).unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(second.edges.len(), 1);
    assert_eq!(second.edges[0].id.as_str(), "edge-available");
    assert!(second.nodes.iter().any(|node| node.key == key("available")));
    assert!(second.next_cursor.is_none());
  }

  #[tokio::test]
  async fn direct_neighbor_incomplete_final_edge_uses_a_terminal_cursor() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_relation(relation("root", "missing", "edge-missing", 9_000));
    let service = service(repository);
    let first = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 1, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();

    assert!(first.edges.is_empty());
    assert!(first.truncated);
    let cursor = first
      .next_cursor
      .expect("cursor after incomplete final edge");

    let terminal = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 1, GraphFilter::default(), Some(cursor)).unwrap(),
      )
      .await
      .unwrap();

    assert!(terminal.edges.is_empty());
    assert!(!terminal.truncated);
    assert!(terminal.next_cursor.is_none());
  }

  #[tokio::test]
  async fn direct_neighbor_node_cap_keeps_repeated_endpoint_paths_before_continuing() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("shared"))
      .with_node(node("later"))
      .with_relation(relation("root", "shared", "edge-shared-first", 9_000))
      .with_relation(relation("root", "shared", "edge-shared-second", 8_000))
      .with_relation(relation("root", "later", "edge-later", 7_000));
    let service = service(repository);
    let first = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 200, GraphFilter::default(), None).unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(
      first
        .edges
        .iter()
        .map(|edge| edge.id.as_str())
        .collect::<Vec<_>>(),
      vec!["edge-shared-first", "edge-shared-second"]
    );
    assert_eq!(first.nodes.len(), 2);
    assert!(first.truncated);
    let cursor = first
      .next_cursor
      .clone()
      .expect("cursor after repeated endpoint paths");
    assert_eq!(cursor.after.edge_id.as_str(), "edge-shared-second");

    let second = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 200, GraphFilter::default(), Some(cursor))
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(second.edges.len(), 1);
    assert_eq!(second.edges[0].id.as_str(), "edge-later");
    assert!(second.next_cursor.is_none());
  }

  #[tokio::test]
  async fn direct_neighbor_cursor_rejects_a_changed_relation_filter() {
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("root"))
      .with_node(node("first"))
      .with_node(node("second"))
      .with_relation(relation("root", "first", "edge-1", 9_000))
      .with_relation(relation("root", "second", "edge-2", 8_000));
    let service = service(repository);
    let filter = GraphFilter {
      relation_types: [GraphRelationType::Hypernym].into_iter().collect(),
    };
    let first = service
      .neighbors(GraphNeighborRequest::new(key("root"), 2, 1, filter.clone(), None).unwrap())
      .await
      .unwrap();

    let result = service
      .neighbors(
        GraphNeighborRequest::new(key("root"), 2, 1, GraphFilter::default(), first.next_cursor)
          .unwrap(),
      )
      .await;

    assert_eq!(
      result,
      Err(GraphReadError::Validation(
        GraphValidationError::CursorFilterMismatch
      ))
    );
  }

  #[tokio::test]
  async fn derived_scale_edges_cannot_accept_feedback() {
    let scale = SemanticScale {
      id: id("temperature"),
      release_id: id("release-1"),
      label: "temperature".to_string(),
      scope: GraphScope::default(),
      evidence: evidence(),
      members: vec![
        SemanticScaleMember {
          sense: key("warm"),
          ordinal: 1,
          evidence: evidence(),
        },
        SemanticScaleMember {
          sense: key("hot"),
          ordinal: 2,
          evidence: evidence(),
        },
      ],
      ranking: ranking(7_000),
    };
    let repository = InMemoryGraphRepository::new(content())
      .with_node(node("warm"))
      .with_node(node("hot"))
      .with_scale(scale);
    let result = service(repository)
      .neighbors(GraphNeighborRequest::defaults(key("warm")))
      .await
      .unwrap();

    assert_eq!(result.edges.len(), 1);
    assert!(result.edges[0].relation_version.is_none());
    assert!(!result.edges[0].accepts_feedback());
  }

  #[tokio::test]
  async fn scale_root_expands_to_deterministic_non_feedback_memberships() {
    let scale = SemanticScale {
      id: id("temperature"),
      release_id: id("release-1"),
      label: "temperature".to_string(),
      scope: GraphScope::default(),
      evidence: evidence(),
      members: vec![
        SemanticScaleMember {
          sense: key("hot"),
          ordinal: 2,
          evidence: evidence(),
        },
        SemanticScaleMember {
          sense: key("warm"),
          ordinal: 1,
          evidence: evidence(),
        },
      ],
      ranking: ranking(7_000),
    };
    let root = scale_key("temperature");
    let repository = InMemoryGraphRepository::new(content())
      .with_node(scale_node("temperature"))
      .with_node(node("warm"))
      .with_node(node("hot"))
      .with_scale(scale);
    let result = service(repository)
      .neighbors(GraphNeighborRequest::defaults(root.clone()))
      .await
      .unwrap();

    assert_eq!(result.edges.len(), 2);
    assert_eq!(
      result.edges[0].id.as_str(),
      "derived:scale:release-1:temperature:member:0000000001:warm"
    );
    assert_eq!(
      result.edges[1].id.as_str(),
      "derived:scale:release-1:temperature:member:0000000002:hot"
    );
    assert!(result.edges.iter().all(|edge| {
      edge.source == root
        && edge.relation_type == GraphRelationType::ScaleContains
        && edge.relation_version.is_none()
        && !edge.accepts_feedback()
    }));
    for edge in &result.edges {
      assert!(result.nodes.iter().any(|node| node.key == edge.source));
      assert!(result.nodes.iter().any(|node| node.key == edge.target));
    }
  }
}
