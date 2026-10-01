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
    knowledge_hydration::CanonicalAssertionProjectionRef,
    knowledge_view::{
      KnowledgeEvidenceState, KnowledgeViewBranch, KnowledgeViewItem, KnowledgeViewRequest,
      KnowledgeViewSuperset, UsefulRootPath, VerifiedKnowledgeStep,
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
  /// The selected closed lens has no sound relation policy in this release.
  #[error("knowledge view lens is unavailable")]
  LensUnavailable,
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
    {
      return Err(KnowledgeViewServiceError::InconsistentData);
    }
    ensure_deadline(context)?;
    // The public token is an authenticated k1 cursor, never a retrieval-data continuation token.
    // This service materializes its bounded superset, so it accepts only the initial page.
    if request.cursor.is_some() {
      return Err(KnowledgeViewServiceError::InconsistentData);
    }
    let policy = policy_for(request.lens).ok_or(KnowledgeViewServiceError::LensUnavailable)?;
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

    for _depth in 0..policy.max_depth {
      let mut layer = Vec::new();
      for (current, root_path) in frontier {
        let mut dependency_cursor = None;
        for page in 0..8 {
          ensure_deadline(context)?;
          let result = self
            .retrieval
            .search_neighbors(
              context,
              NeighborSearchRequest {
                node_id: current.id().clone(),
                // A useful path is item -> ... -> root, so only edges directed into the current
                // endpoint can extend it without inventing an inverse traversal.
                direction: NeighborDirection::Incoming,
                relation_types: relations.clone(),
                verification_states: vec![RetrievalVerificationState::Verified],
                languages: vec![request.target_language.clone()],
                domain_ids: Vec::new(),
                release_id: request.release.release_id.clone(),
                execution: self.execution.clone(),
                limit: policy.max_items.min(50),
                cursor: dependency_cursor,
              },
            )
            .await
            .map_err(map_retrieval_error)?;
          ensure_deadline(context)?;
          if result.release_id != request.release.release_id
            || result.root_node_id != *current.id()
            || result.execution.validate_against(&self.execution).is_err()
            || result.neighbors.len() > policy.max_items.min(50)
          {
            return Err(KnowledgeViewServiceError::InconsistentData);
          }
          for candidate in result.neighbors {
            if candidate.edge.verification_state != RetrievalVerificationState::Verified
              || !policy
                .relations
                .contains(&candidate.edge.relation_type.relation_type())
              || candidate.edge.relation_registry_version
                != self.execution.relationship_registry_version
              || candidate.edge.target_node_id != *current.id()
              || candidate.edge.source_node_id != candidate.node.node_id
            {
              return Err(KnowledgeViewServiceError::InconsistentData);
            }
            let family: CanonicalNodeFamily = candidate.node.node_type.into();
            if !policy.node_families.contains(&family) {
              continue;
            }
            let node = CanonicalNodeId::publisher_assigned(family, candidate.node.node_id.clone());
            let mut path = vec![candidate.edge.assertion_projection()];
            path.extend(root_path.clone());
            layer.push((node, candidate.edge.relation_type.relation_type(), path));
          }
          dependency_cursor = result.next_cursor;
          if dependency_cursor.is_none() {
            break;
          }
          if page == 7 {
            return Err(KnowledgeViewServiceError::InconsistentData);
          }
        }
      }
      layer.sort_by_key(|(node, relation, path)| {
        (
          deterministic_order_key(&policy, *relation, node.family(), node.id()),
          path
            .iter()
            .map(|step| step.edge_id.to_string())
            .collect::<Vec<_>>(),
        )
      });
      let mut next = Vec::new();
      for (node, relation, path) in layer {
        if !visited.insert(node.clone()) {
          continue;
        }
        gathered.push((node.clone(), relation, path.clone()));
        next.push((node, path));
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
    self.hydrate_and_compose(context, request, gathered).await
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
      ensure_deadline(context)?;
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
    ensure_deadline(context)?;
    if hydrated.len() != projections.len() {
      return Err(KnowledgeViewServiceError::PartialPublication);
    }
    let proofs = hydrated
      .into_iter()
      .map(|value| (value.traversal().edge_id.clone(), value))
      .collect::<BTreeMap<_, _>>();
    if proofs.len() != projections.len() {
      return Err(KnowledgeViewServiceError::InconsistentData);
    }

    let node_ids = std::iter::once(request.root.node.id().clone())
      .chain(gathered.iter().map(|item| item.0.id().clone()))
      .collect::<Vec<_>>();
    ensure_deadline(context)?;
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
    ensure_deadline(context)?;
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
      ensure_deadline(context)?;
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
      ensure_deadline(context)?;
      if scales.len() != scale_ids.len() {
        return Err(KnowledgeViewServiceError::PartialPublication);
      }
      let expected_scale_ids = scale_ids.iter().collect::<BTreeSet<_>>();
      let returned_scale_ids = scales
        .iter()
        .map(|scale| &scale.scale_id)
        .collect::<BTreeSet<_>>();
      if returned_scale_ids != expected_scale_ids {
        return Err(KnowledgeViewServiceError::InconsistentData);
      }
      for scale in scales {
        scale
          .validate()
          .map_err(|_| KnowledgeViewServiceError::InconsistentData)?;
        if !scale
          .members
          .iter()
          .any(|member| member.node_id == *request.root.node.id())
        {
          return Err(KnowledgeViewServiceError::InconsistentData);
        }
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
      for projection in path {
        let proof = proofs
          .get(&projection.edge_id)
          .cloned()
          .ok_or(KnowledgeViewServiceError::PartialPublication)?;
        steps.push(
          VerifiedKnowledgeStep::from_hydrated(proof, request.release.clone())
            .map_err(|_| KnowledgeViewServiceError::InconsistentData)?,
        );
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
    let item_ids = items
      .iter()
      .skip(1)
      .map(|item| item.node.clone())
      .collect::<Vec<_>>();
    let branches = if item_ids.is_empty() {
      Vec::new()
    } else {
      vec![KnowledgeViewBranch {
        order: 1,
        reason: branch,
        item_ids,
      }]
    };
    let result = KnowledgeViewSuperset {
      request,
      items,
      branches,
      truncated: false,
      next_cursor: None,
    };
    result
      .validate()
      .map_err(|_| KnowledgeViewServiceError::InconsistentData)?;
    ensure_deadline(context)?;
    Ok(result)
  }
}

fn ensure_deadline(context: &RequestContext) -> Result<(), KnowledgeViewServiceError> {
  if context.remaining_budget().is_zero() {
    Err(KnowledgeViewServiceError::DeadlineExceeded)
  } else {
    Ok(())
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
  use std::sync::Mutex;

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
        EdgeSearchRequest, EdgeSearchResult, NeighborProjectionExecutionProof,
        NeighborSearchResult, NodeSearchRequest, NodeSearchResult, ScaleSearchRequest,
        ScaleSearchResult, RELATION_REGISTRY_VERSION,
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

  struct PagingRetrieval {
    execution: NeighborProjectionExecutionProof,
    calls: Mutex<Vec<(NeighborDirection, Option<String>)>>,
  }

  #[async_trait]
  impl RetrievalDataPort for PagingRetrieval {
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
      request: NeighborSearchRequest,
    ) -> Result<NeighborSearchResult, RetrievalDataError> {
      let mut calls = self.calls.lock().unwrap();
      calls.push((request.direction, request.cursor.clone()));
      if calls.len() == 1 {
        Ok(NeighborSearchResult {
          release_id: request.release_id,
          execution: self.execution.clone(),
          root_node_id: request.node_id,
          neighbors: Vec::new(),
          next_cursor: Some("dependency-page-2".into()),
        })
      } else {
        Err(RetrievalDataError::Unavailable)
      }
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

  fn execution_proof(
    expected: &NeighborProjectionExecutionExpectation,
  ) -> NeighborProjectionExecutionProof {
    NeighborProjectionExecutionProof {
      content: expected.content.clone(),
      node_collection_id: expected.node_collection_id.clone(),
      node_collection_content_hash: expected.node_collection_content_hash.clone(),
      edge_collection_id: expected.edge_collection_id.clone(),
      edge_collection_content_hash: expected.edge_collection_content_hash.clone(),
      relationship_registry_version: expected.relationship_registry_version,
      edge_dense_input_version: expected.edge_dense_input_version.clone(),
      edge_lexical_input_version: expected.edge_lexical_input_version.clone(),
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

  #[tokio::test]
  async fn public_cursor_is_never_forwarded_as_a_dependency_cursor() {
    let release = pin("release-1");
    let service = KnowledgeViewService::new(
      Arc::new(UnavailableRetrieval),
      Arc::new(UnavailableCanonical),
      execution(release.clone()),
    );
    let mut input = request(release.clone());
    input.cursor = Some("k1.secret-public-token".into());
    assert_eq!(
      service.build(&context(&release), input).await,
      Err(KnowledgeViewServiceError::InconsistentData)
    );
  }

  #[tokio::test]
  async fn lens_without_a_sound_registry_policy_fails_closed() {
    let release = pin("release-1");
    let service = KnowledgeViewService::new(
      Arc::new(UnavailableRetrieval),
      Arc::new(UnavailableCanonical),
      execution(release.clone()),
    );
    let mut input = request(release.clone());
    input.lens = KnowledgeLens::Mechanism;
    assert_eq!(
      service.build(&context(&release), input).await,
      Err(KnowledgeViewServiceError::LensUnavailable)
    );
  }

  #[tokio::test]
  async fn child_pagination_is_exhausted_with_incoming_traversal_only() {
    let release = pin("release-1");
    let expected = execution(release.clone());
    let retrieval = Arc::new(PagingRetrieval {
      execution: execution_proof(&expected),
      calls: Mutex::new(Vec::new()),
    });
    let service =
      KnowledgeViewService::new(retrieval.clone(), Arc::new(UnavailableCanonical), expected);
    assert_eq!(
      service.build(&context(&release), request(release)).await,
      Err(KnowledgeViewServiceError::DependencyUnavailable)
    );
    assert_eq!(
      *retrieval.calls.lock().unwrap(),
      vec![
        (NeighborDirection::Incoming, None),
        (
          NeighborDirection::Incoming,
          Some("dependency-page-2".into())
        ),
      ]
    );
  }
}
