//! Strict outbound HTTP/1.1 client for island-port canonical reads over its Unix socket.
//!
//! Wire DTOs remain private to this adapter. Every successful response is reconstructed through
//! existing domain constructors; incomplete, incompatible, or internally inconsistent data fails
//! closed without exposing response bodies or request content in errors.

use std::{
  collections::{BTreeMap, BTreeSet},
  fmt,
  sync::Arc,
  time::Duration,
};

#[cfg(unix)]
use std::path::Path;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
  canonical::{
    normalize_lookup_key, CanonicalId, CanonicalReleasePin, CanonicalStatus, EvidenceConfidence,
    EvidenceFragment, EvidenceKind, FormKind, LanguageTag, Lexeme, LexicalPartOfSpeech,
    LexicalSource, Sense, SourcePermissions, WordForm,
  },
  canonical_content::{
    CanonicalEvidenceLineage, CanonicalEvidenceOrigin, CanonicalExample, CanonicalFactualAssertion,
    CanonicalPronunciation, CanonicalSenseDetails, CanonicalSenseDetailsInput, Collocation,
    CollocationConstruction, CollocationRole, CollocationTerm, EtymologyAssertion, EtymologyKind,
    EtymologyScope, GeneratedEvidenceReview, GrammarPattern, GrammarPatternKind, HistoricalRange,
    LearnerPitfall, LearnerPitfallKind, LocalizedGloss, PronunciationNotation, PronunciationScope,
    SenseContentTarget, SenseHistoryAssertion, SenseHistoryEventKind, UsageLabel, UsageLabelKind,
  },
  canonical_translation::{
    CanonicalMeaningScope, CanonicalRevision, CanonicalTranslationId, CanonicalTranslationRevision,
    CanonicalTranslationUnit, DomainId, MeaningComposition, SourceFingerprint,
    SOURCE_FINGERPRINT_VERSION,
  },
  domain_assessment::{
    CanonicalDomain, DomainCoverageState, DomainInventory, DomainKnowledgeProfile,
    LocalizedDomainDefinition, LocalizedDomainTerm, MAX_DOMAIN_INVENTORY, MAX_DOMAIN_TERM_CHARS,
  },
  knowledge_hydration::{
    CanonicalFactRef, HydratedKnowledgeNode, HydratedScaleMember, HydratedSemanticScale,
    KnowledgeCondition, KnowledgeFact, SemanticScaleDirection, MAX_KNOWLEDGE_HYDRATION_ITEMS,
  },
  retrieval::{CanonicalCandidate, LexicalMatchKind, RepositoryMatch, RetrievalScore},
  retrieval_data::{RetrievalNodeType, RetrievalRelation, RetrievalVerificationState},
};
use crate::ports::canonical_read::{
  CanonicalCandidateQuery, CanonicalDomainQuery, CanonicalFactQuery, CanonicalKnowledgeNodeQuery,
  CanonicalReadContext, CanonicalReadError, CanonicalReadPort, CanonicalScaleQuery,
  CanonicalSenseQuery, CanonicalTranslationQuery,
};

/// Canonical-data wire schema implemented by this client.
pub const ISLAND_PORT_SCHEMA_VERSION: &str = "canonical-data-v1";
const MAX_RESPONSE_BYTES: usize = 1_048_576;

/// Per-call transport metadata propagated without becoming domain state.
#[derive(Clone, PartialEq, Eq)]
pub struct IslandPortCallContext {
  request_id: String,
  deadline_at: String,
  timeout: Duration,
}

impl IslandPortCallContext {
  /// Creates bounded transport context with an already formatted RFC 3339 deadline.
  ///
  /// # Errors
  ///
  /// Returns an error for blank identifiers/deadlines or a zero timeout.
  pub fn new(
    request_id: impl Into<String>,
    deadline_at: impl Into<String>,
    timeout: Duration,
  ) -> Result<Self, IslandPortClientError> {
    let request_id = request_id.into();
    let deadline_at = deadline_at.into();
    if request_id.is_empty()
      || request_id.len() > 128
      || !request_id.bytes().all(|byte| byte.is_ascii_graphic())
      || deadline_at.len() > 64
      || time::OffsetDateTime::parse(&deadline_at, &time::format_description::well_known::Rfc3339)
        .is_err()
      || timeout.is_zero()
    {
      return Err(IslandPortClientError::InvalidRequest);
    }
    Ok(Self {
      request_id,
      deadline_at,
      timeout,
    })
  }

  fn effective_timeout(&self) -> Result<Duration, IslandPortClientError> {
    let deadline = time::OffsetDateTime::parse(
      &self.deadline_at,
      &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| IslandPortClientError::InvalidRequest)?;
    let remaining = deadline - time::OffsetDateTime::now_utc();
    let remaining = Duration::try_from(remaining).map_err(|_| IslandPortClientError::Timeout)?;
    if remaining.is_zero() {
      return Err(IslandPortClientError::Timeout);
    }
    Ok(self.timeout.min(remaining))
  }
}

impl fmt::Debug for IslandPortCallContext {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortCallContext([redacted])")
  }
}

/// Closed, redacted failure from the outbound canonical-read client.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum IslandPortClientError {
  /// Caller supplied invalid bounded transport or read parameters.
  #[error("island-port request is invalid")]
  InvalidRequest,
  /// No canonical value exists in the pinned release.
  #[error("canonical value was not found")]
  NotFound,
  /// The explicitly pinned content release is no longer readable.
  #[error("island-port content release is unavailable")]
  ContentReleaseUnavailable,
  /// The peer did not implement the required adapter or canonical schema.
  #[error("island-port schema is incompatible")]
  SchemaIncompatible,
  /// The local dependency could not complete the operation.
  #[error("island-port is unavailable")]
  Unavailable,
  /// The call exceeded its request deadline.
  #[error("island-port request timed out")]
  Timeout,
  /// The response was oversized, malformed, incomplete, or contradicted domain invariants.
  #[error("island-port returned inconsistent canonical data")]
  InconsistentData,
}

/// Minimal request executor used to test the wire contract without a database or live socket.
#[async_trait]
pub trait IslandPortTransport: Send + Sync {
  /// Sends one bounded JSON request to a fixed island-port operation.
  async fn post_json(
    &self,
    path: &'static str,
    body: Vec<u8>,
    timeout: Duration,
  ) -> Result<Vec<u8>, IslandPortClientError>;
}

/// Production HTTP client bound to island-port's Unix-domain socket.
#[cfg(unix)]
pub struct UnixIslandPortTransport {
  client: reqwest::Client,
}

#[cfg(unix)]
impl UnixIslandPortTransport {
  /// Creates a client that can only connect through `socket_path`.
  ///
  /// # Errors
  ///
  /// Returns a redacted unavailable error if the HTTP client cannot be built.
  pub fn new(socket_path: impl AsRef<Path>) -> Result<Self, IslandPortClientError> {
    let client = reqwest::Client::builder()
      .unix_socket(socket_path.as_ref())
      .build()
      .map_err(|_| IslandPortClientError::Unavailable)?;
    Ok(Self { client })
  }
}

#[cfg(unix)]
#[async_trait]
impl IslandPortTransport for UnixIslandPortTransport {
  async fn post_json(
    &self,
    path: &'static str,
    body: Vec<u8>,
    timeout: Duration,
  ) -> Result<Vec<u8>, IslandPortClientError> {
    let mut response = self
      .client
      .post(format!("http://island-port{path}"))
      .header(reqwest::header::CONTENT_TYPE, "application/json")
      .timeout(timeout)
      .body(body)
      .send()
      .await
      .map_err(|error| {
        if error.is_timeout() {
          IslandPortClientError::Timeout
        } else {
          IslandPortClientError::Unavailable
        }
      })?;
    if !response.status().is_success() {
      return Err(IslandPortClientError::Unavailable);
    }
    if response
      .headers()
      .get(reqwest::header::CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .is_none_or(|value| !value.eq_ignore_ascii_case("application/json"))
      || response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
      return Err(IslandPortClientError::InconsistentData);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
      .chunk()
      .await
      .map_err(|_| IslandPortClientError::Unavailable)?
    {
      if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
        return Err(IslandPortClientError::InconsistentData);
      }
      body.extend_from_slice(&chunk);
    }
    Ok(body)
  }
}

/// Outbound canonical-read adapter with no inbound listener or database capability.
pub struct IslandPortCanonicalClient {
  transport: Arc<dyn IslandPortTransport>,
}

impl fmt::Debug for IslandPortCanonicalClient {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("IslandPortCanonicalClient([redacted])")
  }
}

impl IslandPortCanonicalClient {
  /// Creates a client over an injected bounded transport.
  pub fn new(transport: Arc<dyn IslandPortTransport>) -> Self {
    Self { transport }
  }

