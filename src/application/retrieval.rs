//! Canonical hybrid retrieval with a deterministic lexical-only fallback.

use std::{
  collections::{BTreeMap, BTreeSet},
  sync::Arc,
};

use thiserror::Error;

use crate::{
  domain::{
    canonical::{ActiveContentVersion, SenseId},
    retrieval::{
      fuse_candidates, CandidateLoadRequest, CanonicalCandidate, HydratedVectorMatch,
      LexicalSearchRequest, RankedCandidate, RetrievalRequest, VectorPurpose, VectorSearchRequest,
    },
  },
  ports::{
    canonical_repository::{CanonicalRepository, CanonicalRepositoryError},
    vector_retriever::{VectorRetriever, VectorRetrieverError},
  },
};

/// Retrieval path used to create a deterministic canonical candidate list.
pub use crate::domain::retrieval::RetrievalPath;

/// Result of canonical retrieval before any model generation or HTTP response assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalOutcome {
  /// Exact compatible lexical and vector versions selected for this retrieval.
  pub content: ActiveContentVersion,
  /// Whether vector signals participated in the ranked output.
  pub path: RetrievalPath,
  /// Bounded, deduplicated, deterministically ranked canonical senses.
  pub candidates: Vec<RankedCandidate>,
}

/// Failure that prevents canonical lexical retrieval from producing a safe fallback.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalRetrievalError {
  /// The required canonical repository could not serve a consistent read.
  #[error(transparent)]
  Repository(#[from] CanonicalRepositoryError),
}

/// Orchestrates canonical lexical and vector retrieval without invoking a learning model.
#[derive(Clone)]
pub struct CanonicalRetrievalService {
  repository: Arc<dyn CanonicalRepository>,
  vector_retriever: Arc<dyn VectorRetriever>,
}

impl CanonicalRetrievalService {
  /// Creates a retrieval service from explicit repository and vector ports.
  pub fn new(
    repository: Arc<dyn CanonicalRepository>,
    vector_retriever: Arc<dyn VectorRetriever>,
  ) -> Self {
    Self {
      repository,
      vector_retriever,
    }
  }

  /// Resolves the immutable content tuple currently selected for new canonical lookups.
  ///
  /// # Errors
  ///
  /// Returns an error when the authoritative canonical repository cannot read its active pointer.
  pub async fn active_content_version(
    &self,
  ) -> Result<ActiveContentVersion, CanonicalRetrievalError> {
    Ok(self.repository.active_content_version().await?)
  }

  /// Retrieves canonical candidates and falls back to lexical-only results when vectors fail.
  ///
  /// The service never calls a model. A repository failure remains an error because it is the
  /// authoritative canonical store; a vector failure is explicitly degraded to deterministic
  /// exact, phrase, lemma, morphology, and full-text results.
  ///
  /// # Errors
  ///
  /// Returns an error when the active content pointer or required lexical repository read fails.
  pub async fn retrieve(
    &self,
    request: RetrievalRequest,
  ) -> Result<RetrievalOutcome, CanonicalRetrievalError> {
    let content = self.active_content_version().await?;
    self.retrieve_with_content(request, content).await
  }

  /// Retrieves candidates against an already selected immutable content tuple.
  ///
  /// This method is intended for callers that must make the content tuple part of an external
  /// consistency boundary, such as a public snapshot cache key. The supplied tuple is used for
  /// every lexical and vector request; it is not replaced by a later active-pointer read.
  ///
  /// # Errors
  ///
  /// Returns an error when a required canonical repository read fails. Vector failures retain the
  /// same lexical-only degradation behavior as [`Self::retrieve`].
  pub async fn retrieve_with_content(
    &self,
    request: RetrievalRequest,
    content: ActiveContentVersion,
  ) -> Result<RetrievalOutcome, CanonicalRetrievalError> {
    let lexical_request = LexicalSearchRequest {
      retrieval: request.clone(),
      content: content.clone(),
    };
    let lexical_matches = self.repository.search_lexical(&lexical_request).await?;

    let (path, vector_matches) = match self.vector_matches(&request, &content).await {
      Ok(matches) => (RetrievalPath::Hybrid, matches),
      Err(VectorStageError::Vector(
        VectorRetrieverError::Unavailable | VectorRetrieverError::InvalidResponse,
      )) => (RetrievalPath::LexicalFallback, Vec::new()),
      Err(VectorStageError::Repository(error)) => return Err(error.into()),
    };
    let mut candidates = fuse_candidates(
      &content,
      request.evidence_use,
      lexical_matches,
      vector_matches,
    );
    candidates.truncate(request.limit);

    Ok(RetrievalOutcome {
      content,
      path,
      candidates,
    })
  }

