//! Public-only canonical lookup snapshot cache keys, values, and privacy boundaries.

use std::{
  collections::BTreeSet,
  fmt,
  hash::{Hash, Hasher},
};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
  canonical::{ActiveContentVersion, EvidenceUse, LanguageTag},
  retrieval::{RankedCandidate, RetrievalPath, RetrievalRequest, RetrievalValidationError},
};

/// Validation failure while constructing a public canonical lookup cache contract.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CanonicalLookupCacheValidationError {
  /// A cache-affecting policy version was blank after trimming surrounding whitespace.
  #[error("canonical-card policy version must not be blank")]
  BlankPolicyVersion,
  /// The canonical schema version in an active content tuple was blank.
  #[error("active canonical schema version must not be blank")]
  BlankSchemaVersion,
  /// The deterministic ranking version in an active content tuple was blank.
  #[error("active canonical ranking version must not be blank")]
  BlankRankingVersion,
  /// The source retrieval request was invalid for a bounded public lookup.
  #[error(transparent)]
  Retrieval(#[from] RetrievalValidationError),
  /// A snapshot candidate exceeded the candidate limit encoded in its cache key.
  #[error("canonical lookup snapshot exceeds its cache-key candidate limit")]
  CandidateLimitExceeded,
  /// Snapshot candidates were not in their deterministic zero-free rank sequence.
  #[error("canonical lookup snapshot has a non-deterministic rank sequence")]
  InvalidRankSequence,
  /// The same canonical sense appeared more than once in one snapshot.
  #[error("canonical lookup snapshot has duplicate canonical senses")]
  DuplicateSense,
  /// A candidate did not match the key's language, release, lifecycle, or public source policy.
  #[error("canonical lookup snapshot contains a candidate ineligible for public caching")]
  IneligibleCandidate,
}

/// One nonblank implementation or policy version that changes a canonical card.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalCardPolicyVersion(String);

impl CanonicalCardPolicyVersion {
  /// Creates a policy version after trimming surrounding whitespace.
  ///
  /// # Errors
  ///
  /// Returns an error when `value` is blank.
  pub fn new(value: impl AsRef<str>) -> Result<Self, CanonicalLookupCacheValidationError> {
    let value = value.as_ref().trim();
    if value.is_empty() {
      return Err(CanonicalLookupCacheValidationError::BlankPolicyVersion);
    }
    Ok(Self(value.to_string()))
  }

  /// Returns the stable policy-version label.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

/// Retrieval and presentation policy versions that affect a public canonical card.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalCardPolicyVersions {
  retrieval_policy_version: CanonicalCardPolicyVersion,
  presentation_policy_version: CanonicalCardPolicyVersion,
}

impl CanonicalCardPolicyVersions {
  /// Creates explicit versions for retrieval and public-card presentation policy.
  ///
  /// Retrieval policy covers query handling, candidate selection, and fusion behavior.
  /// Presentation policy covers public-card fields, attribution, and global display policy. A
  /// requester-specific mature-content choice is not a presentation policy: it must bypass this
  /// shared cache through [`CanonicalLookupCacheEligibility`].
  ///
  /// # Errors
  ///
  /// Returns an error when either version is blank.
  pub fn new(
    retrieval_policy_version: impl AsRef<str>,
    presentation_policy_version: impl AsRef<str>,
  ) -> Result<Self, CanonicalLookupCacheValidationError> {
    Ok(Self {
      retrieval_policy_version: CanonicalCardPolicyVersion::new(retrieval_policy_version)?,
      presentation_policy_version: CanonicalCardPolicyVersion::new(presentation_policy_version)?,
    })
  }

  /// Returns the version of retrieval policy represented by this cache contract.
  pub fn retrieval_policy_version(&self) -> &CanonicalCardPolicyVersion {
    &self.retrieval_policy_version
  }

