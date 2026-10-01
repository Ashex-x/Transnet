//! Canonical-root-first, bounded hybrid nomination and authoritative hydration.

use std::{
  collections::{BTreeMap, BTreeSet},
  sync::Arc,
};

use thiserror::Error;

use crate::{
  domain::{
    canonical::{CanonicalId, EvidenceUse},
    model_runtime::{CancellationSignal, EmbeddingInput},
    request_context::RequestContext,
    retrieval_data::{
      DenseQueryVector, NodeCandidate, NodeMatchMechanism, NodeSearchRequest, RetrievalFilters,
      RetrievalNodeType, RetrievalVerificationState,
    },
    root_retrieval::{
      CanonicalRoot, CanonicalRootResolution, DenseQueryExecutionReceipt, RankedRootItem,
      RootEvidenceState, RootQueryExecutionReceipts, RootRetrievalCoverage,
      RootRetrievalExecutionSpec, RootRetrievalOutcome, RootRetrievalRequest, RootRetrievalResult,
      MAX_ROOT_LABEL_BYTES,
    },
  },
  ports::{
    canonical_read::{
      CanonicalKnowledgeNodeQuery, CanonicalReadContext, CanonicalReadError, CanonicalReadPort,
    },
    model_runtime::{EmbeddingPort, EmbeddingRequest, ModelOperationContext, ModelOperationError},
    retrieval_data::{RetrievalDataError, RetrievalDataPort},
    root_retrieval::{
      CanonicalRootError, CanonicalRootPort, QueryLexicalEncoderError, QueryLexicalEncoderPort,
    },
  },
};

/// Closed failures that prevent a trustworthy root result.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RootRetrievalError {
  /// Canonical-data could not establish a valid root or hydration result.
  #[error("canonical root retrieval failed")]
  Canonical,
  /// A caller pin contradicted the resolved canonical release.
  #[error("root retrieval release is inconsistent")]
  ReleaseMismatch,
  /// Signal generation returned malformed data.
  #[error("root retrieval signal is invalid")]
  InvalidSignal,
  /// Retrieval-data returned malformed, stale, or ineligible data.
  #[error("root retrieval projection is inconsistent")]
  InconsistentProjection,
  /// A required operation exhausted the shared deadline.
  #[error("root retrieval deadline exceeded")]
  DeadlineExceeded,
}

/// Root-first retrieval composition over canonical, model, lexical, and retrieval-data ports.
pub struct BoundedRootRetrievalService {
  canonical: Arc<dyn CanonicalRootPort>,
  canonical_hydration: Arc<dyn CanonicalReadPort>,
  embeddings: Arc<dyn EmbeddingPort>,
  lexical: Arc<dyn QueryLexicalEncoderPort>,
  retrieval: Arc<dyn RetrievalDataPort>,
  execution: RootRetrievalExecutionSpec,
}

impl BoundedRootRetrievalService {
  /// Creates a service whose dependencies retain no request query or vectors.
  pub fn new(
    canonical: Arc<dyn CanonicalRootPort>,
    canonical_hydration: Arc<dyn CanonicalReadPort>,
    embeddings: Arc<dyn EmbeddingPort>,
    lexical: Arc<dyn QueryLexicalEncoderPort>,
    retrieval: Arc<dyn RetrievalDataPort>,
    execution: RootRetrievalExecutionSpec,
  ) -> Self {
    Self {
      canonical,
      canonical_hydration,
      embeddings,
      lexical,
      retrieval,
      execution,
    }
  }

