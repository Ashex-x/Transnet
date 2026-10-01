//! Release-pinned guided knowledge-view composition over authoritative outbound ports.

use std::{
  collections::{BTreeMap, BTreeSet},
  sync::Arc,
};

use thiserror::Error;

use crate::{
  application::knowledge_view_policy::{deterministic_order_key, policy_for},
  domain::{
    assertion::{CanonicalNodeFamily, CanonicalNodeId},
    canonical::EvidenceUse,
    knowledge_hydration::{CanonicalAssertionProjectionRef, HydratedAssertionProjection},
    knowledge_view::{
      KnowledgeEvidenceState, KnowledgeViewItem, KnowledgeViewRequest, KnowledgeViewSuperset,
      UsefulRootPath, VerifiedKnowledgeStep,
    },
    request_context::RequestContext,
    retrieval_data::{
      NeighborDirection, NeighborProjectionExecutionExpectation, NeighborSearchRequest,
      RetrievalRelation, RetrievalVerificationState,
    },
  },
  ports::{
    canonical_read::{
      CanonicalAssertionQuery, CanonicalKnowledgeNodeQuery, CanonicalReadContext,
      CanonicalReadError, CanonicalReadPort, CanonicalScaleQuery,
    },
    retrieval_data::{RetrievalDataError, RetrievalDataPort},
  },
};

/// Closed failures that distinguish projection gaps from dependency failures.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgeViewServiceError {
  /// The request, release pin, or returned immutable proof contradicted the contract.
  #[error("knowledge view data is inconsistent")]
  InconsistentData,
  /// Retrieval nominated a published record that canonical-data could not hydrate.
  #[error("knowledge view publication is partial")]
  PartialPublication,
  /// A required canonical-data or retrieval-data dependency was unavailable.
  #[error("knowledge view dependency is unavailable")]
  DependencyUnavailable,
  /// The shared request deadline was exhausted.
  #[error("knowledge view deadline exceeded")]
  DeadlineExceeded,
}

/// Guided-view service that retains no query-derived or request-derived state.
pub struct KnowledgeViewService {
  retrieval: Arc<dyn RetrievalDataPort>,
  canonical: Arc<dyn CanonicalReadPort>,
  execution: NeighborProjectionExecutionExpectation,
}

impl KnowledgeViewService {
  /// Creates a service bound to one immutable retrieval projection expectation.
  pub fn new(
    retrieval: Arc<dyn RetrievalDataPort>,
    canonical: Arc<dyn CanonicalReadPort>,
    execution: NeighborProjectionExecutionExpectation,
  ) -> Self {
    Self {
      retrieval,
      canonical,
      execution,
    }
  }

