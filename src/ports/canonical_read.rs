//! Request-scoped canonical-only reads independent of island-port transport details.

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
  canonical::{CanonicalReleasePin, EvidenceUse, LanguageTag, SenseId},
  canonical_content::CanonicalSenseDetails,
  canonical_translation::{CanonicalTranslationRevision, DomainId, SourceFingerprint},
  domain_assessment::DomainInventory,
  knowledge_hydration::{
    CanonicalAssertionProjectionRef, HydratedAssertionProjection, HydratedKnowledgeNode,
    HydratedSemanticScale,
  },
  retrieval::{LexicalMatchKind, RepositoryMatch},
  retrieval_data::RetrievalVerificationState,
};

/// Request-scoped correlation and deadline carried through one canonical read flow.
#[derive(Clone)]
pub struct CanonicalReadContext {
  /// Opaque correlation identifier; never contains request text.
  pub request_id: String,
  /// Absolute RFC 3339 deadline shared by every read in the flow.
  pub deadline_at: String,
  /// Maximum per-call duration, further bounded by the absolute deadline.
  pub timeout: Duration,
}

/// Redacted closed failure from a canonical authority.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum CanonicalReadError {
  /// Request context or bounded input was invalid.
  #[error("canonical read request is invalid")]
  InvalidRequest,
  /// The requested pinned value is absent.
  #[error("canonical value was not found")]
  NotFound,
  /// An explicitly pinned immutable content release is no longer readable.
  #[error("canonical content release is unavailable")]
  ContentReleaseUnavailable,
  /// The peer does not implement the required adapter or canonical schema.
  #[error("canonical read schema is incompatible")]
  SchemaIncompatible,
  /// The canonical authority is unavailable.
  #[error("canonical authority is unavailable")]
  Unavailable,
  /// The shared deadline has elapsed.
  #[error("canonical read timed out")]
  Timeout,
  /// The authority supplied contradictory or malformed data.
  #[error("canonical authority returned inconsistent data")]
  InconsistentData,
}

/// Selectors for one reviewed translation candidate query.
pub struct CanonicalTranslationQuery {
  /// Versioned candidate fingerprint, not a proof of source equality.
  pub source_fingerprint: SourceFingerprint,
  /// Source language.
  pub source_language: LanguageTag,
  /// Target language.
  pub target_language: LanguageTag,
  /// Optional sense constraint.
  pub sense_id: Option<SenseId>,
  /// Bounded canonical domain constraints.
  pub domain_ids: Vec<DomainId>,
  /// Optional dialect constraint.
  pub dialect: Option<LanguageTag>,
  /// Optional register constraint.
  pub register: Option<String>,
  /// Maximum candidate count.
  pub limit: usize,
}

/// One request-local derived form used only for retrieval.
pub struct CanonicalLookupForm {
  /// Derived lookup spelling.
  pub form: String,
  /// Closed match class.
  pub match_class: LexicalMatchKind,
  /// Deterministic form precedence.
  pub rank: usize,
}

/// Selectors for release-pinned lexical candidate resolution.
pub struct CanonicalCandidateQuery {
  /// Bounded request-local forms.
  pub lookup_forms: Vec<CanonicalLookupForm>,
  /// Request normalization version.
  pub normalizer_version: String,
  /// Source language.
  pub source_language: LanguageTag,
  /// Explanation language.
  pub explanation_language: LanguageTag,
  /// Optional dialect.
  pub dialect: Option<LanguageTag>,
  /// Required source permission operation.
  pub evidence_use: EvidenceUse,
  /// Maximum candidate count.
  pub limit: usize,
}

/// Selectors for one reviewed sense-detail aggregate.
pub struct CanonicalSenseQuery {
  /// Stable sense identity.
  pub sense_id: SenseId,
  /// Explanation language.
  pub explanation_language: LanguageTag,
  /// Optional pronunciation dialect.
  pub dialect: Option<LanguageTag>,
  /// Required source permission operation.
  pub evidence_use: EvidenceUse,
}

/// Bounded release-pinned selectors for the canonical domain inventory.
pub struct CanonicalDomainQuery {
  /// Normalized multilingual labels that nominated domain assessment.
  pub normalized_labels: Vec<String>,
  /// Optional stable request-local scope discriminator.
  pub scope_key: Option<String>,
  /// Languages useful for labels and RAG coverage.
  pub languages: Vec<LanguageTag>,
  /// Maximum canonical records returned; application validation applies the domain inventory cap.
  pub limit: usize,
}

/// Exact authoritative assertion projections selected by retrieval.
pub struct CanonicalAssertionQuery {
  /// Ordered exact projections; omitted assertions preserve the relative request order.
  pub projections: Vec<CanonicalAssertionProjectionRef>,
  /// Evidence operation every returned assertion lineage must permit.
  pub evidence_use: EvidenceUse,
  /// Eligible authority verification states.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Maximum assertions returned.
  pub limit: usize,
}

/// Complete semantic scales selected by retrieval.
pub struct CanonicalScaleQuery {
  /// Ordered stable scale identities.
  pub scale_ids: Vec<crate::domain::canonical::CanonicalId>,
  /// Canonical member for which scope and conditions must be eligible.
  pub for_node_id: crate::domain::canonical::CanonicalId,
  /// Eligible authority verification states.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Maximum complete scales returned.
  pub limit: usize,
}

/// Authoritative display-safe values for nominated knowledge nodes.
pub struct CanonicalKnowledgeNodeQuery {
  /// Ordered stable node identities.
  pub node_ids: Vec<crate::domain::canonical::CanonicalId>,
  /// Required source permission operation.
  pub evidence_use: EvidenceUse,
  /// Maximum nodes returned.
  pub limit: usize,
}

/// Read-only authority capability; every downstream call accepts the same explicit pin.
#[async_trait]
pub trait CanonicalReadPort: Send + Sync {
  /// Selects the active canonical-only release once for a new request.
  async fn active_release(
    &self,
    context: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError>;

  /// Returns bounded fingerprint candidates without deciding source equality.
  async fn translations(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalTranslationQuery,
  ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError>;

  /// Returns bounded authoritative lexical candidates without ranking them.
  async fn candidates(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalCandidateQuery,
  ) -> Result<Vec<RepositoryMatch>, CanonicalReadError>;

  /// Returns one complete evidence-backed canonical sense aggregate.
  async fn sense(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalSenseQuery,
  ) -> Result<CanonicalSenseDetails, CanonicalReadError>;

  /// Returns the bounded canonical domain inventory and explicit catalog-completeness signal.
  ///
  /// The default preserves compatibility for authorities that have not implemented domain reads;
  /// application assessment maps that failure to `uncertain` rather than inferring novelty.
  async fn domains(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalDomainQuery,
  ) -> Result<DomainInventory, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  /// Hydrates ordered exact assertion revisions and validates the selected traversal proof.
  async fn canonical_assertions(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalAssertionQuery,
  ) -> Result<Vec<HydratedAssertionProjection>, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  /// Hydrates complete semantic scales and never reconstructs them from pairwise edges.
  async fn semantic_scales(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalScaleQuery,
  ) -> Result<Vec<HydratedSemanticScale>, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }

  /// Hydrates authoritative display-safe values for nominated canonical nodes.
  async fn knowledge_nodes(
    &self,
    _context: &CanonicalReadContext,
    _pin: &CanonicalReleasePin,
    _query: CanonicalKnowledgeNodeQuery,
  ) -> Result<Vec<HydratedKnowledgeNode>, CanonicalReadError> {
    Err(CanonicalReadError::SchemaIncompatible)
  }
}
