//! Deterministic request-local domain assessment over one canonical release pin.

use std::{collections::BTreeSet, sync::Arc};

use crate::{
  domain::{
    canonical::CanonicalReleasePin,
    domain_assessment::{
      DomainAssessment, DomainAssessmentError, DomainNomination, MAX_BROADER_DOMAINS,
      MAX_DOMAIN_INVENTORY, MAX_DOMAIN_PROSE_CHARS, MAX_DOMAIN_TERM_CHARS,
    },
  },
  ports::canonical_read::{CanonicalDomainQuery, CanonicalReadContext, CanonicalReadPort},
};

/// Request-local assessment over a read-only canonical-domain authority.
#[derive(Clone)]
pub struct DomainAssessmentService {
  authority: Arc<dyn CanonicalReadPort>,
}

impl DomainAssessmentService {
  /// Creates an assessment service that never stores proposals or nominations.
  pub fn new(authority: Arc<dyn CanonicalReadPort>) -> Self {
    Self { authority }
  }

  /// Validates one closed nomination against a successful release-pinned domain inventory.
  ///
  /// Authority failures and incomplete catalogs yield [`DomainAssessment::Uncertain`]. A
  /// `proposed_new` outcome is possible only when the authority explicitly confirms that the
  /// eligible catalog was complete and every proposed broader ID belongs to its allowlist.
  ///
  /// # Errors
  ///
  /// Returns a closed validation failure for malformed bounded input or a model-selected
  /// canonical identity outside the supplied allowlist. Failed or inconsistent inventories map
  /// to `uncertain`.
  pub async fn assess(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalDomainQuery,
    nomination: DomainNomination,
  ) -> Result<DomainAssessment, DomainAssessmentError> {
    validate_pin(pin)?;
    validate_query(&query)?;
    let inventory = match self.authority.domains(context, pin, query).await {
      Ok(inventory) => inventory,
      Err(_) => return Ok(DomainAssessment::Uncertain),
    };
    if inventory
      .domains()
      .iter()
      .any(|domain| domain.release_id() != &pin.release_id)
    {
      return Ok(DomainAssessment::Uncertain);
    }
    let allowlist = inventory
      .domains()
      .iter()
      .map(|domain| domain.domain_id())
      .collect::<BTreeSet<_>>();

    match nomination {
      DomainNomination::Existing { domain_ids, reason } => {
        validate_reason(&reason)?;
        if domain_ids.is_empty() || domain_ids.len() > MAX_BROADER_DOMAINS {
          return Err(DomainAssessmentError::TooManyValues);
        }
        if domain_ids.windows(2).any(|pair| pair[0] >= pair[1])
          || domain_ids.iter().any(|id| !allowlist.contains(id))
        {
          return Err(DomainAssessmentError::InvalidNomination);
        }
        Ok(DomainAssessment::Existing { domain_ids, reason })
      }
      DomainNomination::ProposedNew(proposal) => {
        if !inventory.catalog_complete() {
          return Ok(DomainAssessment::Uncertain);
        }
        if inventory.domains().iter().any(|domain| {
          domain.labels().iter().chain(domain.aliases()).any(|term| {
            term.language() == proposal.label().language() && term.text() == proposal.label().text()
          })
        }) {
          return Err(DomainAssessmentError::InvalidNomination);
        }
        if proposal
          .broader_domain_ids()
          .iter()
          .any(|id| !allowlist.contains(id))
        {
          return Err(DomainAssessmentError::InvalidNomination);
        }
        Ok(DomainAssessment::ProposedNew(proposal))
      }
      DomainNomination::General { reason } => {
        validate_reason(&reason)?;
        Ok(DomainAssessment::General { reason })
      }
      DomainNomination::Uncertain { reason } => {
        validate_reason(&reason)?;
        Ok(DomainAssessment::Uncertain)
      }
    }
  }
}