  async fn vector_matches(
    &self,
    request: &RetrievalRequest,
    content: &ActiveContentVersion,
  ) -> Result<Vec<HydratedVectorMatch>, VectorStageError> {
    let vector_request = vector_request(request, content);
    let vector_matches = self.vector_retriever.search(&vector_request).await?;
    if vector_matches
      .iter()
      .any(|vector_match| !vector_match.matches_request(&vector_request))
    {
      return Err(VectorRetrieverError::InvalidResponse.into());
    }

    let sense_ids = vector_matches
      .iter()
      .filter_map(|vector_match| vector_match.target.sense_id().cloned())
      .collect::<BTreeSet<_>>();
    if sense_ids.is_empty() {
      return Ok(Vec::new());
    }

    let hydrated = self
      .repository
      .load_candidates(&CandidateLoadRequest {
        release_id: content.release_id.clone(),
        evidence_use: request.evidence_use,
        sense_ids: sense_ids.iter().cloned().collect(),
      })
      .await?;
    let candidates_by_sense = canonical_candidates_by_sense(
      hydrated,
      &sense_ids,
      &content.release_id,
      request.evidence_use,
    );

    Ok(
      vector_matches
        .into_iter()
        .filter_map(|vector_match| {
          let sense_id = vector_match.target.sense_id()?.clone();
          let candidate = candidates_by_sense.get(&sense_id)?.clone();
          Some(HydratedVectorMatch {
            candidate,
            vector: vector_match,
          })
        })
        .collect(),
    )
  }
}

#[derive(Debug)]
enum VectorStageError {
  Vector(VectorRetrieverError),
  Repository(CanonicalRepositoryError),
}

impl From<VectorRetrieverError> for VectorStageError {
  fn from(error: VectorRetrieverError) -> Self {
    Self::Vector(error)
  }
}

impl From<CanonicalRepositoryError> for VectorStageError {
  fn from(error: CanonicalRepositoryError) -> Self {
    Self::Repository(error)
  }
}

fn vector_request(
  request: &RetrievalRequest,
  content: &ActiveContentVersion,
) -> VectorSearchRequest {
  let primary_language = request.language.primary_language();
  let is_english = primary_language.as_str() == "en";
  let purpose = if is_english {
    VectorPurpose::CanonicalEnglishSense
  } else {
    VectorPurpose::LocalizedGloss
  };
  VectorSearchRequest {
    query: request.query.clone(),
    release_id: content.release_id.clone(),
    vector_collection_id: content.vector_collection_id.clone(),
    purpose,
    content_language: if is_english {
      primary_language
    } else {
      request.language.clone()
    },
    limit: request.limit,
  }
}

fn canonical_candidates_by_sense(
  hydrated: Vec<CanonicalCandidate>,
  requested_sense_ids: &BTreeSet<SenseId>,
  release_id: &crate::domain::canonical::ReleaseId,
  evidence_use: crate::domain::canonical::EvidenceUse,
) -> BTreeMap<SenseId, CanonicalCandidate> {
  let mut by_sense = BTreeMap::new();
  for candidate in hydrated {
    if !requested_sense_ids.contains(&candidate.sense.id)
      || !candidate.is_eligible_for(release_id, evidence_use)
    {
      continue;
    }
    match by_sense.entry(candidate.sense.id.clone()) {
      std::collections::btree_map::Entry::Occupied(mut entry) => {
        if candidate < *entry.get() {
          entry.insert(candidate);
        }
      }
      std::collections::btree_map::Entry::Vacant(entry) => {
        entry.insert(candidate);
      }
    }
  }
  by_sense
}
