//! Validated guided knowledge views and bounded evidence-backed paths.

use std::{
  collections::{BTreeMap, BTreeSet},
  fmt,
};

use thiserror::Error;

use super::{
  assertion::{CanonicalNodeFamily, CanonicalNodeId},
  canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
  knowledge_hydration::HydratedAssertionProjection,
  translation_turn::ResponseLevel,
};

/// Maximum independently verified paths returned for one request.
pub const MAX_KNOWLEDGE_PATHS: usize = 3;
/// Maximum assertion traversals in one returned path.
pub const MAX_KNOWLEDGE_PATH_HOPS: usize = 3;
/// Maximum items admitted to one validated guided view.
pub const MAX_KNOWLEDGE_VIEW_ITEMS: usize = 75;
/// Maximum independently citable evidence records on one verified traversal.
pub const MAX_KNOWLEDGE_STEP_EVIDENCE: usize = 32;

/// Closed server-owned knowledge projections exposed to callers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum KnowledgeLens {
  /// Definitions, equivalence, and taxonomy around a selected meaning.
  Meaning,
  /// Explicit opposition, near-equivalence, and common confusion.
  Contrast,
  /// Reviewed construction, collocation, and suitability relationships.
  Usage,
  /// Inflectional and derivational form relationships.
  Form,
  /// Reviewed historical derivation relationships.
  Origin,
  /// Explicit domain membership and domain-scoped relationships.
  Domain,
  /// Evidence-backed mechanisms, dependencies, and processes.
  Mechanism,
  /// Reviewed uses, implementations, standards, and technologies.
  Application,
}

impl KnowledgeLens {
  /// Returns the exact public wire spelling.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Meaning => "meaning",
      Self::Contrast => "contrast",
      Self::Usage => "usage",
      Self::Form => "form",
      Self::Origin => "origin",
      Self::Domain => "domain",
      Self::Mechanism => "mechanism",
      Self::Application => "application",
    }
  }

  /// Parses only the eight frozen public spellings.
  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "meaning" => Some(Self::Meaning),
      "contrast" => Some(Self::Contrast),
      "usage" => Some(Self::Usage),
      "form" => Some(Self::Form),
      "origin" => Some(Self::Origin),
      "domain" => Some(Self::Domain),
      "mechanism" => Some(Self::Mechanism),
      "application" => Some(Self::Application),
      _ => None,
    }
  }

  /// Returns the server-owned relevance branch used by this projection.
  pub const fn relevance_reason(self) -> KnowledgeRelevanceReason {
    match self {
      Self::Meaning => KnowledgeRelevanceReason::Meaning,
      Self::Contrast => KnowledgeRelevanceReason::Contrast,
      Self::Usage => KnowledgeRelevanceReason::Usage,
      Self::Form => KnowledgeRelevanceReason::Form,
      Self::Origin => KnowledgeRelevanceReason::Origin,
      Self::Domain => KnowledgeRelevanceReason::Domain,
      Self::Mechanism => KnowledgeRelevanceReason::Mechanism,
      Self::Application => KnowledgeRelevanceReason::Application,
    }
  }
}

/// One typed publisher-owned root used by views and paths.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct KnowledgeRoot {
  /// Family-qualified canonical node identity.
  pub node: CanonicalNodeId,
}

/// Request for one deterministic guided view.
#[derive(Clone, Eq, PartialEq)]
pub struct KnowledgeViewRequest {
  /// Selected canonical root.
  pub root: KnowledgeRoot,
  /// Closed server-owned projection policy.
  pub lens: KnowledgeLens,
  /// Language used for reviewed display material.
  pub target_language: LanguageTag,
  /// Breadth projection over one validated superset.
  pub response_level: ResponseLevel,
  /// Exact authoritative content release and schema.
  pub release: CanonicalReleasePin,
  /// Opaque continuation token bound by the HTTP boundary.
  pub cursor: Option<String>,
}

impl fmt::Debug for KnowledgeViewRequest {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("KnowledgeViewRequest")
      .field("root", &self.root)
      .field("lens", &self.lens)
      .field("target_language", &self.target_language)
      .field("response_level", &self.response_level)
      .field("release", &self.release)
      .field("cursor", &self.cursor.as_ref().map(|_| "REDACTED"))
      .finish()
  }
}

