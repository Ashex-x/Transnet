//! Exhaustive server-owned policy for guided knowledge lenses.

use crate::domain::{
  assertion::CanonicalNodeFamily,
  graph::GraphRelationType,
  knowledge_view::{KnowledgeLens, KnowledgeRelevanceReason},
  translation_turn::ResponseLevel,
};

/// Evidence eligibility applied before any lens limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LensEvidencePolicy {
  /// Only hydrated verified assertions with display-eligible evidence may connect items.
  VerifiedDisplayEligible,
}

/// Frozen traversal and budget policy derived entirely from a selected lens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnowledgeLensPolicy {
  /// Selected public lens.
  pub lens: KnowledgeLens,
  /// Ordered branch used for deterministic grouping.
  pub branch: KnowledgeRelevanceReason,
  /// Exact eligible relationship families.
  pub relations: &'static [GraphRelationType],
  /// Exact eligible opposite-endpoint families.
  pub node_families: &'static [CanonicalNodeFamily],
  /// Maximum traversal hops used to compose this view.
  pub max_depth: u8,
  /// Maximum verified items gathered before response projection.
  pub max_items: usize,
  /// Required evidence policy.
  pub evidence: LensEvidencePolicy,
}

/// Returns the exhaustive immutable policy for one closed lens.
pub const fn policy_for(lens: KnowledgeLens) -> Option<KnowledgeLensPolicy> {
  use CanonicalNodeFamily as N;
  use GraphRelationType as R;
  let (branch, relations, families, depth, items): (_, &'static [_], &'static [_], _, _) =
    match lens {
      KnowledgeLens::Meaning => (
        KnowledgeRelevanceReason::Meaning,
        &[
          R::Synonym,
          R::TranslationEquivalent,
          R::Hypernym,
          R::Hyponym,
        ],
        &[N::LexicalSense, N::Phrase, N::MultilingualTerm, N::Concept],
        2,
        50,
      ),
      KnowledgeLens::Contrast => (
        KnowledgeRelevanceReason::Contrast,
        &[
          R::Antonym,
          R::NearSynonym,
          R::ConfusableWith,
          R::LowerDegree,
          R::HigherDegree,
        ],
        &[N::LexicalSense, N::Phrase, N::Concept, N::SemanticScale],
        2,
        40,
      ),
      KnowledgeLens::Usage => (
        KnowledgeRelevanceReason::Usage,
        &[
          R::ConstructionMember,
          R::HasConstructionMember,
          R::AssociatedWith,
        ],
        &[
          N::LexicalSense,
          N::Phrase,
          N::GrammarPattern,
          N::Collocation,
        ],
        1,
        30,
      ),
      KnowledgeLens::Form => (
        KnowledgeRelevanceReason::Form,
        &[
          R::InflectionOf,
          R::HasInflection,
          R::DerivationallyRelatedTo,
        ],
        &[N::LexicalSense, N::Phrase],
        1,
        30,
      ),
      KnowledgeLens::Origin => (
        KnowledgeRelevanceReason::Origin,
        &[R::EtymologicallyDerivedFrom, R::EtymologicalSourceOf],
        &[N::LexicalSense, N::Phrase, N::MultilingualTerm],
        2,
        30,
      ),
      KnowledgeLens::Domain => (
        KnowledgeRelevanceReason::Domain,
        &[R::Hypernym, R::Hyponym, R::AssociatedWith],
        &[N::Domain, N::Concept, N::LexicalSense, N::Phenomenon],
        2,
        40,
      ),
      KnowledgeLens::Mechanism | KnowledgeLens::Application => return None,
    };
  Some(KnowledgeLensPolicy {
    lens,
    branch,
    relations,
    node_families: families,
    max_depth: depth,
    max_items: items,
    evidence: LensEvidencePolicy::VerifiedDisplayEligible,
  })
}

/// Returns the response-level item budget without changing policy eligibility or ranking.
pub const fn response_item_budget(policy: &KnowledgeLensPolicy, level: ResponseLevel) -> usize {
  let requested = match level {
    ResponseLevel::Brief => 8,
    ResponseLevel::Standard => 24,
    ResponseLevel::Full => policy.max_items,
  };
  if requested < policy.max_items {
    requested
  } else {
    policy.max_items
  }
}

/// Deterministic ordering key for equally eligible canonical items.
pub fn deterministic_order_key(
  policy: &KnowledgeLensPolicy,
  relation: GraphRelationType,
  node_family: CanonicalNodeFamily,
  node_id: &crate::domain::canonical::CanonicalId,
) -> (usize, usize, String) {
  let relation_rank = policy
    .relations
    .iter()
    .position(|value| *value == relation)
    .unwrap_or(usize::MAX);
  let family_rank = policy
    .node_families
    .iter()
    .position(|value| *value == node_family)
    .unwrap_or(usize::MAX);
  (relation_rank, family_rank, node_id.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn implemented_lenses_are_bounded_and_technical_lenses_fail_closed() {
    let lenses = [
      KnowledgeLens::Meaning,
      KnowledgeLens::Contrast,
      KnowledgeLens::Usage,
      KnowledgeLens::Form,
      KnowledgeLens::Origin,
      KnowledgeLens::Domain,
      KnowledgeLens::Mechanism,
      KnowledgeLens::Application,
    ];
    for lens in lenses {
      let Some(policy) = policy_for(lens) else {
        assert!(matches!(
          lens,
          KnowledgeLens::Mechanism | KnowledgeLens::Application
        ));
        continue;
      };
      assert_eq!(policy.branch, lens.relevance_reason());
      assert!(!policy.relations.is_empty());
      assert!(!policy.node_families.is_empty());
      assert!(policy
        .node_families
        .iter()
        .copied()
        .all(crate::domain::knowledge_view::hydration_family_supported));
      assert!((1..=2).contains(&policy.max_depth));
      assert!(policy.max_items <= 75);
      assert_eq!(policy.evidence, LensEvidencePolicy::VerifiedDisplayEligible);
      assert!(
        response_item_budget(&policy, ResponseLevel::Brief)
          <= response_item_budget(&policy, ResponseLevel::Standard)
      );
      assert!(
        response_item_budget(&policy, ResponseLevel::Standard)
          <= response_item_budget(&policy, ResponseLevel::Full)
      );
    }
  }
}
