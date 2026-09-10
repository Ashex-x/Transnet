//! Deterministic in-memory canonical repository and vector adapter for tests.

use std::collections::BTreeSet;

use async_trait::async_trait;

use crate::{
  domain::{
    canonical::ActiveContentVersion,
    retrieval::{
      CandidateLoadRequest, CanonicalCandidate, LexicalSearchRequest, RepositoryMatch, VectorMatch,
      VectorSearchRequest,
    },
  },
  ports::{
    canonical_repository::{CanonicalRepository, CanonicalRepositoryError},
    vector_retriever::{VectorRetriever, VectorRetrieverError},
  },
};

/// Behavior used by the in-memory vector port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InMemoryVectorAvailability {
  /// Return configured vector records after applying all required metadata filters.
  Available,
  /// Return the typed dependency failure used to exercise lexical-only fallback.
  Unavailable,
}

/// Test adapter that implements both canonical repository and vector retrieval ports.
///
/// It is intentionally deterministic: canonical candidates are searched through normalized lemma
/// and form keys, while vector records are preloaded and filtered by their full request metadata.
/// Production storage adapters remain separate infrastructure work.
#[derive(Debug, Clone)]
pub struct InMemoryRetrievalAdapter {
  active_content: ActiveContentVersion,
  candidates: Vec<CanonicalCandidate>,
  vector_matches: Vec<VectorMatch>,
  vector_availability: InMemoryVectorAvailability,
}

impl InMemoryRetrievalAdapter {
  /// Creates an empty adapter pinned to one active compatible content version.
  pub fn new(active_content: ActiveContentVersion) -> Self {
    Self {
      active_content,
      candidates: Vec::new(),
      vector_matches: Vec::new(),
      vector_availability: InMemoryVectorAvailability::Available,
    }
  }

  /// Adds a canonical candidate available to lexical searches and vector hydration.
  pub fn with_candidate(mut self, candidate: CanonicalCandidate) -> Self {
    self.candidates.push(candidate);
    self
  }

  /// Adds one precomputed vector record to the immutable in-memory collection.
  pub fn with_vector_match(mut self, vector_match: VectorMatch) -> Self {
    self.vector_matches.push(vector_match);
    self
  }

  /// Sets whether vector reads succeed or exercise the typed fallback path.
  pub fn with_vector_availability(mut self, availability: InMemoryVectorAvailability) -> Self {
    self.vector_availability = availability;
    self
  }
}

#[async_trait]
impl CanonicalRepository for InMemoryRetrievalAdapter {
  async fn active_content_version(&self) -> Result<ActiveContentVersion, CanonicalRepositoryError> {
    Ok(self.active_content.clone())
  }

  async fn search_lexical(
    &self,
    request: &LexicalSearchRequest,
  ) -> Result<Vec<RepositoryMatch>, CanonicalRepositoryError> {
    let mut matches = self
      .candidates
      .iter()
      .filter(|candidate| {
        candidate.lexeme.language == request.retrieval.language
          && candidate.is_eligible_for(&request.content.release_id, request.retrieval.evidence_use)
      })
      .filter_map(|candidate| {
        candidate
          .in_memory_match(&request.retrieval.query)
          .map(|(kind, score)| RepositoryMatch {
            candidate: candidate.clone(),
            kind,
            score,
          })
      })
      .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
      right
        .score
        .cmp(&left.score)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left.candidate.sense.id.cmp(&right.candidate.sense.id))
    });
    matches.truncate(request.retrieval.limit);
    Ok(matches)
  }

  async fn load_candidates(
    &self,
    request: &CandidateLoadRequest,
  ) -> Result<Vec<CanonicalCandidate>, CanonicalRepositoryError> {
    let requested_sense_ids = request.sense_ids.iter().collect::<BTreeSet<_>>();
    let mut candidates = self
      .candidates
      .iter()
      .filter(|candidate| {
        requested_sense_ids.contains(&candidate.sense.id)
          && candidate.is_eligible_for(&request.release_id, request.evidence_use)
      })
      .cloned()
      .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.sense.id.cmp(&right.sense.id));
    Ok(candidates)
  }
}