/// Request for bounded verified paths between two canonical nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgePathRequest {
  /// Starting canonical root.
  pub from: KnowledgeRoot,
  /// Destination canonical root.
  pub to: KnowledgeRoot,
  /// Language used for reviewed display material.
  pub target_language: LanguageTag,
  /// Exact authoritative content release and schema.
  pub release: CanonicalReleasePin,
}

/// Public truth class kept separate throughout composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnowledgeEvidenceState {
  /// Published canonical assertion with eligible authoritative evidence.
  Verified,
  /// Request-local explanation grounded in verified material.
  Inferred,
  /// Request-local similarity or model nomination without factual force.
  Exploratory,
}

/// Why an item is useful in a lens without generated relationship prose.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum KnowledgeRelevanceReason {
  /// Directly defines or classifies the root.
  Meaning,
  /// Establishes a reviewed distinction or opposition.
  Contrast,
  /// Shows reviewed usage or construction behavior.
  Usage,
  /// Shows a morphological relationship.
  Form,
  /// Shows reviewed historical origin.
  Origin,
  /// Places the item in an explicit canonical domain.
  Domain,
  /// Participates in an explicit mechanism or process.
  Mechanism,
  /// Shows an explicit application or implementation.
  Application,
}

/// Exact immutable canonical assertion traversal used by a displayed connection.
#[derive(Clone, Eq, PartialEq)]
pub struct VerifiedKnowledgeStep {
  projection: HydratedAssertionProjection,
  release: CanonicalReleasePin,
}

impl fmt::Debug for VerifiedKnowledgeStep {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("VerifiedKnowledgeStep(REDACTED)")
  }
}

impl VerifiedKnowledgeStep {
  /// Creates a step only from an exact canonical hydration result under its full content pin.
  pub fn from_hydrated(
    projection: HydratedAssertionProjection,
    release: CanonicalReleasePin,
  ) -> Result<Self, KnowledgeViewValidationError> {
    let assertion = projection.assertion();
    let traversal = projection.traversal();
    if assertion.release_id != release.release_id
      || assertion.evidence_ids.is_empty()
      || assertion.evidence_ids.len() > MAX_KNOWLEDGE_STEP_EVIDENCE
      || traversal.source == traversal.target
    {
      return Err(KnowledgeViewValidationError::InvalidProof);
    }
    Ok(Self {
      projection,
      release,
    })
  }

  /// Returns the exact canonical hydration proof retained by this step.
  pub fn projection(&self) -> &HydratedAssertionProjection {
    &self.projection
  }

  /// Returns the complete immutable content pin used for hydration.
  pub fn release(&self) -> &CanonicalReleasePin {
    &self.release
  }

  fn source(&self) -> &CanonicalNodeId {
    &self.projection.traversal().source
  }

  fn target(&self) -> &CanonicalNodeId {
    &self.projection.traversal().target
  }

  fn edge_id(&self) -> &CanonicalId {
    &self.projection.traversal().edge_id
  }

  fn assertion_id(&self) -> &CanonicalId {
    &self.projection.traversal().assertion_id
  }
}

/// Explicit verified connection from one displayed node back to the selected root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsefulRootPath {
  /// Root at which this path must terminate.
  pub root: CanonicalNodeId,
  /// Displayed node at which this path begins.
  pub item: CanonicalNodeId,
  /// One to three contiguous verified assertion traversals.
  pub steps: Vec<VerifiedKnowledgeStep>,
}

impl UsefulRootPath {
  /// Validates a short, contiguous, cycle-free path ending at the declared root.
  pub fn validate(&self) -> Result<(), KnowledgeViewValidationError> {
    validate_path(&self.item, &self.root, &self.steps, None)
  }

  fn validate_for(
    &self,
    release: &CanonicalReleasePin,
  ) -> Result<(), KnowledgeViewValidationError> {
    validate_path(&self.item, &self.root, &self.steps, Some(release))
  }
}

