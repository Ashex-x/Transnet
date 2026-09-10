//! Integration coverage for the pure canonical lookup-card application service.

use std::sync::Arc;

use transnet::{
  adapters::in_memory_retrieval::{InMemoryRetrievalAdapter, InMemoryVectorAvailability},
  application::{
    canonical_lookup_card::CanonicalLookupCardService, retrieval::CanonicalRetrievalService,
  },
  domain::{
    canonical::{
      ActiveContentVersion, CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment,
      EvidenceKind, EvidenceUse, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech, Sense,
      SourcePermissions, WordForm,
    },
    lookup_card::CanonicalLookupCardCoverageState,
    retrieval::{
      CanonicalCandidate, RetrievalRequest, RetrievalScore, VectorMatch, VectorPurpose,
      VectorTarget,
    },
  },
  ports::{canonical_repository::CanonicalRepository, vector_retriever::VectorRetriever},
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
    ranking_version: "rank-v1".to_string(),
  }
}

fn permissions() -> SourcePermissions {
  SourcePermissions {
    storage: true,
    display: true,
    embedding: true,
    model_processing: true,
    api_redistribution: true,
  }
}

fn candidate(sense_id: &str, sense_key: &str) -> CanonicalCandidate {
  let lexeme_id = id(&format!("lexeme-{sense_id}"));
  let evidence_id = id(&format!("evidence-{sense_id}"));
  CanonicalCandidate {
    lexeme: Lexeme {
      id: lexeme_id.clone(),
      release_id: id("release-1"),
      language: language(),
      lemma: "hot".to_string(),
      normalized_lemma: "hot".to_string(),
      part_of_speech: LexicalPartOfSpeech::Adjective,
      status: CanonicalStatus::Active,
    },
    sense: Sense {
      id: id(sense_id),
      lexeme_id: lexeme_id.clone(),
      release_id: id("release-1"),
      sense_key: sense_key.to_string(),
      definition: format!("definition for {sense_key}"),
      definition_evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    },
    forms: vec![WordForm {
      id: id(&format!("form-{sense_id}")),
      lexeme_id,
      release_id: id("release-1"),
      form: "hotter".to_string(),
      normalized_form: "hotter".to_string(),
      kind: FormKind::Inflection,
      morphology: Some("comparative".to_string()),
      evidence_ids: vec![evidence_id.clone()],
      status: CanonicalStatus::Active,
    }],
    evidence: vec![EvidenceFragment {
      id: evidence_id,
      source_id: id("source-licensed"),
      source_reference: format!("source-{sense_key}"),
      release_id: id("release-1"),
      language: language(),
      kind: EvidenceKind::Definition,
      confidence: EvidenceConfidence::High,
      text: format!("licensed evidence for {sense_key}"),
      content_hash: format!("hash-{sense_key}"),
      permissions: permissions(),
      status: CanonicalStatus::Active,
    }],
  }
}

fn service(adapter: InMemoryRetrievalAdapter) -> CanonicalLookupCardService {
  let adapter = Arc::new(adapter);
  let repository: Arc<dyn CanonicalRepository> = adapter.clone();
  let vectors: Arc<dyn VectorRetriever> = adapter;
  CanonicalLookupCardService::new(CanonicalRetrievalService::new(repository, vectors))
}

fn request() -> RetrievalRequest {
  RetrievalRequest::new(" HOT ", language(), EvidenceUse::ApiRedistribution, 4).unwrap()
}

#[tokio::test]
async fn service_preserves_hybrid_ranking_and_assertion_provenance() {
  let warm = candidate("sense-warm", "warmth");
  let heat = candidate("sense-heat", "temperature");
  let vector = VectorMatch {
    target: VectorTarget::Sense(heat.sense.id.clone()),
    score: RetrievalScore::new(9_500).unwrap(),
    release_id: id("release-1"),
    vector_collection_id: id("vectors-1"),
    purpose: VectorPurpose::CanonicalEnglishSense,
    content_language: language(),
  };
  let card = service(
    InMemoryRetrievalAdapter::new(content())
      .with_candidate(warm)
      .with_candidate(heat)
      .with_vector_match(vector),
  )
  .lookup(request())
  .await
  .unwrap();

  assert_eq!(card.query.normalized_query, "hot");
  assert_eq!(card.candidates[0].sense.id.as_str(), "sense-heat");
  assert_eq!(card.candidates[0].rank, 1);
  assert_eq!(card.candidates[0].forms[0].assertion.text, "hotter");
  assert_eq!(
    card.candidates[0]
      .sense
      .definition
      .as_ref()
      .unwrap()
      .evidence[0]
      .provenance
      .source_id
      .as_str(),
    "source-licensed"
  );
  assert_eq!(
    card.coverage.retrieval.state,
    CanonicalLookupCardCoverageState::Available
  );
}

#[tokio::test]
async fn service_returns_lexical_card_when_vector_dependency_is_unavailable() {
  let card = service(
    InMemoryRetrievalAdapter::new(content())
      .with_candidate(candidate("sense-heat", "temperature"))
      .with_vector_availability(InMemoryVectorAvailability::Unavailable),
  )
  .lookup(request())
  .await
  .unwrap();

  assert_eq!(card.candidates.len(), 1);
  assert_eq!(
    card.coverage.retrieval.state,
    CanonicalLookupCardCoverageState::VectorDegraded
  );
  assert!(card.candidates[0].sense.definition.is_some());
}