  /// Resolves one canonical root before creating retrieval signals or contacting retrieval-data.
  ///
  /// Vector and sparse failures degrade to an explicit canonical-only result. Every returned
  /// nomination is rehydrated from canonical-data; similarity never establishes a fact.
  ///
  /// # Errors
  ///
  /// Returns a content-free error for contradictory releases, malformed signals, deadline
  /// exhaustion, or projection/canonical inconsistency.
  pub async fn retrieve(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    request: RootRetrievalRequest,
  ) -> Result<RootRetrievalResult, RootRetrievalError> {
    ensure_active(context, cancellation)?;
    let resolution = self
      .canonical
      .resolve_root(
        context,
        &request.query,
        &request.source_language,
        &request.explanation_language,
      )
      .await
      .map_err(map_canonical_error)?;
    let root = match resolution {
      CanonicalRootResolution::NotFound => return Ok(RootRetrievalResult::NotFound),
      CanonicalRootResolution::Ambiguous(candidates) => {
        validate_ambiguous_roots(context, &self.execution, &candidates)?;
        return Ok(RootRetrievalResult::Ambiguous(candidates));
      }
      CanonicalRootResolution::Resolved(root) => root,
    };
    validate_canonical_root(&root)?;
    if context
      .content_release()
      .is_some_and(|release| release != &root.content.release_id)
      || self.execution.content() != &root.content
    {
      return Err(RootRetrievalError::ReleaseMismatch);
    }
    let retrieval_context = context
      .clone()
      .with_content_release(root.content.release_id.clone())
      .map_err(|_| RootRetrievalError::ReleaseMismatch)?;
    let root_item = RankedRootItem {
      node_id: root.node_id.clone(),
      revision: root.revision,
      node_type: root.node_family,
      sense_id: Some(root.sense_id.clone()),
      canonical_label: root.canonical_label.clone(),
      evidence_ids: root.evidence_ids.clone(),
      evidence_state: RootEvidenceState::Verified,
      rank: 1,
      ranking_basis_points: 10_000,
      matched_by: vec![NodeMatchMechanism::CanonicalLabel],
    };

    ensure_active(&retrieval_context, cancellation)?;
    let dense = match self
      .embeddings
      .embed(
        ModelOperationContext {
          request: &retrieval_context,
          cancellation,
        },
        EmbeddingRequest {
          input: EmbeddingInput::new(request.query.as_str())
            .map_err(|_| RootRetrievalError::InvalidSignal)?,
        },
      )
      .await
    {
      Ok(value) => value,
      Err(ModelOperationError::Unavailable) => {
        return Ok(canonical_only(root.content.release_id, root_item));
      }
      Err(ModelOperationError::DeadlineExceeded | ModelOperationError::Cancelled) => {
        return Err(RootRetrievalError::DeadlineExceeded);
      }
      Err(ModelOperationError::InvalidOutput) => return Err(RootRetrievalError::InvalidSignal),
    };
    if dense.model_version() != self.execution.dense_artifact_revision()
      || dense.values().len() != self.execution.dense_dimensions()
    {
      return Err(RootRetrievalError::InvalidSignal);
    }
    let dense_vector = DenseQueryVector::new(dense.values().to_vec())
      .map_err(|_| RootRetrievalError::InvalidSignal)?;
    let sparse = match self
      .lexical
      .encode(&retrieval_context, &root.content.release_id, &request.query)
      .await
    {
      Ok(value) => value,
      Err(QueryLexicalEncoderError::Unavailable) => {
        return Ok(canonical_only(root.content.release_id, root_item));
      }
      Err(QueryLexicalEncoderError::Timeout) => {
        return Err(RootRetrievalError::DeadlineExceeded);
      }
      Err(QueryLexicalEncoderError::InvalidEncoding) => {
        return Err(RootRetrievalError::InvalidSignal);
      }
    };
    if sparse.receipt.release_id != root.content.release_id
      || sparse.receipt.encoder_identity != self.execution.sparse_encoder_identity()
      || sparse.receipt.encoder_revision != self.execution.sparse_encoder_revision()
      || sparse.receipt.input_version != self.execution.sparse_input_version()
    {
      return Err(RootRetrievalError::InvalidSignal);
    }
    let mut filters = RetrievalFilters::verified(root.content.release_id.clone());
    filters.languages.push(request.source_language);
    let search = NodeSearchRequest {
      dense_vector,
      sparse_vector: sparse.vector,
      execution: self.execution.projection_expectation(),
      filters,
      limit: request.limit,
    };
    let result = match self
      .retrieval
      .search_nodes(&retrieval_context, search)
      .await
    {
      Ok(value) => value,
      Err(RetrievalDataError::Unavailable) => {
        return Ok(canonical_only(root.content.release_id, root_item));
      }
      Err(RetrievalDataError::Timeout) => return Err(RootRetrievalError::DeadlineExceeded),
      Err(_) => return Err(RootRetrievalError::InconsistentProjection),
    };
    if result.release_id != root.content.release_id || result.candidates.len() > request.limit {
      return Err(RootRetrievalError::InconsistentProjection);
    }
    if result.execution.collection_id != *self.execution.node_collection_id()
      || result.execution.collection_content_hash != self.execution.node_collection_content_hash()
      || result
        .execution
        .validate_against(&self.execution.projection_expectation())
        .is_err()
    {
      return Err(RootRetrievalError::InconsistentProjection);
    }

    let candidates = validate_and_deduplicate(result.candidates, &root.node_id)?;
    let node_ids = candidates
      .iter()
      .map(|candidate| candidate.node_id.clone())
      .collect::<Vec<_>>();
    let canonical_context = CanonicalReadContext {
      request_id: retrieval_context.request_id().as_str().to_string(),
      deadline_at: retrieval_context.deadline_rfc3339(),
      timeout: retrieval_context.remaining_budget(),
    };
    let hydrated = self
      .canonical_hydration
      .knowledge_nodes(
        &canonical_context,
        &root.content,
        CanonicalKnowledgeNodeQuery {
          node_ids: node_ids.clone(),
          evidence_use: EvidenceUse::ApiRedistribution,
          limit: request.limit,
        },
      )
      .await
      .map_err(map_canonical_read_error)?;
    if hydrated
      .iter()
      .any(|node| !node_ids.contains(&node.node_id))
    {
      return Err(RootRetrievalError::InconsistentProjection);
    }
    let mut hydrated_by_id = BTreeMap::new();
    for node in hydrated {
      if hydrated_by_id.insert(node.node_id.clone(), node).is_some() {
        return Err(RootRetrievalError::InconsistentProjection);
      }
    }
    for candidate in &candidates {
      if let Some(node) = hydrated_by_id.get(&candidate.node_id) {
        if candidate.payload.node_type != node.node_type
          || candidate.payload.sense_id.as_ref() != node.sense_id.as_ref()
          || candidate.payload.canonical_label != node.canonical_label
          || !valid_label(&node.canonical_label)
          || (node.node_type == RetrievalNodeType::LexicalSense && node.sense_id.is_none())
          || node.validate().is_err()
        {
          return Err(RootRetrievalError::InconsistentProjection);
        }
      }
    }
    let coverage = if hydrated_by_id.len() == node_ids.len() {
      RootRetrievalCoverage::Complete
    } else {
      RootRetrievalCoverage::PartialPublication
    };
    let mut exploratory = candidates
      .into_iter()
      .filter_map(|candidate| {
        let hydrated = hydrated_by_id.get(&candidate.node_id)?;
        Some(RankedRootItem {
          node_id: hydrated.node_id.clone(),
          revision: hydrated.revision,
          node_type: hydrated.node_type,
          sense_id: hydrated.sense_id.clone(),
          canonical_label: hydrated.canonical_label.clone(),
          evidence_ids: hydrated.evidence_ids.clone(),
          evidence_state: RootEvidenceState::Exploratory,
          rank: 0,
          ranking_basis_points: ranking_score(&candidate),
          matched_by: sorted_mechanisms(&candidate.matched_by),
        })
      })
      .collect::<Vec<_>>();
    exploratory.sort_by(|left, right| {
      right
        .ranking_basis_points
        .cmp(&left.ranking_basis_points)
        .then_with(|| left.node_id.cmp(&right.node_id))
    });
    for (index, item) in exploratory.iter_mut().enumerate() {
      item.rank = index + 1;
    }

    Ok(RootRetrievalResult::Resolved(Box::new(
      RootRetrievalOutcome {
        release_id: root.content.release_id.clone(),
        root: root_item,
        inferred: Vec::new(),
        exploratory,
        coverage,
        execution_receipts: Some(RootQueryExecutionReceipts {
          dense: DenseQueryExecutionReceipt {
            release_id: root.content.release_id.clone(),
            artifact_revision: dense.model_version().as_str().to_string(),
            dimensions: dense.values().len(),
          },
          sparse: sparse.receipt,
        }),
      },
    )))
  }
}