/// One item in a validated view superset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeViewItem {
  /// Stable typed canonical identity.
  pub node: CanonicalNodeId,
  /// Stable server-owned branch identity.
  pub branch: KnowledgeRelevanceReason,
  /// Deterministic one-based order within the response.
  pub order: u16,
  /// Truth class preserved by every response projection.
  pub evidence_state: KnowledgeEvidenceState,
  /// Explicit verified path for factual items other than the root.
  pub path_to_root: Option<UsefulRootPath>,
}

/// One deterministic response branch containing an ordered partition of non-root items.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeViewBranch {
  /// Stable one-based branch order.
  pub order: u16,
  /// Server-owned relevance group.
  pub reason: KnowledgeRelevanceReason,
  /// Ordered canonical identities assigned to this branch.
  pub item_ids: Vec<CanonicalNodeId>,
}

/// One validated, release-pinned superset from which response levels are projected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeViewSuperset {
  /// Request whose lens and pin own this result.
  pub request: KnowledgeViewRequest,
  /// Ordered items; the root is always first.
  pub items: Vec<KnowledgeViewItem>,
  /// Ordered branches that partition every non-root item exactly once.
  pub branches: Vec<KnowledgeViewBranch>,
  /// Whether eligible items remain after this page.
  pub truncated: bool,
  /// Opaque next-page state produced after deterministic ordering.
  pub next_cursor: Option<String>,
}

impl KnowledgeViewSuperset {
  /// Validates item identity, order, root reachability, and truth-state separation.
  pub fn validate(&self) -> Result<(), KnowledgeViewValidationError> {
    if self.items.is_empty() || self.items.len() > MAX_KNOWLEDGE_VIEW_ITEMS {
      return Err(KnowledgeViewValidationError::InvalidCount);
    }
    let root = &self.request.root.node;
    let mut identities = BTreeSet::new();
    for (index, item) in self.items.iter().enumerate() {
      if item.order as usize != index + 1
        || item.branch != self.request.lens.relevance_reason()
        || !identities.insert(item.node.clone())
      {
        return Err(KnowledgeViewValidationError::InvalidOrdering);
      }
      if index == 0 {
        if &item.node != root
          || item.evidence_state != KnowledgeEvidenceState::Verified
          || item.path_to_root.is_some()
        {
          return Err(KnowledgeViewValidationError::OrphanItem);
        }
        continue;
      }
      match item.evidence_state {
        KnowledgeEvidenceState::Verified | KnowledgeEvidenceState::Inferred => {
          let path = item
            .path_to_root
            .as_ref()
            .ok_or(KnowledgeViewValidationError::OrphanItem)?;
          if path.item != item.node || &path.root != root {
            return Err(KnowledgeViewValidationError::OrphanItem);
          }
          path.validate_for(&self.request.release)?;
        }
        KnowledgeEvidenceState::Exploratory => {
          return Err(KnowledgeViewValidationError::SimilarityAsProof);
        }
      }
    }
    if self.truncated != self.next_cursor.is_some() {
      return Err(KnowledgeViewValidationError::InvalidCursorState);
    }
    let mut branched = BTreeSet::new();
    for (index, branch) in self.branches.iter().enumerate() {
      if branch.order as usize != index + 1 || branch.item_ids.is_empty() {
        return Err(KnowledgeViewValidationError::InvalidOrdering);
      }
      for id in &branch.item_ids {
        let item = self
          .items
          .iter()
          .skip(1)
          .find(|item| &item.node == id)
          .ok_or(KnowledgeViewValidationError::OrphanItem)?;
        if item.branch != branch.reason || !branched.insert(id.clone()) {
          return Err(KnowledgeViewValidationError::InvalidOrdering);
        }
      }
    }
    if branched.len() != self.items.len().saturating_sub(1) {
      return Err(KnowledgeViewValidationError::OrphanItem);
    }
    Ok(())
  }

  /// Deterministically projects breadth without changing included items' truth or order.
  pub fn project(&self) -> Vec<&KnowledgeViewItem> {
    let limit = match self.request.response_level {
      ResponseLevel::Brief => 8,
      ResponseLevel::Standard => 24,
      ResponseLevel::Full => MAX_KNOWLEDGE_VIEW_ITEMS,
    };
    self.items.iter().take(limit).collect()
  }
}