  /// Builds one deterministic factual view from exact hydrated assertion traversals.
  ///
  /// # Errors
  ///
  /// Returns a closed error when the request pin, dependency result, publication coverage, or
  /// immutable proof is unavailable or inconsistent. Similarity is never accepted as proof.
  pub async fn build(
    &self,
    context: &RequestContext,
    request: KnowledgeViewRequest,
  ) -> Result<KnowledgeViewSuperset, KnowledgeViewServiceError> {
    if context.content_release() != Some(&request.release.release_id)
      || request.release != self.execution.content
      || self.execution.validate().is_err()
      || context.remaining_budget().is_zero()
    {
      return Err(KnowledgeViewServiceError::InconsistentData);
    }
    let policy = policy_for(request.lens);
    let relations = policy
      .relations
      .iter()
      .map(|relation| {
        RetrievalRelation::from_wire_name(relation.rule().qdrant_wire_name.unwrap_or(""))
          .map_err(|_| KnowledgeViewServiceError::InconsistentData)
      })
      .collect::<Result<Vec<_>, _>>()?;
    let mut frontier = vec![(request.root.node.clone(), Vec::new())];
    let mut visited = BTreeSet::from([request.root.node.clone()]);
    let mut gathered = Vec::new();
    let mut next_cursor = None;

    for depth in 0..policy.max_depth {
      let mut next = Vec::new();
      for (current, root_path) in frontier {
        let result = self
          .retrieval
          .search_neighbors(
            context,
            NeighborSearchRequest {
              node_id: current.id().clone(),
              direction: NeighborDirection::Both,
              relation_types: relations.clone(),
              verification_states: vec![RetrievalVerificationState::Verified],
              languages: vec![request.target_language.clone()],
              domain_ids: Vec::new(),
              release_id: request.release.release_id.clone(),
              execution: self.execution.clone(),
              limit: policy.max_items.min(50),
              cursor: if depth == 0 {
                request.cursor.clone()
              } else {
                None
              },
            },
          )
          .await
          .map_err(map_retrieval_error)?;
        if result.release_id != request.release.release_id
          || result.root_node_id != *current.id()
          || result.execution.validate_against(&self.execution).is_err()
          || result.neighbors.len() > policy.max_items.min(50)
        {
          return Err(KnowledgeViewServiceError::InconsistentData);
        }
        if depth == 0 {
          next_cursor = result.next_cursor.clone();
        }
        for candidate in result.neighbors {
          if candidate.edge.verification_state != RetrievalVerificationState::Verified
            || !policy
              .relations
              .contains(&candidate.edge.relation_type.relation_type())
            || candidate.edge.relation_registry_version
              != self.execution.relationship_registry_version
            || candidate.node.node_id == *current.id()
          {
            return Err(KnowledgeViewServiceError::InconsistentData);
          }
          let opposite = if candidate.edge.source_node_id == *current.id() {
            &candidate.edge.target_node_id
          } else if candidate.edge.target_node_id == *current.id() {
            &candidate.edge.source_node_id
          } else {
            return Err(KnowledgeViewServiceError::InconsistentData);
          };
          if opposite != &candidate.node.node_id {
            return Err(KnowledgeViewServiceError::InconsistentData);
          }
          let family: CanonicalNodeFamily = candidate.node.node_type.into();
          if !policy.node_families.contains(&family) {
            continue;
          }
          let node = CanonicalNodeId::publisher_assigned(family, candidate.node.node_id.clone());
          if !visited.insert(node.clone()) {
            continue;
          }
          let mut path = root_path.clone();
          path.push(candidate.edge.assertion_projection());
          gathered.push((
            node.clone(),
            candidate.edge.relation_type.relation_type(),
            path.clone(),
          ));
          if gathered.len() >= policy.max_items.saturating_sub(1) {
            break;
          }
          next.push((node, path));
        }
        if gathered.len() >= policy.max_items.saturating_sub(1) {
          break;
        }
      }
      frontier = next;
      if frontier.is_empty() || gathered.len() >= policy.max_items.saturating_sub(1) {
        break;
      }
    }

    gathered.sort_by_key(|(node, relation, _)| {
      deterministic_order_key(&policy, *relation, node.family(), node.id())
    });
    self
      .hydrate_and_compose(context, request, gathered, next_cursor)
      .await
  }

