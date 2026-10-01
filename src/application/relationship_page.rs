//! Relationship-centered lexical-page assembly from release-pinned verified material.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::domain::{
  knowledge_view::{KnowledgeEvidenceState, KnowledgeLens, KnowledgeViewSuperset},
  relationship_page::{
    LabeledAlternative, PageDomainContext, PageFact, PageGeneratedExample,
    PageInferredExplanation, PageNamedPath, PageRelationship, PageRelationshipGroup,
    PageSemanticScale, ProjectedRelationshipPage, RelationshipGroupKind,
    RelationshipPageRequest, RelationshipPageSummaryRef, RelationshipPageSuperset,
    RelationshipPageValidationError, RelationshipPageVersionMetadata,
  },
  translation_turn::{ProjectedTranslationResult, TranslationResultKind},
};

/// Verified and request-local inputs admitted by the page composer.
pub struct RelationshipPageMaterial {
  /// Authoritative leading summary.
  pub summary: RelationshipPageSummaryRef,
  /// Request-local assessment plus release-pinned profiles.
  pub domain: PageDomainContext,
  /// Exact assertion hydrations referenced by all factual items.
  pub facts: Vec<PageFact>,
  /// Complete authoritative semantic scales.
  pub scales: Vec<PageSemanticScale>,
  /// Guided views produced by [`crate::application::knowledge_views::KnowledgeViewService`].
  pub views: Vec<KnowledgeViewSuperset>,
  /// Independently useful paths produced by the bounded path application service.
  pub paths: Vec<PageNamedPath>,
  /// Explicitly labeled request-local generated examples.
  pub generated_examples: Vec<PageGeneratedExample>,
  /// Evidence-grounded request-local explanations.
  pub inferred_explanations: Vec<PageInferredExplanation>,
  /// Explicitly requested and evaluated alternatives.
  pub alternatives: Vec<LabeledAlternative>,
}

/// Explicit result distinguishing full verified composition from canonical-only degradation.
pub enum RelationshipPageOutcome {
  /// Verified knowledge material was available and admitted.
  Complete(ProjectedRelationshipPage),
  /// Dependencies supplied no verified relationships, but the canonical summary remains useful.
  CanonicalOnly(ProjectedRelationshipPage),
}

/// One lexical translation with its embedded relationship-centered page.
pub struct LexicalRelationshipResult {
  /// Existing lexical translation outcome.
  pub translation: ProjectedTranslationResult,
  /// Page outcome bound to the same request-local lexical result.
  pub relationship_page: RelationshipPageOutcome,
}

/// Content-free composition failures.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RelationshipPageCompositionError {
  /// Relationship pages may be attached only to word or phrase translation results.
  #[error("relationship page requires a lexical translation result")]
  NonLexicalResult,
  /// A view crossed the requested root or immutable release pin.
  #[error("relationship page material is inconsistent")]
  InconsistentMaterial,
  /// The assembled page failed the frozen domain contract.
  #[error("relationship page validation failed")]
  InvalidPage,
}

impl From<RelationshipPageValidationError> for RelationshipPageCompositionError {
  fn from(_: RelationshipPageValidationError) -> Self {
    Self::InvalidPage
  }
}

/// Stateless composer that never persists generated or query-derived page material.
pub struct RelationshipPageComposer;

impl RelationshipPageComposer {
  /// Composes, validates, projects, and embeds a page into one lexical translation result.
  ///
  /// # Errors
  ///
  /// Rejects non-lexical results, mixed roots or releases, non-verified factual view items, and
  /// any page that violates the domain-level proof or projection contract.
  pub fn compose(
    translation: ProjectedTranslationResult,
    request: RelationshipPageRequest,
    material: RelationshipPageMaterial,
  ) -> Result<LexicalRelationshipResult, RelationshipPageCompositionError> {
    if !matches!(translation.translation.kind(), TranslationResultKind::Word | TranslationResultKind::Phrase) {
      return Err(RelationshipPageCompositionError::NonLexicalResult);
    }
    let mut groups = Vec::new();
    let mut admitted_assertions = BTreeSet::new();
    for view in material.views {
      if view.request.root.node != request.root || view.request.release != request.release {
        return Err(RelationshipPageCompositionError::InconsistentMaterial);
      }
      view.validate().map_err(|_| RelationshipPageCompositionError::InconsistentMaterial)?;
      let mut relationships = Vec::new();
      for item in view.items.into_iter().skip(1) {
        if item.evidence_state != KnowledgeEvidenceState::Verified {
          continue;
        }
        let path = item.path_to_root.ok_or(RelationshipPageCompositionError::InconsistentMaterial)?;
        let assertion_id = path.steps.first()
          .ok_or(RelationshipPageCompositionError::InconsistentMaterial)?
          .projection().assertion().assertion_id.clone();
        if admitted_assertions.insert(assertion_id.clone()) {
          relationships.push(PageRelationship {
            node: item.node,
            assertion_id,
            path_to_root: path,
            usefulness: u16::MAX.saturating_sub(item.order),
          });
        }
      }
      if !relationships.is_empty() {
        groups.push(PageRelationshipGroup { kind: group_kind(view.request.lens), relationships });
      }
    }
    groups.sort_by_key(|group| group.kind.rank());
    if groups.windows(2).any(|pair| pair[0].kind == pair[1].kind) {
      return Err(RelationshipPageCompositionError::InconsistentMaterial);
    }
    let superset = RelationshipPageSuperset {
      request,
      summary: material.summary,
      domain: material.domain,
      facts: material.facts,
      scales: material.scales,
      groups,
      paths: material.paths,
      generated_examples: material.generated_examples,
      inferred_explanations: material.inferred_explanations,
      exploratory_items: Vec::new(),
      alternatives: material.alternatives,
      gap_proposals: Vec::new(),
      versions: RelationshipPageVersionMetadata::default(),
    };
    let degraded = superset.groups.is_empty();
    let page = superset.project()?;
    Ok(LexicalRelationshipResult {
      translation,
      relationship_page: if degraded {
        RelationshipPageOutcome::CanonicalOnly(page)
      } else {
        RelationshipPageOutcome::Complete(page)
      },
    })
  }
}

