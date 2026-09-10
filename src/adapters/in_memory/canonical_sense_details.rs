//! Deterministic in-memory canonical sense-details repository.

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{
  domain::{
    canonical::{ReleaseId, SenseId},
    canonical_content::CanonicalSenseDetails,
  },
  ports::canonical_sense_details_repository::{
    CanonicalSenseDetailsReadRequest, CanonicalSenseDetailsRepository,
    CanonicalSenseDetailsRepositoryError,
  },
};

/// Immutable in-memory canonical sense-detail records for tests and local development.
///
/// Records are keyed by their target's exact release and sense IDs. The adapter is deterministic
/// and does not emulate a database, active-content pointer, authorization, or source-policy
/// workflow; callers must provide an already pinned release in each read request.
#[derive(Debug, Clone, Default)]
pub struct InMemoryCanonicalSenseDetailsRepository {
  details: BTreeMap<(ReleaseId, SenseId), CanonicalSenseDetails>,
}

impl InMemoryCanonicalSenseDetailsRepository {
  /// Creates an empty immutable canonical sense-details repository.
  pub fn new() -> Self {
    Self::default()
  }

  /// Adds or replaces one complete bounded aggregate using its exact target key.
  pub fn with_details(mut self, details: CanonicalSenseDetails) -> Self {
    let target = details.target();
    self.details.insert(
      (target.release_id().clone(), target.sense_id().clone()),
      details,
    );
    self
  }
}

#[async_trait]
impl CanonicalSenseDetailsRepository for InMemoryCanonicalSenseDetailsRepository {
  async fn load(
    &self,
    request: &CanonicalSenseDetailsReadRequest,
  ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError> {
    let Some(details) = self
      .details
      .get(&(request.release_id().clone(), request.sense_id().clone()))
      .cloned()
    else {
      return Ok(None);
    };

    if !details.is_eligible_for(
      request.release_id(),
      request.sense_id(),
      request.evidence_use(),
    ) {
      return Ok(None);
    }

    Ok(Some(details))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    domain::{
      canonical::{
        CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind,
        EvidenceUse, LanguageTag, Lexeme, LexicalPartOfSpeech, LexicalSource, Sense,
        SourcePermissions,
      },
      canonical_content::{
        CanonicalDetailKind, CanonicalEvidenceLineage, CanonicalEvidenceOrigin,
        CanonicalFactualAssertion, CanonicalSenseDetailsInput, LocalizedGloss, SenseContentTarget,
      },
    },
    ports::canonical_sense_details_repository::CanonicalSenseDetailsRepository,
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language(value: &str) -> LanguageTag {
    LanguageTag::parse(value).unwrap()
  }

  fn details(release_id: &str, sense_id: &str, api_redistribution: bool) -> CanonicalSenseDetails {
    let release_id = id(release_id);
    let lexeme = Lexeme {
      id: id("lexeme-run"),
      release_id: release_id.clone(),
      language: language("en-US"),
      lemma: "run".to_string(),
      normalized_lemma: "run".to_string(),
      part_of_speech: LexicalPartOfSpeech::Verb,
      status: CanonicalStatus::Active,
    };
    let sense = Sense {
      id: id(sense_id),
      lexeme_id: lexeme.id.clone(),
      release_id: release_id.clone(),
      sense_key: "run-1".to_string(),
      definition: "move quickly".to_string(),
      definition_evidence_ids: vec![id("definition-evidence")],
      status: CanonicalStatus::Active,
    };
    let target = SenseContentTarget::new(&lexeme, &sense).unwrap();
    let permissions = SourcePermissions {
      storage: true,
      display: true,
      embedding: false,
      model_processing: false,
      api_redistribution,
    };
    let source_id = id("source-1");
    let lineage = CanonicalEvidenceLineage::new(
      LexicalSource {
        id: source_id.clone(),
        name: "Licensed source".to_string(),
        version: "2026.09".to_string(),
        license: "LicenseRef-Test".to_string(),
        attribution: None,
        permissions,
      },
      EvidenceFragment {
        id: id("gloss-evidence"),
        source_id,
        source_reference: "gloss-1".to_string(),
        release_id: release_id.clone(),
        language: language("zh-CN"),
        kind: EvidenceKind::LocalizedGloss,
        confidence: EvidenceConfidence::High,
        text: "跑".to_string(),
        content_hash: "hash-gloss".to_string(),
        permissions,
        status: CanonicalStatus::Active,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap();
    let assertion = CanonicalFactualAssertion::new(
      CanonicalDetailKind::LocalizedGloss,
      release_id,
      CanonicalStatus::Active,
      "跑",
      vec![lineage],
    )
    .unwrap();
    let gloss =
      LocalizedGloss::new(id("gloss-1"), target.clone(), language("zh-CN"), assertion).unwrap();
    CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
      target,
      localized_glosses: vec![gloss],
      pronunciations: Vec::new(),
      usage_labels: Vec::new(),
      grammar_patterns: Vec::new(),
      collocations: Vec::new(),
      examples: Vec::new(),
      pitfalls: Vec::new(),
      etymologies: Vec::new(),
      history: Vec::new(),
    })
    .unwrap()
  }

  #[tokio::test]
  async fn reads_only_the_exact_release_and_sense_key() {
    let repository = InMemoryCanonicalSenseDetailsRepository::new()
      .with_details(details("release-1", "sense-a", true))
      .with_details(details("release-2", "sense-a", true));

    let matching = repository
      .load(&CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-a"),
        EvidenceUse::Display,
      ))
      .await
      .unwrap();
    let missing = repository
      .load(&CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-missing"),
        EvidenceUse::Display,
      ))
      .await
      .unwrap();

    assert_eq!(
      matching.unwrap().target().release_id().as_str(),
      "release-1"
    );
    assert_eq!(missing, None);
  }

  #[tokio::test]
  async fn refuses_an_aggregate_without_the_requested_source_permission() {
    let repository = InMemoryCanonicalSenseDetailsRepository::new().with_details(details(
      "release-1",
      "sense-a",
      false,
    ));

    let result = repository
      .load(&CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-a"),
        EvidenceUse::ApiRedistribution,
      ))
      .await
      .unwrap();

    assert_eq!(result, None);
  }
}
