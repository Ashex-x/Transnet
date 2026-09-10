//! Immutable content-release staging, validation, publication, and rollback invariants.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::domain::canonical::{
  ActiveContentVersion, LexiconRelease, ReleaseId, ReleaseStatus, SourceId,
};

/// A mandatory independently evaluated gate for publishing canonical content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentReleaseGate {
  /// Structural links, release lineage, and canonical entity invariants were checked.
  StructuralIntegrity,
  /// Every intended assertion use is permitted by its source-policy snapshot.
  SourcePermissions,
  /// The derived vector collection passed its configured reconciliation check for the staged release.
  VectorReconciliation,
  /// Retrieval and teaching-quality evaluation met the configured release threshold.
  RetrievalEvaluation,
  /// Required human review completed for the proposed content scope.
  HumanReview,
  /// Safety, licensing-removal, and policy checks permit serving the release.
  Safety,
}

/// Result recorded for one required content-release gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentReleaseGateResult {
  /// The gate passed for the staged immutable content.
  Passed,
  /// The gate did not pass and publication must remain blocked.
  Failed,
}

/// Aggregate state of all required content-release gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentReleaseValidationState {
  /// At least one required gate has not yet reported a result.
  Pending,
  /// Every required gate reported a passing result.
  Passed,
  /// At least one required gate reported a failing result.
  Failed,
}

/// Explicit policy that names every gate required to publish one release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentReleaseGatePolicy {
  required_gates: BTreeSet<ContentReleaseGate>,
}

impl ContentReleaseGatePolicy {
  /// Creates the baseline policy used until a narrower, approved release policy is configured.
  ///
  /// The policy deliberately requires both vector reconciliation and human review. A production
  /// deployment may add policy enforcement around this value but must not publish by omitting
  /// gates accidentally.
  pub fn strict() -> Self {
    Self {
      required_gates: [
        ContentReleaseGate::StructuralIntegrity,
        ContentReleaseGate::SourcePermissions,
        ContentReleaseGate::VectorReconciliation,
        ContentReleaseGate::RetrievalEvaluation,
        ContentReleaseGate::HumanReview,
        ContentReleaseGate::Safety,
      ]
      .into_iter()
      .collect(),
    }
  }

  /// Creates an explicit nonempty set of gates required for a release.
  ///
  /// # Errors
  ///
  /// Returns an error when no required gate is supplied.
  pub fn new(
    required_gates: impl IntoIterator<Item = ContentReleaseGate>,
  ) -> Result<Self, ContentReleaseValidationError> {
    let required_gates = required_gates.into_iter().collect::<BTreeSet<_>>();
    if required_gates.is_empty() {
      return Err(ContentReleaseValidationError::NoRequiredGates);
    }

    Ok(Self { required_gates })
  }

  /// Returns the immutable set of gates required by this policy.
  pub fn required_gates(&self) -> &BTreeSet<ContentReleaseGate> {
    &self.required_gates
  }

  /// Returns whether `gate` must pass before publication.
  pub fn requires(&self, gate: ContentReleaseGate) -> bool {
    self.required_gates.contains(&gate)
  }
}