  async fn hydrate_and_compose(
    &self,
    context: &RequestContext,
    request: KnowledgeViewRequest,
    gathered: Vec<(
      CanonicalNodeId,
      crate::domain::graph::GraphRelationType,
      Vec<CanonicalAssertionProjectionRef>,
    )>,
    next_cursor: Option<String>,
  ) -> Result<KnowledgeViewSuperset, KnowledgeViewServiceError> {
    let mut seen_edges = BTreeSet::new();
    let mut projections = Vec::new();
    for (_, _, path) in &gathered {
      for projection in path {
        if seen_edges.insert(projection.edge_id.clone()) {
          projections.push(projection.clone());
        }
      }
    }
    let canonical_context = CanonicalReadContext {
      request_id: context.request_id().as_str().to_string(),
      deadline_at: context.deadline_rfc3339(),
      timeout: context.remaining_budget(),
    };
    let hydrated = if projections.is_empty() {
      Vec::new()
    } else {
      self
        .canonical
        .canonical_assertions(
          &canonical_context,
          &request.release,
          CanonicalAssertionQuery {
            projections: projections.clone(),
            evidence_use: EvidenceUse::ApiRedistribution,
            verification_states: vec![RetrievalVerificationState::Verified],
            limit: projections.len(),
          },
        )
        .await
        .map_err(map_canonical_error)?
    };
    if hydrated.len() != projections.len() {
      return Err(KnowledgeViewServiceError::PartialPublication);
    }
    let proofs = hydrated
      .iter()
      .map(|value| (value.traversal().edge_id.clone(), value))
      .collect::<BTreeMap<_, _>>();
    if proofs.len() != hydrated.len() {
      return Err(KnowledgeViewServiceError::InconsistentData);
    }

    let node_ids = std::iter::once(request.root.node.id().clone())
      .chain(gathered.iter().map(|item| item.0.id().clone()))
      .collect::<Vec<_>>();
    let nodes = self
      .canonical
      .knowledge_nodes(
        &canonical_context,
        &request.release,
        CanonicalKnowledgeNodeQuery {
          node_ids: node_ids.clone(),
          evidence_use: EvidenceUse::ApiRedistribution,
          limit: node_ids.len(),
        },
      )
      .await
      .map_err(map_canonical_error)?;
    if nodes.len() != node_ids.len() {
      return Err(KnowledgeViewServiceError::PartialPublication);
    }
    let node_map = nodes
      .iter()
      .map(|node| (node.node_id.clone(), node))
      .collect::<BTreeMap<_, _>>();
    for expected in &node_ids {
      let node = node_map
        .get(expected)
        .ok_or(KnowledgeViewServiceError::PartialPublication)?;
      node
        .validate()
        .map_err(|_| KnowledgeViewServiceError::InconsistentData)?;
      let expected_family = if expected == request.root.node.id() {
        request.root.node.family()
      } else {
        gathered
          .iter()
          .find(|item| item.0.id() == expected)
          .map(|item| item.0.family())
          .ok_or(KnowledgeViewServiceError::InconsistentData)?
      };
      if CanonicalNodeFamily::from(node.node_type) != expected_family {
        return Err(KnowledgeViewServiceError::InconsistentData);
      }
    }
    let scale_ids = gathered
      .iter()
      .filter(|item| item.0.family() == CanonicalNodeFamily::SemanticScale)
      .map(|item| item.0.id().clone())
      .collect::<Vec<_>>();
    if !scale_ids.is_empty() {
      let scales = self
        .canonical
        .semantic_scales(
          &canonical_context,
          &request.release,
          CanonicalScaleQuery {
            scale_ids: scale_ids.clone(),
            for_node_id: request.root.node.id().clone(),
            verification_states: vec![RetrievalVerificationState::Verified],
            limit: scale_ids.len(),
          },
        )
        .await
        .map_err(map_canonical_error)?;
      if scales.len() != scale_ids.len() {
        return Err(KnowledgeViewServiceError::PartialPublication);
      }
      for scale in scales {
        scale
          .validate()
          .map_err(|_| KnowledgeViewServiceError::InconsistentData)?;
      }
    }

    let branch = request.lens.relevance_reason();
    let mut items = vec![KnowledgeViewItem {
      node: request.root.node.clone(),
      branch,
      order: 1,
      evidence_state: KnowledgeEvidenceState::Verified,
      path_to_root: None,
    }];
    for (index, (node, _, path)) in gathered.into_iter().enumerate() {
      let mut steps = Vec::with_capacity(path.len());
      for projection in path.into_iter().rev() {
        let proof: &&HydratedAssertionProjection = proofs
          .get(&projection.edge_id)
          .ok_or(KnowledgeViewServiceError::PartialPublication)?;
        let traversal = proof.traversal();
        steps.push(VerifiedKnowledgeStep {
          edge_id: traversal.edge_id.clone(),
          relationship_revision: traversal.relationship_revision,
          assertion_id: traversal.assertion_id.clone(),
          assertion_revision: traversal.assertion_revision,
          traversal_id: traversal.traversal_id.clone(),
          relation_registry_revision: traversal.relation_registry_revision,
          source: traversal.source.clone(),
          target: traversal.target.clone(),
          evidence_ids: proof.assertion().evidence_ids.clone(),
        });
      }
      items.push(KnowledgeViewItem {
        node: node.clone(),
        branch,
        order: (index + 2) as u16,
        evidence_state: KnowledgeEvidenceState::Verified,
        path_to_root: Some(UsefulRootPath {
          root: request.root.node.clone(),
          item: node,
          steps,
        }),
      });
    }
    let result = KnowledgeViewSuperset {
      request,
      items,
      next_cursor,
    };
    result
      .validate()
      .map_err(|_| KnowledgeViewServiceError::InconsistentData)?;
    Ok(result)
  }
}

fn map_retrieval_error(error: RetrievalDataError) -> KnowledgeViewServiceError {
  match error {
    RetrievalDataError::Timeout => KnowledgeViewServiceError::DeadlineExceeded,
    RetrievalDataError::Unavailable => KnowledgeViewServiceError::DependencyUnavailable,
    RetrievalDataError::NotFound => KnowledgeViewServiceError::PartialPublication,
    _ => KnowledgeViewServiceError::InconsistentData,
  }
}

fn map_canonical_error(error: CanonicalReadError) -> KnowledgeViewServiceError {
  match error {
    CanonicalReadError::Timeout => KnowledgeViewServiceError::DeadlineExceeded,
    CanonicalReadError::Unavailable => KnowledgeViewServiceError::DependencyUnavailable,
    CanonicalReadError::NotFound => KnowledgeViewServiceError::PartialPublication,
    _ => KnowledgeViewServiceError::InconsistentData,
  }
}