fn canonical_only(release_id: CanonicalId, root: RankedRootItem) -> RootRetrievalResult {
  RootRetrievalResult::Resolved(Box::new(RootRetrievalOutcome {
    release_id,
    root,
    inferred: Vec::new(),
    exploratory: Vec::new(),
    coverage: RootRetrievalCoverage::CanonicalOnly,
    execution_receipts: None,
  }))
}

fn ensure_active(
  context: &RequestContext,
  cancellation: &CancellationSignal,
) -> Result<(), RootRetrievalError> {
  if cancellation.is_cancelled() || context.remaining_budget().is_zero() {
    Err(RootRetrievalError::DeadlineExceeded)
  } else {
    Ok(())
  }
}

fn map_canonical_error(error: CanonicalRootError) -> RootRetrievalError {
  match error {
    CanonicalRootError::Timeout => RootRetrievalError::DeadlineExceeded,
    CanonicalRootError::InconsistentData => RootRetrievalError::InconsistentProjection,
    _ => RootRetrievalError::Canonical,
  }
}

fn map_canonical_read_error(error: CanonicalReadError) -> RootRetrievalError {
  match error {
    CanonicalReadError::Timeout => RootRetrievalError::DeadlineExceeded,
    CanonicalReadError::InconsistentData => RootRetrievalError::InconsistentProjection,
    _ => RootRetrievalError::Canonical,
  }
}

