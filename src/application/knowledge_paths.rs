//! Deterministic bounded search for fully hydrated canonical knowledge paths.

use std::{
  collections::{BTreeSet, VecDeque},
  sync::Arc,
};

use thiserror::Error;

use crate::{
  domain::{
    assertion::{CanonicalNodeFamily, CanonicalNodeId},
    canonical::EvidenceUse,
    knowledge_hydration::{CanonicalAssertionProjectionRef, HydratedAssertionProjection},
    knowledge_view::{
      KnowledgePathOutcome, KnowledgePathRequest, KnowledgePathResult, VerifiedKnowledgePath,
      VerifiedKnowledgeStep, MAX_KNOWLEDGE_PATHS, MAX_KNOWLEDGE_PATH_HOPS,
    },
    model_runtime::CancellationSignal,
    request_context::RequestContext,
    retrieval_data::{
      NeighborDirection, NeighborProjectionExecutionExpectation, NeighborSearchRequest,
      NeighborSearchResult, RetrievalRelation, RetrievalVerificationState,
      MAX_RETRIEVAL_DATA_RESULTS,
    },
  },
  ports::{
    canonical_read::{
      CanonicalAssertionQuery, CanonicalKnowledgeNodeQuery, CanonicalReadContext,
      CanonicalReadError, CanonicalReadPort,
    },
    retrieval_data::{RetrievalDataError, RetrievalDataPort},
  },
};

/// Maximum distinct intermediate nodes expanded by one path request.
pub const MAX_KNOWLEDGE_PATH_EXPANSIONS: usize = 50;
/// Maximum retrieval pages consumed while proving one bounded-search outcome.
pub const MAX_KNOWLEDGE_PATH_PAGES: usize = 12;
/// Frozen useful-path relation policy revision.
pub const KNOWLEDGE_PATH_POLICY_VERSION: u32 = 1;

/// Content-free failures that preclude a trustworthy bounded-path outcome.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgePathSearchError {
  /// The request, context pin, or configured immutable execution pin is inconsistent.
  #[error("knowledge path request is invalid")]
  InvalidRequest,
  /// The selected immutable release is no longer readable.
  #[error("knowledge path release is unavailable")]
  ContentReleaseUnavailable,
  /// A dependency could not complete an otherwise eligible search.
  #[error("knowledge path dependency is unavailable")]
  DependencyUnavailable,
  /// The shared request deadline elapsed or cancellation was requested.
  #[error("knowledge path request did not complete before its deadline")]
  DeadlineExceeded,
  /// Retrieval and canonical data disagree about an immutable proof.
  #[error("knowledge path proof is inconsistent")]
  InconsistentProof,
  /// The bounded search could not be completed, so absence cannot be claimed.
  #[error("knowledge path search is incomplete")]
  IncompleteSearch,
}

/// Request-local path composition over retrieval nominations and canonical assertion hydration.
pub struct BoundedKnowledgePathService {
  retrieval: Arc<dyn RetrievalDataPort>,
  canonical: Arc<dyn CanonicalReadPort>,
  execution: NeighborProjectionExecutionExpectation,
}

impl BoundedKnowledgePathService {
  /// Creates a service fixed to one immutable release trio and relation-registry revision.
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

  /// Returns the immutable retrieval execution served by this application instance.
  pub fn execution(&self) -> &NeighborProjectionExecutionExpectation {
    &self.execution
  }

