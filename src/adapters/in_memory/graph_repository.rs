//! Deterministic in-memory canonical graph repository for tests and local development.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;

use crate::{
  domain::graph::{
    compare_graph_edge_ordering_keys, compare_graph_edges, GraphContentVersion, GraphFilter,
    GraphNode, GraphNodeKey, GraphNodeKind, SemanticScale, StoredGraphRelation,
  },
  ports::graph_repository::{
    GraphAdjacency, GraphAdjacencyRequest, GraphNeighborPage, GraphNeighborPageRequest,
    GraphRepository, GraphRepositoryError,
  },
};

/// Shareable immutable in-memory graph data pinned to one graph content version.
///
/// Builder methods deliberately retain every configured record and sort reads by stable IDs, which
/// makes the adapter suitable for deterministic application tests. It does not emulate database
/// transactions, authorization, or public/private topology cache layers.
#[derive(Debug, Clone)]
pub struct InMemoryGraphRepository {
  content: GraphContentVersion,
  nodes: BTreeMap<GraphNodeKey, GraphNode>,
  relations: Vec<StoredGraphRelation>,
  scales: Vec<SemanticScale>,
}

impl InMemoryGraphRepository {
  /// Creates an empty immutable graph repository for `content`.
  pub fn new(content: GraphContentVersion) -> Self {
    Self {
      content,
      nodes: BTreeMap::new(),
      relations: Vec::new(),
      scales: Vec::new(),
    }
  }

  /// Adds or replaces one typed graph node.
  pub fn with_node(mut self, node: GraphNode) -> Self {
    self.nodes.insert(node.key.clone(), node);
    self
  }

  /// Adds one canonical relation available to its two endpoint reads.
  pub fn with_relation(mut self, relation: StoredGraphRelation) -> Self {
    self.relations.push(relation);
    self
  }

  /// Adds one semantic scale available to every member sense and its scale root.
  pub fn with_scale(mut self, scale: SemanticScale) -> Self {
    self.scales.push(scale);
    self
  }
}

#[async_trait]
impl GraphRepository for InMemoryGraphRepository {
  async fn active_graph_content(&self) -> Result<GraphContentVersion, GraphRepositoryError> {
    Ok(self.content.clone())
  }

  async fn load_nodes(
    &self,
    _content: &GraphContentVersion,
    keys: &[GraphNodeKey],
  ) -> Result<Vec<GraphNode>, GraphRepositoryError> {
    let mut nodes = keys
      .iter()
      .filter_map(|key| self.nodes.get(key).cloned())
      .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.key.cmp(&right.key));
    nodes.dedup_by(|left, right| left.key == right.key);
    Ok(nodes)
  }

  async fn adjacency(
    &self,
    request: &GraphAdjacencyRequest,
  ) -> Result<GraphAdjacency, GraphRepositoryError> {
    if request.content != self.content {
      return Err(GraphRepositoryError::InconsistentData);
    }
    let mut relations = self
      .relations
      .iter()
      .filter(|relation| relation.source == request.node || relation.target == request.node)
      .filter(|relation| relation_filter_allows(&request.filter, relation, &request.node))
      .cloned()
      .collect::<Vec<_>>();
    relations.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));

    let remaining = request.record_limit.saturating_sub(relations.len());
    relations.truncate(request.record_limit);
    let mut scales = if remaining == 0 {
      Vec::new()
    } else {
      self
        .scales
        .iter()
        .filter(|scale| {
          (request.node.kind == GraphNodeKind::Scale && scale.id == request.node.id)
            || scale
              .members
              .iter()
              .any(|member| member.sense == request.node)
        })
        .filter(|scale| scale_filter_allows(&request.filter, scale, &request.node))
        .cloned()
        .collect::<Vec<_>>()
    };
    scales.sort_by(|left, right| left.id.cmp(&right.id));
    scales.truncate(remaining);

    Ok(GraphAdjacency { relations, scales })
  }

  async fn neighbor_page(
    &self,
    request: &GraphNeighborPageRequest,
  ) -> Result<GraphNeighborPage, GraphRepositoryError> {
    if request.content != self.content || request.edge_limit == 0 {
      return Err(GraphRepositoryError::InconsistentData);
    }

    let mut edges = self
      .relations
      .iter()
      .filter_map(|relation| relation.project_from(&request.node))
      .chain(
        self
          .scales
          .iter()
          .flat_map(|scale| scale.project_from(&request.node)),
      )
      .filter(|edge| request.filter.allows(edge.relation_type))
      .collect::<Vec<_>>();
    edges.sort_by(compare_graph_edges);
    let mut seen = BTreeSet::new();
    edges.retain(|edge| seen.insert(edge.id.clone()));
    if let Some(after) = &request.after {
      edges.retain(|edge| compare_graph_edge_ordering_keys(&edge.ordering_key(), after).is_gt());
    }

    let has_more = edges.len() > request.edge_limit;
    edges.truncate(request.edge_limit);
    Ok(GraphNeighborPage { edges, has_more })
  }
}

fn relation_filter_allows(
  filter: &GraphFilter,
  relation: &StoredGraphRelation,
  node: &GraphNodeKey,
) -> bool {
  filter.relation_types.is_empty()
    || relation
      .project_from(node)
      .is_some_and(|edge| filter.allows(edge.relation_type))
}

fn scale_filter_allows(filter: &GraphFilter, scale: &SemanticScale, node: &GraphNodeKey) -> bool {
  filter.relation_types.is_empty()
    || scale
      .project_from(node)
      .into_iter()
      .any(|edge| filter.allows(edge.relation_type))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalId, EvidenceConfidence},
      graph::{
        GraphEdgeId, GraphEvidence, GraphNodeKind, GraphRanking, GraphRelationType, GraphScope,
        GraphScore, GraphScoreComponents, RelationVersion,
      },
    },
    ports::graph_repository::GraphRepository,
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

  fn relation(edge_id: &str) -> StoredGraphRelation {
    StoredGraphRelation {
      edge_id: GraphEdgeId::stored(id(edge_id)),
      relation_version: RelationVersion::new(1).unwrap(),
      source: key("a"),
      target: key("b"),
      relation_type: GraphRelationType::Hypernym,
      evidence: GraphEvidence::new(vec![id("evidence-1")], EvidenceConfidence::High).unwrap(),
      scope: GraphScope::default(),
      feedback_capabilities: Default::default(),
      ranking: GraphRanking {
        display_rank: GraphScore::new(5_000).unwrap(),
        components: GraphScoreComponents {
          evidence: GraphScore::new(5_000).unwrap(),
          community: None,
          pedagogical: None,
        },
        ranking_version: "graph-rank-v1".to_string(),
      },
    }
  }

  #[tokio::test]
  async fn adjacency_is_stably_ordered_and_filtered_by_projected_relation_type() {
    let repository = InMemoryGraphRepository::new(content())
      .with_relation(relation("edge-z"))
      .with_relation(relation("edge-a"));
    let request = GraphAdjacencyRequest {
      content: content(),
      node: key("b"),
      filter: GraphFilter {
        relation_types: [GraphRelationType::Hyponym].into_iter().collect(),
      },
      record_limit: 10,
    };

    let adjacency = repository.adjacency(&request).await.unwrap();

    assert_eq!(adjacency.relations.len(), 2);
    assert_eq!(adjacency.relations[0].edge_id.as_str(), "edge-a");
  }
}