/// Validation failure while constructing or transitioning a content release.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ContentReleaseValidationError {
  /// The source manifest hash was blank after trimming surrounding whitespace.
  #[error("content release source manifest hash must not be blank")]
  BlankSourceManifestHash,
  /// The canonical schema version was blank after trimming surrounding whitespace.
  #[error("content release schema version must not be blank")]
  BlankSchemaVersion,
  /// The deterministic ranking version was blank after trimming surrounding whitespace.
  #[error("content release ranking version must not be blank")]
  BlankRankingVersion,
  /// The staged release does not match the release in its pinned active content version.
  #[error("staged release ID must match the pinned active content release ID")]
  ReleaseIdMismatch,
  /// No source-policy records were declared for a staged release.
  #[error("content release must declare at least one source")]
  NoSources,
  /// No validation gate was required for a staged release.
  #[error("content release must require at least one validation gate")]
  NoRequiredGates,
  /// A result was recorded for a gate that this release does not require.
  #[error("content release gate is not required by this release")]
  GateNotRequired,
  /// A staging-only operation was attempted after the release left staging.
  #[error("content release is no longer in staging")]
  ReleaseNotStaging,
  /// An operation required a retired release that can serve as a rollback target.
  #[error("content release is not a retained rollback target")]
  ReleaseNotRetired,
  /// An operation required the release selected by the active-content pointer.
  #[error("content release is not the published active release")]
  ReleaseNotPublished,
  /// A vector build was started more than once for an immutable release.
  #[error("content release already has a vector build")]
  VectorBuildAlreadyStarted,
  /// A vector reconciliation was reported before starting its immutable vector build.
  #[error("content release does not have a vector build")]
  VectorBuildNotStarted,
  /// A vector build was reconciled after it was already marked ready.
  #[error("content release vector build is already ready")]
  VectorBuildAlreadyReady,
  /// A vector build did not pin exactly the staged active-content tuple.
  #[error("vector build does not match the staged active content version")]
  VectorBindingMismatch,
  /// A vector build had no expected records to reconcile.
  #[error("vector build must expect at least one record")]
  ZeroExpectedVectorRecords,
  /// The vector collection record count did not equal the expected canonical count.
  #[error("vector reconciliation mismatch: expected {expected} records, found {indexed}")]
  VectorReconciliationMismatch {
    /// Expected derived vector record count from the staged canonical release.
    expected: u64,
    /// Observed record count in the immutable vector collection.
    indexed: u64,
  },
  /// Publication was attempted before every required gate passed.
  #[error("content release still has pending validation gates")]
  RequiredGatesPending,
  /// Publication was attempted while at least one required gate was failing.
  #[error("content release has a failed validation gate")]
  RequiredGateFailed,
  /// Publication was attempted before the pinned vector collection was reconciled and ready.
  #[error("content release vector build is not ready")]
  VectorBuildNotReady,
}

/// Immutable staging input for one lexical release and its compatible vector pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentReleaseDraft {
  release: LexiconRelease,
  active_content: ActiveContentVersion,
  source_ids: BTreeSet<SourceId>,
  gate_policy: ContentReleaseGatePolicy,
}

impl ContentReleaseDraft {
  /// Creates a staging draft pinned to one exact lexical, vector, schema, and ranking tuple.
  ///
  /// The caller supplies an already-computed source manifest hash and source-policy identifiers.
  /// This type deliberately does not accept source text, embeddings, or generated material.
  ///
  /// # Errors
  ///
  /// Returns an error when required identifiers, versions, sources, or gate policy are invalid.
  pub fn new(
    release_id: ReleaseId,
    source_manifest_hash: impl Into<String>,
    active_content: ActiveContentVersion,
    source_ids: impl IntoIterator<Item = SourceId>,
    gate_policy: ContentReleaseGatePolicy,
  ) -> Result<Self, ContentReleaseValidationError> {
    let source_manifest_hash = source_manifest_hash.into();
    if source_manifest_hash.trim().is_empty() {
      return Err(ContentReleaseValidationError::BlankSourceManifestHash);
    }
    validate_active_content_version(&active_content)?;
    if active_content.release_id != release_id {
      return Err(ContentReleaseValidationError::ReleaseIdMismatch);
    }

    let source_ids = source_ids.into_iter().collect::<BTreeSet<_>>();
    if source_ids.is_empty() {
      return Err(ContentReleaseValidationError::NoSources);
    }

    Ok(Self {
      release: LexiconRelease {
        id: release_id,
        source_manifest_hash,
        status: ReleaseStatus::Staging,
        rollback_predecessor: None,
      },
      active_content,
      source_ids,
      gate_policy,
    })
  }

  /// Returns the immutable lexical release identifier reserved by this draft.
  pub fn release_id(&self) -> &ReleaseId {
    &self.release.id
  }

  /// Returns the complete active-content tuple the release may publish after validation.
  pub fn active_content_version(&self) -> &ActiveContentVersion {
    &self.active_content
  }

