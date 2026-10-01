//! Relationship-centered lexical-page assembly from release-pinned verified material.

use std::{collections::BTreeSet, sync::Arc};

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  knowledge_view::{KnowledgeEvidenceState, KnowledgeLens, KnowledgeViewSuperset},
  model_runtime::CancellationSignal,
  relationship_page::{
    LabeledAlternative, PageDomainContext, PageFact, PageGeneratedExample, PageInferredExplanation,
    PageNamedPath, PageRelationship, PageRelationshipGroup, PageSemanticScale,
    ProjectedRelationshipPage, RelationshipGroupKind, RelationshipPageRequest,
    RelationshipPageSummaryRef, RelationshipPageSuperset, RelationshipPageValidationError,
    RelationshipPageVersionMetadata,
  },
  request_context::RequestContext,
  translation_turn::{ProjectedTranslationResult, TranslationResultKind, TranslationTurn},
};

/// Release-pinned inputs returned by the complete relationship-page authority composition.
pub struct RelationshipPageInputs {
  /// Request bound to the resolved canonical lexical root and release.
  pub request: RelationshipPageRequest,
  /// Fully hydrated authoritative and request-local material.
  pub material: RelationshipPageMaterial,
}

/// Optional runtime authority that resolves and hydrates one lexical relationship page.
#[async_trait]
pub trait RelationshipPageMaterialPort: Send + Sync {
  /// Returns no inputs for unresolved, ambiguous, or non-established lexical results.
  async fn material(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    turn: &TranslationTurn,
    translation: &ProjectedTranslationResult,
  ) -> Result<Option<RelationshipPageInputs>, RelationshipPageMaterialError>;
}

/// Content-free failure while resolving or hydrating page material.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RelationshipPageMaterialError {
  /// A required canonical or retrieval dependency was unavailable.
  #[error("relationship page dependency unavailable")]
  Unavailable,
  /// Returned material contradicted the root or immutable release.
  #[error("relationship page dependency returned inconsistent material")]
  Inconsistent,
  /// The shared request deadline elapsed.
  #[error("relationship page deadline exceeded")]
  DeadlineExceeded,
  /// The request was cancelled.
  #[error("relationship page request cancelled")]
  Cancelled,
}

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
#[derive(Clone)]
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

/// Optional request-local runtime that invokes authoritative material only for lexical results.
pub struct RelationshipPageRuntime {
  source: Arc<dyn RelationshipPageMaterialPort>,
}

impl RelationshipPageRuntime {
  /// Creates a runtime from one atomic material authority.
  pub fn new(source: Arc<dyn RelationshipPageMaterialPort>) -> Self {
    Self { source }
  }