#[cfg(test)]
mod tests {
  use async_trait::async_trait;
  use time::{format_description::well_known::Rfc3339, OffsetDateTime};

  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
      embedding_input::{EDGE_DENSE_INPUT_VERSION, EDGE_LEXICAL_INPUT_VERSION},
      knowledge_view::{KnowledgeLens, KnowledgeRoot},
      request_context::RequestId,
      retrieval_data::{
        EdgeSearchRequest, EdgeSearchResult, NeighborSearchResult, NodeSearchRequest,
        NodeSearchResult, ScaleSearchRequest, ScaleSearchResult, RELATION_REGISTRY_VERSION,
      },
      translation_turn::ResponseLevel,
    },
    ports::canonical_read::{
      CanonicalCandidateQuery, CanonicalDomainQuery, CanonicalSenseQuery, CanonicalTranslationQuery,
    },
  };

  struct UnavailableRetrieval;

  #[async_trait]
  impl RetrievalDataPort for UnavailableRetrieval {
    async fn search_nodes(
      &self,
      _: &RequestContext,
      _: NodeSearchRequest,
    ) -> Result<NodeSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_scales(
      &self,
      _: &RequestContext,
      _: ScaleSearchRequest,
    ) -> Result<ScaleSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_edges(
      &self,
      _: &RequestContext,
      _: EdgeSearchRequest,
    ) -> Result<EdgeSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
    async fn search_neighbors(
      &self,
      _: &RequestContext,
      _: NeighborSearchRequest,
    ) -> Result<NeighborSearchResult, RetrievalDataError> {
      Err(RetrievalDataError::Unavailable)
    }
  }

  struct UnavailableCanonical;

  #[async_trait]
  impl CanonicalReadPort for UnavailableCanonical {
    async fn active_release(
      &self,
      _: &CanonicalReadContext,
    ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn translations(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalTranslationQuery,
    ) -> Result<
      Vec<crate::domain::canonical_translation::CanonicalTranslationRevision>,
      CanonicalReadError,
    > {
      Err(CanonicalReadError::Unavailable)
    }
    async fn candidates(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalCandidateQuery,
    ) -> Result<Vec<crate::domain::retrieval::RepositoryMatch>, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn sense(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalSenseQuery,
    ) -> Result<crate::domain::canonical_content::CanonicalSenseDetails, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
    async fn domains(
      &self,
      _: &CanonicalReadContext,
      _: &CanonicalReleasePin,
      _: CanonicalDomainQuery,
    ) -> Result<crate::domain::domain_assessment::DomainInventory, CanonicalReadError> {
      Err(CanonicalReadError::Unavailable)
    }
  }

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn pin(value: &str) -> CanonicalReleasePin {
    CanonicalReleasePin::new(id(value), "canonical-v1".into()).unwrap()
  }

  fn execution(pin: CanonicalReleasePin) -> NeighborProjectionExecutionExpectation {
    NeighborProjectionExecutionExpectation {
      content: pin,
      node_collection_id: id("nodes-1"),
      node_collection_content_hash: format!("sha256:{}", "1".repeat(64)),
      edge_collection_id: id("edges-1"),
      edge_collection_content_hash: format!("sha256:{}", "2".repeat(64)),
      relationship_registry_version: RELATION_REGISTRY_VERSION,
      edge_dense_input_version: EDGE_DENSE_INPUT_VERSION.into(),
      edge_lexical_input_version: EDGE_LEXICAL_INPUT_VERSION.into(),
    }
  }

  fn context(pin: &CanonicalReleasePin) -> RequestContext {
    RequestContext::new(
      RequestId::new("request-1").unwrap(),
      OffsetDateTime::parse("2099-01-01T00:00:00.000000Z", &Rfc3339).unwrap(),
      "transnet-v1",
      Some(pin.release_id.clone()),
    )
    .unwrap()
  }

  fn request(pin: CanonicalReleasePin) -> KnowledgeViewRequest {
    KnowledgeViewRequest {
      root: KnowledgeRoot {
        node: CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id("root-1")),
      },
      lens: KnowledgeLens::Meaning,
      target_language: LanguageTag::parse("en").unwrap(),
      response_level: ResponseLevel::Standard,
      release: pin,
      cursor: None,
    }
  }

  #[tokio::test]
  async fn dependency_failure_is_not_reported_as_partial_publication() {
    let release = pin("release-1");
    let service = KnowledgeViewService::new(
      Arc::new(UnavailableRetrieval),
      Arc::new(UnavailableCanonical),
      execution(release.clone()),
    );
    assert_eq!(
      service.build(&context(&release), request(release)).await,
      Err(KnowledgeViewServiceError::DependencyUnavailable)
    );
  }

  #[tokio::test]
  async fn contradictory_full_release_pin_fails_before_dependency_access() {
    let release = pin("release-1");
    let other = pin("release-2");
    let service = KnowledgeViewService::new(
      Arc::new(UnavailableRetrieval),
      Arc::new(UnavailableCanonical),
      execution(release.clone()),
    );
    assert_eq!(
      service.build(&context(&release), request(other)).await,
      Err(KnowledgeViewServiceError::InconsistentData)
    );
  }
}