  /// Returns the version of public-card presentation policy represented by this cache contract.
  pub fn presentation_policy_version(&self) -> &CanonicalCardPolicyVersion {
    &self.presentation_policy_version
  }
}

/// Public-only lookup inputs that can safely participate in a shared canonical-card cache.
///
/// The request always uses [`EvidenceUse::ApiRedistribution`]. It contains no identity, context,
/// history, mature-content choice, private feedback, or personal projection. Its `Debug`
/// implementation deliberately omits the normalized query text.
#[derive(Clone, PartialEq, Eq)]
pub struct PublicCanonicalLookupRequest {
  retrieval: RetrievalRequest,
  policy_versions: CanonicalCardPolicyVersions,
}

impl PublicCanonicalLookupRequest {
  /// Validates public canonical lookup inputs and fixes evidence use to API redistribution.
  ///
  /// # Errors
  ///
  /// Returns an error when the query is blank or `candidate_limit` is outside the retrieval bound.
  pub fn new(
    query: &str,
    language: LanguageTag,
    candidate_limit: usize,
    policy_versions: CanonicalCardPolicyVersions,
  ) -> Result<Self, CanonicalLookupCacheValidationError> {
    Ok(Self {
      retrieval: RetrievalRequest::new(
        query,
        language,
        EvidenceUse::ApiRedistribution,
        candidate_limit,
      )?,
      policy_versions,
    })
  }

  /// Returns the normalized retrieval request pinned to API-redistributable evidence.
  pub fn retrieval(&self) -> &RetrievalRequest {
    &self.retrieval
  }

  /// Returns every public-card policy version that participates in cache identity.
  pub fn policy_versions(&self) -> &CanonicalCardPolicyVersions {
    &self.policy_versions
  }
}

impl fmt::Debug for PublicCanonicalLookupRequest {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("PublicCanonicalLookupRequest")
      .field("query", &"[redacted]")
      .field("language", &self.retrieval.language)
      .field("candidate_limit", &self.retrieval.limit)
      .field("evidence_use", &"api_redistribution")
      .field("policy_versions", &self.policy_versions)
      .finish()
  }
}

/// Stable public-only identity for one rebuildable canonical-card snapshot.
///
/// The key includes immutable lexical release, vector collection, canonical schema, ranking,
/// retrieval policy, presentation policy, normalized lookup language, candidate limit, and a
/// SHA-256 fingerprint of the normalized query. It never retains raw query text.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalLookupSnapshotKey {
  content: ActiveContentVersion,
  policy_versions: CanonicalCardPolicyVersions,
  language: LanguageTag,
  candidate_limit: usize,
  query_fingerprint: [u8; 32],
}

impl CanonicalLookupSnapshotKey {
  /// Builds a cache key from a public request and the active immutable content tuple.
  ///
  /// # Errors
  ///
  /// Returns an error when the content tuple has a blank schema or ranking version.
  pub fn new(
    request: &PublicCanonicalLookupRequest,
    content: ActiveContentVersion,
  ) -> Result<Self, CanonicalLookupCacheValidationError> {
    validate_content(&content)?;
    Ok(Self {
      content,
      policy_versions: request.policy_versions.clone(),
      language: request.retrieval.language.clone(),
      candidate_limit: request.retrieval.limit,
      query_fingerprint: query_fingerprint(&request.retrieval.query),
    })
  }

  /// Returns the full immutable content tuple encoded in this key.
  pub fn content(&self) -> &ActiveContentVersion {
    &self.content
  }

  /// Returns the retrieval and presentation policy versions encoded in this key.
  pub fn policy_versions(&self) -> &CanonicalCardPolicyVersions {
    &self.policy_versions
  }

  /// Returns the normalized lookup language encoded in this key.
  pub fn language(&self) -> &LanguageTag {
    &self.language
  }

  /// Returns the bounded candidate count encoded in this key.
  pub fn candidate_limit(&self) -> usize {
    self.candidate_limit
  }
}