  /// Reads the authoritative active canonical-only release without requesting a vector version.
  pub async fn active_release(
    &self,
    context: &IslandPortCallContext,
  ) -> Result<Option<CanonicalReleasePin>, IslandPortClientError> {
    let envelope = RequestEnvelope {
      context: ActiveReleaseContextDto {
        request_id: context.request_id.clone(),
        deadline_at: context.deadline_at.clone(),
        schema_version: ISLAND_PORT_SCHEMA_VERSION,
      },
      input: EmptyInputDto {},
    };
    let response: ActiveReleaseResponseDto = self
      .call("/api/v1/releases/active", context, &envelope)
      .await?;
    if response.request_id != context.request_id
      || response.schema_version != ISLAND_PORT_SCHEMA_VERSION
    {
      return Err(IslandPortClientError::SchemaIncompatible);
    }
    let value = match response.outcome {
      OutcomeDto::Ok if response.error.is_none() => response
        .value
        .ok_or(IslandPortClientError::InconsistentData)?,
      OutcomeDto::NotFound if response.value.is_none() && response.error.is_some() => {
        return Ok(None)
      }
      OutcomeDto::VersionMismatch if response.value.is_none() => {
        return Err(
          response
            .error
            .as_ref()
            .filter(|error| error.code == "schema_incompatible")
            .map_or(IslandPortClientError::InconsistentData, |_| {
              IslandPortClientError::SchemaIncompatible
            }),
        )
      }
      OutcomeDto::Unavailable if response.value.is_none() && response.error.is_some() => {
        return Err(IslandPortClientError::Unavailable)
      }
      OutcomeDto::Timeout if response.value.is_none() && response.error.is_some() => {
        return Err(IslandPortClientError::Timeout)
      }
      _ => return Err(IslandPortClientError::InconsistentData),
    };
    if value.content_release.len() > 128
      || !value
        .content_release
        .bytes()
        .all(|byte| byte.is_ascii_graphic())
    {
      return Err(IslandPortClientError::InconsistentData);
    }
    let release_id = CanonicalId::new(&value.content_release)
      .map_err(|_| IslandPortClientError::InconsistentData)?;
    CanonicalReleasePin::new(release_id, value.canonical_schema_version)
      .ok_or(IslandPortClientError::InconsistentData)
      .map(Some)
  }

  /// Resolves reviewed translation candidates from one immutable release.
  pub async fn resolve_translations(
    &self,
    context: &IslandPortCallContext,
    release_id: &crate::domain::canonical::ReleaseId,
    input: TranslationResolveInput,
  ) -> Result<Vec<CanonicalTranslationRevision>, IslandPortClientError> {
    if input.limit == 0 || input.limit > 50 {
      return Err(IslandPortClientError::InvalidRequest);
    }
    let envelope = RequestEnvelope {
      context: context_dto(context, release_id),
      input: TranslationResolveInputDto::from(input),
    };
    let response: TranslationResolveResponseDto = self
      .call("/api/v1/translations/resolve", context, &envelope)
      .await?;
    validate_response_context(&response.common, context, release_id)?;
    let value = response.common.value()?;
    value
      .matches
      .into_iter()
      .map(|candidate| candidate.into_domain(release_id))
      .collect()
  }

  /// Resolves release-pinned lexical candidates without ranking or producing presentation cards.
  pub async fn resolve_basic_card_candidates(
    &self,
    context: &IslandPortCallContext,
    release_id: &crate::domain::canonical::ReleaseId,
    input: BasicCardResolveInput,
  ) -> Result<Vec<RepositoryMatch>, IslandPortClientError> {
    if input.lookup_forms.is_empty() || input.limit == 0 || input.limit > 50 {
      return Err(IslandPortClientError::InvalidRequest);
    }
    let evidence_use = input.evidence_use;
    let submitted_lookup_forms = input
      .lookup_forms
      .iter()
      .map(|form| form.form.clone())
      .collect::<BTreeSet<_>>();
    let envelope = RequestEnvelope {
      context: context_dto(context, release_id),
      input: BasicCardResolveInputDto::from(input),
    };
    let response: BasicCardResolveResponseDto = self
      .call("/api/v1/basic-cards/resolve", context, &envelope)
      .await?;
    validate_response_context(&response.common, context, release_id)?;
    let mut value = response.common.value()?;
    if value.truncated {
      return Err(IslandPortClientError::InconsistentData);
    }
    value.matches.append(&mut value.alternatives);
    value
      .matches
      .into_iter()
      .map(|candidate| candidate.into_domain(release_id, evidence_use, &submitted_lookup_forms))
      .collect()
  }

  /// Reads independently constructible, evidence-backed details for one pinned sense.
  pub async fn get_sense(
    &self,
    context: &IslandPortCallContext,
    pin: &CanonicalReleasePin,
    input: SenseGetInput,
  ) -> Result<CanonicalSenseDetails, IslandPortClientError> {
    let envelope = RequestEnvelope {
      context: context_dto(context, &pin.release_id),
      input: SenseGetInputDto::from(input),
    };
    let response: SenseGetResponseDto = self.call("/api/v1/senses/get", context, &envelope).await?;
    validate_response_context(&response.common, context, &pin.release_id)?;
    let value = response.common.value()?;
    if value.canonical_schema_version.as_deref() != Some(pin.canonical_schema_version.as_str()) {
      return Err(IslandPortClientError::SchemaIncompatible);
    }
    value.into_domain(&pin.release_id)
  }

  /// Reads a bounded canonical-domain allowlist and explicit catalog-completeness signal.
  pub async fn resolve_domains(
    &self,
    context: &IslandPortCallContext,
    release_id: &crate::domain::canonical::ReleaseId,
    input: DomainResolveInput,
  ) -> Result<DomainInventory, IslandPortClientError> {
    if input.normalized_labels.is_empty()
      || input.normalized_labels.len() > MAX_DOMAIN_INVENTORY
      || input.normalized_labels.iter().any(|label| {
        label.trim() != label || label.is_empty() || label.chars().count() > MAX_DOMAIN_TERM_CHARS
      })
      || input
        .normalized_labels
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
      || input.languages.is_empty()
      || input.languages.len() > 8
      || input.languages.windows(2).any(|pair| pair[0] >= pair[1])
      || input.scope_key.as_ref().is_some_and(|scope| {
        scope.trim() != scope || scope.is_empty() || scope.chars().count() > MAX_DOMAIN_TERM_CHARS
      })
      || input.limit == 0
      || input.limit > MAX_DOMAIN_INVENTORY
    {
      return Err(IslandPortClientError::InvalidRequest);
    }
    let limit = input.limit;
    let envelope = RequestEnvelope {
      context: context_dto(context, release_id),
      input: DomainResolveInputDto::from(input),
    };
    let response: DomainResolveResponseDto = self
      .call("/api/v1/domains/resolve", context, &envelope)
      .await?;
    validate_response_context(&response.common, context, release_id)?;
    let value = response.common.value()?;
    if value.candidates.len() > limit || value.candidates.len() > MAX_DOMAIN_INVENTORY {
      return Err(IslandPortClientError::InconsistentData);
    }
    DomainInventory::new(
      value
        .candidates
        .into_iter()
        .map(|candidate| candidate.into_domain(release_id))
        .collect::<Result<_, _>>()?,
      value.catalog_complete,
    )
    .map_err(inconsistent)
  }