fn validate_and_deduplicate(
  candidates: Vec<NodeCandidate>,
  root_id: &CanonicalId,
) -> Result<Vec<NodeCandidate>, RootRetrievalError> {
  let mut by_id = BTreeMap::new();
  for candidate in candidates {
    if &candidate.node_id == root_id {
      continue;
    }
    let mechanisms = candidate
      .matched_by
      .iter()
      .copied()
      .collect::<BTreeSet<_>>();
    if candidate.payload.verification_state != RetrievalVerificationState::Verified
      || candidate.matched_by.is_empty()
      || mechanisms.len() != candidate.matched_by.len()
      || !valid_label(&candidate.payload.canonical_label)
      || (candidate.payload.node_type == RetrievalNodeType::LexicalSense
        && candidate.payload.sense_id.is_none())
    {
      return Err(RootRetrievalError::InconsistentProjection);
    }
    match by_id.get(&candidate.node_id) {
      Some(existing) if existing != &candidate => {
        return Err(RootRetrievalError::InconsistentProjection);
      }
      Some(_) => {}
      None => {
        by_id.insert(candidate.node_id.clone(), candidate);
      }
    }
  }
  Ok(by_id.into_values().collect())
}

fn ranking_score(candidate: &NodeCandidate) -> u16 {
  let similarity = (candidate.score.get() * 9_000.0).round() as u16;
  let mechanism_bonus = u16::try_from(
    candidate
      .matched_by
      .iter()
      .copied()
      .collect::<BTreeSet<_>>()
      .len(),
  )
  .unwrap_or(0)
  .min(4)
    * 250;
  similarity.saturating_add(mechanism_bonus).min(9_999)
}

fn sorted_mechanisms(values: &[NodeMatchMechanism]) -> Vec<NodeMatchMechanism> {
  let mut values = values.to_vec();
  values.sort();
  values.dedup();
  values
}

fn validate_canonical_root(root: &CanonicalRoot) -> Result<(), RootRetrievalError> {
  if root.node_family != RetrievalNodeType::LexicalSense
    || root.revision == 0
    || !valid_label(&root.canonical_label)
    || root.evidence_ids.is_empty()
    || root.evidence_ids.windows(2).any(|pair| pair[0] >= pair[1])
  {
    return Err(RootRetrievalError::Canonical);
  }
  Ok(())
}

fn validate_ambiguous_roots(
  context: &RequestContext,
  execution: &RootRetrievalExecutionSpec,
  candidates: &[CanonicalRoot],
) -> Result<(), RootRetrievalError> {
  if candidates.len() < 2 {
    return Err(RootRetrievalError::Canonical);
  }
  let mut node_ids = BTreeSet::new();
  let mut sense_ids = BTreeSet::new();
  for candidate in candidates {
    validate_canonical_root(candidate)?;
    if candidate.content != *execution.content()
      || context
        .content_release()
        .is_some_and(|release| release != &candidate.content.release_id)
    {
      return Err(RootRetrievalError::ReleaseMismatch);
    }
    if !node_ids.insert(candidate.node_id.clone()) || !sense_ids.insert(candidate.sense_id.clone())
    {
      return Err(RootRetrievalError::Canonical);
    }
  }
  Ok(())
}

fn valid_label(value: &str) -> bool {
  !value.trim().is_empty()
    && value.len() <= MAX_ROOT_LABEL_BYTES
    && !value.chars().any(char::is_control)
}
