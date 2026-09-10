//! Release-pinned reads of bounded canonical sense details.
//!
//! This application service composes only the canonical sense-details port and the reviewed
//! domain aggregate. It does not choose an active content release, expose HTTP, or persist data.

use std::sync::Arc;

use thiserror::Error;

use crate::{
  domain::canonical_content::CanonicalSenseDetails,
  ports::canonical_sense_details_repository::{
    CanonicalSenseDetailsReadRequest, CanonicalSenseDetailsRepository,
    CanonicalSenseDetailsRepositoryError,
  },
};

/// Failure while loading a release-pinned canonical sense-details aggregate.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalSenseDetailsReadError {
  /// The canonical detail repository could not satisfy the pinned read contract.
  #[error(transparent)]
  Repository(#[from] CanonicalSenseDetailsRepositoryError),
}

/// Loads one complete, bounded canonical detail aggregate through an explicit read port.
///
/// The service does not use the existing lexical-search repository because sense-detail loading
/// has a distinct release-pinning and evidence-permission contract. It repeats the port's target,
/// lifecycle, and evidence checks so an adapter defect cannot expose an unrelated or forbidden
/// detail aggregate.
#[derive(Clone)]
pub struct CanonicalSenseDetailsService {
  repository: Arc<dyn CanonicalSenseDetailsRepository>,
}

impl CanonicalSenseDetailsService {
  /// Creates a canonical sense-details service from an explicit read-only repository port.
  pub fn new(repository: Arc<dyn CanonicalSenseDetailsRepository>) -> Self {
    Self { repository }
  }

  /// Loads one complete bounded aggregate pinned to the requested release and sense.
  ///
  /// An absent or no-longer-servable aggregate returns `Ok(None)`. A repository response for a
  /// different release or sense is inconsistent data and returns an error; no partial aggregate
  /// is exposed when any assertion fails the requested permission operation.
  ///
  /// # Errors
  ///
  /// Returns an error when the repository is unavailable or violates the exact pinned-target
  /// contract.
  pub async fn read(
    &self,
    request: CanonicalSenseDetailsReadRequest,
  ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsReadError> {
    let Some(details) = self.repository.load(&request).await? else {
      return Ok(None);
    };

    if details.target().release_id() != request.release_id()
      || details.target().sense_id() != request.sense_id()
    {
      return Err(CanonicalSenseDetailsRepositoryError::InconsistentData.into());
    }
    if !details.is_eligible_for(
      request.release_id(),
      request.sense_id(),
      request.evidence_use(),
    ) {
      tracing::warn!("discarded ineligible canonical sense details from repository");
      return Ok(None);
    }

    Ok(Some(details))
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use async_trait::async_trait;

  use super::*;
  use crate::{
    adapters::in_memory::InMemoryCanonicalSenseDetailsRepository,
    domain::{
      canonical::{
        CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind,
        EvidenceUse, LanguageTag, Lexeme, LexicalPartOfSpeech, LexicalSource, Sense,
        SourcePermissions,
      },
      canonical_content::{
        CanonicalDetailKind, CanonicalEvidenceLineage, CanonicalEvidenceOrigin,
        CanonicalFactualAssertion, CanonicalSenseDetails, CanonicalSenseDetailsInput,
        LocalizedGloss, SenseContentTarget,
      },
    },
    ports::canonical_sense_details_repository::{
      CanonicalSenseDetailsReadRequest, CanonicalSenseDetailsRepository,
      CanonicalSenseDetailsRepositoryError,
    },
  };

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn language(value: &str) -> LanguageTag {
    LanguageTag::parse(value).unwrap()
  }

  fn permissions(api_redistribution: bool) -> SourcePermissions {
    SourcePermissions {
      storage: true,
      display: true,
      embedding: false,
      model_processing: false,
      api_redistribution,
    }
  }

  fn details(
    release_id: &str,
    sense_id: &str,
    status: CanonicalStatus,
    api_redistribution: bool,
  ) -> CanonicalSenseDetails {
    let release_id = id(release_id);
    let lexeme = Lexeme {
      id: id("lexeme-run"),
      release_id: release_id.clone(),
      language: language("en-US"),
      lemma: "run".to_string(),
      normalized_lemma: "run".to_string(),
      part_of_speech: LexicalPartOfSpeech::Verb,
      status,
    };
    let sense = Sense {
      id: id(sense_id),
      lexeme_id: lexeme.id.clone(),
      release_id: release_id.clone(),
      sense_key: "run-1".to_string(),
      definition: "move quickly".to_string(),
      definition_evidence_ids: vec![id("definition-evidence")],
      status,
    };
    let target = SenseContentTarget::new(&lexeme, &sense).unwrap();
    let source_id = id("source-1");
    let permissions = permissions(api_redistribution);
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
        status,
      },
      CanonicalEvidenceOrigin::LicensedSource,
    )
    .unwrap();
    let assertion = CanonicalFactualAssertion::new(
      CanonicalDetailKind::LocalizedGloss,
      release_id,
      status,
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
  async fn reads_one_complete_release_pinned_aggregate() {
    let repository = Arc::new(InMemoryCanonicalSenseDetailsRepository::new().with_details(
      details("release-1", "sense-run", CanonicalStatus::Active, true),
    ));
    let service = CanonicalSenseDetailsService::new(repository);

    let result = service
      .read(CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-run"),
        EvidenceUse::ApiRedistribution,
      ))
      .await
      .unwrap();

    assert_eq!(result.unwrap().localized_glosses().len(), 1);
  }

  #[tokio::test]
  async fn returns_no_aggregate_when_source_permission_or_status_is_not_servable() {
    let permission_repository = Arc::new(
      InMemoryCanonicalSenseDetailsRepository::new().with_details(details(
        "release-1",
        "sense-run",
        CanonicalStatus::Active,
        false,
      )),
    );
    let permission_service = CanonicalSenseDetailsService::new(permission_repository);
    let permission_result = permission_service
      .read(CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-run"),
        EvidenceUse::ApiRedistribution,
      ))
      .await
      .unwrap();

    let inactive_repository = Arc::new(
      InMemoryCanonicalSenseDetailsRepository::new().with_details(details(
        "release-1",
        "sense-run",
        CanonicalStatus::Draft,
        true,
      )),
    );
    let inactive_service = CanonicalSenseDetailsService::new(inactive_repository);
    let inactive_result = inactive_service
      .read(CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-run"),
        EvidenceUse::Display,
      ))
      .await
      .unwrap();