fn group_kind(lens: KnowledgeLens) -> RelationshipGroupKind {
  match lens {
    KnowledgeLens::Meaning => RelationshipGroupKind::Meaning,
    KnowledgeLens::Contrast => RelationshipGroupKind::Contrast,
    KnowledgeLens::Usage => RelationshipGroupKind::Suitability,
    KnowledgeLens::Form => RelationshipGroupKind::Morphology,
    KnowledgeLens::Origin => RelationshipGroupKind::CulturalExtension,
    KnowledgeLens::Domain => RelationshipGroupKind::UsageConvention,
    KnowledgeLens::Mechanism => RelationshipGroupKind::Mechanism,
    KnowledgeLens::Application => RelationshipGroupKind::Application,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{
    assertion::{CanonicalNodeFamily, CanonicalNodeId},
    canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
    domain_assessment::DomainAssessment,
    model_runtime::GenerationProfile,
    translation_turn::{
      ResponseLevel, TranslationReview, TranslationTurnResult, TranslationVersionMetadata,
      TurnLanguage,
    },
  };

  fn pin() -> CanonicalReleasePin {
    CanonicalReleasePin::new(CanonicalId::new("release-1").unwrap(), "canonical-v1".into())
      .unwrap()
  }

  fn translation(unit: TranslationTurnResult) -> ProjectedTranslationResult {
    ProjectedTranslationResult {
      translation: unit,
      metadata: TranslationVersionMetadata {
        schema_version: "translation-result-v1",
        normalizer_version: "normalizer-v1",
        projection_version: "projection-v1",
        response_level: ResponseLevel::Standard,
        model_versions: vec!["model-v1".into()],
        prompt_versions: vec!["prompt-v1".into()],
        inference_profiles: vec![GenerationProfile::Fast],
        reasoning_escalated: false,
        retrieval_version: None,
        content_release: Some("release-1".into()),
      },
      external_sources: Vec::new(),
    }
  }

  fn request() -> RelationshipPageRequest {
    RelationshipPageRequest {
      root: CanonicalNodeId::publisher_assigned(
        CanonicalNodeFamily::LexicalSense,
        CanonicalId::new("sense-1").unwrap(),
      ),
      target_language: LanguageTag::parse("en").unwrap(),
      response_level: ResponseLevel::Standard,
      release: pin(),
      max_alternatives: 0,
    }
  }

  fn material() -> RelationshipPageMaterial {
    RelationshipPageMaterial {
      summary: RelationshipPageSummaryRef::BasicCard {
        sense_id: CanonicalId::new("sense-1").unwrap(),
      },
      domain: PageDomainContext {
        assessment: DomainAssessment::General {
          reason: "general usage".into(),
        },
        profiles: Vec::new(),
      },
      facts: Vec::new(),
      scales: Vec::new(),
      views: Vec::new(),
      paths: Vec::new(),
      generated_examples: Vec::new(),
      inferred_explanations: Vec::new(),
      alternatives: Vec::new(),
    }
  }

  #[test]
  fn canonical_summary_is_an_explicit_degraded_outcome() {
    let lexical = TranslationTurnResult::Word {
      detected_source_language: TurnLanguage::parse("en").unwrap(),
      translations: Vec::new(),
      annotations: Vec::new(),
      review: TranslationReview::clean(),
    };
    let result = RelationshipPageComposer::compose(translation(lexical), request(), material())
      .unwrap();
    assert!(matches!(
      result.relationship_page,
      RelationshipPageOutcome::CanonicalOnly(_)
    ));
  }

  #[test]
  fn passage_cannot_receive_a_relationship_page() {
    let passage = TranslationTurnResult::passage(
      "hello".into(),
      TurnLanguage::parse("en").unwrap(),
      TurnLanguage::parse("zh-CN").unwrap(),
    );
    assert!(matches!(
      RelationshipPageComposer::compose(translation(passage), request(), material()),
      Err(RelationshipPageCompositionError::NonLexicalResult)
    ));
  }
}