impl Hash for CanonicalLookupSnapshotKey {
  fn hash<State: Hasher>(&self, state: &mut State) {
    self.content.release_id.hash(state);
    self.content.vector_collection_id.hash(state);
    self.content.schema_version.hash(state);
    self.content.ranking_version.hash(state);
    self.policy_versions.hash(state);
    self.language.hash(state);
    self.candidate_limit.hash(state);
    self.query_fingerprint.hash(state);
  }
}

impl fmt::Debug for CanonicalLookupSnapshotKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CanonicalLookupSnapshotKey")
      .field("content", &self.content)
      .field("policy_versions", &self.policy_versions)
      .field("language", &self.language)
      .field("candidate_limit", &self.candidate_limit)
      .field("query_fingerprint", &"[redacted]")
      .finish()
  }
}

/// Rebuildable public canonical-card data retained by the snapshot-cache service.
///
/// Its `Debug` implementation exposes only version metadata, retrieval cardinality, and no card
/// or evidence text. The actual candidate payload is filtered to API-redistributable evidence
/// before construction succeeds. The snapshot also retains the retrieval path that built its
/// candidates. The full redacted key is retained so an adapter that returns a mismatched value
/// can never turn a different query, language, or candidate limit into a hit.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalLookupSnapshot {
  key: CanonicalLookupSnapshotKey,
  retrieval_path: RetrievalPath,
  candidates: Vec<RankedCandidate>,
}

impl CanonicalLookupSnapshot {
  /// Creates a public-only snapshot whose candidates are compatible with `key`.
  ///
  /// Every candidate must use the key's release and lookup language, have API-redistributable
  /// evidence, appear once, and retain its deterministic rank. Non-permitted unreferenced
  /// evidence is removed before the snapshot is stored as defense in depth.
  ///
  /// # Errors
  ///
  /// Returns an error when the ranked candidates violate the key's public-content contract.
  pub fn new(
    key: &CanonicalLookupSnapshotKey,
    candidates: Vec<RankedCandidate>,
    retrieval_path: RetrievalPath,
  ) -> Result<Self, CanonicalLookupCacheValidationError> {
    if candidates.len() > key.candidate_limit {
      return Err(CanonicalLookupCacheValidationError::CandidateLimitExceeded);
    }

    let mut sense_ids = BTreeSet::new();
    let candidates = candidates
      .into_iter()
      .enumerate()
      .map(|(index, mut candidate)| {
        if candidate.rank != index + 1 {
          return Err(CanonicalLookupCacheValidationError::InvalidRankSequence);
        }
        if !sense_ids.insert(candidate.candidate.sense.id.clone()) {
          return Err(CanonicalLookupCacheValidationError::DuplicateSense);
        }
        if candidate.candidate.lexeme.language != key.language
          || !candidate
            .candidate
            .is_eligible_for(&key.content.release_id, EvidenceUse::ApiRedistribution)
        {
          return Err(CanonicalLookupCacheValidationError::IneligibleCandidate);
        }
        candidate.candidate.evidence.retain(|evidence| {
          evidence.permits(&key.content.release_id, EvidenceUse::ApiRedistribution)
        });
        Ok(candidate)
      })
      .collect::<Result<Vec<_>, _>>()?;

    Ok(Self {
      key: key.clone(),
      retrieval_path,
      candidates,
    })
  }

  /// Returns whether this value exactly matches every input encoded in `key`.
  pub fn matches_key(&self, key: &CanonicalLookupSnapshotKey) -> bool {
    self.key == *key
  }

  /// Returns the full redacted cache key carried by this snapshot.
  pub fn key(&self) -> &CanonicalLookupSnapshotKey {
    &self.key
  }

  /// Returns the immutable content tuple used to construct the snapshot.
  pub fn content(&self) -> &ActiveContentVersion {
    self.key.content()
  }