  /// Hydrates exact immutable fact revisions in request order after omitted ineligible values.
  pub async fn get_knowledge_facts(
    &self,
    context: &IslandPortCallContext,
    pin: &CanonicalReleasePin,
    input: KnowledgeFactsGetInput,
  ) -> Result<Vec<KnowledgeFact>, IslandPortClientError> {
    validate_exact_fact_input(&input)?;
    let requested = input.facts.clone();
    let limit = input.limit;
    let input = KnowledgeFactsGetInputDto::new(input, pin);
    let envelope = RequestEnvelope {
      context: context_dto(context, &pin.release_id),
      input,
    };
    let response: KnowledgeFactsGetResponseDto = self
      .call("/api/v1/knowledge-facts/get", context, &envelope)
      .await?;
    validate_pinned_response_context(&response.common, context, pin)?;
    let facts = response.common.value()?.facts;
    if facts.len() > limit {
      return Err(IslandPortClientError::InconsistentData);
    }
    let facts = facts
      .into_iter()
      .map(KnowledgeFactDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    validate_fact_subsequence(&requested, &facts)?;
    Ok(facts)
  }

  /// Hydrates complete authoritative semantic scales for one explicit member.
  pub async fn get_semantic_scales(
    &self,
    context: &IslandPortCallContext,
    pin: &CanonicalReleasePin,
    input: SemanticScalesGetInput,
  ) -> Result<Vec<HydratedSemanticScale>, IslandPortClientError> {
    validate_id_input(&input.scale_ids, input.limit)?;
    validate_verified(&input.verification_states)?;
    let requested = input.scale_ids.clone();
    let for_node_id = input.for_node_id.clone();
    let limit = input.limit;
    let input = SemanticScalesGetInputDto::new(input, pin);
    let envelope = RequestEnvelope {
      context: context_dto(context, &pin.release_id),
      input,
    };
    let response: SemanticScalesGetResponseDto = self
      .call("/api/v1/semantic-scales/get", context, &envelope)
      .await?;
    validate_pinned_response_context(&response.common, context, pin)?;
    let scales = response.common.value()?.scales;
    if scales.len() > limit {
      return Err(IslandPortClientError::InconsistentData);
    }
    let scales = scales
      .into_iter()
      .map(|scale| scale.into_domain(&for_node_id))
      .collect::<Result<Vec<_>, _>>()?;
    validate_id_subsequence(&requested, scales.iter().map(|scale| &scale.scale_id))?;
    Ok(scales)
  }

  /// Hydrates authoritative display-safe values for nominated canonical nodes.
  pub async fn get_knowledge_nodes(
    &self,
    context: &IslandPortCallContext,
    pin: &CanonicalReleasePin,
    input: KnowledgeNodesGetInput,
  ) -> Result<Vec<HydratedKnowledgeNode>, IslandPortClientError> {
    validate_id_input(&input.node_ids, input.limit)?;
    let requested = input.node_ids.clone();
    let limit = input.limit;
    let input = KnowledgeNodesGetInputDto::new(input, pin);
    let envelope = RequestEnvelope {
      context: context_dto(context, &pin.release_id),
      input,
    };
    let response: KnowledgeNodesGetResponseDto = self
      .call("/api/v1/knowledge-nodes/get", context, &envelope)
      .await?;
    validate_pinned_response_context(&response.common, context, pin)?;
    let nodes = response.common.value()?.nodes;
    if nodes.len() > limit {
      return Err(IslandPortClientError::InconsistentData);
    }
    let nodes = nodes
      .into_iter()
      .map(KnowledgeNodeDto::into_domain)
      .collect::<Result<Vec<_>, _>>()?;
    validate_id_subsequence(&requested, nodes.iter().map(|node| &node.node_id))?;
    Ok(nodes)
  }

  async fn call<T: Serialize, R: DeserializeOwned>(
    &self,
    path: &'static str,
    context: &IslandPortCallContext,
    request: &T,
  ) -> Result<R, IslandPortClientError> {
    let body = serde_json::to_vec(request).map_err(|_| IslandPortClientError::InvalidRequest)?;
    let timeout = context.effective_timeout()?;
    let bytes = self.transport.post_json(path, body, timeout).await?;
    if bytes.len() > MAX_RESPONSE_BYTES {
      return Err(IslandPortClientError::InconsistentData);
    }
    serde_json::from_slice(&bytes).map_err(|_| IslandPortClientError::InconsistentData)
  }
}

/// Request-local translation candidate selectors accepted by the outbound adapter.
pub struct TranslationResolveInput {
  /// Versioned source fingerprint.
  pub source_fingerprint: SourceFingerprint,
  /// Source language.
  pub source_language: LanguageTag,
  /// Target language.
  pub target_language: LanguageTag,
  /// Optional sense constraint.
  pub sense_id: Option<crate::domain::canonical::SenseId>,
  /// Ordered domain constraints.
  pub domain_ids: Vec<DomainId>,
  /// Optional dialect.
  pub dialect: Option<LanguageTag>,
  /// Optional register.
  pub register: Option<String>,
  /// Candidate bound.
  pub limit: usize,
}

/// One request-local derived lookup form.
pub struct LookupFormInput {
  /// Derived form, never a stable identity.
  pub form: String,
  /// Closed match class used to derive the form.
  pub match_class: LexicalMatchKind,
  /// Deterministic request-local precedence.
  pub rank: usize,
}

/// Candidate-resolution input for the compact canonical read.
pub struct BasicCardResolveInput {
  /// Bounded ordered derived forms.
  pub lookup_forms: Vec<LookupFormInput>,
  /// Version of request-local normalization.
  pub normalizer_version: String,
  /// Source language.
  pub source_language: LanguageTag,
  /// Explanation language.
  pub explanation_language: LanguageTag,
  /// Optional dialect.
  pub dialect: Option<LanguageTag>,
  /// Evidence operation whose permissions island-port must enforce.
  pub evidence_use: crate::domain::canonical::EvidenceUse,
  /// Candidate bound.
  pub limit: usize,
}

/// Exact sense-detail input for one immutable release.
pub struct SenseGetInput {
  /// Stable canonical sense identity.
  pub sense_id: crate::domain::canonical::SenseId,
  /// Requested explanation language.
  pub explanation_language: LanguageTag,
  /// Optional pronunciation dialect.
  pub dialect: Option<LanguageTag>,
  /// Evidence operation whose permissions island-port must enforce.
  pub evidence_use: crate::domain::canonical::EvidenceUse,
}

/// Bounded canonical-domain inventory selectors accepted by the outbound adapter.
pub struct DomainResolveInput {
  /// Normalized multilingual labels that nominated assessment.
  pub normalized_labels: Vec<String>,
  /// Optional request-local scope discriminator.
  pub scope_key: Option<String>,
  /// Languages useful for labels and coverage.
  pub languages: Vec<LanguageTag>,
  /// Candidate limit.
  pub limit: usize,
}

/// Exact fact revisions accepted by the authoritative hydration operation.
pub struct KnowledgeFactsGetInput {
  /// Ordered exact fact revisions.
  pub facts: Vec<CanonicalFactRef>,
  /// Eligible verification states; canonical hydration currently accepts only `verified`.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Response bound.
  pub limit: usize,
}

/// Complete semantic-scale selectors accepted by the authoritative hydration operation.
pub struct SemanticScalesGetInput {
  /// Ordered stable scale identities.
  pub scale_ids: Vec<CanonicalId>,
  /// Required explicit member for scope and condition eligibility.
  pub for_node_id: CanonicalId,
  /// Eligible verification states; canonical hydration currently accepts only `verified`.
  pub verification_states: Vec<RetrievalVerificationState>,
  /// Response bound.
  pub limit: usize,
}

/// Display-safe canonical node selectors accepted by the hydration operation.
pub struct KnowledgeNodesGetInput {
  /// Ordered stable node identities.
  pub node_ids: Vec<CanonicalId>,
  /// Required evidence permission enforced by island-port.
  pub evidence_use: crate::domain::canonical::EvidenceUse,
  /// Response bound.
  pub limit: usize,
}

impl From<IslandPortClientError> for CanonicalReadError {
  fn from(error: IslandPortClientError) -> Self {
    match error {
      IslandPortClientError::InvalidRequest => Self::InvalidRequest,
      IslandPortClientError::NotFound => Self::NotFound,
      IslandPortClientError::ContentReleaseUnavailable => Self::ContentReleaseUnavailable,
      IslandPortClientError::SchemaIncompatible => Self::SchemaIncompatible,
      IslandPortClientError::Unavailable => Self::Unavailable,
      IslandPortClientError::Timeout => Self::Timeout,
      IslandPortClientError::InconsistentData => Self::InconsistentData,
    }
  }
}

fn call_context(
  context: &CanonicalReadContext,
) -> Result<IslandPortCallContext, CanonicalReadError> {
  IslandPortCallContext::new(&context.request_id, &context.deadline_at, context.timeout)
    .map_err(Into::into)
}

#[async_trait]
impl CanonicalReadPort for IslandPortCanonicalClient {
  async fn active_release(
    &self,
    context: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
    self
      .active_release(&call_context(context)?)
      .await
      .map_err(Into::into)
  }