/// One independently verified path returned between two selected roots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedKnowledgePath {
  /// Deterministic one-based result order.
  pub order: u8,
  /// Contiguous verified assertion traversals.
  pub steps: Vec<VerifiedKnowledgeStep>,
}

/// Normal bounded-search outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KnowledgePathOutcome {
  /// One to three independently verified paths.
  Connected(Vec<VerifiedKnowledgePath>),
  /// A complete bounded eligible search found no verified path.
  NoVerifiedPath,
}

/// Validated path response under one immutable release pin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgePathResult {
  /// Original typed and release-pinned request.
  pub request: KnowledgePathRequest,
  /// Connected paths or the narrow successful-empty outcome.
  pub outcome: KnowledgePathOutcome,
}

impl KnowledgePathResult {
  /// Validates count, ordering, endpoints, cycles, and duplicate paths.
  pub fn validate(&self) -> Result<(), KnowledgeViewValidationError> {
    if self.request.from == self.request.to {
      return Err(KnowledgeViewValidationError::InvalidPath);
    }
    let KnowledgePathOutcome::Connected(paths) = &self.outcome else {
      return Ok(());
    };
    if paths.is_empty() || paths.len() > MAX_KNOWLEDGE_PATHS {
      return Err(KnowledgeViewValidationError::InvalidCount);
    }
    let mut fingerprints = BTreeSet::new();
    for (index, path) in paths.iter().enumerate() {
      if path.order as usize != index + 1 {
        return Err(KnowledgeViewValidationError::InvalidOrdering);
      }
      validate_path(
        &self.request.from.node,
        &self.request.to.node,
        &path.steps,
        Some(&self.request.release),
      )?;
      let fingerprint = path
        .steps
        .iter()
        .map(|step| step.edge_id().clone())
        .collect::<Vec<_>>();
      if !fingerprints.insert(fingerprint) {
        return Err(KnowledgeViewValidationError::DuplicatePath);
      }
    }
    Ok(())
  }
}

/// Closed failures for view and path admission.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgeViewValidationError {
  /// A result exceeded a closed item, path, or hop bound.
  #[error("knowledge result count is invalid")]
  InvalidCount,
  /// A stable revision was zero.
  #[error("knowledge proof revision is invalid")]
  InvalidRevision,
  /// Evidence or endpoint proof was absent or malformed.
  #[error("knowledge connection proof is invalid")]
  InvalidProof,
  /// Stable identities or result ordinals were duplicated or unordered.
  #[error("knowledge result ordering is invalid")]
  InvalidOrdering,
  /// A displayed factual item had no useful explicit path to the root.
  #[error("knowledge view contains an orphan item")]
  OrphanItem,
  /// A connection path was discontinuous or contained a cycle.
  #[error("knowledge path is disconnected or cyclic")]
  InvalidPath,
  /// Two returned paths contained the same assertion projections.
  #[error("knowledge path is duplicated")]
  DuplicatePath,
  /// Similarity or request-local material was used as factual proof.
  #[error("similarity cannot prove a knowledge connection")]
  SimilarityAsProof,
  /// Pagination metadata contradicted whether the result was truncated.
  #[error("knowledge cursor state is inconsistent")]
  InvalidCursorState,
}

fn validate_path(
  start: &CanonicalNodeId,
  end: &CanonicalNodeId,
  steps: &[VerifiedKnowledgeStep],
  expected_release: Option<&CanonicalReleasePin>,
) -> Result<(), KnowledgeViewValidationError> {
  if steps.is_empty() || steps.len() > MAX_KNOWLEDGE_PATH_HOPS {
    return Err(KnowledgeViewValidationError::InvalidCount);
  }
  let mut current = start;
  let mut visited = BTreeSet::from([start.clone()]);
  let mut edges = BTreeSet::new();
  let mut assertions = BTreeSet::new();
  for step in steps {
    if step.release().release_id != step.projection().assertion().release_id
      || expected_release.is_some_and(|expected| step.release() != expected)
      || !edges.insert(step.edge_id().clone())
      || !assertions.insert(step.assertion_id().clone())
    {
      return Err(KnowledgeViewValidationError::InvalidPath);
    }
    current = if step.source() == current {
      step.target()
    } else {
      return Err(KnowledgeViewValidationError::InvalidPath);
    };
    if !visited.insert(current.clone()) {
      return Err(KnowledgeViewValidationError::InvalidPath);
    }
  }
  if current != end {
    return Err(KnowledgeViewValidationError::InvalidPath);
  }
  Ok(())
}

