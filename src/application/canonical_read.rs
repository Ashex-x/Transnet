//! One-request canonical-only composition over release-pinned read capabilities.
//!
//! No model, vector index, HTTP handler, or storage implementation participates here. The active
//! release is selected once and retained even if the authority changes its active pointer.

use std::sync::Arc;

use crate::{
  application::canonical_lookup_card::CanonicalLookupCardMapper,
  domain::{
    canonical::{
      normalize_lookup_key, CanonicalReleasePin, LanguageTag, CANONICAL_LOOKUP_NORMALIZER_VERSION,
    },
    canonical_content::CanonicalSenseDetails,
    canonical_translation::{
      CanonicalTranslationRevision, SourceFingerprint, MAX_CANONICAL_SOURCE_CHARS,
    },
    lookup_card::CanonicalLookupCard,
    retrieval::{rank_lexical_candidates, LexicalMatchKind, RetrievalRequest},
  },
  ports::canonical_read::{
    CanonicalCandidateQuery, CanonicalLookupForm, CanonicalReadContext, CanonicalReadError,
    CanonicalReadPort, CanonicalSenseQuery, CanonicalTranslationQuery,
  },
};

/// Complete request-local canonical-only read assembled without a vector-version placeholder.
pub struct CanonicalReadOutcome {
  /// Immutable release used by every authority read in this flow.
  pub pin: CanonicalReleasePin,
  /// Evidence-backed deterministic card retaining genuine sense ambiguity.
  pub card: CanonicalLookupCard<CanonicalReleasePin>,
  /// Details only when exactly one sense is resolved.
  pub sense_details: Option<CanonicalSenseDetails>,
  /// Reviewed translations whose stored source exactly matches the request.
  pub translations: Vec<CanonicalTranslationRevision>,
}

/// Canonical-only application flow backed by a read capability, not a transport client.
#[derive(Clone)]
pub struct CanonicalReadService {
  authority: Arc<dyn CanonicalReadPort>,
}

impl CanonicalReadService {
  /// Creates a request-local composition over a canonical authority port.
  pub fn new(authority: Arc<dyn CanonicalReadPort>) -> Self {
    Self { authority }
  }

  /// Resolves reviewed translations, a basic card, and unambiguous sense details under one pin.
  ///
  /// The source is used only in memory for exact candidate verification. A fingerprint hit alone
  /// never establishes translation identity. Every downstream read receives the same release.
  ///
  /// # Errors
  ///
  /// Returns a closed authority failure; no partial card is returned on a failed downstream read.
  pub async fn resolve(
    &self,
    context: &CanonicalReadContext,
    request: RetrievalRequest,
    source_text: &str,
    target_language: LanguageTag,
  ) -> Result<CanonicalReadOutcome, CanonicalReadError> {
    if source_text.chars().count() > MAX_CANONICAL_SOURCE_CHARS
      || request.query.len() > MAX_CANONICAL_SOURCE_CHARS * 4
      || normalize_lookup_key(source_text) != request.query
    {
      return Err(CanonicalReadError::InvalidRequest);
    }
    let pin = self
      .authority
      .active_release(context)
      .await?
      .ok_or(CanonicalReadError::NotFound)?;

    let translation_candidates = self
      .authority
      .translations(
        context,
        &pin,
        CanonicalTranslationQuery {
          source_fingerprint: SourceFingerprint::compute(source_text, &request.language),
          source_language: request.language.clone(),
          target_language: target_language.clone(),
          sense_id: None,
          domain_ids: Vec::new(),
          dialect: None,
          register: None,
          limit: request.limit,
        },
      )
      .await?;
    if translation_candidates.len() > request.limit {
      return Err(CanonicalReadError::InconsistentData);
    }
    if translation_candidates
      .iter()
      .any(|candidate| candidate.release_id() != &pin.release_id)
    {
      return Err(CanonicalReadError::VersionMismatch);
    }
    let translations = translation_candidates
      .into_iter()
      .filter(|candidate| {
        candidate.release_id() == &pin.release_id
          && candidate.source_language() == &request.language
          && candidate.target_language() == &target_language
          && candidate.verifies_source(source_text)
      })
      .collect();

    let lexical_matches = self
      .authority
      .candidates(
        context,
        &pin,
        CanonicalCandidateQuery {
          lookup_forms: vec![CanonicalLookupForm {
            form: request.query.clone(),
            match_class: LexicalMatchKind::ExactCanonical,
            rank: 0,
          }],
          normalizer_version: CANONICAL_LOOKUP_NORMALIZER_VERSION.to_string(),
          source_language: request.language.clone(),
          explanation_language: request.language.clone(),
          dialect: None,
          evidence_use: request.evidence_use,
          limit: request.limit,
        },
      )
      .await?;
    if lexical_matches.len() > request.limit {
      return Err(CanonicalReadError::InconsistentData);
    }
    if lexical_matches.iter().any(|matched| {
      matched.candidate.lexeme.release_id != pin.release_id
        || matched.candidate.sense.release_id != pin.release_id
        || matched
          .candidate
          .forms
          .iter()
          .any(|form| form.release_id != pin.release_id)
        || matched
          .candidate
          .evidence
          .iter()
          .any(|evidence| evidence.release_id != pin.release_id)
    }) {
      return Err(CanonicalReadError::VersionMismatch);
    }
    let ranked = rank_lexical_candidates(&pin.release_id, request.evidence_use, lexical_matches);
    let card = CanonicalLookupCardMapper::assemble_canonical(&request, pin.clone(), ranked);
    let sense_details = if card.candidates.len() == 1 {
      let sense_id = card.candidates[0].sense.id.clone();
      let details = self
        .authority
        .sense(
          context,
          &pin,
          CanonicalSenseQuery {
            sense_id: sense_id.clone(),
            explanation_language: request.language.clone(),
            dialect: None,
            evidence_use: request.evidence_use,
          },
        )
        .await?;
      if details.target().release_id() != &pin.release_id
        || details.target().sense_id() != &sense_id
        || !details.is_eligible_for(&pin.release_id, &sense_id, request.evidence_use)
      {
        return Err(CanonicalReadError::InconsistentData);
      }
      Some(details)
    } else {
      None
    };

    Ok(CanonicalReadOutcome {
      pin,
      card,
      sense_details,
      translations,
    })
  }
}
