//! Application entry point for deterministic release-pinned projection preparation.

use crate::domain::canonical::ReleaseId;
use crate::domain::graph::PublishedRelationship;
use crate::domain::knowledge_projection::{
  build_edge_projection, build_node_projection, CanonicalNodeProjectionInput, EdgeProjectionBuild,
  NodeProjectionBuild, ProjectionEmbeddingSpec, ProjectionValidationError,
};

/// Request-independent builder pinned to one canonical release and embedding specification.
#[derive(Debug, Clone)]
pub struct KnowledgeProjectionBuilder {
  release_id: ReleaseId,
  embedding: ProjectionEmbeddingSpec,
}

impl KnowledgeProjectionBuilder {
  /// Creates a builder without contacting Qdrant or an embedding provider.
  pub fn new(release_id: ReleaseId, embedding: ProjectionEmbeddingSpec) -> Self {
    Self {
      release_id,
      embedding,
    }
  }

  /// Produces the deterministic node-first pre-publication artifact.
  ///
  /// # Errors
  ///
  /// Returns an error when canonical input is ineligible, unresolved, or contradictory.
  pub fn build_nodes(
    &self,
    inputs: Vec<CanonicalNodeProjectionInput>,
  ) -> Result<NodeProjectionBuild, ProjectionValidationError> {
    build_node_projection(self.release_id.clone(), self.embedding.clone(), inputs)
  }

  /// Produces the deterministic edge artifact against an exact verified node build.
  ///
  /// # Errors
  ///
  /// Returns an error when relationship admission or endpoint reconciliation fails.
  pub fn build_edges(
    &self,
    nodes: &NodeProjectionBuild,
    relationships: Vec<PublishedRelationship>,
  ) -> Result<EdgeProjectionBuild, ProjectionValidationError> {
    if nodes.release_id != self.release_id {
      return Err(ProjectionValidationError::NodeBuildMismatch);
    }
    build_edge_projection(nodes, self.embedding.clone(), relationships)
  }
}