  /// Finds up to three deterministic directed paths of no more than three verified hops.
  ///
  /// Every traversed edge is hydrated as its exact canonical assertion and declared binary
  /// traversal before it may enter the frontier. A successful empty outcome is emitted only after
  /// the eligible bounded search completes without stale, partial, or unavailable dependencies.
  ///
  /// # Errors
  ///
  /// Returns a content-free failure when the full pin is inconsistent, the search cannot finish,
  /// or retrieval and canonical proof disagree.
  pub async fn find(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: KnowledgePathRequest,
  ) -> Result<KnowledgePathResult, KnowledgePathSearchError> {
    self.validate_request(context, cancellation, &request)?;
    self.validate_roots(context, cancellation, &request).await?;

    let mut frontier = VecDeque::from([SearchState {
      node: request.from.node.clone(),
      steps: Vec::new(),
      visited: BTreeSet::from([request.from.node.clone()]),
    }]);
    let mut paths = Vec::new();
    let mut expansions = 0usize;
    let mut pages = 0usize;

    while let Some(state) = frontier.pop_front() {
      ensure_active(context, cancellation)?;
      if state.steps.len() >= MAX_KNOWLEDGE_PATH_HOPS {
        continue;
      }
      if expansions == MAX_KNOWLEDGE_PATH_EXPANSIONS {
        return Err(KnowledgePathSearchError::IncompleteSearch);
      }
      expansions += 1;

      let neighbors = self
        .complete_neighbors(context, cancellation, &request, &state.node, &mut pages)
        .await?;
      let hydrated = self
        .hydrate(context, cancellation, &request, &neighbors)
        .await?;
      let mut next_states = Vec::new();
      for (candidate, proof) in neighbors.into_iter().zip(hydrated) {
        let traversal = proof.traversal();
        let expected = candidate.edge.assertion_projection();
        if !projection_matches(&expected, &proof)
          || traversal.source != state.node
          || traversal.target.id() != &candidate.node.node_id
          || traversal.target.family() != CanonicalNodeFamily::from(candidate.node.node_type)
        {
          return Err(KnowledgePathSearchError::InconsistentProof);
        }
        if state.visited.contains(&traversal.target) {
          continue;
        }
        let next_node = traversal.target.clone();
        let step = step_from(proof, request.release.clone())?;
        let mut steps = state.steps.clone();
        steps.push(step);
        if next_node == request.to.node {
          paths.push(VerifiedKnowledgePath { order: 0, steps });
          if paths.len() == MAX_KNOWLEDGE_PATHS {
            ensure_active(context, cancellation)?;
            return finalize(request, paths);
          }
          continue;
        }
        let mut visited = state.visited.clone();
        visited.insert(next_node.clone());
        next_states.push(SearchState {
          node: next_node,
          steps,
          visited,
        });
      }
      next_states.sort_by_key(state_key);
      frontier.extend(next_states);
    }

    ensure_active(context, cancellation)?;
    if paths.is_empty() {
      let result = KnowledgePathResult {
        request,
        outcome: KnowledgePathOutcome::NoVerifiedPath,
      };
      result
        .validate()
        .map_err(|_| KnowledgePathSearchError::InconsistentProof)?;
      Ok(result)
    } else {
      finalize(request, paths)
    }
  }

  fn validate_request(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: &KnowledgePathRequest,
  ) -> Result<(), KnowledgePathSearchError> {
    ensure_active(context, cancellation)?;
    if request.from == request.to
      || context.content_release() != Some(&request.release.release_id)
      || self.execution.content != request.release
      || self.execution.validate().is_err()
    {
      return Err(KnowledgePathSearchError::InvalidRequest);
    }
    Ok(())
  }

  async fn complete_neighbors(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: &KnowledgePathRequest,
    root: &CanonicalNodeId,
    pages: &mut usize,
  ) -> Result<Vec<crate::domain::retrieval_data::NeighborCandidate>, KnowledgePathSearchError> {
    let mut cursor = None;
    let mut collected = Vec::new();
    let mut edge_ids = BTreeSet::new();
    loop {
      ensure_active(context, cancellation)?;
      if *pages == MAX_KNOWLEDGE_PATH_PAGES {
        return Err(KnowledgePathSearchError::IncompleteSearch);
      }
      *pages += 1;
      let search = NeighborSearchRequest {
        node_id: root.id().clone(),
        direction: NeighborDirection::Outgoing,
        relation_types: eligible_relations(),
        verification_states: vec![RetrievalVerificationState::Verified],
        languages: vec![request.target_language.clone()],
        domain_ids: Vec::new(),
        release_id: request.release.release_id.clone(),
        execution: self.execution.clone(),
        limit: MAX_RETRIEVAL_DATA_RESULTS,
        cursor,
      };
      let response = self.retrieval.search_neighbors(context, search).await;
      ensure_active(context, cancellation)?;
      let response = response.map_err(map_retrieval_error)?;
      validate_neighbor_page(&response, root, request, &self.execution)?;
      for candidate in response.neighbors {
        if !edge_ids.insert(candidate.edge.edge_id.clone()) {
          return Err(KnowledgePathSearchError::InconsistentProof);
        }
        collected.push(candidate);
      }
      cursor = response.next_cursor;
      if cursor.is_none() {
        break;
      }
    }
    collected.sort_by(|left, right| {
      (
        &left.edge.edge_id,
        &left.edge.assertion_id,
        &left.node.node_id,
      )
        .cmp(&(
          &right.edge.edge_id,
          &right.edge.assertion_id,
          &right.node.node_id,
        ))
    });
    Ok(collected)
  }