  async fn translations(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalTranslationQuery,
  ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError> {
    self
      .resolve_translations(
        &call_context(context)?,
        &pin.release_id,
        TranslationResolveInput {
          source_fingerprint: query.source_fingerprint,
          source_language: query.source_language,
          target_language: query.target_language,
          sense_id: query.sense_id,
          domain_ids: query.domain_ids,
          dialect: query.dialect,
          register: query.register,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn candidates(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalCandidateQuery,
  ) -> Result<Vec<RepositoryMatch>, CanonicalReadError> {
    self
      .resolve_basic_card_candidates(
        &call_context(context)?,
        &pin.release_id,
        BasicCardResolveInput {
          lookup_forms: query
            .lookup_forms
            .into_iter()
            .map(|form| LookupFormInput {
              form: form.form,
              match_class: form.match_class,
              rank: form.rank,
            })
            .collect(),
          normalizer_version: query.normalizer_version,
          source_language: query.source_language,
          explanation_language: query.explanation_language,
          dialect: query.dialect,
          evidence_use: query.evidence_use,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn sense(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalSenseQuery,
  ) -> Result<CanonicalSenseDetails, CanonicalReadError> {
    self
      .get_sense(
        &call_context(context)?,
        pin,
        SenseGetInput {
          sense_id: query.sense_id,
          explanation_language: query.explanation_language,
          dialect: query.dialect,
          evidence_use: query.evidence_use,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn domains(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalDomainQuery,
  ) -> Result<DomainInventory, CanonicalReadError> {
    self
      .resolve_domains(
        &call_context(context)?,
        &pin.release_id,
        DomainResolveInput {
          normalized_labels: query.normalized_labels,
          scope_key: query.scope_key,
          languages: query.languages,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn knowledge_facts(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalFactQuery,
  ) -> Result<Vec<KnowledgeFact>, CanonicalReadError> {
    self
      .get_knowledge_facts(
        &call_context(context)?,
        pin,
        KnowledgeFactsGetInput {
          facts: query.facts,
          verification_states: query.verification_states,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn semantic_scales(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalScaleQuery,
  ) -> Result<Vec<HydratedSemanticScale>, CanonicalReadError> {
    self
      .get_semantic_scales(
        &call_context(context)?,
        pin,
        SemanticScalesGetInput {
          scale_ids: query.scale_ids,
          for_node_id: query.for_node_id,
          verification_states: query.verification_states,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }

  async fn knowledge_nodes(
    &self,
    context: &CanonicalReadContext,
    pin: &CanonicalReleasePin,
    query: CanonicalKnowledgeNodeQuery,
  ) -> Result<Vec<HydratedKnowledgeNode>, CanonicalReadError> {
    self
      .get_knowledge_nodes(
        &call_context(context)?,
        pin,
        KnowledgeNodesGetInput {
          node_ids: query.node_ids,
          evidence_use: query.evidence_use,
          limit: query.limit,
        },
      )
      .await
      .map_err(Into::into)
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope<C, T> {
  context: C,
  input: T,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestContextDto {
  request_id: String,
  deadline_at: String,
  schema_version: &'static str,
  content_release: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ActiveReleaseContextDto {
  request_id: String,
  deadline_at: String,
  schema_version: &'static str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct EmptyInputDto {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveReleaseValueDto {
  content_release: String,
  canonical_schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveReleaseResponseDto {
  request_id: String,
  schema_version: String,
  outcome: OutcomeDto,
  value: Option<ActiveReleaseValueDto>,
  error: Option<ErrorDto>,
}

fn context_dto(
  context: &IslandPortCallContext,
  release_id: &crate::domain::canonical::ReleaseId,
) -> RequestContextDto {
  RequestContextDto {
    request_id: context.request_id.clone(),
    deadline_at: context.deadline_at.clone(),
    schema_version: ISLAND_PORT_SCHEMA_VERSION,
    content_release: release_id.as_str().to_string(),
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct CommonResponseDto<T> {
  request_id: String,
  schema_version: String,
  outcome: OutcomeDto,
  content_release: Option<String>,
  canonical_schema_version: Option<String>,
  value: Option<T>,
  error: Option<ErrorDto>,
}

impl<T> CommonResponseDto<T> {
  fn value(self) -> Result<T, IslandPortClientError> {
    match self.outcome {
      OutcomeDto::Ok => {
        if self.error.is_some() {
          return Err(IslandPortClientError::InconsistentData);
        }
        self.value.ok_or(IslandPortClientError::InconsistentData)
      }
      OutcomeDto::NotFound => self.closed_error(IslandPortClientError::NotFound),
      OutcomeDto::VersionMismatch => self.closed_version_error(
        "schema_incompatible",
        IslandPortClientError::SchemaIncompatible,
      ),
      OutcomeDto::ContentReleaseUnavailable => self.closed_version_error(
        "content_release_unavailable",
        IslandPortClientError::ContentReleaseUnavailable,
      ),
      OutcomeDto::Unavailable => self.closed_error(IslandPortClientError::Unavailable),
      OutcomeDto::Timeout => self.closed_error(IslandPortClientError::Timeout),
    }
  }

  fn closed_error(self, error: IslandPortClientError) -> Result<T, IslandPortClientError> {
    if self.value.is_some() || self.error.is_none() {
      Err(IslandPortClientError::InconsistentData)
    } else {
      Err(error)
    }
  }

  fn closed_version_error(
    self,
    code: &str,
    error: IslandPortClientError,
  ) -> Result<T, IslandPortClientError> {
    if self.value.is_some() || self.error.as_ref().is_none_or(|value| value.code != code) {
      Err(IslandPortClientError::InconsistentData)
    } else {
      Err(error)
    }
  }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum OutcomeDto {
  Ok,
  NotFound,
  VersionMismatch,
  ContentReleaseUnavailable,
  Unavailable,
  Timeout,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorDto {
  #[allow(dead_code)]
  code: String,
  #[allow(dead_code)]
  message: String,
  #[allow(dead_code)]
  retryable: bool,
}

fn validate_response_context<T>(
  response: &CommonResponseDto<T>,
  context: &IslandPortCallContext,
  release_id: &crate::domain::canonical::ReleaseId,
) -> Result<(), IslandPortClientError> {
  if response.request_id != context.request_id
    || response.schema_version != ISLAND_PORT_SCHEMA_VERSION
  {
    return Err(IslandPortClientError::SchemaIncompatible);
  }
  match response.content_release.as_deref() {
    Some(value) if value != release_id.as_str() => {
      return Err(IslandPortClientError::InconsistentData)
    }
    None if response.outcome == OutcomeDto::Ok => {
      return Err(IslandPortClientError::InconsistentData)
    }
    _ => {}
  }
  Ok(())
}

fn validate_pinned_response_context<T>(
  response: &CommonResponseDto<T>,
  context: &IslandPortCallContext,
  pin: &CanonicalReleasePin,
) -> Result<(), IslandPortClientError> {
  validate_response_context(response, context, &pin.release_id)?;
  if response.canonical_schema_version.as_deref() != Some(pin.canonical_schema_version.as_str()) {
    return Err(IslandPortClientError::SchemaIncompatible);
  }
  Ok(())
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TranslationResolveInputDto {
  source_fingerprint: String,
  normalizer_version: &'static str,
  source_language: String,
  target_language: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  sense_id: Option<String>,
  domain_ids: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  dialect: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  register: Option<String>,
  limit: usize,
}

impl From<TranslationResolveInput> for TranslationResolveInputDto {
  fn from(value: TranslationResolveInput) -> Self {
    Self {
      source_fingerprint: value.source_fingerprint.to_storage_key(),
      normalizer_version: SOURCE_FINGERPRINT_VERSION,
      source_language: value.source_language.to_string(),
      target_language: value.target_language.to_string(),
      sense_id: value.sense_id.map(|id| id.as_str().to_string()),
      domain_ids: value
        .domain_ids
        .into_iter()
        .map(|id| id.as_str().to_string())
        .collect(),
      dialect: value.dialect.map(|value| value.to_string()),
      register: value.register,
      limit: value.limit,
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct TranslationResolveResponseDto {
  common: CommonResponseDto<TranslationResolveValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TranslationResolveValueDto {
  matches: Vec<TranslationMatchDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TranslationMatchDto {
  translation_id: String,
  revision: u64,
  unit: TranslationUnitDto,
  source_fingerprint: String,
  source: LocalizedTextDto,
  target: LocalizedTextDto,
  scope: Option<MeaningScopeDto>,
  evidence_ids: Vec<String>,
}

impl TranslationMatchDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<CanonicalTranslationRevision, IslandPortClientError> {
    let unit = self.unit.into_domain();
    let source_language = language(&self.source.language)?;
    let target_language = language(&self.target.language)?;
    let scope = self
      .scope
      .map(|value| value.into_domain(unit))
      .transpose()?;
    CanonicalTranslationRevision::new(
      CanonicalTranslationId::new(self.translation_id).map_err(inconsistent)?,
      CanonicalRevision::new(self.revision).map_err(inconsistent)?,
      release_id.clone(),
      unit,
      source_language,
      target_language,
      self.source.text,
      self.target.text,
      SourceFingerprint::parse(&self.source_fingerprint).map_err(inconsistent)?,
      scope,
      self
        .evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
    )
    .map_err(inconsistent)
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TranslationUnitDto {
  Word,
  Phrase,
  Passage,
}

impl TranslationUnitDto {
  const fn into_domain(self) -> CanonicalTranslationUnit {
    match self {
      Self::Word => CanonicalTranslationUnit::Word,
      Self::Phrase => CanonicalTranslationUnit::Phrase,
      Self::Passage => CanonicalTranslationUnit::Passage,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalizedTextDto {
  text: String,
  language: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct DomainResolveInputDto {
  normalized_labels: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  scope_key: Option<String>,
  languages: Vec<String>,
  limit: usize,
}

impl From<DomainResolveInput> for DomainResolveInputDto {
  fn from(value: DomainResolveInput) -> Self {
    Self {
      normalized_labels: value.normalized_labels,
      scope_key: value.scope_key,
      languages: value
        .languages
        .into_iter()
        .map(|language| language.to_string())
        .collect(),
      limit: value.limit,
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct DomainResolveResponseDto {
  common: CommonResponseDto<DomainResolveValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainResolveValueDto {
  candidates: Vec<DomainCandidateDto>,
  catalog_complete: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainCandidateDto {
  domain_id: String,
  revision: u64,
  labels: Vec<LocalizedTextDto>,
  aliases: Vec<LocalizedTextDto>,
  definitions: Vec<LocalizedTextDto>,
  inclusion_scope: Vec<String>,
  exclusion_scope: Vec<String>,
  broader_domain_ids: Vec<String>,
  knowledge_profile: DomainKnowledgeProfileDto,
}

impl DomainCandidateDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<CanonicalDomain, IslandPortClientError> {
    CanonicalDomain::new(
      release_id.clone(),
      DomainId::new(self.domain_id).map_err(inconsistent)?,
      CanonicalRevision::new(self.revision).map_err(inconsistent)?,
      localized_domain_terms(self.labels)?,
      localized_domain_terms(self.aliases)?,
      localized_domain_definitions(self.definitions)?,
      self.inclusion_scope,
      self.exclusion_scope,
      self
        .broader_domain_ids
        .into_iter()
        .map(|id| DomainId::new(id).map_err(inconsistent))
        .collect::<Result<_, _>>()?,
      self.knowledge_profile.into_domain()?,
    )
    .map_err(inconsistent)
  }
}

fn localized_domain_terms(
  values: Vec<LocalizedTextDto>,
) -> Result<Vec<LocalizedDomainTerm>, IslandPortClientError> {
  values
    .into_iter()
    .map(|value| {
      LocalizedDomainTerm::new(language(&value.language)?, value.text).map_err(inconsistent)
    })
    .collect()
}

fn localized_domain_definitions(
  values: Vec<LocalizedTextDto>,
) -> Result<Vec<LocalizedDomainDefinition>, IslandPortClientError> {
  values
    .into_iter()
    .map(|value| {
      LocalizedDomainDefinition::new(language(&value.language)?, value.text).map_err(inconsistent)
    })
    .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainKnowledgeProfileDto {
  available_fact_families: Vec<String>,
  languages: Vec<String>,
  verified_fact_count: u64,
  coverage_state: DomainCoverageStateDto,
}

impl DomainKnowledgeProfileDto {
  fn into_domain(self) -> Result<DomainKnowledgeProfile, IslandPortClientError> {
    DomainKnowledgeProfile::new(
      self.available_fact_families,
      self
        .languages
        .into_iter()
        .map(|value| language(&value))
        .collect::<Result<_, _>>()?,
      self.verified_fact_count,
      self.coverage_state.into_domain(),
    )
    .map_err(inconsistent)
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum DomainCoverageStateDto {
  Seed,
  Partial,
  Curated,
}

impl DomainCoverageStateDto {
  const fn into_domain(self) -> DomainCoverageState {
    match self {
      Self::Seed => DomainCoverageState::Seed,
      Self::Partial => DomainCoverageState::Partial,
      Self::Curated => DomainCoverageState::Curated,
    }
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeaningScopeDto {
  lexeme_id: String,
  sense_id: String,
  part_of_speech: PartOfSpeechDto,
  composition: CompositionDto,
  domain_ids: Vec<String>,
}

impl MeaningScopeDto {
  fn into_domain(
    self,
    unit: CanonicalTranslationUnit,
  ) -> Result<CanonicalMeaningScope, IslandPortClientError> {
    CanonicalMeaningScope::new(
      canonical_id(self.lexeme_id)?,
      canonical_id(self.sense_id)?,
      self.part_of_speech.into_domain(),
      self.composition.into_domain(),
      self
        .domain_ids
        .into_iter()
        .map(|id| DomainId::new(id).map_err(inconsistent))
        .collect::<Result<_, _>>()?,
      unit,
    )
    .map_err(inconsistent)
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PartOfSpeechDto {
  Noun,
  Verb,
  Adjective,
  Adverb,
  Pronoun,
  Preposition,
  Conjunction,
  Determiner,
  Interjection,
  Numeral,
  Other,
}

impl PartOfSpeechDto {
  const fn into_domain(self) -> LexicalPartOfSpeech {
    match self {
      Self::Noun => LexicalPartOfSpeech::Noun,
      Self::Verb => LexicalPartOfSpeech::Verb,
      Self::Adjective => LexicalPartOfSpeech::Adjective,
      Self::Adverb => LexicalPartOfSpeech::Adverb,
      Self::Pronoun => LexicalPartOfSpeech::Pronoun,
      Self::Preposition => LexicalPartOfSpeech::Preposition,
      Self::Conjunction => LexicalPartOfSpeech::Conjunction,
      Self::Determiner => LexicalPartOfSpeech::Determiner,
      Self::Interjection => LexicalPartOfSpeech::Interjection,
      Self::Numeral => LexicalPartOfSpeech::Numeral,
      Self::Other => LexicalPartOfSpeech::Other,
    }
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompositionDto {
  Compositional,
  PhraseLevel,
}

impl CompositionDto {
  const fn into_domain(self) -> MeaningComposition {
    match self {
      Self::Compositional => MeaningComposition::Compositional,
      Self::PhraseLevel => MeaningComposition::PhraseLevel,
    }
  }
}

// The candidate and sense DTO implementations are kept below so transport types never enter
// domain or application modules.

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct BasicCardResolveInputDto {
  lookup_forms: Vec<LookupFormDto>,
  normalizer_version: String,
  source_language: String,
  explanation_language: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  dialect: Option<String>,
  evidence_use: EvidenceUseDto,
  limit: usize,
}

impl From<BasicCardResolveInput> for BasicCardResolveInputDto {
  fn from(value: BasicCardResolveInput) -> Self {
    Self {
      lookup_forms: value
        .lookup_forms
        .into_iter()
        .map(LookupFormDto::from)
        .collect(),
      normalizer_version: value.normalizer_version,
      source_language: value.source_language.to_string(),
      explanation_language: value.explanation_language.to_string(),
      dialect: value.dialect.map(|value| value.to_string()),
      evidence_use: EvidenceUseDto::from(value.evidence_use),
      limit: value.limit,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct LookupFormDto {
  form: String,
  match_class: MatchKindDto,
  rank: usize,
}

impl From<LookupFormInput> for LookupFormDto {
  fn from(value: LookupFormInput) -> Self {
    Self {
      form: value.form,
      match_class: MatchKindDto::from(value.match_class),
      rank: value.rank,
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MatchKindDto {
  ExactCanonical,
  ExactAlias,
  Inflection,
  SpellingCorrection,
  Transliteration,
  Semantic,
}

impl From<LexicalMatchKind> for MatchKindDto {
  fn from(value: LexicalMatchKind) -> Self {
    match value {
      LexicalMatchKind::ExactCanonical => Self::ExactCanonical,
      LexicalMatchKind::ExactAlias => Self::ExactAlias,
      LexicalMatchKind::Inflection => Self::Inflection,
      LexicalMatchKind::SpellingCorrection => Self::SpellingCorrection,
      LexicalMatchKind::Transliteration => Self::Transliteration,
      LexicalMatchKind::Semantic => Self::Semantic,
    }
  }
}

impl MatchKindDto {
  const fn into_domain(self) -> LexicalMatchKind {
    match self {
      Self::ExactCanonical => LexicalMatchKind::ExactCanonical,
      Self::ExactAlias => LexicalMatchKind::ExactAlias,
      Self::Inflection => LexicalMatchKind::Inflection,
      Self::SpellingCorrection => LexicalMatchKind::SpellingCorrection,
      Self::Transliteration => LexicalMatchKind::Transliteration,
      Self::Semantic => LexicalMatchKind::Semantic,
    }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceUseDto {
  Storage,
  Display,
  Embedding,
  ModelProcessing,
  ApiRedistribution,
}

impl From<crate::domain::canonical::EvidenceUse> for EvidenceUseDto {
  fn from(value: crate::domain::canonical::EvidenceUse) -> Self {
    use crate::domain::canonical::EvidenceUse;
    match value {
      EvidenceUse::Storage => Self::Storage,
      EvidenceUse::Display => Self::Display,
      EvidenceUse::Embedding => Self::Embedding,
      EvidenceUse::ModelProcessing => Self::ModelProcessing,
      EvidenceUse::ApiRedistribution => Self::ApiRedistribution,
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct BasicCardResolveResponseDto {
  common: CommonResponseDto<BasicCardResolveValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BasicCardResolveValueDto {
  matches: Vec<CandidateMatchDto>,
  alternatives: Vec<CandidateMatchDto>,
  truncated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateMatchDto {
  matched_form_id: String,
  matched_form: String,
  match_class: MatchKindDto,
  lexical_score_basis_points: u16,
  candidate: CanonicalCandidateDto,
}

impl CandidateMatchDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
    evidence_use: crate::domain::canonical::EvidenceUse,
    submitted_lookup_forms: &BTreeSet<String>,
  ) -> Result<RepositoryMatch, IslandPortClientError> {
    if self.matched_form.trim().is_empty() {
      return Err(IslandPortClientError::InconsistentData);
    }
    let matched_form_id = canonical_id(self.matched_form_id)?;
    let candidate = self.candidate.into_domain(release_id, evidence_use)?;
    let matched_form = self.matched_form;
    if !submitted_lookup_forms.contains(&normalize_lookup_key(&matched_form)) {
      return Err(IslandPortClientError::InconsistentData);
    }
    let repository_match = RepositoryMatch {
      candidate,
      matched_form_id: Some(matched_form_id),
      matched_form,
      kind: self.match_class.into_domain(),
      score: RetrievalScore::new(self.lexical_score_basis_points).map_err(inconsistent)?,
    };
    if !repository_match.has_verified_source() {
      return Err(IslandPortClientError::InconsistentData);
    }
    Ok(repository_match)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalCandidateDto {
  lexeme: LexemeDto,
  sense: SenseDto,
  forms: Vec<WordFormDto>,
  evidence: Vec<EvidenceFragmentDto>,
  sources: Vec<CandidateSourceDto>,
}

impl CanonicalCandidateDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
    evidence_use: crate::domain::canonical::EvidenceUse,
  ) -> Result<CanonicalCandidate, IslandPortClientError> {
    let sources = self
      .sources
      .into_iter()
      .map(|value| value.into_domain(release_id, evidence_use))
      .collect::<Result<Vec<_>, _>>()?;
    let source_ids = sources
      .iter()
      .map(|source| &source.id)
      .collect::<BTreeSet<_>>();
    if source_ids.len() != sources.len() {
      return Err(IslandPortClientError::InconsistentData);
    }
    let candidate = CanonicalCandidate {
      lexeme: self.lexeme.into_domain(release_id)?,
      sense: self.sense.into_domain(release_id)?,
      forms: self
        .forms
        .into_iter()
        .map(|value| value.into_domain(release_id))
        .collect::<Result<_, _>>()?,
      evidence: self
        .evidence
        .into_iter()
        .map(|value| value.into_domain(release_id))
        .collect::<Result<_, _>>()?,
      sources,
    };
    let evidence_ids = candidate
      .evidence
      .iter()
      .map(|fragment| &fragment.id)
      .collect::<BTreeSet<_>>();
    if evidence_ids.len() != candidate.evidence.len()
      || candidate.evidence.iter().any(|fragment| {
        fragment.text.chars().count() > 4_096
          || fragment.source_reference.chars().count() > 256
          || candidate.source_for(fragment).is_none_or(|source| {
            !source.permissions.allows(evidence_use)
              || !fragment.permissions.allows(evidence_use)
              || !fragment.permissions.storage
              || !source.permissions.storage
              || !permissions_are_subset(fragment.permissions, source.permissions)
          })
      })
      || candidate.sources.iter().any(|source| {
        !candidate
          .evidence
          .iter()
          .any(|fragment| fragment.source_id == source.id)
      })
    {
      return Err(IslandPortClientError::InconsistentData);
    }
    if !candidate.is_eligible_for(release_id, evidence_use) {
      return Err(IslandPortClientError::InconsistentData);
    }
    Ok(candidate)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateSourceDto {
  release_id: String,
  source: SourceDto,
}

impl CandidateSourceDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
    evidence_use: crate::domain::canonical::EvidenceUse,
  ) -> Result<LexicalSource, IslandPortClientError> {
    if self.release_id != release_id.as_str() {
      return Err(IslandPortClientError::InconsistentData);
    }
    let source = self.source.into_domain()?;
    if !source.permissions.allows(evidence_use)
      || source.attribution.as_ref().is_none_or(|attribution| {
        attribution.trim().is_empty() || attribution.chars().count() > 256
      })
    {
      return Err(IslandPortClientError::InconsistentData);
    }
    Ok(source)
  }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LexemeDto {
  id: String,
  language: String,
  lemma: String,
  lemma_evidence_ids: Vec<String>,
  normalized_lemma: String,
  part_of_speech: PartOfSpeechDto,
  status: StatusDto,
}

impl LexemeDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<Lexeme, IslandPortClientError> {
    Ok(Lexeme {
      id: canonical_id(self.id)?,
      release_id: release_id.clone(),
      language: language(&self.language)?,
      lemma: self.lemma,
      lemma_evidence_ids: self
        .lemma_evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<Vec<_>, _>>()?,
      normalized_lemma: self.normalized_lemma,
      part_of_speech: self.part_of_speech.into_domain(),
      status: self.status.into_domain(),
    })
  }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SenseDto {
  id: String,
  lexeme_id: String,
  sense_key: String,
  definition: String,
  definition_evidence_ids: Vec<String>,
  status: StatusDto,
}

impl SenseDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<Sense, IslandPortClientError> {
    Ok(Sense {
      id: canonical_id(self.id)?,
      lexeme_id: canonical_id(self.lexeme_id)?,
      release_id: release_id.clone(),
      sense_key: self.sense_key,
      definition: self.definition,
      definition_evidence_ids: self
        .definition_evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
      status: self.status.into_domain(),
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WordFormDto {
  id: String,
  lexeme_id: String,
  form: String,
  normalized_form: String,
  kind: FormKindDto,
  morphology: Option<String>,
  evidence_ids: Vec<String>,
  status: StatusDto,
}

impl WordFormDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<WordForm, IslandPortClientError> {
    Ok(WordForm {
      id: canonical_id(self.id)?,
      lexeme_id: canonical_id(self.lexeme_id)?,
      release_id: release_id.clone(),
      form: self.form,
      normalized_form: self.normalized_form,
      kind: self.kind.into_domain(),
      morphology: self.morphology,
      evidence_ids: self
        .evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
      status: self.status.into_domain(),
    })
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum FormKindDto {
  Lemma,
  SpellingVariant,
  Inflection,
  Phrase,
  Alias,
}

impl FormKindDto {
  const fn into_domain(self) -> FormKind {
    match self {
      Self::Lemma => FormKind::Lemma,
      Self::SpellingVariant => FormKind::SpellingVariant,
      Self::Inflection => FormKind::Inflection,
      Self::Phrase => FormKind::Phrase,
      Self::Alias => FormKind::Alias,
    }
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StatusDto {
  Active,
  Draft,
  Quarantined,
  Retired,
}

impl StatusDto {
  const fn into_domain(self) -> CanonicalStatus {
    match self {
      Self::Active => CanonicalStatus::Active,
      Self::Draft => CanonicalStatus::Draft,
      Self::Quarantined => CanonicalStatus::Quarantined,
      Self::Retired => CanonicalStatus::Retired,
    }
  }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PermissionsDto {
  storage: bool,
  display: bool,
  embedding: bool,
  model_processing: bool,
  api_redistribution: bool,
}

impl PermissionsDto {
  const fn into_domain(self) -> SourcePermissions {
    SourcePermissions {
      storage: self.storage,
      display: self.display,
      embedding: self.embedding,
      model_processing: self.model_processing,
      api_redistribution: self.api_redistribution,
    }
  }
}

fn permissions_are_subset(asset: SourcePermissions, source: SourcePermissions) -> bool {
  [
    crate::domain::canonical::EvidenceUse::Storage,
    crate::domain::canonical::EvidenceUse::Display,
    crate::domain::canonical::EvidenceUse::Embedding,
    crate::domain::canonical::EvidenceUse::ModelProcessing,
    crate::domain::canonical::EvidenceUse::ApiRedistribution,
  ]
  .into_iter()
  .all(|operation| !asset.allows(operation) || source.allows(operation))
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceFragmentDto {
  id: String,
  source_id: String,
  source_reference: String,
  language: String,
  kind: EvidenceKindDto,
  confidence: ConfidenceDto,
  text: String,
  content_hash: String,
  permissions: PermissionsDto,
  status: StatusDto,
}

impl EvidenceFragmentDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<EvidenceFragment, IslandPortClientError> {
    Ok(EvidenceFragment {
      id: canonical_id(self.id)?,
      source_id: canonical_id(self.source_id)?,
      source_reference: self.source_reference,
      release_id: release_id.clone(),
      language: language(&self.language)?,
      kind: self.kind.into_domain(),
      confidence: self.confidence.into_domain(),
      text: self.text,
      content_hash: self.content_hash,
      permissions: self.permissions.into_domain(),
      status: self.status.into_domain(),
    })
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceKindDto {
  Definition,
  LocalizedGloss,
  Example,
  Pronunciation,
  Usage,
  Etymology,
  Other,
}

impl EvidenceKindDto {
  const fn into_domain(self) -> EvidenceKind {
    match self {
      Self::Definition => EvidenceKind::Definition,
      Self::LocalizedGloss => EvidenceKind::LocalizedGloss,
      Self::Example => EvidenceKind::Example,
      Self::Pronunciation => EvidenceKind::Pronunciation,
      Self::Usage => EvidenceKind::Usage,
      Self::Etymology => EvidenceKind::Etymology,
      Self::Other => EvidenceKind::Other,
    }
  }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConfidenceDto {
  High,
  Medium,
  Low,
}

impl ConfidenceDto {
  const fn into_domain(self) -> EvidenceConfidence {
    match self {
      Self::High => EvidenceConfidence::High,
      Self::Medium => EvidenceConfidence::Medium,
      Self::Low => EvidenceConfidence::Low,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SenseGetInputDto {
  sense_id: String,
  explanation_language: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  dialect: Option<String>,
  evidence_use: EvidenceUseDto,
}

impl From<SenseGetInput> for SenseGetInputDto {
  fn from(value: SenseGetInput) -> Self {
    Self {
      sense_id: value.sense_id.as_str().to_string(),
      explanation_language: value.explanation_language.to_string(),
      dialect: value.dialect.map(|value| value.to_string()),
      evidence_use: EvidenceUseDto::from(value.evidence_use),
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct SenseGetResponseDto {
  common: CommonResponseDto<SenseDetailsDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SenseDetailsDto {
  canonical_schema_version: Option<String>,
  target: SenseTargetDto,
  lineages: StrictMap<LineageDto>,
  localized_glosses: Vec<LocalizedGlossDto>,
  pronunciations: Vec<PronunciationDto>,
  usage_labels: Vec<UsageLabelDto>,
  grammar_patterns: Vec<GrammarPatternDto>,
  collocations: Vec<CollocationDto>,
  examples: Vec<ExampleDto>,
  pitfalls: Vec<PitfallDto>,
  etymologies: Vec<EtymologyDto>,
  history: Vec<HistoryDto>,
}

impl SenseDetailsDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<CanonicalSenseDetails, IslandPortClientError> {
    let lexeme = self.target.lexeme.into_domain(release_id)?;
    let sense = self.target.sense.into_domain(release_id)?;
    let target = SenseContentTarget::new(&lexeme, &sense).map_err(inconsistent)?;
    let lineages = self
      .lineages
      .0
      .into_iter()
      .map(|(id, dto)| {
        let lineage = dto.into_domain(release_id)?;
        if lineage.fragment().id.as_str() != id {
          return Err(IslandPortClientError::InconsistentData);
        }
        Ok((id, lineage))
      })
      .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut used = BTreeSet::new();
    for evidence_id in lexeme
      .lemma_evidence_ids
      .iter()
      .chain(sense.definition_evidence_ids.iter())
    {
      let lineage = lineages
        .get(evidence_id.as_str())
        .ok_or(IslandPortClientError::InconsistentData)?;
      if !lineage.permits(release_id, crate::domain::canonical::EvidenceUse::Display) {
        return Err(IslandPortClientError::InconsistentData);
      }
      used.insert(evidence_id.as_str().to_string());
    }
    let mut assertion = |dto: AssertionDto,
                         expected: crate::domain::canonical_content::CanonicalDetailKind|
     -> Result<CanonicalFactualAssertion, IslandPortClientError> {
      dto.into_domain(release_id, expected, &lineages, &mut used)
    };

    let localized_glosses = self
      .localized_glosses
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::LocalizedGloss,
        )?;
        LocalizedGloss::new(
          canonical_id(dto.id)?,
          target.clone(),
          language(&dto.language)?,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let pronunciations = self
      .pronunciations
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::Pronunciation,
        )?;
        CanonicalPronunciation::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.scope.into_domain(),
          language(&dto.dialect)?,
          dto.notation.into_domain(),
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let usage_labels = self
      .usage_labels
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::UsageLabel,
        )?;
        UsageLabel::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.kind.into_domain(),
          dto.code,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let grammar_patterns = self
      .grammar_patterns
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::GrammarPattern,
        )?;
        GrammarPattern::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.kind.into_domain(),
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let collocations = self
      .collocations
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::Collocation,
        )?;
        Collocation::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.target_role.into_domain(),
          dto.construction.into_domain(),
          dto.head.into_domain()?,
          dto.dependent.into_domain()?,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let examples = self
      .examples
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::Example,
        )?;
        CanonicalExample::new(
          canonical_id(dto.id)?,
          target.clone(),
          language(&dto.language)?,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let pitfalls = self
      .pitfalls
      .into_iter()
      .map(|dto| {
        let mistake = assertion(
          dto.mistake,
          crate::domain::canonical_content::CanonicalDetailKind::Pitfall,
        )?;
        let correction = assertion(
          dto.correction,
          crate::domain::canonical_content::CanonicalDetailKind::Pitfall,
        )?;
        LearnerPitfall::new(
          canonical_id(dto.id)?,
          target.clone(),
          language(&dto.learner_language)?,
          dto.kind.into_domain(),
          mistake,
          correction,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let etymologies = self
      .etymologies
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::Etymology,
        )?;
        EtymologyAssertion::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.scope.into_domain(),
          dto.kind.into_domain(),
          language(&dto.source_language)?,
          dto.period.into_domain()?,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;
    let history = self
      .history
      .into_iter()
      .map(|dto| {
        let fact = assertion(
          dto.assertion,
          crate::domain::canonical_content::CanonicalDetailKind::SenseHistory,
        )?;
        SenseHistoryAssertion::new(
          canonical_id(dto.id)?,
          target.clone(),
          dto.kind.into_domain(),
          dto.period.into_domain()?,
          fact,
        )
        .map_err(inconsistent)
      })
      .collect::<Result<_, _>>()?;

    if used.len() != lineages.len() {
      return Err(IslandPortClientError::InconsistentData);
    }
    CanonicalSenseDetails::new(CanonicalSenseDetailsInput {
      target,
      localized_glosses,
      pronunciations,
      usage_labels,
      grammar_patterns,
      collocations,
      examples,
      pitfalls,
      etymologies,
      history,
    })
    .map_err(inconsistent)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SenseTargetDto {
  lexeme: LexemeDto,
  sense: SenseDto,
}

struct StrictMap<T>(BTreeMap<String, T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for StrictMap<T> {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
      type Value = StrictMap<T>;
      fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an indexed object with unique keys")
      }
      fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut access: A,
      ) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = access.next_entry::<String, T>()? {
          if values.insert(key, value).is_some() {
            return Err(serde::de::Error::custom(
              "duplicate indexed lineage identifier",
            ));
          }
        }
        Ok(StrictMap(values))
      }
    }
    deserializer.deserialize_map(Visitor(std::marker::PhantomData))
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineageDto {
  source: SourceDto,
  fragment: EvidenceFragmentDto,
  origin: OriginDto,
}

impl LineageDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
  ) -> Result<CanonicalEvidenceLineage, IslandPortClientError> {
    CanonicalEvidenceLineage::new(
      self.source.into_domain()?,
      self.fragment.into_domain(release_id)?,
      self.origin.into_domain()?,
    )
    .map_err(inconsistent)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDto {
  id: String,
  name: String,
  version: String,
  license: String,
  attribution: Option<String>,
  permissions: PermissionsDto,
}

impl SourceDto {
  fn into_domain(self) -> Result<LexicalSource, IslandPortClientError> {
    Ok(LexicalSource {
      id: canonical_id(self.id)?,
      name: self.name,
      version: self.version,
      license: self.license,
      attribution: self.attribution,
      permissions: self.permissions.into_domain(),
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum OriginDto {
  LicensedSource,
  Generated {
    generation_id: String,
    review: ReviewDto,
  },
}

impl OriginDto {
  fn into_domain(self) -> Result<CanonicalEvidenceOrigin, IslandPortClientError> {
    Ok(match self {
      Self::LicensedSource => CanonicalEvidenceOrigin::LicensedSource,
      Self::Generated {
        generation_id,
        review,
      } => CanonicalEvidenceOrigin::Generated {
        generation_id: canonical_id(generation_id)?,
        review: review.into_domain()?,
      },
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum ReviewDto {
  Unreviewed,
  Rejected,
  ReviewedAndPromoted { review_id: String },
}

impl ReviewDto {
  fn into_domain(self) -> Result<GeneratedEvidenceReview, IslandPortClientError> {
    Ok(match self {
      Self::Unreviewed => GeneratedEvidenceReview::Unreviewed,
      Self::Rejected => GeneratedEvidenceReview::Rejected,
      Self::ReviewedAndPromoted { review_id } => GeneratedEvidenceReview::ReviewedAndPromoted {
        review_id: canonical_id(review_id)?,
      },
    })
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssertionDto {
  text: String,
  status: StatusDto,
  evidence_ids: Vec<String>,
}

impl AssertionDto {
  fn into_domain(
    self,
    release_id: &crate::domain::canonical::ReleaseId,
    kind: crate::domain::canonical_content::CanonicalDetailKind,
    lineages: &BTreeMap<String, CanonicalEvidenceLineage>,
    used: &mut BTreeSet<String>,
  ) -> Result<CanonicalFactualAssertion, IslandPortClientError> {
    let mut evidence = Vec::with_capacity(self.evidence_ids.len());
    for id in self.evidence_ids {
      if !used.insert(id.clone()) && !lineages.contains_key(&id) {
        return Err(IslandPortClientError::InconsistentData);
      }
      evidence.push(
        lineages
          .get(&id)
          .cloned()
          .ok_or(IslandPortClientError::InconsistentData)?,
      );
    }
    CanonicalFactualAssertion::new(
      kind,
      release_id.clone(),
      self.status.into_domain(),
      self.text,
      evidence,
    )
    .map_err(inconsistent)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalizedGlossDto {
  id: String,
  language: String,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PronunciationDto {
  id: String,
  scope: PronunciationScopeDto,
  dialect: String,
  notation: PronunciationNotationDto,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageLabelDto {
  id: String,
  kind: UsageKindDto,
  code: String,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrammarPatternDto {
  id: String,
  kind: GrammarKindDto,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollocationDto {
  id: String,
  target_role: CollocationRoleDto,
  construction: CollocationConstructionDto,
  head: CollocationTermDto,
  dependent: CollocationTermDto,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExampleDto {
  id: String,
  language: String,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PitfallDto {
  id: String,
  learner_language: String,
  kind: PitfallKindDto,
  mistake: AssertionDto,
  correction: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EtymologyDto {
  id: String,
  scope: EtymologyScopeDto,
  kind: EtymologyKindDto,
  source_language: String,
  period: PeriodDto,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryDto {
  id: String,
  kind: HistoryKindDto,
  period: PeriodDto,
  assertion: AssertionDto,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollocationTermDto {
  surface: String,
  lexeme_id: Option<String>,
  sense_id: Option<String>,
}
impl CollocationTermDto {
  fn into_domain(self) -> Result<CollocationTerm, IslandPortClientError> {
    CollocationTerm::new(
      self.surface,
      self.lexeme_id.map(canonical_id).transpose()?,
      self.sense_id.map(canonical_id).transpose()?,
    )
    .map_err(inconsistent)
  }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeriodDto {
  first_year: Option<i32>,
  last_year: Option<i32>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeFactsGetInputDto {
  facts: Vec<CanonicalFactRefDto>,
  content_release: String,
  canonical_schema_version: String,
  verification_states: Vec<&'static str>,
  limit: usize,
}

impl KnowledgeFactsGetInputDto {
  fn new(value: KnowledgeFactsGetInput, pin: &CanonicalReleasePin) -> Self {
    Self {
      facts: value
        .facts
        .into_iter()
        .map(|fact| CanonicalFactRefDto {
          fact_id: fact.fact_id.to_string(),
          revision: fact.revision,
        })
        .collect(),
      content_release: pin.release_id.to_string(),
      canonical_schema_version: pin.canonical_schema_version.clone(),
      verification_states: value
        .verification_states
        .into_iter()
        .map(verification_state_name)
        .collect(),
      limit: value.limit,
    }
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CanonicalFactRefDto {
  fact_id: String,
  revision: u32,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct KnowledgeFactsGetResponseDto {
  common: CommonResponseDto<KnowledgeFactsValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeFactsValueDto {
  facts: Vec<KnowledgeFactDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeFactDto {
  fact_id: String,
  revision: u32,
  statement: String,
  subject_node_id: String,
  predicate: String,
  relation_registry_version: u32,
  object_node_id: String,
  domain_ids: Vec<String>,
  applicable_sense_ids: Vec<String>,
  conditions: Vec<KnowledgeConditionDto>,
  evidence_ids: Vec<String>,
  provenance: Vec<String>,
  verification_state: String,
}

impl KnowledgeFactDto {
  fn into_domain(self) -> Result<KnowledgeFact, IslandPortClientError> {
    if self.verification_state != "verified" {
      return Err(IslandPortClientError::InconsistentData);
    }
    let fact = KnowledgeFact {
      fact_id: canonical_id(self.fact_id)?,
      revision: self.revision,
      statement: self.statement,
      subject_node_id: canonical_id(self.subject_node_id)?,
      predicate: RetrievalRelation::from_wire_name(&self.predicate).map_err(inconsistent)?,
      relation_registry_version: self.relation_registry_version,
      object_node_id: canonical_id(self.object_node_id)?,
      domain_ids: self
        .domain_ids
        .into_iter()
        .map(DomainId::new)
        .collect::<Result<_, _>>()
        .map_err(inconsistent)?,
      applicable_sense_ids: self
        .applicable_sense_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
      conditions: self
        .conditions
        .into_iter()
        .map(KnowledgeConditionDto::into_domain)
        .collect::<Result<_, _>>()?,
      evidence_ids: self
        .evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
      provenance: self
        .provenance
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
    };
    fact.validate().map_err(inconsistent)?;
    Ok(fact)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeConditionDto {
  condition_id: String,
  condition_type: String,
  parameter_ids: Vec<String>,
}

impl KnowledgeConditionDto {
  fn into_domain(self) -> Result<KnowledgeCondition, IslandPortClientError> {
    let condition = KnowledgeCondition {
      condition_id: canonical_id(self.condition_id)?,
      condition_type: self.condition_type,
      parameter_ids: self
        .parameter_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
    };
    condition.validate().map_err(inconsistent)?;
    Ok(condition)
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SemanticScalesGetInputDto {
  scale_ids: Vec<String>,
  for_node_id: String,
  content_release: String,
  canonical_schema_version: String,
  verification_states: Vec<&'static str>,
  limit: usize,
}

impl SemanticScalesGetInputDto {
  fn new(value: SemanticScalesGetInput, pin: &CanonicalReleasePin) -> Self {
    Self {
      scale_ids: value
        .scale_ids
        .into_iter()
        .map(|id| id.to_string())
        .collect(),
      for_node_id: value.for_node_id.to_string(),
      content_release: pin.release_id.to_string(),
      canonical_schema_version: pin.canonical_schema_version.clone(),
      verification_states: value
        .verification_states
        .into_iter()
        .map(verification_state_name)
        .collect(),
      limit: value.limit,
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct SemanticScalesGetResponseDto {
  common: CommonResponseDto<SemanticScalesValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticScalesValueDto {
  scales: Vec<SemanticScaleDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticScaleDto {
  scale_id: String,
  revision: u32,
  dimension: String,
  direction: String,
  domain_ids: Vec<String>,
  conditions: Vec<KnowledgeConditionDto>,
  members: Vec<ScaleMemberDto>,
  evidence_ids: Vec<String>,
  verification_state: String,
}

impl SemanticScaleDto {
  fn into_domain(
    self,
    for_node_id: &CanonicalId,
  ) -> Result<HydratedSemanticScale, IslandPortClientError> {
    if self.verification_state != "verified" {
      return Err(IslandPortClientError::InconsistentData);
    }
    let direction = match self.direction.as_str() {
      "increasing" => SemanticScaleDirection::Increasing,
      "decreasing" => SemanticScaleDirection::Decreasing,
      _ => return Err(IslandPortClientError::InconsistentData),
    };
    let scale = HydratedSemanticScale {
      scale_id: canonical_id(self.scale_id)?,
      revision: self.revision,
      dimension: self.dimension,
      direction,
      domain_ids: self
        .domain_ids
        .into_iter()
        .map(DomainId::new)
        .collect::<Result<_, _>>()
        .map_err(inconsistent)?,
      conditions: self
        .conditions
        .into_iter()
        .map(KnowledgeConditionDto::into_domain)
        .collect::<Result<_, _>>()?,
      members: self
        .members
        .into_iter()
        .map(ScaleMemberDto::into_domain)
        .collect::<Result<_, _>>()?,
      evidence_ids: self
        .evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
    };
    scale.validate().map_err(inconsistent)?;
    if !scale
      .members
      .iter()
      .any(|member| member.node_id == *for_node_id)
    {
      return Err(IslandPortClientError::InconsistentData);
    }
    Ok(scale)
  }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScaleMemberDto {
  node_id: String,
  position: u32,
}

impl ScaleMemberDto {
  fn into_domain(self) -> Result<HydratedScaleMember, IslandPortClientError> {
    Ok(HydratedScaleMember {
      node_id: canonical_id(self.node_id)?,
      position: self.position,
    })
  }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeNodesGetInputDto {
  node_ids: Vec<String>,
  content_release: String,
  canonical_schema_version: String,
  evidence_use: EvidenceUseDto,
  limit: usize,
}

impl KnowledgeNodesGetInputDto {
  fn new(value: KnowledgeNodesGetInput, pin: &CanonicalReleasePin) -> Self {
    Self {
      node_ids: value
        .node_ids
        .into_iter()
        .map(|id| id.to_string())
        .collect(),
      content_release: pin.release_id.to_string(),
      canonical_schema_version: pin.canonical_schema_version.clone(),
      evidence_use: EvidenceUseDto::from(value.evidence_use),
      limit: value.limit,
    }
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct KnowledgeNodesGetResponseDto {
  common: CommonResponseDto<KnowledgeNodesValueDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeNodesValueDto {
  nodes: Vec<KnowledgeNodeDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeNodeDto {
  node_id: String,
  revision: u32,
  node_type: String,
  sense_id: Option<String>,
  canonical_label: String,
  language: Option<String>,
  domain_ids: Vec<String>,
  evidence_ids: Vec<String>,
  verification_state: String,
}

impl KnowledgeNodeDto {
  fn into_domain(self) -> Result<HydratedKnowledgeNode, IslandPortClientError> {
    if self.verification_state != "verified" {
      return Err(IslandPortClientError::InconsistentData);
    }
    let node = HydratedKnowledgeNode {
      node_id: canonical_id(self.node_id)?,
      revision: self.revision,
      node_type: RetrievalNodeType::new(self.node_type).map_err(inconsistent)?,
      sense_id: self.sense_id.map(canonical_id).transpose()?,
      canonical_label: self.canonical_label,
      language: self.language.as_deref().map(language).transpose()?,
      domain_ids: self
        .domain_ids
        .into_iter()
        .map(DomainId::new)
        .collect::<Result<_, _>>()
        .map_err(inconsistent)?,
      evidence_ids: self
        .evidence_ids
        .into_iter()
        .map(canonical_id)
        .collect::<Result<_, _>>()?,
    };
    node.validate().map_err(inconsistent)?;
    Ok(node)
  }
}

fn validate_exact_fact_input(input: &KnowledgeFactsGetInput) -> Result<(), IslandPortClientError> {
  if input.facts.is_empty()
    || input.facts.len() > MAX_KNOWLEDGE_HYDRATION_ITEMS
    || input.limit == 0
    || input.limit > MAX_KNOWLEDGE_HYDRATION_ITEMS
    || input.facts.iter().any(|fact| fact.revision == 0)
    || input
      .facts
      .iter()
      .map(|fact| &fact.fact_id)
      .collect::<BTreeSet<_>>()
      .len()
      != input.facts.len()
  {
    return Err(IslandPortClientError::InvalidRequest);
  }
  validate_verified(&input.verification_states)
}

fn validate_id_input(ids: &[CanonicalId], limit: usize) -> Result<(), IslandPortClientError> {
  if ids.is_empty()
    || ids.len() > MAX_KNOWLEDGE_HYDRATION_ITEMS
    || limit == 0
    || limit > MAX_KNOWLEDGE_HYDRATION_ITEMS
    || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
  {
    return Err(IslandPortClientError::InvalidRequest);
  }
  Ok(())
}

fn validate_verified(states: &[RetrievalVerificationState]) -> Result<(), IslandPortClientError> {
  if states != [RetrievalVerificationState::Verified] {
    return Err(IslandPortClientError::InvalidRequest);
  }
  Ok(())
}

fn validate_fact_subsequence(
  requested: &[CanonicalFactRef],
  returned: &[KnowledgeFact],
) -> Result<(), IslandPortClientError> {
  let mut position = 0;
  for fact in returned {
    let Some(offset) = requested[position..]
      .iter()
      .position(|item| item.fact_id == fact.fact_id && item.revision == fact.revision)
    else {
      return Err(IslandPortClientError::InconsistentData);
    };
    position += offset + 1;
  }
  Ok(())
}

fn validate_id_subsequence<'a>(
  requested: &[CanonicalId],
  returned: impl Iterator<Item = &'a CanonicalId>,
) -> Result<(), IslandPortClientError> {
  let mut position = 0;
  for id in returned {
    let Some(offset) = requested[position..].iter().position(|item| item == id) else {
      return Err(IslandPortClientError::InconsistentData);
    };
    position += offset + 1;
  }
  Ok(())
}

const fn verification_state_name(state: RetrievalVerificationState) -> &'static str {
  match state {
    RetrievalVerificationState::Verified => "verified",
    RetrievalVerificationState::Exploratory => "exploratory",
  }
}
impl PeriodDto {
  fn into_domain(self) -> Result<HistoricalRange, IslandPortClientError> {
    HistoricalRange::new(self.first_year, self.last_year).map_err(inconsistent)
  }
}

macro_rules! string_enum {
  ($wire:ident => $domain:ty { $($variant:ident),+ $(,)? }) => {
    #[derive(Deserialize)] #[serde(rename_all = "snake_case")] enum $wire { $($variant),+ }
    impl $wire { const fn into_domain(self) -> $domain { match self { $(Self::$variant => <$domain>::$variant),+ } } }
  };
}
string_enum!(PronunciationScopeDto => PronunciationScope { LexemeWide, SenseSpecific });
string_enum!(PronunciationNotationDto => PronunciationNotation { Ipa, Phonemic, SourceDefined });
string_enum!(UsageKindDto => UsageLabelKind { Register, Domain, Dialect, Connotation, Politeness, Datedness, Sensitivity, Frequency, Level });
string_enum!(GrammarKindDto => GrammarPatternKind { Valency, Construction, Government, Morphology, SourceDefined });
string_enum!(CollocationRoleDto => CollocationRole { Head, Dependent });
string_enum!(CollocationConstructionDto => CollocationConstruction { VerbObject, AdjectiveNoun, AdverbModifier, NounCompound, GovernedComplement, SourceDefined });
string_enum!(PitfallKindDto => LearnerPitfallKind { FalseFriend, Confusable, LiteralTranslation, CommonError });
string_enum!(EtymologyScopeDto => EtymologyScope { LexemeWide, SenseSpecific });
string_enum!(EtymologyKindDto => EtymologyKind { BorrowedFrom, DerivedFrom, CognateWith, OriginSummary });
string_enum!(HistoryKindDto => SenseHistoryEventKind { Attestation, SemanticShift, ScopeChange, Retirement });

fn canonical_id(value: String) -> Result<CanonicalId, IslandPortClientError> {
  CanonicalId::new(value).map_err(inconsistent)
}

fn language(value: &str) -> Result<LanguageTag, IslandPortClientError> {
  LanguageTag::parse(value).map_err(inconsistent)
}

fn inconsistent<T>(_: T) -> IslandPortClientError {
  IslandPortClientError::InconsistentData
}