  /// Returns every source-policy record declared by the release manifest.
  pub fn source_ids(&self) -> &BTreeSet<SourceId> {
    &self.source_ids
  }

  /// Returns the validation policy frozen into this staging draft.
  pub fn gate_policy(&self) -> &ContentReleaseGatePolicy {
    &self.gate_policy
  }

  /// Converts the validated draft into its mutable staging aggregate.
  pub fn into_release(self) -> ContentRelease {
    ContentRelease {
      release: self.release,
      active_content: self.active_content,
      source_ids: self.source_ids,
      gate_policy: self.gate_policy,
      gate_results: BTreeMap::new(),
      vector_build: None,
    }
  }
}

/// Lifecycle metadata for count reconciliation of an immutable derived vector collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorCollectionBuild {
  active_content: ActiveContentVersion,
  expected_records: u64,
  indexed_records: Option<u64>,
  state: VectorCollectionBuildState,
}

impl VectorCollectionBuild {
  /// Starts a build for exactly one active-content tuple and expected record count.
  ///
  /// The build metadata is an attestation boundary: a production worker performs embeddings plus
  /// record-identity and content-hash reconciliation outside this domain model, then records the
  /// bounded count. This type prevents a count mismatch but does not inspect vector payloads.
  ///
  /// # Errors
  ///
  /// Returns an error when the tuple has blank versions or no records are expected.
  pub fn new(
    active_content: ActiveContentVersion,
    expected_records: u64,
  ) -> Result<Self, ContentReleaseValidationError> {
    validate_active_content_version(&active_content)?;
    if expected_records == 0 {
      return Err(ContentReleaseValidationError::ZeroExpectedVectorRecords);
    }

    Ok(Self {
      active_content,
      expected_records,
      indexed_records: None,
      state: VectorCollectionBuildState::Building,
    })
  }

  /// Returns the exact lexical, vector, schema, and ranking tuple this build is pinned to.
  pub fn active_content_version(&self) -> &ActiveContentVersion {
    &self.active_content
  }

  /// Returns the canonical record count the collection must contain before publication.
  pub fn expected_records(&self) -> u64 {
    self.expected_records
  }

  /// Returns the reconciled vector record count after a successful reconciliation.
  pub fn indexed_records(&self) -> Option<u64> {
    self.indexed_records
  }

  /// Returns the lifecycle state of this derived vector collection.
  pub fn state(&self) -> VectorCollectionBuildState {
    self.state
  }

  /// Records the expected-count portion of completed vector reconciliation for this collection.
  ///
  /// # Errors
  ///
  /// Returns an error when the build is already ready or `indexed_records` differs from the
  /// expected canonical count. Callers must perform record-identity and content-hash verification
  /// before this attestation; a mismatch leaves the build unready so a worker can inspect or
  /// rebuild the collection without publication becoming possible.
  pub fn reconcile(&mut self, indexed_records: u64) -> Result<(), ContentReleaseValidationError> {
    if self.state == VectorCollectionBuildState::Ready {
      return Err(ContentReleaseValidationError::VectorBuildAlreadyReady);
    }
    if indexed_records != self.expected_records {
      return Err(
        ContentReleaseValidationError::VectorReconciliationMismatch {
          expected: self.expected_records,
          indexed: indexed_records,
        },
      );
    }

    self.indexed_records = Some(indexed_records);
    self.state = VectorCollectionBuildState::Ready;
    Ok(())
  }
}

/// State of an immutable vector collection build associated with a staged release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorCollectionBuildState {
  /// The collection is being built or awaits a successful reconciliation count.
  Building,
  /// The exact collection count reconciled and may participate in publication gating.
  Ready,
}

/// Mutable staging aggregate that becomes an immutable published content release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRelease {
  release: LexiconRelease,
  active_content: ActiveContentVersion,
  source_ids: BTreeSet<SourceId>,
  gate_policy: ContentReleaseGatePolicy,
  gate_results: BTreeMap<ContentReleaseGate, ContentReleaseGateResult>,
  vector_build: Option<VectorCollectionBuild>,
}