  async fn hydrate(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: &KnowledgePathRequest,
    neighbors: &[crate::domain::retrieval_data::NeighborCandidate],
  ) -> Result<Vec<HydratedAssertionProjection>, KnowledgePathSearchError> {
    if neighbors.is_empty() {
      return Ok(Vec::new());
    }
    ensure_active(context, cancellation)?;
    let projections = neighbors
      .iter()
      .map(|candidate| candidate.edge.assertion_projection())
      .collect::<Vec<_>>();
    let canonical_context = CanonicalReadContext {
      request_id: context.request_id().as_str().to_string(),
      deadline_at: context.deadline_rfc3339(),
      timeout: context.remaining_budget(),
    };
    let hydrated = self
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
      .await;
    ensure_active(context, cancellation)?;
    let hydrated = hydrated.map_err(map_canonical_error)?;
    if hydrated.len() != projections.len() {
      return Err(KnowledgePathSearchError::IncompleteSearch);
    }
    if hydrated
      .iter()
      .zip(&projections)
      .any(|(value, expected)| !projection_matches(expected, value))
    {
      return Err(KnowledgePathSearchError::InconsistentProof);
    }
    Ok(hydrated)
  }

  async fn validate_roots(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: &KnowledgePathRequest,
  ) -> Result<(), KnowledgePathSearchError> {
    let canonical_context = CanonicalReadContext {
      request_id: context.request_id().as_str().to_string(),
      deadline_at: context.deadline_rfc3339(),
      timeout: context.remaining_budget(),
    };
    let expected = [&request.from.node, &request.to.node];
    let roots = self
      .canonical
      .knowledge_nodes(
        &canonical_context,
        &request.release,
        CanonicalKnowledgeNodeQuery {
          node_ids: expected.iter().map(|node| node.id().clone()).collect(),
          evidence_use: EvidenceUse::ApiRedistribution,
          limit: expected.len(),
        },
      )
      .await;
    ensure_active(context, cancellation)?;
    let roots = roots.map_err(map_canonical_error)?;
    if roots.len() != expected.len()
      || roots.iter().zip(expected).any(|(root, expected)| {
        root.validate().is_err()
          || root.node_id != *expected.id()
          || CanonicalNodeFamily::from(root.node_type) != expected.family()
      })
    {
      return Err(KnowledgePathSearchError::IncompleteSearch);
    }
    Ok(())
  }
}

#[derive(Clone)]
struct SearchState {
  node: CanonicalNodeId,
  steps: Vec<VerifiedKnowledgeStep>,
  visited: BTreeSet<CanonicalNodeId>,
}

fn state_key(state: &SearchState) -> (CanonicalNodeFamily, String, Vec<String>) {
  (
    state.node.family(),
    state.node.id().to_string(),
    state
      .steps
      .iter()
      .map(|step| step.projection().traversal().edge_id.to_string())
      .collect(),
  )
}

fn projection_matches(
  expected: &CanonicalAssertionProjectionRef,
  hydrated: &HydratedAssertionProjection,
) -> bool {
  let traversal = hydrated.traversal();
  let assertion = hydrated.assertion();
  expected.edge_id == traversal.edge_id
    && expected.relationship_revision == traversal.relationship_revision
    && expected.assertion_id == assertion.assertion_id
    && expected.assertion_revision == assertion.assertion_revision
    && expected.traversal_id == traversal.traversal_id
    && expected.source_node_id == *traversal.source.id()
    && expected.target_node_id == *traversal.target.id()
    && expected.relation_type == traversal.relation_type
    && expected.relation_registry_revision == traversal.relation_registry_revision
    && hydrated.registry().registry_revision == traversal.relation_registry_revision
    && !assertion.evidence_ids.is_empty()
}

fn step_from(
  hydrated: HydratedAssertionProjection,
  release: crate::domain::canonical::CanonicalReleasePin,
) -> Result<VerifiedKnowledgeStep, KnowledgePathSearchError> {
  VerifiedKnowledgeStep::from_hydrated(hydrated, release)
    .map_err(|_| KnowledgePathSearchError::InconsistentProof)
}

fn validate_neighbor_page(
  response: &NeighborSearchResult,
  root: &CanonicalNodeId,
  request: &KnowledgePathRequest,
  execution: &NeighborProjectionExecutionExpectation,
) -> Result<(), KnowledgePathSearchError> {
  if response.release_id != request.release.release_id
    || response.root_node_id != *root.id()
    || response.execution.validate_against(execution).is_err()
    || response.neighbors.len() > MAX_RETRIEVAL_DATA_RESULTS
    || response.neighbors.iter().any(|candidate| {
      candidate.edge.verification_state != RetrievalVerificationState::Verified
        || candidate.edge.relation_registry_version != execution.relationship_registry_version
        || candidate.edge.assertion_revision == 0
        || candidate.edge.relationship_revision == 0
        || candidate.edge.source_node_id != *root.id()
        || candidate.edge.target_node_id != candidate.node.node_id
    })
  {
    return Err(KnowledgePathSearchError::InconsistentProof);
  }
  Ok(())
}