#[async_trait]
impl VectorRetriever for InMemoryRetrievalAdapter {
  async fn search(
    &self,
    request: &VectorSearchRequest,
  ) -> Result<Vec<VectorMatch>, VectorRetrieverError> {
    if self.vector_availability == InMemoryVectorAvailability::Unavailable {
      return Err(VectorRetrieverError::Unavailable);
    }

    let mut matches = self
      .vector_matches
      .iter()
      .filter(|vector_match| vector_match.matches_request(request))
      .cloned()
      .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
      right
        .score
        .cmp(&left.score)
        .then_with(|| left.target.cmp(&right.target))
    });
    matches.truncate(request.limit);
    Ok(matches)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use crate::{
    application::retrieval::{CanonicalRetrievalService, RetrievalPath},
    domain::{
      canonical::{
        CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind,
        EvidenceUse, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech, Sense, SourcePermissions,
        WordForm,
      },
      retrieval::{
        RetrievalRequest, RetrievalScore, VectorPurpose, VectorSearchRequest, VectorTarget,
      },
    },
    ports::vector_retriever::VectorRetriever,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language() -> LanguageTag {
    LanguageTag::parse("en").unwrap()
  }

  fn content() -> ActiveContentVersion {
    ActiveContentVersion {
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "lookup-rank-v1".to_string(),
    }
  }

  fn candidate() -> CanonicalCandidate {
    let release_id = id("release-1");
    let lexeme_id = id("lexeme-hot");
    let evidence_id = id("evidence-hot");
    CanonicalCandidate {
      lexeme: Lexeme {
        id: lexeme_id.clone(),
        release_id: release_id.clone(),
        language: language(),
        lemma: "hot".to_string(),
        normalized_lemma: "hot".to_string(),
        part_of_speech: LexicalPartOfSpeech::Adjective,
        status: CanonicalStatus::Active,
      },
      sense: Sense {
        id: id("sense-hot"),
        lexeme_id: lexeme_id.clone(),
        release_id: release_id.clone(),
        sense_key: "temperature".to_string(),
        definition: "having a high temperature".to_string(),
        definition_evidence_ids: vec![evidence_id.clone()],
        status: CanonicalStatus::Active,
      },
      forms: vec![WordForm {
        id: id("form-hotter"),
        lexeme_id,
        release_id: release_id.clone(),
        form: "hotter".to_string(),
        normalized_form: "hotter".to_string(),
        kind: FormKind::Inflection,
        morphology: Some("comparative".to_string()),
        evidence_ids: vec![evidence_id.clone()],
        status: CanonicalStatus::Active,
      }],
      evidence: vec![EvidenceFragment {
        id: evidence_id,
        source_id: id("source-1"),
        source_reference: "definition-1".to_string(),
        release_id,
        language: language(),
        kind: EvidenceKind::Definition,
        confidence: EvidenceConfidence::High,
        text: "having a high temperature".to_string(),
        content_hash: "hash-1".to_string(),
        permissions: SourcePermissions {
          storage: true,
          display: true,
          embedding: true,
          model_processing: true,
          api_redistribution: true,
        },
        status: CanonicalStatus::Active,
      }],
    }
  }

  fn matching_vector() -> VectorMatch {
    VectorMatch {
      target: VectorTarget::Sense(id("sense-hot")),
      score: RetrievalScore::new(9_200).unwrap(),
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      purpose: VectorPurpose::CanonicalEnglishSense,
      content_language: language(),
    }
  }

  async fn retrieve(
    adapter: InMemoryRetrievalAdapter,
  ) -> crate::application::retrieval::RetrievalOutcome {
    let adapter = Arc::new(adapter);
    let repository: Arc<dyn CanonicalRepository> = adapter.clone();
    let vector_retriever: Arc<dyn VectorRetriever> = adapter;
    CanonicalRetrievalService::new(repository, vector_retriever)
      .retrieve(RetrievalRequest::for_public_api("hotter", language()).unwrap())
      .await
      .unwrap()
  }

  #[tokio::test]
  async fn adapter_supports_deduplicated_hybrid_retrieval() {
    let outcome = retrieve(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_match(matching_vector()),
    )
    .await;

    assert_eq!(outcome.path, RetrievalPath::Hybrid);
    assert_eq!(outcome.candidates.len(), 1);
    assert_eq!(
      outcome.candidates[0].candidate.sense.id.as_str(),
      "sense-hot"
    );
    assert_eq!(
      outcome.candidates[0]
        .features
        .morphology
        .unwrap()
        .basis_points(),
      7_500
    );
    assert_eq!(
      outcome.candidates[0]
        .features
        .vector_similarity
        .unwrap()
        .basis_points(),
      9_200
    );
  }

  #[tokio::test]
  async fn vector_unavailability_preserves_lexical_only_results() {
    let outcome = retrieve(
      InMemoryRetrievalAdapter::new(content())
        .with_candidate(candidate())
        .with_vector_availability(InMemoryVectorAvailability::Unavailable),
    )
    .await;

    assert_eq!(outcome.path, RetrievalPath::LexicalFallback);
    assert_eq!(outcome.candidates.len(), 1);
    assert_eq!(outcome.candidates[0].features.vector_similarity, None);
  }

  #[tokio::test]
  async fn vector_adapter_applies_release_and_metadata_filters() {
    let adapter = InMemoryRetrievalAdapter::new(content()).with_vector_match(VectorMatch {
      vector_collection_id: id("vectors-other"),
      ..matching_vector()
    });
    let request = VectorSearchRequest {
      query: "hotter".to_string(),
      release_id: id("release-1"),
      vector_collection_id: id("vectors-1"),
      purpose: VectorPurpose::CanonicalEnglishSense,
      content_language: language(),
      limit: 4,
    };

    let matches = adapter.search(&request).await.unwrap();

    assert!(matches.is_empty());
  }

  #[test]
  fn request_uses_api_permission_and_normalizes_text() {
    let request = RetrievalRequest::for_public_api("  HOTTER  ", language()).unwrap();

    assert_eq!(request.query, "hotter");
    assert_eq!(request.evidence_use, EvidenceUse::ApiRedistribution);
  }
}