  /// Attaches a validated page only when an exact lexical root resolves.
  pub async fn enrich(
    &self,
    context: &RequestContext,
    cancellation: &CancellationSignal,
    turn: &TranslationTurn,
    mut translation: ProjectedTranslationResult,
  ) -> Result<ProjectedTranslationResult, RelationshipPageMaterialError> {
    if !matches!(
      translation.translation.kind(),
      TranslationResultKind::Word | TranslationResultKind::Phrase
    ) {
      return Ok(translation);
    }
    let Some(inputs) = self
      .source
      .material(context, cancellation, turn, &translation)
      .await?
    else {
      return Ok(translation);
    };
    if inputs.request.max_alternatives != turn.guidance().max_alternatives
      || inputs.request.response_level != turn.response_level()
      || inputs.request.target_language.as_str() != turn.target_language().as_str()
    {
      return Err(RelationshipPageMaterialError::Inconsistent);
    }
    let composed = RelationshipPageComposer::compose(translation, inputs.request, inputs.material)
      .map_err(|_| RelationshipPageMaterialError::Inconsistent)?;
    let (page, canonical_only) = match composed.relationship_page {
      RelationshipPageOutcome::Complete(page) => (page, false),
      RelationshipPageOutcome::CanonicalOnly(page) => (page, true),
    };
    translation = composed.translation;
    translation.metadata.content_release = Some(page.release.release_id.to_string());
    translation.relationship_page = Some(page);
    translation.relationship_page_canonical_only = canonical_only;
    Ok(translation)
  }
}

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
    if !matches!(
      translation.translation.kind(),
      TranslationResultKind::Word | TranslationResultKind::Phrase
    ) {
      return Err(RelationshipPageCompositionError::NonLexicalResult);
    }
    let translations = translation.translation.translations();
    if translations.len() != request.translation_choices.len()
      || translations
        .iter()
        .zip(&request.translation_choices)
        .any(|(translation, choice)| {
          translation.translation_id != choice.translation_id
            || usize::from(choice.order) != translation.order
        })
    {
      return Err(RelationshipPageCompositionError::InconsistentMaterial);
    }
    let mut groups = Vec::new();
    let mut admitted_assertions = BTreeSet::new();
    for view in material.views {
      if view.request.root.node != request.root || view.request.release != request.release {
        return Err(RelationshipPageCompositionError::InconsistentMaterial);
      }
      view
        .validate()
        .map_err(|_| RelationshipPageCompositionError::InconsistentMaterial)?;
      let mut relationships = Vec::new();
      for item in view.items.into_iter().skip(1) {
        if item.evidence_state != KnowledgeEvidenceState::Verified {
          continue;
        }
        let path = item
          .path_to_root
          .ok_or(RelationshipPageCompositionError::InconsistentMaterial)?;
        let assertion_id = path
          .steps
          .first()
          .ok_or(RelationshipPageCompositionError::InconsistentMaterial)?
          .projection()
          .assertion()
          .assertion_id
          .clone();
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
        groups.push(PageRelationshipGroup {
          kind: group_kind(view.request.lens),
          relationships,
        });
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
    let degraded = superset.groups.is_empty()
      && superset.paths.is_empty()
      && superset.inferred_explanations.is_empty();
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
  use std::sync::Mutex;

  fn pin() -> CanonicalReleasePin {
    CanonicalReleasePin::new(
      CanonicalId::new("release-1").unwrap(),
      "canonical-v1".into(),
    )
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
      relationship_page: None,
      relationship_page_canonical_only: false,
    }
  }

  fn request() -> RelationshipPageRequest {
    RelationshipPageRequest {
      root: CanonicalNodeId::publisher_assigned(
        CanonicalNodeFamily::LexicalSense,
        CanonicalId::new("sense-1").unwrap(),
      ),
      target_language: LanguageTag::parse("zh-CN").unwrap(),
      response_level: ResponseLevel::Standard,
      release: pin(),
      max_alternatives: 0,
      translation_choices: vec![crate::domain::relationship_page::PageTranslationChoice {
        translation_id: "translation_0".into(),
        order: 0,
      }],
    }
  }

  fn material() -> RelationshipPageMaterial {
    RelationshipPageMaterial {
      summary: RelationshipPageSummaryRef::BasicCard {
        sense_id: CanonicalId::new("sense-1").unwrap(),
        release: pin(),
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

  struct MaterialSource(Mutex<Option<RelationshipPageInputs>>);

  #[async_trait]
  impl RelationshipPageMaterialPort for MaterialSource {
    async fn material(
      &self,
      _context: &RequestContext,
      _cancellation: &CancellationSignal,
      _turn: &TranslationTurn,
      _translation: &ProjectedTranslationResult,
    ) -> Result<Option<RelationshipPageInputs>, RelationshipPageMaterialError> {
      Ok(self.0.lock().unwrap().take())
    }
  }

  #[test]
  fn canonical_summary_is_an_explicit_degraded_outcome() {
    let lexical = TranslationTurnResult::Word {
      detected_source_language: TurnLanguage::parse("en").unwrap(),
      translations: vec![crate::domain::translation_turn::TurnTranslation {
        translation_id: "translation_0".into(),
        order: 0,
        text: "你好".into(),
        language: TurnLanguage::Chinese,
        meaning: Some("a greeting".into()),
        details: None,
      }],
      annotations: Vec::new(),
      review: TranslationReview::clean(),
    };
    let result =
      RelationshipPageComposer::compose(translation(lexical), request(), material()).unwrap();
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

  #[tokio::test]
  async fn runtime_embeds_canonical_only_page_only_for_a_resolved_lexical_result() {
    let runtime = RelationshipPageRuntime::new(Arc::new(MaterialSource(Mutex::new(Some(
      RelationshipPageInputs {
        request: request(),
        material: material(),
      },
    )))));
    let turn = TranslationTurn::new(crate::domain::translation_turn::TranslationTurnRequest {
      text: Some("term".into()),
      input: None,
      source_language: "en".into(),
      target_language: "zh-CN".into(),
      response_level: "standard".into(),
      history: Vec::new(),
      guidance: None,
    })
    .unwrap();
    let context = RequestContext::new(
      crate::domain::request_context::RequestId::new("relationship-test").unwrap(),
      time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        + time::Duration::seconds(5),
      "translation-result-v1",
      None,
    )
    .unwrap();
    let result = runtime
      .enrich(
        &context,
        &CancellationSignal::default(),
        &turn,
        translation(
          crate::domain::translation_turn::TranslationTurnResult::lexical(
            crate::domain::translation_turn::LexicalTurnDraft {
              translations: vec![crate::domain::translation_turn::LexicalMeaningDraft {
                text: "词".into(),
                meaning: "meaning".into(),
                part_of_speech: "noun".into(),
                phrase_type: String::new(),
                aliases: Vec::new(),
                examples: Vec::new(),
                usage_notes: Vec::new(),
              }],
            },
            crate::domain::translation_turn::TranslationUnit::Word,
            TurnLanguage::English,
            TurnLanguage::Chinese,
          ),
        ),
      )
      .await
      .unwrap();
    assert!(result.relationship_page.is_some());
    assert!(result.relationship_page_canonical_only);
    assert_eq!(
      result.metadata.content_release.as_deref(),
      Some("release-1")
    );
  }
}