impl ContentRelease {
  /// Returns immutable lexical-release metadata and lifecycle status.
  pub fn release(&self) -> &LexiconRelease {
    &self.release
  }

  /// Returns the release's stable lexical-content identifier.
  pub fn id(&self) -> &ReleaseId {
    &self.release.id
  }

  /// Returns the exact compatible canonical and vector tuple selected on publication.
  pub fn active_content_version(&self) -> &ActiveContentVersion {
    &self.active_content
  }

  /// Returns source-policy identifiers declared by the immutable source manifest.
  pub fn source_ids(&self) -> &BTreeSet<SourceId> {
    &self.source_ids
  }

  /// Returns whether this release has any content derived from `source_id`.
  pub fn uses_source(&self, source_id: &SourceId) -> bool {
    self.source_ids.contains(source_id)
  }

  /// Returns the validation policy frozen into the staging release.
  pub fn gate_policy(&self) -> &ContentReleaseGatePolicy {
    &self.gate_policy
  }

  /// Returns the current result for one required gate, when it has been recorded.
  pub fn gate_result(&self, gate: ContentReleaseGate) -> Option<ContentReleaseGateResult> {
    self.gate_results.get(&gate).copied()
  }

  /// Returns whether required gate results are pending, passing, or failing.
  pub fn validation_state(&self) -> ContentReleaseValidationState {
    if self
      .gate_policy
      .required_gates()
      .iter()
      .any(|gate| self.gate_result(*gate) == Some(ContentReleaseGateResult::Failed))
    {
      return ContentReleaseValidationState::Failed;
    }
    if self
      .gate_policy
      .required_gates()
      .iter()
      .all(|gate| self.gate_result(*gate) == Some(ContentReleaseGateResult::Passed))
    {
      ContentReleaseValidationState::Passed
    } else {
      ContentReleaseValidationState::Pending
    }
  }

  /// Returns the current derived vector build, when one has been started.
  pub fn vector_build(&self) -> Option<&VectorCollectionBuild> {
    self.vector_build.as_ref()
  }

  /// Records the latest result for one mandatory validation gate while the release is staging.
  ///
  /// A failed result may later be replaced with a passing result after a corrected immutable
  /// staging import is re-evaluated. Published and retained releases never accept new results.
  ///
  /// # Errors
  ///
  /// Returns an error when the release left staging or the gate is not required by its policy.
  pub fn record_gate_result(
    &mut self,
    gate: ContentReleaseGate,
    result: ContentReleaseGateResult,
  ) -> Result<(), ContentReleaseValidationError> {
    self.ensure_staging()?;
    if !self.gate_policy.requires(gate) {
      return Err(ContentReleaseValidationError::GateNotRequired);
    }

    self.gate_results.insert(gate, result);
    Ok(())
  }

  /// Starts the one immutable vector build compatible with this staging release.
  ///
  /// # Errors
  ///
  /// Returns an error when the release left staging, already has a build, or the supplied vector
  /// tuple differs from the staged canonical, vector, schema, or ranking tuple.
  pub fn begin_vector_build(
    &mut self,
    build: VectorCollectionBuild,
  ) -> Result<(), ContentReleaseValidationError> {
    self.ensure_staging()?;
    if self.vector_build.is_some() {
      return Err(ContentReleaseValidationError::VectorBuildAlreadyStarted);
    }
    if build.active_content_version() != self.active_content_version() {
      return Err(ContentReleaseValidationError::VectorBindingMismatch);
    }

    self.vector_build = Some(build);
    Ok(())
  }

  /// Records the reconciled vector record count for the pinned immutable collection.
  ///
  /// # Errors
  ///
  /// Returns an error when no build exists, the release left staging, or the count is not exact.
  pub fn reconcile_vector_build(
    &mut self,
    indexed_records: u64,
  ) -> Result<(), ContentReleaseValidationError> {
    self.ensure_staging()?;
    let Some(vector_build) = self.vector_build.as_mut() else {
      return Err(ContentReleaseValidationError::VectorBuildNotStarted);
    };
    vector_build.reconcile(indexed_records)
  }