  /// Returns the retrieval path that built this immutable snapshot.
  ///
  /// A snapshot returned from the shared cache is always [`RetrievalPath::Hybrid`]. A
  /// lexical-only snapshot may be returned only from an uncached rebuild so that vector recovery
  /// can improve the next public lookup.
  pub const fn retrieval_path(&self) -> RetrievalPath {
    self.retrieval_path
  }

  /// Returns the retrieval and presentation policy versions used to construct the snapshot.
  pub fn policy_versions(&self) -> &CanonicalCardPolicyVersions {
    self.key.policy_versions()
  }

  /// Returns the bounded, deterministic public canonical candidates.
  pub fn candidates(&self) -> &[RankedCandidate] {
    &self.candidates
  }
}

impl fmt::Debug for CanonicalLookupSnapshot {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CanonicalLookupSnapshot")
      .field("key", &self.key)
      .field("retrieval_path", &self.retrieval_path)
      .field("candidate_count", &self.candidates.len())
      .field("candidates", &"[redacted]")
      .finish()
  }
}

/// Why a lookup must bypass the shared public canonical-card cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalLookupCacheBypassReason {
  /// Request-specific context could affect candidate selection or presentation.
  Context,
  /// Identity or authorization could affect the result.
  Identity,
  /// Saved, recent, or otherwise private lookup history could affect the result.
  History,
  /// Incognito behavior must not read or write shared cache state.
  Incognito,
  /// A requester-specific mature-content choice could affect the result.
  MatureContentChoice,
  /// Private feedback could affect the result.
  PrivateFeedback,
  /// A learner-specific projection could affect the result.
  PersonalProjection,
}

/// Explicit classification of whether a lookup may use the shared public snapshot cache.
///
/// This type never accepts the corresponding private data. A caller that has any such input must
/// choose a bypass reason. The cache service still builds an uncached public canonical snapshot,
/// but never reads, writes, or stores shared cache state for that request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalLookupCacheEligibility {
  /// The request has only public canonical lookup inputs and may use the shared cache.
  Public,
  /// The request has a private or request-specific influence and must not use the shared cache.
  Bypass(CanonicalLookupCacheBypassReason),
}

impl CanonicalLookupCacheEligibility {
  /// Creates a public-cache eligibility declaration.
  pub const fn public() -> Self {
    Self::Public
  }

  /// Creates an explicit context-sensitive cache bypass.
  pub const fn with_context() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::Context)
  }

  /// Creates an explicit identity-sensitive cache bypass.
  pub const fn with_identity() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::Identity)
  }

  /// Creates an explicit history-sensitive cache bypass.
  pub const fn with_history() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::History)
  }

  /// Creates an explicit incognito cache bypass.
  pub const fn incognito() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::Incognito)
  }

  /// Creates an explicit mature-content-choice cache bypass.
  pub const fn with_mature_content_choice() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::MatureContentChoice)
  }

  /// Creates an explicit private-feedback cache bypass.
  pub const fn with_private_feedback() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::PrivateFeedback)
  }

  /// Creates an explicit personal-projection cache bypass.
  pub const fn with_personal_projection() -> Self {
    Self::Bypass(CanonicalLookupCacheBypassReason::PersonalProjection)
  }
}

fn validate_content(
  content: &ActiveContentVersion,
) -> Result<(), CanonicalLookupCacheValidationError> {
  if content.schema_version.trim().is_empty() {
    return Err(CanonicalLookupCacheValidationError::BlankSchemaVersion);
  }
  if content.ranking_version.trim().is_empty() {
    return Err(CanonicalLookupCacheValidationError::BlankRankingVersion);
  }
  Ok(())
}