fn finalize(
  request: KnowledgePathRequest,
  mut paths: Vec<VerifiedKnowledgePath>,
) -> Result<KnowledgePathResult, KnowledgePathSearchError> {
  paths.sort_by(|left, right| {
    let left_key = left
      .steps
      .iter()
      .map(|step| step.projection().traversal().edge_id.to_string())
      .collect::<Vec<_>>();
    let right_key = right
      .steps
      .iter()
      .map(|step| step.projection().traversal().edge_id.to_string())
      .collect::<Vec<_>>();
    (left.steps.len(), left_key).cmp(&(right.steps.len(), right_key))
  });
  paths.dedup_by(|left, right| {
    left
      .steps
      .iter()
      .map(|step| &step.projection().traversal().edge_id)
      .eq(
        right
          .steps
          .iter()
          .map(|step| &step.projection().traversal().edge_id),
      )
  });
  paths.truncate(MAX_KNOWLEDGE_PATHS);
  for (index, path) in paths.iter_mut().enumerate() {
    path.order = (index + 1) as u8;
  }
  let result = KnowledgePathResult {
    request,
    outcome: KnowledgePathOutcome::Connected(paths),
  };
  result
    .validate()
    .map_err(|_| KnowledgePathSearchError::InconsistentProof)?;
  Ok(result)
}

fn ensure_active(
  context: &RequestContext,
  cancellation: &CancellationSignal,
) -> Result<(), KnowledgePathSearchError> {
  if cancellation.is_cancelled() || context.remaining_budget().is_zero() {
    Err(KnowledgePathSearchError::DeadlineExceeded)
  } else {
    Ok(())
  }
}

fn map_retrieval_error(error: RetrievalDataError) -> KnowledgePathSearchError {
  match error {
    RetrievalDataError::ContentReleaseUnavailable => {
      KnowledgePathSearchError::ContentReleaseUnavailable
    }
    RetrievalDataError::NotFound => KnowledgePathSearchError::IncompleteSearch,
    RetrievalDataError::Unavailable => KnowledgePathSearchError::DependencyUnavailable,
    RetrievalDataError::Timeout => KnowledgePathSearchError::DeadlineExceeded,
    RetrievalDataError::InvalidRequest
    | RetrievalDataError::SchemaIncompatible
    | RetrievalDataError::InconsistentData => KnowledgePathSearchError::InconsistentProof,
  }
}

fn map_canonical_error(error: CanonicalReadError) -> KnowledgePathSearchError {
  match error {
    CanonicalReadError::ContentReleaseUnavailable => {
      KnowledgePathSearchError::ContentReleaseUnavailable
    }
    CanonicalReadError::NotFound => KnowledgePathSearchError::IncompleteSearch,
    CanonicalReadError::Unavailable => KnowledgePathSearchError::DependencyUnavailable,
    CanonicalReadError::Timeout => KnowledgePathSearchError::DeadlineExceeded,
    CanonicalReadError::InvalidRequest
    | CanonicalReadError::SchemaIncompatible
    | CanonicalReadError::InconsistentData => KnowledgePathSearchError::InconsistentProof,
  }
}

fn eligible_relations() -> Vec<RetrievalRelation> {
  debug_assert_eq!(KNOWLEDGE_PATH_POLICY_VERSION, 1);
  [
    "synonym",
    "near_synonym",
    "translation_equivalent",
    "antonym",
    "has_subtype",
    "is_a",
    "has_part",
    "part_of",
    "confusable_with",
    "inflection_of",
    "has_inflection",
    "derivationally_related_to",
    "etymologically_derived_from",
    "etymological_source_of",
    "member_of_construction",
    "has_construction_member",
    "scale_contains",
    "member_of_scale",
    "lower_degree_than",
    "higher_degree_than",
  ]
  .into_iter()
  .map(|value| RetrievalRelation::from_wire_name(value).expect("frozen relation registry"))
  .collect()
}