  /// Returns whether this staging release can atomically replace the active-content pointer.
  ///
  /// # Errors
  ///
  /// Returns an error unless the release is staging, every required gate passed, and its vector
  /// build recorded a matching expected count after the configured reconciliation gate passed.
  pub fn ensure_publishable(&self) -> Result<(), ContentReleaseValidationError> {
    self.ensure_staging()?;
    self.ensure_validated_pair()
  }

  /// Marks a publishable staging release as the active immutable lexical release.
  ///
  /// # Errors
  ///
  /// Returns an error unless the release remains fully publishable at this transition point.
  pub(crate) fn mark_published(
    &mut self,
    rollback_predecessor: Option<ReleaseId>,
  ) -> Result<ActiveContentVersion, ContentReleaseValidationError> {
    self.ensure_publishable()?;
    self.release.status = ReleaseStatus::Published;
    self.release.rollback_predecessor = rollback_predecessor;
    Ok(self.active_content.clone())
  }

  /// Restores this retained release as the active release after repository-level rollback checks.
  ///
  /// # Errors
  ///
  /// Returns an error unless the release is retained and still has its validated vector pair.
  pub(crate) fn restore_for_rollback(
    &mut self,
  ) -> Result<ActiveContentVersion, ContentReleaseValidationError> {
    if self.release.status != ReleaseStatus::Retired {
      return Err(ContentReleaseValidationError::ReleaseNotRetired);
    }
    self.ensure_validated_pair()?;
    self.release.status = ReleaseStatus::Published;
    Ok(self.active_content.clone())
  }

  /// Retires the current active release while retaining it only for rollback and history.
  ///
  /// # Errors
  ///
  /// Returns an error unless the release is currently published.
  pub(crate) fn retire(&mut self) -> Result<(), ContentReleaseValidationError> {
    if self.release.status != ReleaseStatus::Published {
      return Err(ContentReleaseValidationError::ReleaseNotPublished);
    }
    self.release.status = ReleaseStatus::Retired;
    Ok(())
  }

  fn ensure_staging(&self) -> Result<(), ContentReleaseValidationError> {
    if self.release.status == ReleaseStatus::Staging {
      Ok(())
    } else {
      Err(ContentReleaseValidationError::ReleaseNotStaging)
    }
  }

  fn ensure_validated_pair(&self) -> Result<(), ContentReleaseValidationError> {
    match self.validation_state() {
      ContentReleaseValidationState::Pending => {
        return Err(ContentReleaseValidationError::RequiredGatesPending);
      }
      ContentReleaseValidationState::Failed => {
        return Err(ContentReleaseValidationError::RequiredGateFailed);
      }
      ContentReleaseValidationState::Passed => {}
    }
    if !self
      .vector_build
      .as_ref()
      .is_some_and(|build| build.state() == VectorCollectionBuildState::Ready)
    {
      return Err(ContentReleaseValidationError::VectorBuildNotReady);
    }

    Ok(())
  }
}

/// Administrative impact reported when a source is permanently quarantined for removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRemovalImpact {
  source_id: SourceId,
  active_release_id: Option<ReleaseId>,
  staging_release_ids: Vec<ReleaseId>,
  retained_release_ids: Vec<ReleaseId>,
}

impl SourceRemovalImpact {
  /// Creates one source-removal impact report from release metadata.
  pub(crate) fn new(
    source_id: SourceId,
    active_release_id: Option<ReleaseId>,
    staging_release_ids: Vec<ReleaseId>,
    retained_release_ids: Vec<ReleaseId>,
  ) -> Self {
    Self {
      source_id,
      active_release_id,
      staging_release_ids,
      retained_release_ids,
    }
  }

  /// Returns the source-policy identifier that was quarantined.
  pub fn source_id(&self) -> &SourceId {
    &self.source_id
  }