    assert_eq!(permission_result, None);
    assert_eq!(inactive_result, None);
  }

  #[tokio::test]
  async fn defensively_hides_an_unservable_aggregate_returned_by_a_repository() {
    let service = CanonicalSenseDetailsService::new(Arc::new(UnfilteredRepository(details(
      "release-1",
      "sense-run",
      CanonicalStatus::Active,
      false,
    ))));

    let result = service
      .read(CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-run"),
        EvidenceUse::ApiRedistribution,
      ))
      .await
      .unwrap();

    assert_eq!(result, None);
  }

  #[tokio::test]
  async fn rejects_a_repository_response_for_another_pinned_target() {
    let service = CanonicalSenseDetailsService::new(Arc::new(WrongTargetRepository));

    let result = service
      .read(CanonicalSenseDetailsReadRequest::new(
        id("release-1"),
        id("sense-requested"),
        EvidenceUse::Display,
      ))
      .await;

    assert_eq!(
      result,
      Err(CanonicalSenseDetailsReadError::Repository(
        CanonicalSenseDetailsRepositoryError::InconsistentData
      ))
    );
  }

  struct WrongTargetRepository;

  #[async_trait]
  impl CanonicalSenseDetailsRepository for WrongTargetRepository {
    async fn load(
      &self,
      _request: &CanonicalSenseDetailsReadRequest,
    ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError> {
      Ok(Some(details(
        "release-1",
        "sense-returned",
        CanonicalStatus::Active,
        true,
      )))
    }
  }

  struct UnfilteredRepository(CanonicalSenseDetails);

  #[async_trait]
  impl CanonicalSenseDetailsRepository for UnfilteredRepository {
    async fn load(
      &self,
      _request: &CanonicalSenseDetailsReadRequest,
    ) -> Result<Option<CanonicalSenseDetails>, CanonicalSenseDetailsRepositoryError> {
      Ok(Some(self.0.clone()))
    }
  }
}