fn validate_pin(pin: &CanonicalReleasePin) -> Result<(), DomainAssessmentError> {
  if CanonicalReleasePin::new(pin.release_id.clone(), pin.canonical_schema_version.clone()).as_ref()
    != Some(pin)
  {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

fn validate_query(query: &CanonicalDomainQuery) -> Result<(), DomainAssessmentError> {
  if query.normalized_labels.is_empty()
    || query.normalized_labels.len() > MAX_DOMAIN_INVENTORY
    || query.languages.is_empty()
    || query.languages.len() > 8
    || query.limit == 0
    || query.limit > MAX_DOMAIN_INVENTORY
    || query.normalized_labels.iter().any(|label| {
      label.trim() != label || label.is_empty() || label.chars().count() > MAX_DOMAIN_TERM_CHARS
    })
    || query
      .normalized_labels
      .windows(2)
      .any(|pair| pair[0] >= pair[1])
    || query.languages.windows(2).any(|pair| pair[0] >= pair[1])
    || query.scope_key.as_ref().is_some_and(|scope| {
      scope.trim() != scope || scope.is_empty() || scope.chars().count() > MAX_DOMAIN_TERM_CHARS
    })
  {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

fn validate_reason(reason: &str) -> Result<(), DomainAssessmentError> {
  if reason.trim() != reason || reason.is_empty() || reason.chars().count() > MAX_DOMAIN_PROSE_CHARS
  {
    return Err(DomainAssessmentError::InvalidValue);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::{sync::Mutex, time::Duration};

  use async_trait::async_trait;

  use super::*;
  use crate::{
    domain::{
      canonical::{CanonicalId, LanguageTag},
      canonical_translation::{CanonicalRevision, DomainId},
      domain_assessment::{
        CanonicalDomain, DomainCoverageState, DomainInventory, DomainKnowledgeProfile,
        LocalizedDomainDefinition, LocalizedDomainTerm, ProposedDomain,
      },
    },
    ports::canonical_read::{CanonicalReadError, CanonicalReadPort},
  };

  struct Authority {
    result: Mutex<Option<Result<DomainInventory, CanonicalReadError>>>,
  }

  #[async_trait]
  impl CanonicalReadPort for Authority {
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
      _query: crate::ports::canonical_read::CanonicalTranslationQuery,
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
      _query: crate::ports::canonical_read::CanonicalCandidateQuery,
    ) -> Result<Vec<crate::domain::retrieval::RepositoryMatch>, CanonicalReadError> {
      unreachable!()
    }

    async fn sense(
      &self,
      _context: &CanonicalReadContext,
      _pin: &CanonicalReleasePin,
      _query: crate::ports::canonical_read::CanonicalSenseQuery,
    ) -> Result<crate::domain::canonical_content::CanonicalSenseDetails, CanonicalReadError> {
      unreachable!()
    }

    async fn domains(
      &self,
      _context: &CanonicalReadContext,
      _pin: &CanonicalReleasePin,
      _query: CanonicalDomainQuery,
    ) -> Result<DomainInventory, CanonicalReadError> {
      self.result.lock().unwrap().take().unwrap()
    }
  }

  fn pin() -> CanonicalReleasePin {
    CanonicalReleasePin::new(
      CanonicalId::new("release-1").unwrap(),
      "canonical-v1".to_string(),
    )
    .unwrap()
  }

  fn context() -> CanonicalReadContext {
    CanonicalReadContext {
      request_id: "req-domain-1".to_string(),
      deadline_at: "2099-01-01T00:00:00Z".to_string(),
      timeout: Duration::from_secs(1),
    }
  }

  fn query() -> CanonicalDomainQuery {
    CanonicalDomainQuery {
      normalized_labels: vec!["weather".to_string()],
      scope_key: Some("earth-atmosphere".to_string()),
      languages: vec![LanguageTag::parse("en").unwrap()],
      limit: 8,
    }
  }

  fn domain() -> CanonicalDomain {
    domain_for_release("release-1")
  }

  fn domain_for_release(release: &str) -> CanonicalDomain {
    CanonicalDomain::new(
      CanonicalId::new(release).unwrap(),
      DomainId::new("domain_weather").unwrap(),
      CanonicalRevision::new(1).unwrap(),
      vec![LocalizedDomainTerm::new(LanguageTag::parse("en").unwrap(), "weather").unwrap()],
      vec![
        LocalizedDomainTerm::new(LanguageTag::parse("en").unwrap(), "meteorological weather")
          .unwrap(),
      ],
      vec![LocalizedDomainDefinition::new(
        LanguageTag::parse("en").unwrap(),
        "Atmospheric conditions.",
      )
      .unwrap()],
      vec!["meteorology".to_string()],
      vec!["climate classification".to_string()],
      vec![],
      DomainKnowledgeProfile::new(
        vec!["definition".to_string()],
        vec![LanguageTag::parse("en").unwrap()],
        4,
        DomainCoverageState::Seed,
      )
      .unwrap(),
    )
    .unwrap()
  }

  fn service(result: Result<DomainInventory, CanonicalReadError>) -> DomainAssessmentService {
    DomainAssessmentService::new(Arc::new(Authority {
      result: Mutex::new(Some(result)),
    }))
  }

  #[tokio::test]
  async fn selects_only_allowlisted_existing_ids() {
    let selected = DomainId::new("domain_weather").unwrap();
    let outcome = service(Ok(DomainInventory::new(vec![domain()], true).unwrap()))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::Existing {
          domain_ids: vec![selected.clone()],
          reason: "The resolved sense is meteorological.".to_string(),
        },
      )
      .await
      .unwrap();
    assert!(
      matches!(outcome, DomainAssessment::Existing { domain_ids, .. } if domain_ids == vec![selected])
    );
  }

  #[tokio::test]
  async fn rejects_identity_outside_allowlist() {
    let result = service(Ok(DomainInventory::new(vec![domain()], true).unwrap()))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::Existing {
          domain_ids: vec![DomainId::new("domain_secret").unwrap()],
          reason: "A model-selected scope.".to_string(),
        },
      )
      .await;
    let Err(error) = result else {
      panic!("an identity outside the allowlist must be rejected");
    };
    assert_eq!(error, DomainAssessmentError::InvalidNomination);
  }

  #[tokio::test]
  async fn dependency_failure_is_always_uncertain() {
    let outcome = service(Err(CanonicalReadError::Unavailable))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::General {
          reason: "Would otherwise be general.".to_string(),
        },
      )
      .await
      .unwrap();
    assert!(matches!(outcome, DomainAssessment::Uncertain));
  }

  #[tokio::test]
  async fn cross_release_inventory_is_uncertain_not_novel() {
    let outcome = service(Ok(
      DomainInventory::new(vec![domain_for_release("release-2")], true).unwrap(),
    ))
    .assess(
      &context(),
      &pin(),
      query(),
      DomainNomination::General {
        reason: "Would otherwise be general.".to_string(),
      },
    )
    .await
    .unwrap();
    assert!(matches!(outcome, DomainAssessment::Uncertain));
  }

  #[tokio::test]
  async fn incomplete_catalog_cannot_prove_proposed_domain() {
    let proposal = ProposedDomain::new(
      LocalizedDomainTerm::new(LanguageTag::parse("en").unwrap(), "micro-weather").unwrap(),
      "A narrow atmospheric scope.",
      vec![DomainId::new("domain_weather").unwrap()],
      "The supplied scope is broader than the resolved sense.",
    )
    .unwrap();
    let outcome = service(Ok(DomainInventory::new(vec![domain()], false).unwrap()))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::ProposedNew(proposal),
      )
      .await
      .unwrap();
    assert!(matches!(outcome, DomainAssessment::Uncertain));
  }

  #[tokio::test]
  async fn complete_catalog_allows_request_local_proposal() {
    let proposal = ProposedDomain::new(
      LocalizedDomainTerm::new(LanguageTag::parse("en").unwrap(), "micro-weather").unwrap(),
      "A narrow atmospheric scope.",
      vec![DomainId::new("domain_weather").unwrap()],
      "The supplied scope is broader than the resolved sense.",
    )
    .unwrap();
    let outcome = service(Ok(DomainInventory::new(vec![domain()], true).unwrap()))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::ProposedNew(proposal),
      )
      .await
      .unwrap();
    assert!(matches!(outcome, DomainAssessment::ProposedNew(_)));
  }

  #[tokio::test]
  async fn proposal_cannot_collide_with_canonical_label_or_alias() {
    for colliding_text in ["weather", "meteorological weather"] {
      let proposal = ProposedDomain::new(
        LocalizedDomainTerm::new(LanguageTag::parse("en").unwrap(), colliding_text).unwrap(),
        "A supposedly new atmospheric scope.",
        vec![],
        "The supplied scopes supposedly do not fit.",
      )
      .unwrap();
      let result = service(Ok(DomainInventory::new(vec![domain()], true).unwrap()))
        .assess(
          &context(),
          &pin(),
          query(),
          DomainNomination::ProposedNew(proposal),
        )
        .await;
      assert!(matches!(
        result,
        Err(DomainAssessmentError::InvalidNomination)
      ));
    }
  }

  #[tokio::test]
  async fn same_text_in_different_language_is_not_an_exact_collision() {
    let proposal = ProposedDomain::new(
      LocalizedDomainTerm::new(LanguageTag::parse("fr").unwrap(), "weather").unwrap(),
      "A language-specific proposed scope.",
      vec![],
      "No supplied French label or alias fits.",
    )
    .unwrap();
    let outcome = service(Ok(DomainInventory::new(vec![domain()], true).unwrap()))
      .assess(
        &context(),
        &pin(),
        query(),
        DomainNomination::ProposedNew(proposal),
      )
      .await
      .unwrap();
    assert!(matches!(outcome, DomainAssessment::ProposedNew(_)));
  }

  #[test]
  fn diagnostics_do_not_contain_request_content() {
    let error = DomainAssessmentError::InvalidNomination;
    assert_eq!(format!("{error:?}"), "InvalidNomination");
    assert_eq!(
      error.to_string(),
      "domain nomination is outside the supplied allowlist"
    );
  }
}