/// Groups validated items by branch without changing deterministic item order.
pub fn group_by_branch(
  items: &[KnowledgeViewItem],
) -> BTreeMap<KnowledgeRelevanceReason, Vec<&KnowledgeViewItem>> {
  let mut grouped = BTreeMap::new();
  for item in items {
    grouped
      .entry(item.branch)
      .or_insert_with(Vec::new)
      .push(item);
  }
  grouped
}

/// Returns whether a family-qualified identity can be represented by hydration's closed catalog.
pub const fn hydration_family_supported(family: CanonicalNodeFamily) -> bool {
  !matches!(family, CanonicalNodeFamily::Lexeme)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn node(value: &str) -> CanonicalNodeId {
    CanonicalNodeId::publisher_assigned(CanonicalNodeFamily::Concept, id(value))
  }

  fn step(from: &str, to: &str, edge: &str) -> VerifiedKnowledgeStep {
    let release = CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap();
    let projection = HydratedAssertionProjection::topology_fixture(
      node(from),
      node(to),
      id(edge),
      id(&format!("a-{edge}")),
      release.release_id.clone(),
    );
    VerifiedKnowledgeStep::from_hydrated(projection, release).unwrap()
  }

  #[test]
  fn lenses_are_closed_and_exact() {
    let names = [
      "meaning",
      "contrast",
      "usage",
      "form",
      "origin",
      "domain",
      "mechanism",
      "application",
    ];
    assert!(names
      .into_iter()
      .all(|name| KnowledgeLens::parse(name).unwrap().as_str() == name));
    assert_eq!(KnowledgeLens::parse("comparison"), None);
  }

  #[test]
  fn verified_view_items_require_a_useful_cycle_free_root_path() {
    let root = node("root");
    let item = node("item");
    let request = KnowledgeViewRequest {
      root: KnowledgeRoot { node: root.clone() },
      lens: KnowledgeLens::Meaning,
      target_language: LanguageTag::parse("en").unwrap(),
      response_level: ResponseLevel::Full,
      release: CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap(),
      cursor: None,
    };
    let valid = KnowledgeViewSuperset {
      request: request.clone(),
      items: vec![
        KnowledgeViewItem {
          node: root.clone(),
          branch: KnowledgeRelevanceReason::Meaning,
          order: 1,
          evidence_state: KnowledgeEvidenceState::Verified,
          path_to_root: None,
        },
        KnowledgeViewItem {
          node: item.clone(),
          branch: KnowledgeRelevanceReason::Meaning,
          order: 2,
          evidence_state: KnowledgeEvidenceState::Verified,
          path_to_root: Some(UsefulRootPath {
            root: root.clone(),
            item: item.clone(),
            steps: vec![step("item", "root", "edge")],
          }),
        },
      ],
      branches: vec![KnowledgeViewBranch {
        order: 1,
        reason: KnowledgeRelevanceReason::Meaning,
        item_ids: vec![item.clone()],
      }],
      truncated: false,
      next_cursor: None,
    };
    assert_eq!(valid.validate(), Ok(()));
    let mut orphan = valid.clone();
    orphan.items[1].path_to_root = None;
    assert_eq!(
      orphan.validate(),
      Err(KnowledgeViewValidationError::OrphanItem)
    );
    let mut exploratory = valid;
    exploratory.items[1].evidence_state = KnowledgeEvidenceState::Exploratory;
    assert_eq!(
      exploratory.validate(),
      Err(KnowledgeViewValidationError::SimilarityAsProof)
    );
  }

  #[test]
  fn grounded_inference_keeps_a_verified_path_and_exploration_is_not_a_view_item() {
    let root = node("root");
    let item = node("item");
    let inferred = KnowledgeViewSuperset {
      request: KnowledgeViewRequest {
        root: KnowledgeRoot { node: root.clone() },
        lens: KnowledgeLens::Mechanism,
        target_language: LanguageTag::parse("en").unwrap(),
        response_level: ResponseLevel::Standard,
        release: CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap(),
        cursor: None,
      },
      items: vec![
        KnowledgeViewItem {
          node: root.clone(),
          branch: KnowledgeRelevanceReason::Mechanism,
          order: 1,
          evidence_state: KnowledgeEvidenceState::Verified,
          path_to_root: None,
        },
        KnowledgeViewItem {
          node: item.clone(),
          branch: KnowledgeRelevanceReason::Mechanism,
          order: 2,
          evidence_state: KnowledgeEvidenceState::Inferred,
          path_to_root: Some(UsefulRootPath {
            root,
            item: item.clone(),
            steps: vec![step("item", "root", "edge")],
          }),
        },
      ],
      branches: vec![KnowledgeViewBranch {
        order: 1,
        reason: KnowledgeRelevanceReason::Mechanism,
        item_ids: vec![item.clone()],
      }],
      truncated: false,
      next_cursor: None,
    };
    assert_eq!(inferred.validate(), Ok(()));
  }

  #[test]
  fn paths_are_bounded_contiguous_unique_and_cycle_free() {
    let request = KnowledgePathRequest {
      from: KnowledgeRoot { node: node("a") },
      to: KnowledgeRoot { node: node("d") },
      target_language: LanguageTag::parse("en").unwrap(),
      release: CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap(),
    };
    let valid = KnowledgePathResult {
      request: request.clone(),
      outcome: KnowledgePathOutcome::Connected(vec![VerifiedKnowledgePath {
        order: 1,
        steps: vec![
          step("a", "b", "e1"),
          step("b", "c", "e2"),
          step("c", "d", "e3"),
        ],
      }]),
    };
    assert_eq!(valid.validate(), Ok(()));
    let duplicate_proof = KnowledgePathResult {
      request: request.clone(),
      outcome: KnowledgePathOutcome::Connected(vec![VerifiedKnowledgePath {
        order: 1,
        steps: vec![
          step("a", "b", "reused"),
          step("b", "c", "reused"),
          step("c", "d", "other"),
        ],
      }]),
    };
    assert_eq!(
      duplicate_proof.validate(),
      Err(KnowledgeViewValidationError::InvalidPath)
    );
    let cyclic = KnowledgePathResult {
      request,
      outcome: KnowledgePathOutcome::Connected(vec![VerifiedKnowledgePath {
        order: 1,
        steps: vec![
          step("a", "b", "e1"),
          step("b", "a", "e2"),
          step("a", "d", "e3"),
        ],
      }]),
    };
    assert_eq!(
      cyclic.validate(),
      Err(KnowledgeViewValidationError::InvalidPath)
    );
  }

  #[test]
  fn directed_steps_cannot_be_traversed_in_reverse() {
    let result = KnowledgePathResult {
      request: KnowledgePathRequest {
        from: KnowledgeRoot {
          node: node("target"),
        },
        to: KnowledgeRoot {
          node: node("source"),
        },
        target_language: LanguageTag::parse("en").unwrap(),
        release: CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap(),
      },
      outcome: KnowledgePathOutcome::Connected(vec![VerifiedKnowledgePath {
        order: 1,
        steps: vec![step("source", "target", "directed-edge")],
      }]),
    };
    assert_eq!(
      result.validate(),
      Err(KnowledgeViewValidationError::InvalidPath)
    );
  }

  #[test]
  fn debug_output_redacts_hydrated_content_and_cursor_state() {
    let step = step("source", "target", "edge");
    let request = KnowledgeViewRequest {
      root: KnowledgeRoot {
        node: node("source"),
      },
      lens: KnowledgeLens::Meaning,
      target_language: LanguageTag::parse("en").unwrap(),
      response_level: ResponseLevel::Brief,
      release: CanonicalReleasePin::new(id("release"), "canonical-v1".into()).unwrap(),
      cursor: Some("private-cursor-value".into()),
    };
    assert_eq!(format!("{step:?}"), "VerifiedKnowledgeStep(REDACTED)");
    let request_debug = format!("{request:?}");
    assert!(request_debug.contains("REDACTED"));
    assert!(!request_debug.contains("private-cursor-value"));
  }
}