fn query_fingerprint(query: &str) -> [u8; 32] {
  Sha256::digest(query.as_bytes()).into()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{
    canonical::{
      CanonicalId, CanonicalStatus, EvidenceConfidence, EvidenceFragment, EvidenceKind, FormKind,
      Lexeme, LexicalPartOfSpeech, Sense, SourcePermissions, WordForm,
    },
    retrieval::{CandidateFeatures, RankedCandidate},
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
      ranking_version: "lookup-rank-v1".to_string(),
    }
  }

  fn policies() -> CanonicalCardPolicyVersions {
    CanonicalCardPolicyVersions::new("retrieval-v1", "presentation-v1").unwrap()
  }

  fn request() -> PublicCanonicalLookupRequest {
    PublicCanonicalLookupRequest::new("  hotter  ", language(), 4, policies()).unwrap()
  }

  fn candidate() -> RankedCandidate {
    let release_id = id("release-1");
    let lexeme_id = id("lexeme-hot");
    let evidence_id = id("evidence-hot");
    RankedCandidate {
      candidate: super::super::retrieval::CanonicalCandidate {
        lexeme: Lexeme {
          id: lexeme_id.clone(),
          release_id: release_id.clone(),
          language: language(),
          lemma: "hot".to_string(),
          normalized_lemma: "hot".to_string(),
          part_of_speech: LexicalPartOfSpeech::Adjective,
          status: CanonicalStatus::Active,
        },
        sense: Sense {
          id: id("sense-hot"),
          lexeme_id: lexeme_id.clone(),
          release_id: release_id.clone(),
          sense_key: "temperature".to_string(),
          definition: "private-looking definition must not appear in debug output".to_string(),
          definition_evidence_ids: vec![evidence_id.clone()],
          status: CanonicalStatus::Active,
        },
        forms: vec![WordForm {
          id: id("form-hotter"),
          lexeme_id,
          release_id: release_id.clone(),
          form: "hotter".to_string(),
          normalized_form: "hotter".to_string(),
          kind: FormKind::Inflection,
          morphology: None,
          evidence_ids: vec![evidence_id.clone()],
          status: CanonicalStatus::Active,
        }],
        evidence: vec![
          EvidenceFragment {
            id: evidence_id,
            source_id: id("source-public"),
            source_reference: "public-definition".to_string(),
            release_id: release_id.clone(),
            language: language(),
            kind: EvidenceKind::Definition,
            confidence: EvidenceConfidence::High,
            text: "public evidence".to_string(),
            content_hash: "hash-public".to_string(),
            permissions: SourcePermissions {
              storage: true,
              display: true,
              embedding: true,
              model_processing: true,
              api_redistribution: true,
            },
            status: CanonicalStatus::Active,
          },
          EvidenceFragment {
            id: id("evidence-private-extra"),
            source_id: id("source-private"),
            source_reference: "private-extra".to_string(),
            release_id,
            language: language(),
            kind: EvidenceKind::Other,
            confidence: EvidenceConfidence::Low,
            text: "private extra evidence".to_string(),
            content_hash: "hash-private".to_string(),
            permissions: SourcePermissions {
              storage: true,
              display: true,
              embedding: false,
              model_processing: false,
              api_redistribution: false,
            },
            status: CanonicalStatus::Active,
          },
        ],
      },
      features: CandidateFeatures::default(),
      rank: 1,
      fusion_score: 0,
    }
  }

  #[test]
  fn key_changes_for_every_content_request_and_policy_input() {
    let request = request();
    let base = CanonicalLookupSnapshotKey::new(&request, content()).unwrap();

    let changed_release = CanonicalLookupSnapshotKey::new(
      &request,
      ActiveContentVersion {
        release_id: id("release-2"),
        ..content()
      },
    )
    .unwrap();
    let changed_vector = CanonicalLookupSnapshotKey::new(
      &request,
      ActiveContentVersion {
        vector_collection_id: id("vectors-2"),
        ..content()
      },
    )
    .unwrap();
    let changed_schema = CanonicalLookupSnapshotKey::new(
      &request,
      ActiveContentVersion {
        schema_version: "canonical-v2".to_string(),
        ..content()
      },
    )
    .unwrap();
    let changed_ranking = CanonicalLookupSnapshotKey::new(
      &request,
      ActiveContentVersion {
        ranking_version: "lookup-rank-v2".to_string(),
        ..content()
      },
    )
    .unwrap();
    let changed_retrieval_policy = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new(
        "hotter",
        language(),
        4,
        CanonicalCardPolicyVersions::new("retrieval-v2", "presentation-v1").unwrap(),
      )
      .unwrap(),
      content(),
    )
    .unwrap();
    let changed_presentation_policy = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new(
        "hotter",
        language(),
        4,
        CanonicalCardPolicyVersions::new("retrieval-v1", "presentation-v2").unwrap(),
      )
      .unwrap(),
      content(),
    )
    .unwrap();
    let changed_query = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new("cold", language(), 4, policies()).unwrap(),
      content(),
    )
    .unwrap();
    let changed_language = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new(
        "hotter",
        LanguageTag::parse("en-GB").unwrap(),
        4,
        policies(),
      )
      .unwrap(),
      content(),
    )
    .unwrap();
    let changed_limit = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new("hotter", language(), 5, policies()).unwrap(),
      content(),
    )
    .unwrap();

    for changed in [
      changed_release,
      changed_vector,
      changed_schema,
      changed_ranking,
      changed_retrieval_policy,
      changed_presentation_policy,
      changed_query,
      changed_language,
      changed_limit,
    ] {
      assert_ne!(base, changed);
    }
  }

  #[test]
  fn debug_output_redacts_query_and_card_text() {
    let request =
      PublicCanonicalLookupRequest::new("private query text", language(), 4, policies()).unwrap();
    let key = CanonicalLookupSnapshotKey::new(&request, content()).unwrap();
    let snapshot =
      CanonicalLookupSnapshot::new(&key, vec![candidate()], RetrievalPath::Hybrid).unwrap();

    let debug = format!("{request:?} {key:?} {snapshot:?}");

    assert!(!debug.contains("private query text"));
    assert!(!debug.contains("private-looking definition"));
    assert!(!debug.contains("private extra evidence"));
    assert!(debug.contains("[redacted]"));
  }

  #[test]
  fn snapshot_removes_unredistributable_extra_evidence() {
    let key = CanonicalLookupSnapshotKey::new(&request(), content()).unwrap();
    let snapshot =
      CanonicalLookupSnapshot::new(&key, vec![candidate()], RetrievalPath::Hybrid).unwrap();

    assert_eq!(snapshot.candidates()[0].candidate.evidence.len(), 1);
    assert_eq!(
      snapshot.candidates()[0].candidate.evidence[0].id.as_str(),
      "evidence-hot"
    );
  }

  #[test]
  fn snapshot_rejects_a_value_keyed_for_a_different_public_query() {
    let key = CanonicalLookupSnapshotKey::new(&request(), content()).unwrap();
    let different_key = CanonicalLookupSnapshotKey::new(
      &PublicCanonicalLookupRequest::new("cold", language(), 4, policies()).unwrap(),
      content(),
    )
    .unwrap();
    let snapshot =
      CanonicalLookupSnapshot::new(&key, vec![candidate()], RetrievalPath::Hybrid).unwrap();

    assert!(!snapshot.matches_key(&different_key));
  }

  #[test]
  fn private_influences_have_explicit_bypasses() {
    let bypasses = [
      CanonicalLookupCacheEligibility::with_context(),
      CanonicalLookupCacheEligibility::with_identity(),
      CanonicalLookupCacheEligibility::with_history(),
      CanonicalLookupCacheEligibility::incognito(),
      CanonicalLookupCacheEligibility::with_mature_content_choice(),
      CanonicalLookupCacheEligibility::with_private_feedback(),
      CanonicalLookupCacheEligibility::with_personal_projection(),
    ];

    assert!(bypasses
      .iter()
      .all(|eligibility| { matches!(eligibility, CanonicalLookupCacheEligibility::Bypass(_)) }));
  }
}