  /// Returns the active release affected by the quarantine, when one is currently unsafe.
  pub fn active_release_id(&self) -> Option<&ReleaseId> {
    self.active_release_id.as_ref()
  }

  /// Returns staged releases that must not continue to publication with this source.
  pub fn staging_release_ids(&self) -> &[ReleaseId] {
    &self.staging_release_ids
  }

  /// Returns retained releases that must never become a rollback target with this source.
  pub fn retained_release_ids(&self) -> &[ReleaseId] {
    &self.retained_release_ids
  }
}

fn validate_active_content_version(
  active_content: &ActiveContentVersion,
) -> Result<(), ContentReleaseValidationError> {
  if active_content.schema_version.trim().is_empty() {
    return Err(ContentReleaseValidationError::BlankSchemaVersion);
  }
  if active_content.ranking_version.trim().is_empty() {
    return Err(ContentReleaseValidationError::BlankRankingVersion);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::CanonicalId;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn content(release_id: &str, vector_collection_id: &str) -> ActiveContentVersion {
    ActiveContentVersion {
      release_id: id(release_id),
      vector_collection_id: id(vector_collection_id),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "lookup-rank-v1".to_string(),
    }
  }

  fn draft(release_id: &str) -> ContentReleaseDraft {
    ContentReleaseDraft::new(
      id(release_id),
      "manifest-sha256",
      content(release_id, "vectors-1"),
      [id("source-1")],
      ContentReleaseGatePolicy::strict(),
    )
    .unwrap()
  }

  #[test]
  fn draft_rejects_mismatched_content_tuple_and_empty_policy() {
    let mismatched = ContentReleaseDraft::new(
      id("release-1"),
      "manifest-sha256",
      content("release-2", "vectors-1"),
      [id("source-1")],
      ContentReleaseGatePolicy::strict(),
    );
    assert_eq!(
      mismatched.unwrap_err(),
      ContentReleaseValidationError::ReleaseIdMismatch
    );

    assert_eq!(
      ContentReleaseGatePolicy::new([]).unwrap_err(),
      ContentReleaseValidationError::NoRequiredGates
    );
  }

  #[test]
  fn release_requires_all_gates_and_an_expected_vector_record_count() {
    let mut release = draft("release-1").into_release();
    assert_eq!(
      release.ensure_publishable(),
      Err(ContentReleaseValidationError::RequiredGatesPending)
    );

    for gate in release.gate_policy().required_gates().clone() {
      release
        .record_gate_result(gate, ContentReleaseGateResult::Passed)
        .unwrap();
    }
    let build = VectorCollectionBuild::new(content("release-1", "vectors-1"), 4).unwrap();
    release.begin_vector_build(build).unwrap();
    assert_eq!(
      release.reconcile_vector_build(3),
      Err(
        ContentReleaseValidationError::VectorReconciliationMismatch {
          expected: 4,
          indexed: 3,
        }
      )
    );
    assert_eq!(
      release.ensure_publishable(),
      Err(ContentReleaseValidationError::VectorBuildNotReady)
    );

    release.reconcile_vector_build(4).unwrap();
    assert_eq!(
      release.validation_state(),
      ContentReleaseValidationState::Passed
    );
    assert_eq!(release.ensure_publishable(), Ok(()));
  }

  #[test]
  fn published_release_refuses_staging_mutations() {
    let mut release = draft("release-1").into_release();
    for gate in release.gate_policy().required_gates().clone() {
      release
        .record_gate_result(gate, ContentReleaseGateResult::Passed)
        .unwrap();
    }
    release
      .begin_vector_build(VectorCollectionBuild::new(content("release-1", "vectors-1"), 1).unwrap())
      .unwrap();
    release.reconcile_vector_build(1).unwrap();
    release.mark_published(None).unwrap();

    assert_eq!(
      release.record_gate_result(ContentReleaseGate::Safety, ContentReleaseGateResult::Passed,),
      Err(ContentReleaseValidationError::ReleaseNotStaging)
    );
  }
}