#[cfg(test)]
mod tests {
  use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
  };

  use async_trait::async_trait;
  use time::OffsetDateTime;

  use super::*;
  use crate::{
    domain::{
      assertion::{
        AssertionParticipant, AssertionParticipantValue, AssertionRegistryEntry,
        BinaryTraversalRule, CanonicalAssertion, ParticipantRoleRule, ParticipantValueRule,
      },
      canonical::{
        CanonicalId, CanonicalReleasePin, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
        EvidenceKind, LanguageTag, LexicalSource, ReleaseId, SourcePermissions,
      },
      canonical_content::{CanonicalEvidenceLineage, CanonicalEvidenceOrigin},
      graph::RelationshipVerificationState,
      knowledge_hydration::SelectedBinaryTraversal,
      knowledge_view::{KnowledgePathOutcome, KnowledgeRoot},
      request_context::RequestId,
      retrieval_data::{
        NeighborCandidate, NeighborEdge, NeighborNode, NeighborProjectionExecutionProof,
        RetrievalNodeType,
      },
    },
    ports::canonical_read::{
      CanonicalCandidateQuery, CanonicalSenseQuery, CanonicalTranslationQuery,
    },
  };

  struct FakeRetrieval {
    execution: NeighborProjectionExecutionProof,
    neighbors: BTreeMap<CanonicalId, Vec<NeighborCandidate>>,
    failure: Option<RetrievalDataError>,
    cancel_after_call: Option<Arc<CancellationSignal>>,
  }

  #[async_trait]
  impl RetrievalDataPort for FakeRetrieval {
    async fn search_nodes(
      &self,
      _context: &RequestContext,
      _request: crate::domain::retrieval_data::NodeSearchRequest,
    ) -> Result<crate::domain::retrieval_data::NodeSearchResult, RetrievalDataError> {
      unreachable!()
    }

    async fn search_scales(
      &self,
      _context: &RequestContext,
      _request: crate::domain::retrieval_data::ScaleSearchRequest,
    ) -> Result<crate::domain::retrieval_data::ScaleSearchResult, RetrievalDataError> {
      unreachable!()
    }

    async fn search_edges(
      &self,
      _context: &RequestContext,
      _request: crate::domain::retrieval_data::EdgeSearchRequest,
    ) -> Result<crate::domain::retrieval_data::EdgeSearchResult, RetrievalDataError> {
      unreachable!()
    }

    async fn search_neighbors(
      &self,
      _context: &RequestContext,
      request: NeighborSearchRequest,
    ) -> Result<NeighborSearchResult, RetrievalDataError> {
      if let Some(error) = self.failure {
        return Err(error);
      }
      if let Some(cancellation) = &self.cancel_after_call {
        cancellation.cancel();
      }
      assert_eq!(request.direction, NeighborDirection::Outgoing);
      assert_eq!(
        request.verification_states,
        vec![RetrievalVerificationState::Verified]
      );
      assert_eq!(request.relation_types.len(), 20);
      assert!(request
        .relation_types
        .iter()
        .all(|relation| relation.wire_name() != "associated_with"));
      assert!(request.cursor.is_none());
      Ok(NeighborSearchResult {
        release_id: request.release_id,
        execution: self.execution.clone(),
        root_node_id: request.node_id.clone(),
        neighbors: self
          .neighbors
          .get(&request.node_id)
          .cloned()
          .unwrap_or_default(),
        next_cursor: None,
      })
    }
  }

  struct FakeCanonical {
    release_id: ReleaseId,
    omit: bool,
    omit_roots: bool,
    calls: Mutex<usize>,
  }

  #[async_trait]
  impl CanonicalReadPort for FakeCanonical {
    async fn active_release(
      &self,
      _context: &CanonicalReadContext,
    ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
      unreachable!()
    }

    async fn translations(
      &self,
      _context: &CanonicalReadContext,
      _pin: &CanonicalReleasePin,
      _query: CanonicalTranslationQuery,
    ) -> Result<
      Vec<crate::domain::canonical_translation::CanonicalTranslationRevision>,
      CanonicalReadError,
    > {
      unreachable!()
    }

    async fn candidates(
      &self,
      _context: &CanonicalReadContext,
      _pin: &CanonicalReleasePin,
      _query: CanonicalCandidateQuery,
    ) -> Result<Vec<crate::domain::retrieval::RepositoryMatch>, CanonicalReadError> {
      unreachable!()
    }

    async fn sense(
      &self,
      _context: &CanonicalReadContext,
      _pin: &CanonicalReleasePin,
      _query: CanonicalSenseQuery,
    ) -> Result<crate::domain::canonical_content::CanonicalSenseDetails, CanonicalReadError> {
      unreachable!()
    }

    async fn canonical_assertions(
      &self,
      _context: &CanonicalReadContext,
      pin: &CanonicalReleasePin,
      query: CanonicalAssertionQuery,
    ) -> Result<Vec<HydratedAssertionProjection>, CanonicalReadError> {
      assert_eq!(&self.release_id, &pin.release_id);
      assert_eq!(query.evidence_use, EvidenceUse::ApiRedistribution);
      assert_eq!(query.limit, query.projections.len());
      *self.calls.lock().unwrap() += 1;
      if self.omit {
        return Ok(Vec::new());
      }
      query
        .projections
        .iter()
        .map(|projection| hydrated(&self.release_id, projection))
        .collect()
    }

    async fn knowledge_nodes(
      &self,
      _context: &CanonicalReadContext,
      pin: &CanonicalReleasePin,
      query: CanonicalKnowledgeNodeQuery,
    ) -> Result<Vec<crate::domain::knowledge_hydration::HydratedKnowledgeNode>, CanonicalReadError>
    {
      assert_eq!(&self.release_id, &pin.release_id);
      assert_eq!(query.evidence_use, EvidenceUse::ApiRedistribution);
      if self.omit_roots {
        return Ok(Vec::new());
      }
      Ok(
        query
          .node_ids
          .into_iter()
          .map(
            |node_id| crate::domain::knowledge_hydration::HydratedKnowledgeNode {
              canonical_label: node_id.to_string(),
              node_id,
              revision: 1,
              node_type: RetrievalNodeType::Concept,
              sense_id: None,
              language: None,
              domain_ids: vec![],
              evidence_ids: vec![id("evidence-root")],
            },
          )
          .collect(),
      )
    }
  }

  #[tokio::test]
  async fn returns_deterministic_shortest_directed_paths() {
    let execution = execution();
    let neighbors = BTreeMap::from([
      (
        id("node-a"),
        vec![
          neighbor("edge-a-c", "node-a", "node-c"),
          neighbor("edge-a-b", "node-a", "node-b"),
        ],
      ),
      (
        id("node-b"),
        vec![
          neighbor("edge-b-a-cycle", "node-b", "node-a"),
          neighbor("edge-b-d", "node-b", "node-d"),
        ],
      ),
      (id("node-c"), vec![neighbor("edge-c-d", "node-c", "node-d")]),
    ]);
    let canonical = Arc::new(FakeCanonical {
      release_id: execution.content.release_id.clone(),
      omit: false,
      omit_roots: false,
      calls: Mutex::new(0),
    });
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors,
        failure: None,
        cancel_after_call: None,
      }),
      canonical.clone(),
      execution.clone(),
    );
    let request = request(&execution.content, "node-a", "node-d");
    let result = service
      .find(
        &context(&execution.content),
        &CancellationSignal::default(),
        request,
      )
      .await
      .unwrap();
    let KnowledgePathOutcome::Connected(paths) = result.outcome else {
      panic!("expected connected paths")
    };
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].order, 1);
    assert_eq!(
      paths[0].steps[0].projection().traversal().edge_id,
      id("edge-a-b")
    );
    assert_eq!(
      paths[1].steps[0].projection().traversal().edge_id,
      id("edge-a-c")
    );
    assert_eq!(*canonical.calls.lock().unwrap(), 3);
  }

  #[tokio::test]
  async fn empty_outcome_requires_complete_hydration() {
    let execution = execution();
    let neighbors =
      BTreeMap::from([(id("node-a"), vec![neighbor("edge-a-b", "node-a", "node-b")])]);
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors,
        failure: None,
        cancel_after_call: None,
      }),
      Arc::new(FakeCanonical {
        release_id: execution.content.release_id.clone(),
        omit: true,
        omit_roots: false,
        calls: Mutex::new(0),
      }),
      execution.clone(),
    );
    let result = service
      .find(
        &context(&execution.content),
        &CancellationSignal::default(),
        request(&execution.content, "node-a", "node-z"),
      )
      .await;
    assert_eq!(result, Err(KnowledgePathSearchError::IncompleteSearch));
  }

  #[tokio::test]
  async fn unavailable_search_never_becomes_no_verified_path() {
    let execution = execution();
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors: BTreeMap::new(),
        failure: Some(RetrievalDataError::Unavailable),
        cancel_after_call: None,
      }),
      Arc::new(FakeCanonical {
        release_id: execution.content.release_id.clone(),
        omit: false,
        omit_roots: false,
        calls: Mutex::new(0),
      }),
      execution.clone(),
    );
    let result = service
      .find(
        &context(&execution.content),
        &CancellationSignal::default(),
        request(&execution.content, "node-a", "node-z"),
      )
      .await;
    assert_eq!(result, Err(KnowledgePathSearchError::DependencyUnavailable));
  }

  #[tokio::test]
  async fn complete_empty_graph_returns_no_verified_path() {
    let execution = execution();
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors: BTreeMap::new(),
        failure: None,
        cancel_after_call: None,
      }),
      Arc::new(FakeCanonical {
        release_id: execution.content.release_id.clone(),
        omit: false,
        omit_roots: false,
        calls: Mutex::new(0),
      }),
      execution.clone(),
    );
    let result = service
      .find(
        &context(&execution.content),
        &CancellationSignal::default(),
        request(&execution.content, "node-a", "node-z"),
      )
      .await
      .unwrap();
    assert_eq!(result.outcome, KnowledgePathOutcome::NoVerifiedPath);
  }

  #[tokio::test]
  async fn unknown_root_never_becomes_no_verified_path() {
    let execution = execution();
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors: BTreeMap::new(),
        failure: Some(RetrievalDataError::Unavailable),
        cancel_after_call: None,
      }),
      Arc::new(FakeCanonical {
        release_id: execution.content.release_id.clone(),
        omit: false,
        omit_roots: true,
        calls: Mutex::new(0),
      }),
      execution.clone(),
    );
    let result = service
      .find(
        &context(&execution.content),
        &CancellationSignal::default(),
        request(&execution.content, "node-unknown", "node-z"),
      )
      .await;
    assert_eq!(result, Err(KnowledgePathSearchError::IncompleteSearch));
  }

  #[tokio::test]
  async fn cancellation_after_dependency_await_prevents_success() {
    let execution = execution();
    let cancellation = Arc::new(CancellationSignal::default());
    let service = BoundedKnowledgePathService::new(
      Arc::new(FakeRetrieval {
        execution: proof(&execution),
        neighbors: BTreeMap::new(),
        failure: None,
        cancel_after_call: Some(cancellation.clone()),
      }),
      Arc::new(FakeCanonical {
        release_id: execution.content.release_id.clone(),
        omit: false,
        omit_roots: false,
        calls: Mutex::new(0),
      }),
      execution.clone(),
    );
    let result = service
      .find(
        &context(&execution.content),
        cancellation.as_ref(),
        request(&execution.content, "node-a", "node-z"),
      )
      .await;
    assert_eq!(result, Err(KnowledgePathSearchError::DeadlineExceeded));
  }

  #[test]
  fn useful_path_policy_is_versioned_and_excludes_topical_edges() {
    assert_eq!(KNOWLEDGE_PATH_POLICY_VERSION, 1);
    let relations = eligible_relations();
    assert_eq!(relations.len(), 20);
    assert!(relations
      .iter()
      .all(|relation| relation.wire_name() != "associated_with"));
  }

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn request(pin: &CanonicalReleasePin, from: &str, to: &str) -> KnowledgePathRequest {
    KnowledgePathRequest {
      from: KnowledgeRoot { node: node(from) },
      to: KnowledgeRoot { node: node(to) },
      target_language: LanguageTag::parse("en").unwrap(),
      release: pin.clone(),
    }
  }

  fn node(value: &str) -> CanonicalNodeId {
    CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id(value))
  }

  fn context(pin: &CanonicalReleasePin) -> RequestContext {
    let deadline = OffsetDateTime::now_utc() + time::Duration::seconds(60);
    let deadline = deadline
      .replace_nanosecond(deadline.nanosecond() / 1_000 * 1_000)
      .unwrap();
    RequestContext::new(
      RequestId::new("request-path-1").unwrap(),
      deadline,
      "transnet-v1",
      Some(pin.release_id.clone()),
    )
    .unwrap()
  }

  fn execution() -> NeighborProjectionExecutionExpectation {
    NeighborProjectionExecutionExpectation {
      content: CanonicalReleasePin::new(id("release-1"), "canonical-v1".into()).unwrap(),
      node_collection_id: id("nodes-1"),
      node_collection_content_hash: hash('a'),
      edge_collection_id: id("edges-1"),
      edge_collection_content_hash: hash('b'),
      relationship_registry_version: 1,
      edge_dense_input_version: "edge-dense-input-v1".into(),
      edge_lexical_input_version: "edge-lexical-input-v1".into(),
    }
  }

  fn proof(expected: &NeighborProjectionExecutionExpectation) -> NeighborProjectionExecutionProof {
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

  fn hash(value: char) -> String {
    format!("sha256:{}", value.to_string().repeat(64))
  }

  fn neighbor(edge_id: &str, source: &str, target: &str) -> NeighborCandidate {
    NeighborCandidate {
      edge: NeighborEdge {
        edge_id: id(edge_id),
        source_node_id: id(source),
        target_node_id: id(target),
        relation_type: RetrievalRelation::from_wire_name("associated_with").unwrap(),
        relation_type_id: id("relation-associated"),
        traversal_id: id("traversal-associated"),
        relation_registry_version: 1,
        assertion_id: id(&format!("assertion-{edge_id}")),
        assertion_revision: 1,
        relationship_revision: 1,
        verification_state: RetrievalVerificationState::Verified,
      },
      node: NeighborNode {
        node_id: id(target),
        node_type: RetrievalNodeType::Concept,
        canonical_label: target.into(),
      },
    }
  }

  fn hydrated(
    release_id: &ReleaseId,
    requested: &CanonicalAssertionProjectionRef,
  ) -> Result<HydratedAssertionProjection, CanonicalReadError> {
    let source = node(requested.source_node_id.as_str());
    let target = node(requested.target_node_id.as_str());
    let source_role = id("role-source");
    let target_role = id("role-target");
    let evidence_id = id(&format!("evidence-{}", requested.edge_id));
    let source_id = id("source-reviewed");
    let permissions = SourcePermissions {
      storage: true,
      display: true,
      embedding: false,
      model_processing: false,
      api_redistribution: true,
    };
    let lineage = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Reviewed source".into(),
        version: "2026-01".into(),
        license: "reviewed".into(),
        attribution: Some("Reviewed source".into()),
        permissions,
      },
      EvidenceFragment {
        id: evidence_id.clone(),
        source_id: source_id.clone(),
        source_reference: "entry-1".into(),
        release_id: release_id.clone(),
        language: LanguageTag::parse("en").unwrap(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "Reviewed support.".into(),
        content_hash: "sha256:evidence".into(),
        permissions,
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .map_err(|_| CanonicalReadError::InconsistentData)?;
    let registry = AssertionRegistryEntry {
      relation_type_id: id("relation-associated"),
      registry_revision: requested.relation_registry_revision,
      participant_roles: vec![
        ParticipantRoleRule {
          role_id: source_role.clone(),
          minimum: 1,
          maximum: 1,
          value_rule: ParticipantValueRule::Entity(BTreeSet::from([CanonicalNodeFamily::Concept])),
        },
        ParticipantRoleRule {
          role_id: target_role.clone(),
          minimum: 1,
          maximum: 1,
          value_rule: ParticipantValueRule::Entity(BTreeSet::from([CanonicalNodeFamily::Concept])),
        },
      ],
      resolved_domains: vec![],
      resolved_conditions: vec![],
      binary_traversals: vec![BinaryTraversalRule {
        traversal_id: requested.traversal_id.clone(),
        source_role_id: source_role.clone(),
        target_role_id: target_role.clone(),
        relation_type: requested.relation_type,
      }],
      requires_evidence: true,
    };
    let assertion = CanonicalAssertion {
      assertion_id: requested.assertion_id.clone(),
      assertion_revision: requested.assertion_revision,
      release_id: release_id.clone(),
      relation_type_id: id("relation-associated"),
      relation_registry_revision: requested.relation_registry_revision,
      statement: "Reviewed relationship.".into(),
      participants: vec![
        AssertionParticipant {
          role_id: source_role,
          ordinal: 0,
          value: AssertionParticipantValue::Entity(source.clone()),
        },
        AssertionParticipant {
          role_id: target_role,
          ordinal: 0,
          value: AssertionParticipantValue::Entity(target.clone()),
        },
      ],
      domain_ids: vec![],
      conditions: vec![],
      applicable_sense_ids: vec![],
      evidence_ids: vec![evidence_id],
      evidence_lineage: vec![lineage],
      provenance_ids: vec![source_id],
      verification_state: RelationshipVerificationState::Verified,
    };
    let traversal = SelectedBinaryTraversal {
      edge_id: requested.edge_id.clone(),
      relationship_revision: requested.relationship_revision,
      assertion_id: requested.assertion_id.clone(),
      assertion_revision: requested.assertion_revision,
      traversal_id: requested.traversal_id.clone(),
      source,
      target,
      relation_type: requested.relation_type,
      relation_registry_revision: requested.relation_registry_revision,
    };
    HydratedAssertionProjection::new(
      release_id,
      EvidenceUse::ApiRedistribution,
      requested,
      assertion,
      registry,
      traversal,
    )
    .map_err(|_| CanonicalReadError::InconsistentData)
  }
}
