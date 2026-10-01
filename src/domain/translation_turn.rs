//! Request-local unified translation values, normalization, and deterministic response projection.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Hard bound for serialized input accepted through either HTTP or the public service API.
pub const MAX_TURN_BYTES: usize = 1_048_576;
/// Version of the request-local translation lookup normalization rules.
pub const NORMALIZER_VERSION: &str = "translation-lookup-nfc-v1";
/// Version of deterministic response breadth rules.
pub const PROJECTION_VERSION: &str = "translation-projection-v1";
/// Version of the response-level-independent translation result schema.
pub const TRANSLATION_RESULT_SCHEMA_VERSION: &str = "translation-result-v1";
/// Long input is conservatively translated as connected text, not classified as one lexical unit.
pub const MAX_LEXICAL_CHARS: usize = 128;
/// Maximum number of deterministic lookup forms emitted for one input.
pub const MAX_DERIVED_FORMS: usize = 4;
/// Maximum Unicode scalar count accepted for one text input or all segments together.
pub const MAX_INPUT_SCALARS: usize = 131_072;
/// Maximum number of structured segments.
pub const MAX_SEGMENTS: usize = 256;
/// Maximum Unicode scalar count in one structured segment.
pub const MAX_SEGMENT_SCALARS: usize = 8_192;
/// Maximum protected ranges in one segment.
pub const MAX_PROTECTED_RANGES: usize = 128;
/// Maximum number of inline images.
pub const MAX_IMAGES: usize = 4;
/// Maximum decoded bytes in one inline image.
pub const MAX_IMAGE_BYTES: usize = 2 * 1_048_576;
/// Maximum pixels along either image dimension.
pub const MAX_IMAGE_DIMENSION: u32 = 4_096;
/// Maximum number of image regions across a request.
pub const MAX_IMAGE_REGIONS: usize = 16;
/// Maximum terminology entries in request-scoped guidance.
pub const MAX_TERMINOLOGY: usize = 128;
/// Maximum Unicode scalar count in one terminology side.
pub const MAX_TERM_SCALARS: usize = 256;
/// Maximum JSON bytes retained for history and guidance passed to one generation prompt.
pub const MAX_GENERATION_CONTEXT_BYTES: usize = 8_192;
/// Maximum Unicode scalar count for caller-owned and response-local opaque identities.
pub const MAX_TRANSLATION_RESULT_ID_SCALARS: usize = 128;
/// Maximum distinct version values reported for one request.
pub const MAX_TRANSLATION_VERSION_VALUES: usize = 16;

/// One prior linguistic turn, without identity, timestamps, or persistence instructions.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationHistory {
  /// Previous source text, retained only during the current request.
  pub source_text: String,
  /// Previous translated text used only for linguistic continuity.
  pub translated_text: String,
  /// Known source language of this prior turn: en or zh-CN.
  pub source_language: String,
  /// Known target language of this prior turn: en or zh-CN.
  pub target_language: String,
}

/// Strict wire input for POST /api/v1/translations.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationTurnRequest {
  /// Legacy text input retained during target-contract migration.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  /// Target tagged input; exactly one of this or legacy `text` is required.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub input: Option<TranslationInput>,
  /// auto, en, or zh-CN.
  pub source_language: String,
  /// en or zh-CN.
  pub target_language: String,
  /// brief, standard, or full; applied only after generation.
  pub response_level: String,
  /// Chronological minimal turns, with no independent item-count cap.
  #[serde(default)]
  pub history: Vec<TranslationHistory>,
  /// Request-scoped professional constraints.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub guidance: Option<TranslationGuidance>,
}

/// Closed translation input union.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TranslationInput {
  /// One lexical or connected text value.
  Text {
    /// Source text to translate.
    text: String,
  },
  /// Ordered document or localization segments.
  Segments {
    /// Segments in caller-defined reading order.
    segments: Vec<TranslationSegment>,
  },
  /// Sanitized inline images with bounded regions.
  ImageRegions {
    /// Inline images containing the requested regions.
    images: Vec<TranslationImage>,
    /// Region identifiers in caller-defined reading order.
    reading_order: Vec<String>,
  },
}

/// Closed discriminator for a validated translation input without exposing its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationInputKind {
  /// One lexical or connected text value.
  Text,
  /// Ordered document or localization segments.
  Segments,
  /// Sanitized inline images with bounded regions.
  ImageRegions,
}

impl TranslationInput {
  /// Returns the content-free discriminator for this input shape.
  pub const fn kind(&self) -> TranslationInputKind {
    match self {
      Self::Text { .. } => TranslationInputKind::Text,
      Self::Segments { .. } => TranslationInputKind::Segments,
      Self::ImageRegions { .. } => TranslationInputKind::ImageRegions,
    }
  }
}

/// One ordered structured translation segment.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationSegment {
  /// Request-local opaque segment identifier.
  pub segment_id: String,
  /// Segment source text.
  pub text: String,
  /// Closed semantic role.
  pub role: SegmentRole,
  /// Closed markup mode.
  pub format: SegmentFormat,
  /// Non-overlapping Unicode-scalar ranges copied unchanged.
  #[serde(default)]
  pub protected_ranges: Vec<ProtectedRange>,
}

/// Closed segment roles.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentRole {
  /// Document or section title.
  Title,
  /// Prose paragraph.
  Paragraph,
  /// One list item.
  ListItem,
  /// Image or table caption.
  Caption,
  /// User-interface copy.
  Ui,
  /// Timed subtitle text.
  Subtitle,
}

/// Closed segment formats.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentFormat {
  /// Unformatted plain text.
  Plain,
  /// Markdown source.
  Markdown,
  /// Sanitized HTML source.
  Html,
}

/// Start-inclusive, end-exclusive Unicode-scalar range.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedRange {
  /// First protected scalar.
  pub start: usize,
  /// Scalar after the protected range.
  pub end: usize,
}

/// One sanitized inline image.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationImage {
  /// Request-local opaque image identifier.
  pub image_id: String,
  /// PNG, JPEG, or WebP media type.
  pub media_type: String,
  /// Standard padded base64 image bytes.
  pub data: String,
  /// Bounded normalized rectangles.
  pub regions: Vec<ImageRegion>,
}

/// One normalized image rectangle.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRegion {
  /// Request-local opaque region identifier.
  pub region_id: String,
  /// Left coordinate.
  pub x: f64,
  /// Top coordinate.
  pub y: f64,
  /// Positive normalized width.
  pub width: f64,
  /// Positive normalized height.
  pub height: f64,
}

/// Request-scoped translation guidance.
#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TranslationGuidance {
  /// General, publication, technical, localization, or subtitles.
  #[serde(default)]
  pub purpose: Option<GuidancePurpose>,
  /// General, professional, specialist, or young-reader audience.
  #[serde(default)]
  pub audience: Option<GuidanceAudience>,
  /// Preserve, neutral, formal, or informal register.
  #[serde(default)]
  pub register: Option<GuidanceRegister>,
  /// Request-local terminology constraints.
  #[serde(default)]
  pub terminology: Vec<TerminologyConstraint>,
  /// Requested alternatives; only zero is currently implemented.
  #[serde(default)]
  pub max_alternatives: u8,
  /// Closed annotation families requested by the caller.
  #[serde(default)]
  pub annotations: Vec<AnnotationFamily>,
  /// Offline, allowed, or required freshness policy.
  #[serde(default)]
  pub freshness: Option<FreshnessPolicy>,
}

/// Closed purposes for a translation request.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuidancePurpose {
  /// General-purpose translation.
  General,
  /// Publication-ready copy.
  Publication,
  /// Technical material.
  Technical,
  /// Product localization.
  Localization,
  /// Subtitle translation.
  Subtitles,
}

/// Closed intended audiences for a translation request.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceAudience {
  /// A general audience.
  General,
  /// A professional audience.
  Professional,
  /// A domain-specialist audience.
  Specialist,
  /// Younger readers.
  YoungReader,
}

/// Closed register preferences for generated text.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceRegister {
  /// Preserve the source register.
  Preserve,
  /// Use neutral language.
  Neutral,
  /// Use formal language.
  Formal,
  /// Use informal language.
  Informal,
}

/// Closed policies for one terminology constraint.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminologyPolicy {
  /// The requested target term must be used.
  Required,
  /// The requested target term should be preferred.
  Preferred,
  /// The requested target term must not be used.
  Forbidden,
}

/// Closed annotation families callers may request.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationFamily {
  /// Meaning ambiguity annotations.
  Ambiguity,
  /// Terminology-choice annotations.
  Terminology,
  /// Register annotations.
  Register,
  /// Cultural-context annotations.
  Culture,
  /// Source-format annotations.
  Format,
  /// Human-review annotations.
  Review,
}

/// Closed live-retrieval policy for a request.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessPolicy {
  /// Never perform live retrieval.
  Offline,
  /// Permit one bounded live-retrieval round.
  Allowed,
  /// Require live retrieval or fail explicitly.
  Required,
}

/// One source-to-target terminology constraint.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminologyConstraint {
  /// Exact source term.
  pub source: String,
  /// Required, preferred, or forbidden target term.
  pub target: String,
  /// Constraint strength.
  pub policy: TerminologyPolicy,
}

/// Source-language selector supported by the initial translation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
  /// Detect English or Simplified Chinese for the current request.
  Auto,
  /// Use the caller-declared supported language.
  Known(TurnLanguage),
}

impl Serialize for SourceLanguage {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.as_str())
  }
}

impl SourceLanguage {
  /// Returns the exact wire selector.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Known(language) => language.as_str(),
    }
  }

  fn parse(value: &str) -> Option<Self> {
    match value {
      "auto" => Some(Self::Auto),
      _ => TurnLanguage::parse(value).map(Self::Known),
    }
  }
}

/// Initial product languages; provider output cannot introduce another language tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnLanguage {
  /// English.
  #[serde(rename = "en")]
  English,
  /// Simplified Chinese.
  #[serde(rename = "zh-CN")]
  Chinese,
}

impl TurnLanguage {
  /// Returns the exact wire language tag.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::English => "en",
      Self::Chinese => "zh-CN",
    }
  }

  /// Parses one exact language tag from the initial closed language set.
  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "en" => Some(Self::English),
      "zh-CN" => Some(Self::Chinese),
      _ => None,
    }
  }
}

/// Deterministic response breadth selected by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseLevel {
  /// Translations and ambiguity labels only.
  Brief,
  /// Concise usage and one example per meaning.
  Standard,
  /// All bounded eligible model-generated lexical details.
  Full,
}

impl ResponseLevel {
  /// Parses one exact response level from the closed wire set.
  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "brief" => Some(Self::Brief),
      "standard" => Some(Self::Standard),
      "full" => Some(Self::Full),
      _ => None,
    }
  }
}

/// Closed input-validation error that never includes the supplied value.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum TurnValidationError {
  /// A required input value is invalid.
  #[error("invalid translation field: {0}")]
  Field(&'static str),
  /// The serialized request exceeds the service byte budget.
  #[error("translation request is too large")]
  TooLarge,
  /// Valid constraints contradict one another.
  #[error("translation constraints conflict: {0}")]
  ConstraintConflict(&'static str),
  /// Input is valid but its application result is not implemented yet.
  #[error("translation input capability is unavailable: {0}")]
  Unsupported(&'static str),
  /// The declared inline-image media type is not supported.
  #[error("unsupported translation image media type")]
  UnsupportedImageMediaType,
}

/// Validated linguistic input; callers cannot mutate it after validation.
#[derive(Clone)]
pub struct TranslationTurn {
  input: TranslationInput,
  source_language: SourceLanguage,
  target_language: TurnLanguage,
  history: Vec<TranslationHistory>,
  guidance: TranslationGuidance,
  response_level: ResponseLevel,
}

impl Serialize for TranslationTurn {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::{Error as _, SerializeStruct as _};

    let text = self
      .text()
      .ok_or_else(|| S::Error::custom("structured translation input has no model serialization"))?;
    let mut state = serializer.serialize_struct("TranslationTurn", 5)?;
    state.serialize_field("text", text)?;
    state.serialize_field("source_language", &self.source_language)?;
    state.serialize_field("target_language", &self.target_language)?;
    state.serialize_field("history", &self.history)?;
    state.serialize_field("guidance", &self.guidance)?;
    state.end()
  }
}

impl TranslationTurn {
  /// Validates closed selectors and every history item without imposing a history-count cap.
  ///
  /// # Errors
  /// Returns a closed field error or a request-size error before any provider is called.
  pub fn new(request: TranslationTurnRequest) -> Result<Self, TurnValidationError> {
    let encoded_bytes = serde_json::to_vec(&request)
      .map_err(|_| TurnValidationError::Field("input"))?
      .len();
    let input = match (request.text, request.input) {
      (Some(text), None) => TranslationInput::Text { text },
      (None, Some(input)) => input,
      _ => return Err(TurnValidationError::Field("input")),
    };
    let guidance = request.guidance.unwrap_or_default();
    validate_guidance(&guidance)?;
    let generation_context_bytes = serde_json::to_vec(&(&request.history, &guidance))
      .map_err(|_| TurnValidationError::Field("generation_context"))?
      .len();
    if generation_context_bytes > MAX_GENERATION_CONTEXT_BYTES {
      return Err(TurnValidationError::Field("generation_context"));
    }
    validate_input(&input, &guidance)?;
    if encoded_bytes > MAX_TURN_BYTES {
      return Err(TurnValidationError::TooLarge);
    }
    let source_language = SourceLanguage::parse(&request.source_language)
      .ok_or(TurnValidationError::Field("source_language"))?;
    let target_language = TurnLanguage::parse(&request.target_language)
      .ok_or(TurnValidationError::Field("target_language"))?;
    let response_level = ResponseLevel::parse(&request.response_level)
      .ok_or(TurnValidationError::Field("response_level"))?;
    for turn in &request.history {
      if turn.source_text.trim().is_empty()
        || turn.translated_text.trim().is_empty()
        || TurnLanguage::parse(&turn.source_language).is_none()
        || TurnLanguage::parse(&turn.target_language).is_none()
      {
        return Err(TurnValidationError::Field("history"));
      }
    }
    Ok(Self {
      input,
      source_language,
      target_language,
      history: request.history,
      guidance,
      response_level,
    })
  }

  /// Returns the complete validated request-local input.
  pub const fn input(&self) -> &TranslationInput {
    &self.input
  }
  /// Returns the input discriminator without exposing request content.
  pub const fn input_kind(&self) -> TranslationInputKind {
    self.input.kind()
  }
  /// Returns the original text for a text input without changing formatting boundaries.
  pub fn text(&self) -> Option<&str> {
    match &self.input {
      TranslationInput::Text { text } => Some(text),
      TranslationInput::Segments { .. } | TranslationInput::ImageRegions { .. } => None,
    }
  }
  /// Returns the source-language selector.
  pub const fn source_language(&self) -> SourceLanguage {
    self.source_language
  }
  /// Returns the requested target language.
  pub fn target_language(&self) -> TurnLanguage {
    self.target_language
  }
  /// Returns chronological request-local context.
  pub fn history(&self) -> &[TranslationHistory] {
    &self.history
  }
  /// Returns validated request-local guidance, which must be discarded with this turn.
  pub const fn guidance(&self) -> &TranslationGuidance {
    &self.guidance
  }
  /// Reports whether accepted guidance needs orchestration that is not composed yet.
  pub fn requires_guidance_execution(&self) -> bool {
    guidance_requires_execution(&self.guidance)
  }
  /// Returns the projection level, never passed to a generation prompt.
  pub fn response_level(&self) -> ResponseLevel {
    self.response_level
  }
  /// Derives one request-local lookup form while preserving significant symbols such as + and #.
  pub fn lookup_form(&self) -> Option<String> {
    self.text().map(|text| {
      TranslationNormalizer::new()
        .normalize(text, self.source_language)
        .primary
    })
  }
  /// Returns whether input is too long or structured to be one lexical unit.
  pub fn requires_passage(&self) -> bool {
    self
      .text()
      .is_none_or(|text| text.chars().count() > MAX_LEXICAL_CHARS || text.contains(['\n', '\r']))
  }
}

fn validate_input(
  input: &TranslationInput,
  guidance: &TranslationGuidance,
) -> Result<(), TurnValidationError> {
  match input {
    TranslationInput::Text { text } => {
      if text.trim().is_empty() || text.chars().count() > MAX_INPUT_SCALARS {
        return Err(TurnValidationError::Field("input.text"));
      }
    }
    TranslationInput::Segments { segments } => {
      if segments.is_empty() || segments.len() > MAX_SEGMENTS {
        return Err(TurnValidationError::Field("input.segments"));
      }
      let mut ids = std::collections::BTreeSet::new();
      let mut total = 0usize;
      for segment in segments {
        let count = segment.text.chars().count();
        total = total
          .checked_add(count)
          .ok_or(TurnValidationError::TooLarge)?;
        if !bounded_id(&segment.segment_id)
          || !ids.insert(&segment.segment_id)
          || segment.text.trim().is_empty()
          || count > MAX_SEGMENT_SCALARS
          || segment.protected_ranges.len() > MAX_PROTECTED_RANGES
        {
          return Err(TurnValidationError::Field("input.segments"));
        }
        let mut protected_ranges = segment
          .protected_ranges
          .iter()
          .map(|range| (range.start, range.end))
          .collect::<Vec<_>>();
        protected_ranges.sort_unstable();
        let mut previous_end = 0;
        for &(start, end) in &protected_ranges {
          if start >= end || end > count || start < previous_end {
            return Err(TurnValidationError::Field(
              "input.segments.protected_ranges",
            ));
          }
          previous_end = end;
        }
        for term in &guidance.terminology {
          let source_present = term_occurrences(&segment.text, &term.source)
            .next()
            .is_some();
          let protected_contains = |value: &str| {
            term_occurrences(&segment.text, value).any(|(start, end)| {
              protected_ranges
                .iter()
                .any(|&(protected_start, protected_end)| {
                  start < protected_end && end > protected_start
                })
            })
          };
          let required_conflict = term.policy == TerminologyPolicy::Required
            && term.source != term.target
            && protected_contains(&term.source);
          let forbidden_conflict = term.policy == TerminologyPolicy::Forbidden
            && source_present
            && protected_contains(&term.target);
          if required_conflict || forbidden_conflict {
            return Err(TurnValidationError::ConstraintConflict(
              "guidance.terminology",
            ));
          }
        }
      }
      if total > MAX_INPUT_SCALARS {
        return Err(TurnValidationError::Field("input.segments"));
      }
    }
    TranslationInput::ImageRegions {
      images,
      reading_order,
    } => validate_images(images, reading_order)?,
  }
  Ok(())
}

fn term_occurrences(text: &str, term: &str) -> std::vec::IntoIter<(usize, usize)> {
  let text = text.chars().collect::<Vec<_>>();
  let term = term.chars().collect::<Vec<_>>();
  let term_len = term.len();
  text
    .windows(term_len)
    .enumerate()
    .filter(move |(_, candidate)| *candidate == term)
    .map(move |(start, _)| (start, start + term_len))
    .collect::<Vec<_>>()
    .into_iter()
}

fn guidance_requires_execution(guidance: &TranslationGuidance) -> bool {
  guidance.purpose.is_some()
    || guidance.audience.is_some()
    || guidance.register.is_some()
    || !guidance.terminology.is_empty()
    || !guidance.annotations.is_empty()
    || matches!(
      guidance.freshness,
      Some(FreshnessPolicy::Allowed | FreshnessPolicy::Required)
    )
}

fn validate_guidance(guidance: &TranslationGuidance) -> Result<(), TurnValidationError> {
  if !guidance.annotations.is_empty() {
    return Err(TurnValidationError::Unsupported("guidance.annotations"));
  }
  if guidance.max_alternatives > 2 {
    return Err(TurnValidationError::Field("guidance.max_alternatives"));
  }
  if guidance.max_alternatives != 0 {
    return Err(TurnValidationError::Unsupported(
      "guidance.max_alternatives",
    ));
  }
  if guidance.terminology.len() > MAX_TERMINOLOGY {
    return Err(TurnValidationError::Field("guidance.terminology"));
  }
  let mut required = std::collections::BTreeMap::new();
  for term in &guidance.terminology {
    if term.source.trim().is_empty()
      || term.target.trim().is_empty()
      || term.source.chars().count() > MAX_TERM_SCALARS
      || term.target.chars().count() > MAX_TERM_SCALARS
    {
      return Err(TurnValidationError::Field("guidance.terminology"));
    }
    if term.policy == TerminologyPolicy::Required
      && required
        .insert(&term.source, &term.target)
        .is_some_and(|old| old != &term.target)
    {
      return Err(TurnValidationError::ConstraintConflict(
        "guidance.terminology",
      ));
    }
    if guidance.terminology.iter().any(|other| {
      other.source == term.source
        && other.target == term.target
        && matches!(
          (term.policy, other.policy),
          (TerminologyPolicy::Required, TerminologyPolicy::Forbidden)
            | (TerminologyPolicy::Forbidden, TerminologyPolicy::Required)
        )
    }) {
      return Err(TurnValidationError::ConstraintConflict(
        "guidance.terminology",
      ));
    }
  }
  Ok(())
}

fn validate_images(
  images: &[TranslationImage],
  reading_order: &[String],
) -> Result<(), TurnValidationError> {
  if images.is_empty() || images.len() > MAX_IMAGES {
    return Err(TurnValidationError::Field("input.images"));
  }
  let mut image_ids = std::collections::BTreeSet::new();
  let mut keys = std::collections::BTreeSet::new();
  for image in images {
    if !bounded_id(&image.image_id)
      || image.image_id.contains(':')
      || !image_ids.insert(&image.image_id)
    {
      return Err(TurnValidationError::Field("input.images"));
    }
    if !matches!(
      image.media_type.as_str(),
      "image/png" | "image/jpeg" | "image/webp"
    ) {
      return Err(TurnValidationError::UnsupportedImageMediaType);
    }
    let bytes = BASE64
      .decode(&image.data)
      .map_err(|_| TurnValidationError::Field("input.images.data"))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
      return Err(TurnValidationError::Field("input.images.data"));
    }
    let (width, height) = image_dimensions(&bytes, &image.media_type)
      .ok_or(TurnValidationError::Field("input.images.data"))?;
    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
      return Err(TurnValidationError::Field("input.images.data"));
    }
    let mut region_ids = std::collections::BTreeSet::new();
    for region in &image.regions {
      if !bounded_id(&region.region_id)
        || region.region_id.contains(':')
        || !region_ids.insert(&region.region_id)
        || ![region.x, region.y, region.width, region.height]
          .iter()
          .all(|v| v.is_finite())
        || region.x < 0.0
        || region.y < 0.0
        || region.width <= 0.0
        || region.height <= 0.0
        || region.x + region.width > 1.0
        || region.y + region.height > 1.0
      {
        return Err(TurnValidationError::Field("input.images.regions"));
      }
      keys.insert(format!("{}:{}", image.image_id, region.region_id));
    }
  }
  if keys.len() > MAX_IMAGE_REGIONS
    || reading_order.len() != keys.len()
    || reading_order
      .iter()
      .collect::<std::collections::BTreeSet<_>>()
      .len()
      != reading_order.len()
    || reading_order.iter().any(|key| !keys.contains(key))
  {
    return Err(TurnValidationError::Field("input.reading_order"));
  }
  Ok(())
}

fn bounded_id(value: &str) -> bool {
  value.trim() == value
    && !value.is_empty()
    && value.chars().count() <= MAX_TRANSLATION_RESULT_ID_SCALARS
}

fn image_dimensions(bytes: &[u8], media_type: &str) -> Option<(u32, u32)> {
  match media_type {
    "image/png" if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" => Some((
      u32::from_be_bytes(bytes[16..20].try_into().ok()?),
      u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    )),
    "image/webp"
      if bytes.len() >= 30
        && &bytes[..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
        && &bytes[12..16] == b"VP8X" =>
    {
      Some((
        1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]),
        1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]),
      ))
    }
    "image/jpeg" => jpeg_dimensions(bytes),
    _ => None,
  }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
  if bytes.get(..2)? != b"\xff\xd8" {
    return None;
  }
  let mut offset = 2;
  while offset + 4 <= bytes.len() {
    if bytes[offset] != 0xff {
      return None;
    }
    let marker = bytes[offset + 1];
    offset += 2;
    if marker == 0xd9 || marker == 0xda {
      return None;
    }
    let length = u16::from_be_bytes([*bytes.get(offset)?, *bytes.get(offset + 1)?]) as usize;
    if length < 2 || offset + length > bytes.len() {
      return None;
    }
    if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) && length >= 7 {
      return Some((
        u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32,
        u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32,
      ));
    }
    offset += length;
  }
  None
}

/// Deterministic bounded lookup forms derived only for the current request.
#[derive(Clone, Serialize)]
pub struct NormalizedTranslationInput {
  /// NFC, case, whitespace, and punctuation-equivalent normalized primary form.
  pub primary: String,
  /// Deduplicated bounded alternatives ordered from strongest to weakest.
  pub derived_forms: Vec<String>,
}

/// Stateless versioned normalizer for translation lookup and intent classification.
#[derive(Debug, Clone, Copy, Default)]
pub struct TranslationNormalizer;

impl TranslationNormalizer {
  /// Creates the stateless normalizer.
  pub const fn new() -> Self {
    Self
  }

  /// Returns the stable ruleset version for response metadata and release compatibility.
  pub const fn version(self) -> &'static str {
    NORMALIZER_VERSION
  }

  /// Derives bounded lookup forms without altering the original request text.
  pub fn normalize(self, value: &str, language: SourceLanguage) -> NormalizedTranslationInput {
    let punctuation_normalized = value.nfc().map(normalize_punctuation).collect::<String>();
    let cased = match language {
      SourceLanguage::Auto | SourceLanguage::Known(TurnLanguage::English) => punctuation_normalized
        .chars()
        .flat_map(char::to_lowercase)
        .collect(),
      SourceLanguage::Known(TurnLanguage::Chinese) => punctuation_normalized,
    };
    let primary = collapse_whitespace(&cased);
    let mut derived_forms = Vec::with_capacity(MAX_DERIVED_FORMS);
    push_distinct(&mut derived_forms, primary.clone());

    let unquoted = primary.trim_matches(['\'', '"']).trim().to_string();
    push_distinct(&mut derived_forms, unquoted);

    let lexical_punctuation = primary
      .chars()
      .map(|character| match character {
        ',' | '.' | ':' | ';' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' => ' ',
        _ => character,
      })
      .collect::<String>();
    push_distinct(
      &mut derived_forms,
      collapse_whitespace(&lexical_punctuation),
    );
    derived_forms.truncate(MAX_DERIVED_FORMS);

    NormalizedTranslationInput {
      primary,
      derived_forms,
    }
  }
}

fn normalize_punctuation(character: char) -> char {
  match character {
    '\u{2018}' | '\u{2019}' | '\u{02bc}' | '\u{ff07}' => '\'',
    '\u{201c}' | '\u{201d}' | '\u{ff02}' => '"',
    '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2212}' | '\u{ff0d}' => '-',
    '\u{ff0b}' => '+',
    '\u{ff03}' => '#',
    '\u{3000}' => ' ',
    other => other,
  }
}

fn collapse_whitespace(value: &str) -> String {
  value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_distinct(forms: &mut Vec<String>, candidate: String) {
  if !candidate.is_empty() && !forms.contains(&candidate) && forms.len() < MAX_DERIVED_FORMS {
    forms.push(candidate);
  }
}

/// Resolved unit, chosen by orchestration rather than the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationUnit {
  /// One lexical word or technical symbol.
  Word,
  /// An established multiword expression or term.
  Phrase,
  /// Connected text, including uncertain short fragments.
  Passage,
}

/// Deterministic classification permits lexical routing only at high confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingConfidence {
  /// Confident lexical classification.
  High,
  /// Insufficient evidence: use connected-text translation.
  Uncertain,
}

/// Closed classification result; None explicitly means an unsupported source language.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnClassification {
  /// Suggested unit family, subject to deterministic conservative routing.
  pub unit: TranslationUnit,
  /// Detected supported source language, or None when unsupported.
  pub detected_source_language: Option<TurnLanguage>,
  /// Confidence in lexical routing.
  pub confidence: RoutingConfidence,
}

/// Stateless classifier that routes only structurally unambiguous lexical units to lexical work.
#[derive(Debug, Clone, Copy, Default)]
pub struct TranslationIntentClassifier;

impl TranslationIntentClassifier {
  /// Creates the deterministic request-local classifier.
  pub const fn new() -> Self {
    Self
  }

  /// Classifies normalized input without consulting provider policy or retaining request content.
  pub fn classify(
    self,
    turn: &TranslationTurn,
    normalized: &NormalizedTranslationInput,
  ) -> TurnClassification {
    let detected_source_language = match turn.source_language() {
      SourceLanguage::Known(language) => Some(language),
      SourceLanguage::Auto => detect_supported_language(&normalized.primary),
    };
    let (unit, confidence) = classify_unit(turn, &normalized.primary);
    TurnClassification {
      unit,
      detected_source_language,
      confidence,
    }
  }
}

fn detect_supported_language(value: &str) -> Option<TurnLanguage> {
  if value.chars().any(is_cjk_ideograph) {
    Some(TurnLanguage::Chinese)
  } else if value.chars().any(char::is_alphabetic) {
    Some(TurnLanguage::English)
  } else {
    None
  }
}

fn is_cjk_ideograph(character: char) -> bool {
  matches!(
    character as u32,
    0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
  )
}

fn classify_unit(turn: &TranslationTurn, normalized: &str) -> (TranslationUnit, RoutingConfidence) {
  if turn.requires_passage() || contains_connected_text_punctuation(normalized) {
    return (TranslationUnit::Passage, RoutingConfidence::Uncertain);
  }

  let words = normalized.split_whitespace().collect::<Vec<_>>();
  if words.len() == 1 && contains_lexical_content(words[0]) {
    return (TranslationUnit::Word, RoutingConfidence::High);
  }

  if is_high_confidence_phrase(normalized, &words) {
    return (TranslationUnit::Phrase, RoutingConfidence::High);
  }

  (TranslationUnit::Passage, RoutingConfidence::Uncertain)
}

fn contains_connected_text_punctuation(value: &str) -> bool {
  value.ends_with(['.', '!', '?', '。', '！', '？'])
    || value
      .chars()
      .any(|character| matches!(character, ',' | ';' | ':' | '，' | '；' | '：'))
}

fn contains_lexical_content(value: &str) -> bool {
  value
    .chars()
    .any(|character| character.is_alphanumeric() || is_cjk_ideograph(character))
}

fn is_high_confidence_phrase(value: &str, words: &[&str]) -> bool {
  if !(2..=6).contains(&words.len()) || words.iter().any(|word| !contains_lexical_content(word)) {
    return false;
  }

  let explicitly_quoted = (value.starts_with('"') && value.ends_with('"'))
    || (value.starts_with('\'') && value.ends_with('\''));
  explicitly_quoted || !looks_like_connected_clause(words)
}

fn looks_like_connected_clause(words: &[&str]) -> bool {
  const CLAUSE_MARKERS: &[&str] = &[
    "i", "you", "he", "she", "it", "we", "they", "this", "that", "these", "those", "am", "is",
    "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did", "can",
    "could", "will", "would", "shall", "should", "may", "might", "must", "what", "when", "where",
    "which", "who", "why", "how", "please",
  ];
  words
    .iter()
    .any(|word| CLAUSE_MARKERS.contains(&word.trim_matches(['\'', '"'])))
}

/// One generated bilingual example, never canonical evidence.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnExample {
  /// Short example in the source language.
  pub source_text: String,
  /// Its translation in the target language.
  pub translated_text: String,
}

/// One generated meaning in a complete, response-level-independent lexical draft.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalMeaningDraft {
  /// Translation of this meaning.
  pub text: String,
  /// Concise distinction from other plausible meanings.
  pub meaning: String,
  /// Word class; an empty value is valid only for phrases.
  pub part_of_speech: String,
  /// Expression type such as idiom or technical term; empty for words.
  pub phrase_type: String,
  /// Bounded alternative lexical forms, not asserted canonical aliases.
  pub aliases: Vec<String>,
  /// Generated bilingual examples.
  pub examples: Vec<TurnExample>,
  /// Concise usage guidance.
  pub usage_notes: Vec<String>,
}

/// Generated meaning candidates without stable IDs, graph claims, or publication metadata.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalTurnDraft {
  /// Ordered materially distinct meanings.
  pub translations: Vec<LexicalMeaningDraft>,
}

impl LexicalTurnDraft {
  /// Rejects oversized, blank, duplicate, or structurally ineligible generated content.
  pub fn is_valid(&self, unit: TranslationUnit) -> bool {
    if self.translations.is_empty()
      || self.translations.len() > 8
      || unit == TranslationUnit::Passage
    {
      return false;
    }
    let mut seen = std::collections::BTreeSet::new();
    self.translations.iter().all(|item| {
      bounded(&item.text, 512)
        && bounded(&item.meaning, 512)
        && seen.insert((item.text.trim(), item.meaning.trim()))
        && match unit {
          TranslationUnit::Word => bounded(&item.part_of_speech, 64),
          TranslationUnit::Phrase => bounded(&item.phrase_type, 64),
          TranslationUnit::Passage => false,
        }
        && item.part_of_speech.len() <= 128
        && item.phrase_type.len() <= 128
        && item.aliases.len() <= 6
        && item.aliases.iter().all(|v| bounded(v, 128))
        && item.examples.len() <= 4
        && item
          .examples
          .iter()
          .all(|v| bounded(&v.source_text, 512) && bounded(&v.translated_text, 512))
        && item.usage_notes.len() <= 6
        && item.usage_notes.iter().all(|v| bounded(v, 512))
    })
  }
}

fn bounded(value: &str, chars: usize) -> bool {
  !value.trim().is_empty() && value.chars().count() <= chars
}

/// Closed annotation emitted only from validated application output.
#[derive(Clone, Serialize)]
pub struct TranslationAnnotation {
  /// Typed annotation family.
  #[serde(rename = "type")]
  pub family: AnnotationFamily,
  /// Closed application-owned reason code.
  pub code: TranslationAnnotationCode,
  /// Concise bounded explanation safe to show with this result.
  pub message: String,
  /// Minimum response breadth at which this supporting annotation is useful.
  #[serde(skip)]
  pub minimum_level: ResponseLevel,
  /// Response-local citations supporting the annotation.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub citations: Vec<CitationReference>,
}

/// Closed application-owned reason code for a typed result annotation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationAnnotationCode {
  /// The source admits more than one material reading.
  AmbiguityDetected,
  /// A request-scoped terminology choice was applied.
  TermSelected,
  /// Protected source content was copied unchanged.
  ProtectedContentPreserved,
  /// Requested register guidance was applied.
  RegisterApplied,
  /// A bounded cultural explanation accompanies the translation.
  CulturalContext,
  /// Source formatting constraints were preserved.
  FormatPreserved,
  /// A closed review issue requires caller attention.
  ReviewRequired,
  /// A generated claim used one or more request-local live sources.
  LiveSourceUsed,
}

/// Human-review state derived from validated issues rather than model confidence prose.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationReviewState {
  /// No material issue requires explicit review.
  Clean,
  /// One or more closed issues should be reviewed by the caller.
  ReviewRecommended,
}

/// Closed review issue codes understood by the target result contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationReviewIssue {
  /// Model or source confidence is too low for silent acceptance.
  LowConfidence,
  /// Source meaning remains materially ambiguous.
  SourceAmbiguous,
  /// Requested terminology could not be satisfied safely.
  TerminologyConflict,
  /// Formatting may not round-trip safely.
  FormatRisk,
  /// Protected source material was not preserved exactly.
  ProtectedContentMismatch,
  /// Visual reading order could not be established confidently.
  VisualOrderUncertain,
  /// Required live supporting material was incomplete.
  LiveSourceIncomplete,
}

/// Deterministic review outcome retained by every response projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TranslationReview {
  /// Aggregate review state.
  pub state: TranslationReviewState,
  /// Ordered, deduplicated closed issues.
  pub issues: Vec<TranslationReviewIssue>,
}

impl TranslationReview {
  /// Creates the issue-free review outcome used by validated ordinary text.
  pub fn clean() -> Self {
    Self {
      state: TranslationReviewState::Clean,
      issues: Vec::new(),
    }
  }
}

/// One applied request-scoped terminology decision.
#[derive(Clone, Serialize)]
pub struct TerminologyDecision {
  /// Request-local source term.
  pub source: String,
  /// Selected target term, absent for a successfully forbidden term.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub target: Option<String>,
  /// Policy enforced for this decision.
  pub policy: TerminologyPolicy,
}

/// Response-local external source descriptor; it never denotes canonical evidence.
#[derive(Clone, Serialize)]
pub struct ExternalSourceReference {
  /// Stable only within this response.
  pub source_id: String,
  /// Bounded display title.
  pub title: String,
  /// Public source URL admitted by the live-retrieval boundary.
  pub url: String,
  /// Closed non-canonical provenance label for request-local live material.
  pub evidence_state: ExternalEvidenceState,
}

/// Closed provenance state for response-local external material.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalEvidenceState {
  /// Request-local public material that is neither canonical nor verified evidence.
  LiveExternal,
}

/// Reference from an annotation or generated claim to a response-local source.
#[derive(Clone, Serialize)]
pub struct CitationReference {
  /// Response-local source identity.
  pub source_id: String,
  /// Deterministic generated claim identity supported by this source.
  pub claim_id: String,
  /// Optional bounded fragment identity owned by the retrieval result.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub fragment_id: Option<String>,
}

/// One ordered document-segment translation.
#[derive(Clone, Serialize)]
pub struct SegmentTranslationResult {
  /// Caller-owned request-local identity returned unchanged.
  pub segment_id: String,
  /// Zero-based request order retained across every projection.
  pub order: usize,
  /// Detected supported source language.
  pub detected_source_language: TurnLanguage,
  /// Primary translation choices for this segment.
  pub translations: Vec<TurnTranslation>,
  /// Typed supporting annotations.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub annotations: Vec<TranslationAnnotation>,
  /// Invariant review outcome.
  pub review: TranslationReview,
}

/// One ordered visual-region translation without returning image bytes or OCR transcripts.
#[derive(Clone, Serialize)]
pub struct ImageRegionTranslationResult {
  /// Caller-owned image identity returned unchanged.
  pub image_id: String,
  /// Caller-owned region identity returned unchanged.
  pub region_id: String,
  /// Zero-based reading order retained across every projection.
  pub order: usize,
  /// Detected supported source language.
  pub detected_source_language: TurnLanguage,
  /// Primary translation choices for this region.
  pub translations: Vec<TurnTranslation>,
  /// Typed supporting annotations.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub annotations: Vec<TranslationAnnotation>,
  /// Invariant review outcome.
  pub review: TranslationReview,
}

/// Validated result superset shared by all translation input families.
#[derive(Clone, Serialize)]
#[serde(tag = "unit", rename_all = "snake_case")]
pub enum TranslationTurnResult {
  /// One lexical word with ordered meaning-specific translations.
  Word {
    /// Detected supported source language.
    detected_source_language: TurnLanguage,
    /// Ordered meaning-specific translations.
    translations: Vec<TurnTranslation>,
    /// Typed supporting annotations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    annotations: Vec<TranslationAnnotation>,
    /// Invariant review outcome.
    review: TranslationReview,
  },
  /// One established phrase with ordered meaning-specific translations.
  Phrase {
    /// Detected supported source language.
    detected_source_language: TurnLanguage,
    /// Ordered meaning-specific translations.
    translations: Vec<TurnTranslation>,
    /// Typed supporting annotations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    annotations: Vec<TranslationAnnotation>,
    /// Invariant review outcome.
    review: TranslationReview,
  },
  /// One connected passage.
  Passage {
    /// Detected supported source language.
    detected_source_language: TurnLanguage,
    /// Primary passage translation.
    translations: Vec<TurnTranslation>,
    /// Typed supporting annotations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    annotations: Vec<TranslationAnnotation>,
    /// Invariant review outcome.
    review: TranslationReview,
  },
  /// Ordered structured document or localization segments.
  Segment {
    /// Ordered results retaining caller IDs and order.
    segments: Vec<SegmentTranslationResult>,
    /// Applied request-scoped terminology decisions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    terminology_decisions: Vec<TerminologyDecision>,
  },
  /// Ordered bounded image regions.
  ImageRegion {
    /// Ordered results retaining caller image and region IDs.
    regions: Vec<ImageRegionTranslationResult>,
    /// Applied request-scoped terminology decisions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    terminology_decisions: Vec<TerminologyDecision>,
  },
}

/// Closed discriminator for every target translation result family.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationResultKind {
  /// One lexical word.
  Word,
  /// One established phrase.
  Phrase,
  /// One connected passage.
  Passage,
  /// Ordered structured text segments.
  Segment,
  /// Ordered regions extracted from images.
  ImageRegion,
}

/// Closed structural failures for a translation result superset.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum TranslationResultValidationError {
  /// A result family has no primary unit or translation.
  #[error("translation result is empty")]
  Empty,
  /// Caller-owned identities are blank, duplicated, or out of request order.
  #[error("translation result identity or order is invalid")]
  InvalidIdentityOrder,
  /// Review state and its closed issue list contradict one another.
  #[error("translation result review state is inconsistent")]
  InvalidReview,
  /// A bounded result value exceeds the target result contract.
  #[error("translation result value is invalid")]
  InvalidValue,
}

/// Version metadata for one projected translation application outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TranslationVersionMetadata {
  /// Translation result schema used by this application outcome.
  pub schema_version: &'static str,
  /// Request-local normalizer version used before routing.
  pub normalizer_version: &'static str,
  /// Deterministic projector version used to select response breadth.
  pub projection_version: &'static str,
  /// Requested response breadth applied to the superset.
  pub response_level: ResponseLevel,
  /// Ordered, de-duplicated model identifiers that actually served the request.
  pub model_versions: Vec<String>,
  /// Ordered, de-duplicated prompt contract versions that actually served the request.
  pub prompt_versions: Vec<String>,
  /// Ordered profiles that actually served this request.
  pub inference_profiles: Vec<crate::domain::model_runtime::GenerationProfile>,
  /// Whether the request consumed its sole reasoning repair.
  pub reasoning_escalated: bool,
  /// Retrieval version, absent until retrieval participates in the request.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub retrieval_version: Option<String>,
  /// Immutable content release, absent until canonical release-pinned data participates.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content_release: Option<String>,
}

/// Transport-independent projected translation plus truthful generation metadata.
#[derive(Clone, Serialize)]
pub struct ProjectedTranslationResult {
  /// Deterministic projection of one complete semantic superset.
  pub translation: TranslationTurnResult,
  /// Versions of only the components that participated in this request.
  pub metadata: TranslationVersionMetadata,
  /// Response-local sources referenced by annotations or generated claims.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub external_sources: Vec<ExternalSourceReference>,
}

impl ProjectedTranslationResult {
  /// Validates the superset and every response-local citation before HTTP serialization.
  pub fn validate(&self) -> Result<(), TranslationResultValidationError> {
    self.translation.validate()?;
    validate_metadata(&self.metadata)?;
    let mut source_ids = std::collections::BTreeSet::new();
    let mut cited_source_ids = std::collections::BTreeSet::new();
    if self.external_sources.len() > 5
      || (!self.external_sources.is_empty() && self.metadata.retrieval_version.is_none())
      || self.external_sources.iter().any(|source| {
        !bounded_id(&source.source_id)
          || !source_ids.insert(source.source_id.as_str())
          || !bounded(&source.title, 512)
          || !bounded(&source.url, 2_048)
      })
    {
      return Err(TranslationResultValidationError::InvalidValue);
    }
    if self.translation.annotations().any(|annotation| {
      let mut citations = std::collections::BTreeSet::new();
      annotation.citations.iter().any(|citation| {
        cited_source_ids.insert(citation.source_id.as_str());
        !source_ids.contains(citation.source_id.as_str())
          || !bounded_id(&citation.source_id)
          || !bounded_id(&citation.claim_id)
          || citation
            .fragment_id
            .as_deref()
            .is_some_and(|value| !bounded_id(value))
          || !citations.insert((
            citation.source_id.as_str(),
            citation.claim_id.as_str(),
            citation.fragment_id.as_deref(),
          ))
      })
    }) || cited_source_ids != source_ids
    {
      return Err(TranslationResultValidationError::InvalidValue);
    }
    Ok(())
  }

  /// Validates this projection against the exact originating request shape and selectors.
  pub fn validate_for_turn(
    &self,
    turn: &TranslationTurn,
  ) -> Result<(), TranslationResultValidationError> {
    self.validate()?;
    if self.metadata.response_level != turn.response_level() {
      return Err(TranslationResultValidationError::InvalidValue);
    }
    validate_result_binding(&self.translation, turn)
  }

  /// Applies deterministic breadth and removes source descriptors no longer cited afterward.
  pub fn project(mut self, level: ResponseLevel) -> Self {
    self.translation = self.translation.project(level);
    self.metadata.response_level = level;
    let cited = self
      .translation
      .annotations()
      .flat_map(|annotation| &annotation.citations)
      .map(|citation| citation.source_id.as_str())
      .collect::<std::collections::BTreeSet<_>>();
    self
      .external_sources
      .retain(|source| cited.contains(source.source_id.as_str()));
    self
  }
}

fn validate_metadata(
  metadata: &TranslationVersionMetadata,
) -> Result<(), TranslationResultValidationError> {
  if metadata.schema_version != TRANSLATION_RESULT_SCHEMA_VERSION
    || metadata.normalizer_version != NORMALIZER_VERSION
    || metadata.projection_version != PROJECTION_VERSION
    || !valid_version_values(&metadata.model_versions)
    || !valid_version_values(&metadata.prompt_versions)
    || metadata.inference_profiles.len() > MAX_TRANSLATION_VERSION_VALUES
    || metadata
      .inference_profiles
      .iter()
      .enumerate()
      .any(|(index, profile)| metadata.inference_profiles[..index].contains(profile))
    || metadata
      .retrieval_version
      .as_deref()
      .is_some_and(|value| !bounded(value, 128))
    || metadata
      .content_release
      .as_deref()
      .is_some_and(|value| !bounded(value, 128))
  {
    return Err(TranslationResultValidationError::InvalidValue);
  }
  Ok(())
}

fn valid_version_values(values: &[String]) -> bool {
  values.len() <= MAX_TRANSLATION_VERSION_VALUES
    && values.iter().all(|value| bounded(value, 128))
    && values
      .iter()
      .enumerate()
      .all(|(index, value)| !values[..index].contains(value))
}

fn validate_result_binding(
  result: &TranslationTurnResult,
  turn: &TranslationTurn,
) -> Result<(), TranslationResultValidationError> {
  let valid = match (turn.input(), result) {
    (
      TranslationInput::Text { .. },
      TranslationTurnResult::Word {
        detected_source_language,
        translations,
        ..
      }
      | TranslationTurnResult::Phrase {
        detected_source_language,
        translations,
        ..
      }
      | TranslationTurnResult::Passage {
        detected_source_language,
        translations,
        ..
      },
    ) => {
      source_matches(turn.source_language(), *detected_source_language)
        && translations
          .iter()
          .all(|translation| translation.language == turn.target_language())
    }
    (
      TranslationInput::Segments { segments },
      TranslationTurnResult::Segment {
        segments: output,
        terminology_decisions,
      },
    ) => {
      segments.len() == output.len()
        && segments.iter().zip(output).all(|(input, result)| {
          input.segment_id == result.segment_id
            && source_matches(turn.source_language(), result.detected_source_language)
            && result
              .translations
              .iter()
              .all(|translation| translation.language == turn.target_language())
        })
        && terminology_matches(turn.guidance(), terminology_decisions)
    }
    (
      TranslationInput::ImageRegions {
        images,
        reading_order,
      },
      TranslationTurnResult::ImageRegion {
        regions,
        terminology_decisions,
      },
    ) => {
      reading_order.len() == regions.len()
        && reading_order.iter().zip(regions).all(|(key, result)| {
          format!("{}:{}", result.image_id, result.region_id) == *key
            && images.iter().any(|image| {
              image.image_id == result.image_id
                && image
                  .regions
                  .iter()
                  .any(|region| region.region_id == result.region_id)
            })
            && source_matches(turn.source_language(), result.detected_source_language)
            && result
              .translations
              .iter()
              .all(|translation| translation.language == turn.target_language())
        })
        && terminology_matches(turn.guidance(), terminology_decisions)
    }
    _ => false,
  };
  if valid {
    Ok(())
  } else {
    Err(TranslationResultValidationError::InvalidIdentityOrder)
  }
}

fn source_matches(expected: SourceLanguage, actual: TurnLanguage) -> bool {
  matches!(expected, SourceLanguage::Auto) || expected == SourceLanguage::Known(actual)
}

fn terminology_matches(guidance: &TranslationGuidance, decisions: &[TerminologyDecision]) -> bool {
  guidance.terminology.len() == decisions.len()
    && guidance
      .terminology
      .iter()
      .zip(decisions)
      .all(|(constraint, decision)| {
        constraint.source == decision.source
          && constraint.policy == decision.policy
          && match constraint.policy {
            TerminologyPolicy::Required | TerminologyPolicy::Preferred => {
              decision.target.as_deref() == Some(constraint.target.as_str())
            }
            TerminologyPolicy::Forbidden => decision.target.is_none(),
          }
      })
}

/// One translation with optional meaning-specific generated detail.
#[derive(Clone, Serialize)]
pub struct TurnTranslation {
  /// Stable position-derived identity within this result unit.
  pub translation_id: String,
  /// Zero-based rank retained across every response projection.
  pub order: usize,
  /// Translated text, including preserved formatting for passages.
  pub text: String,
  /// Target language selected by the caller.
  pub language: TurnLanguage,
  /// Meaning label retained at every response level for lexical translations.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub meaning: Option<String>,
  /// Generated supporting material; omitted for brief responses and plain passages.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<TurnDetails>,
}

/// Model-only supporting material, explicitly separated from canonical knowledge.
#[derive(Clone, Serialize)]
pub struct TurnDetails {
  /// Meaning-specific unit type.
  #[serde(rename = "type")]
  pub unit: TranslationUnit,
  /// Word class, when applicable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub part_of_speech: Option<String>,
  /// Established expression type, when applicable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub phrase_type: Option<String>,
  /// Model-generated alternatives available at full level.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub aliases: Vec<String>,
  /// Generated examples; their provenance is declared by generated=true on this details object.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub examples: Vec<TurnExample>,
  /// Usage notes available at standard and full levels.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub usage_notes: Vec<String>,
  /// Always true for this model-only foundation.
  pub generated: bool,
  /// Always exploratory: a model response alone is not verified or evidence-grounded synthesis.
  pub evidence_state: &'static str,
}

impl TranslationTurnResult {
  /// Attaches validated request-local live citations to every generated result unit.
  pub(crate) fn attach_live_citations(&mut self, citations: Vec<CitationReference>) {
    let annotations = citations
      .chunks(16)
      .map(|citations| TranslationAnnotation {
        family: AnnotationFamily::Review,
        code: TranslationAnnotationCode::LiveSourceUsed,
        message: "Translation used request-local live sources.".into(),
        minimum_level: ResponseLevel::Brief,
        citations: citations.to_vec(),
      })
      .collect::<Vec<_>>();
    match self {
      Self::Word {
        annotations: result_annotations,
        ..
      }
      | Self::Phrase {
        annotations: result_annotations,
        ..
      }
      | Self::Passage {
        annotations: result_annotations,
        ..
      } => result_annotations.extend(annotations),
      Self::Segment { segments, .. } => {
        for segment in segments {
          segment.annotations.extend(annotations.clone());
        }
      }
      Self::ImageRegion { regions, .. } => {
        for region in regions {
          region.annotations.extend(annotations.clone());
        }
      }
    }
  }
  /// Marks an explicitly permitted live-retrieval attempt that safely degraded without sources.
  pub(crate) fn mark_live_retrieval_degraded(&mut self) {
    let annotation = TranslationAnnotation {
      family: AnnotationFamily::Review,
      code: TranslationAnnotationCode::ReviewRequired,
      message: "Live retrieval was unavailable; review freshness-sensitive claims.".into(),
      minimum_level: ResponseLevel::Brief,
      citations: Vec::new(),
    };
    let review = TranslationReview {
      state: TranslationReviewState::ReviewRecommended,
      issues: vec![TranslationReviewIssue::LiveSourceIncomplete],
    };
    match self {
      Self::Word {
        annotations,
        review: result_review,
        ..
      }
      | Self::Phrase {
        annotations,
        review: result_review,
        ..
      }
      | Self::Passage {
        annotations,
        review: result_review,
        ..
      } => {
        annotations.push(annotation);
        *result_review = review;
      }
      Self::Segment { segments, .. } => {
        for segment in segments {
          segment.annotations.push(annotation.clone());
          segment.review = review.clone();
        }
      }
      Self::ImageRegion { regions, .. } => {
        for region in regions {
          region.annotations.push(annotation.clone());
          region.review = review.clone();
        }
      }
    }
  }
  /// Validates identities, order, primary translations, annotations, terminology, and review state.
  pub fn validate(&self) -> Result<(), TranslationResultValidationError> {
    match self {
      Self::Word {
        translations,
        annotations,
        review,
        ..
      } => validate_result_unit(translations, annotations, review, ResultUnitKind::Word),
      Self::Phrase {
        translations,
        annotations,
        review,
        ..
      } => validate_result_unit(translations, annotations, review, ResultUnitKind::Phrase),
      Self::Passage {
        translations,
        annotations,
        review,
        ..
      } => validate_result_unit(translations, annotations, review, ResultUnitKind::Passage),
      Self::Segment {
        segments,
        terminology_decisions,
      } => {
        if segments.is_empty() || segments.len() > MAX_SEGMENTS {
          return Err(TranslationResultValidationError::Empty);
        }
        let mut ids = std::collections::BTreeSet::new();
        for (order, segment) in segments.iter().enumerate() {
          if segment.order != order
            || !bounded_id(&segment.segment_id)
            || !ids.insert(segment.segment_id.as_str())
          {
            return Err(TranslationResultValidationError::InvalidIdentityOrder);
          }
          validate_result_unit(
            &segment.translations,
            &segment.annotations,
            &segment.review,
            ResultUnitKind::Structured,
          )?;
        }
        validate_terminology_decisions(terminology_decisions)
      }
      Self::ImageRegion {
        regions,
        terminology_decisions,
      } => {
        if regions.is_empty() || regions.len() > MAX_IMAGE_REGIONS {
          return Err(TranslationResultValidationError::Empty);
        }
        let mut ids = std::collections::BTreeSet::new();
        for (order, region) in regions.iter().enumerate() {
          if region.order != order
            || !bounded_id(&region.image_id)
            || !bounded_id(&region.region_id)
            || !ids.insert((region.image_id.as_str(), region.region_id.as_str()))
          {
            return Err(TranslationResultValidationError::InvalidIdentityOrder);
          }
          validate_result_unit(
            &region.translations,
            &region.annotations,
            &region.review,
            ResultUnitKind::Structured,
          )?;
        }
        validate_terminology_decisions(terminology_decisions)
      }
    }
  }
  /// Assembles the full lexical superset; callers must validate the model draft first.
  pub fn lexical(
    draft: LexicalTurnDraft,
    unit: TranslationUnit,
    source: TurnLanguage,
    target: TurnLanguage,
  ) -> Self {
    let translations = draft
      .translations
      .into_iter()
      .enumerate()
      .map(|(order, m)| TurnTranslation {
        translation_id: format!("translation_{order}"),
        order,
        text: m.text,
        language: target,
        meaning: Some(m.meaning),
        details: Some(TurnDetails {
          unit,
          part_of_speech: (unit == TranslationUnit::Word).then_some(m.part_of_speech),
          phrase_type: (unit == TranslationUnit::Phrase).then_some(m.phrase_type),
          aliases: m.aliases,
          examples: m.examples,
          usage_notes: m.usage_notes,
          generated: true,
          evidence_state: "exploratory",
        }),
      })
      .collect();
    let annotations = Vec::new();
    let review = TranslationReview::clean();
    match unit {
      TranslationUnit::Word => Self::Word {
        detected_source_language: source,
        translations,
        annotations,
        review,
      },
      TranslationUnit::Phrase => Self::Phrase {
        detected_source_language: source,
        translations,
        annotations,
        review,
      },
      TranslationUnit::Passage => Self::Passage {
        detected_source_language: source,
        translations,
        annotations,
        review,
      },
    }
  }
  /// Produces a plain connected-text result without inventing optional tips or alternatives.
  pub fn passage(text: String, source: TurnLanguage, target: TurnLanguage) -> Self {
    Self::Passage {
      detected_source_language: source,
      translations: vec![TurnTranslation {
        translation_id: "translation_0".into(),
        order: 0,
        text,
        language: target,
        meaning: None,
        details: None,
      }],
      annotations: Vec::new(),
      review: TranslationReview::clean(),
    }
  }

  /// Returns the closed discriminant without exposing variant-specific storage.
  pub const fn kind(&self) -> TranslationResultKind {
    match self {
      Self::Word { .. } => TranslationResultKind::Word,
      Self::Phrase { .. } => TranslationResultKind::Phrase,
      Self::Passage { .. } => TranslationResultKind::Passage,
      Self::Segment { .. } => TranslationResultKind::Segment,
      Self::ImageRegion { .. } => TranslationResultKind::ImageRegion,
    }
  }

  /// Returns ordinary text translations, or an empty slice for structured result families.
  pub fn translations(&self) -> &[TurnTranslation] {
    match self {
      Self::Word { translations, .. }
      | Self::Phrase { translations, .. }
      | Self::Passage { translations, .. } => translations,
      Self::Segment { .. } | Self::ImageRegion { .. } => &[],
    }
  }

  fn annotations(&self) -> impl Iterator<Item = &TranslationAnnotation> {
    let mut values = Vec::new();
    match self {
      Self::Word { annotations, .. }
      | Self::Phrase { annotations, .. }
      | Self::Passage { annotations, .. } => values.extend(annotations),
      Self::Segment { segments, .. } => {
        values.extend(segments.iter().flat_map(|segment| &segment.annotations));
      }
      Self::ImageRegion { regions, .. } => {
        values.extend(regions.iter().flat_map(|region| &region.annotations));
      }
    }
    values.into_iter()
  }
  /// Projects an existing superset without changing translation text, meaning count, or rank.
  pub fn project(mut self, level: ResponseLevel) -> Self {
    let (translations, annotations) = match &mut self {
      Self::Word {
        translations,
        annotations,
        ..
      }
      | Self::Phrase {
        translations,
        annotations,
        ..
      }
      | Self::Passage {
        translations,
        annotations,
        ..
      } => (translations.as_mut_slice(), annotations),
      Self::Segment { segments, .. } => {
        for segment in segments {
          project_translations(&mut segment.translations, level);
          segment
            .annotations
            .retain(|value| visible_at(value.minimum_level, level));
        }
        return self;
      }
      Self::ImageRegion { regions, .. } => {
        for region in regions {
          project_translations(&mut region.translations, level);
          region
            .annotations
            .retain(|value| visible_at(value.minimum_level, level));
        }
        return self;
      }
    };
    project_translations(translations, level);
    annotations.retain(|value| visible_at(value.minimum_level, level));
    self
  }
}

fn validate_result_unit(
  translations: &[TurnTranslation],
  annotations: &[TranslationAnnotation],
  review: &TranslationReview,
  kind: ResultUnitKind,
) -> Result<(), TranslationResultValidationError> {
  if translations.is_empty() {
    return Err(TranslationResultValidationError::Empty);
  }
  if (matches!(kind, ResultUnitKind::Passage | ResultUnitKind::Structured)
    && translations.len() != 1)
    || translations.iter().enumerate().any(|(order, value)| {
      value.order != order
        || value.translation_id != format!("translation_{order}")
        || !bounded_id(&value.translation_id)
        || !bounded(&value.text, 32_768)
        || !validate_translation_shape(value, kind)
    })
  {
    return Err(TranslationResultValidationError::InvalidValue);
  }
  let mut has_review_annotation = false;
  if annotations.len() > 64
    || annotations.iter().any(|value| {
      has_review_annotation |= value.code == TranslationAnnotationCode::ReviewRequired;
      !bounded(&value.message, 512)
        || value.citations.len() > 16
        || annotation_family(value.code) != value.family
        || (value.code == TranslationAnnotationCode::ReviewRequired
          && value.minimum_level != ResponseLevel::Brief)
    })
  {
    return Err(TranslationResultValidationError::InvalidValue);
  }
  let review_is_valid = match review.state {
    TranslationReviewState::Clean => review.issues.is_empty(),
    TranslationReviewState::ReviewRecommended => !review.issues.is_empty(),
  };
  if !review_is_valid
    || has_review_annotation != (review.state == TranslationReviewState::ReviewRecommended)
  {
    return Err(TranslationResultValidationError::InvalidReview);
  }
  if review.issues.windows(2).any(|pair| pair[0] >= pair[1]) {
    return Err(TranslationResultValidationError::InvalidReview);
  }
  Ok(())
}

#[derive(Clone, Copy)]
enum ResultUnitKind {
  Word,
  Phrase,
  Passage,
  Structured,
}

fn validate_translation_shape(value: &TurnTranslation, kind: ResultUnitKind) -> bool {
  match kind {
    ResultUnitKind::Word | ResultUnitKind::Phrase => {
      value
        .meaning
        .as_deref()
        .is_some_and(|text| bounded(text, 512))
        && value
          .details
          .as_ref()
          .is_none_or(|details| validate_details(details, kind))
    }
    ResultUnitKind::Passage | ResultUnitKind::Structured => {
      value.meaning.is_none() && value.details.is_none()
    }
  }
}

fn validate_details(details: &TurnDetails, kind: ResultUnitKind) -> bool {
  let expected_shape = match kind {
    ResultUnitKind::Word => {
      details.unit == TranslationUnit::Word
        && details
          .part_of_speech
          .as_deref()
          .is_some_and(|value| bounded(value, 64))
        && details.phrase_type.is_none()
    }
    ResultUnitKind::Phrase => {
      details.unit == TranslationUnit::Phrase
        && details.part_of_speech.is_none()
        && details
          .phrase_type
          .as_deref()
          .is_some_and(|value| bounded(value, 64))
    }
    ResultUnitKind::Passage | ResultUnitKind::Structured => false,
  };
  expected_shape
    && details.generated
    && details.evidence_state == "exploratory"
    && details.aliases.len() <= 6
    && details.aliases.iter().all(|value| bounded(value, 128))
    && details.examples.len() <= 4
    && details
      .examples
      .iter()
      .all(|value| bounded(&value.source_text, 512) && bounded(&value.translated_text, 512))
    && details.usage_notes.len() <= 6
    && details.usage_notes.iter().all(|value| bounded(value, 512))
}

const fn annotation_family(code: TranslationAnnotationCode) -> AnnotationFamily {
  match code {
    TranslationAnnotationCode::AmbiguityDetected => AnnotationFamily::Ambiguity,
    TranslationAnnotationCode::TermSelected => AnnotationFamily::Terminology,
    TranslationAnnotationCode::ProtectedContentPreserved
    | TranslationAnnotationCode::FormatPreserved => AnnotationFamily::Format,
    TranslationAnnotationCode::RegisterApplied => AnnotationFamily::Register,
    TranslationAnnotationCode::CulturalContext => AnnotationFamily::Culture,
    TranslationAnnotationCode::ReviewRequired => AnnotationFamily::Review,
    TranslationAnnotationCode::LiveSourceUsed => AnnotationFamily::Review,
  }
}

fn validate_terminology_decisions(
  decisions: &[TerminologyDecision],
) -> Result<(), TranslationResultValidationError> {
  if decisions.len() > MAX_TERMINOLOGY {
    return Err(TranslationResultValidationError::InvalidValue);
  }
  let mut sources = std::collections::BTreeSet::new();
  if decisions.iter().any(|value| {
    !bounded(&value.source, MAX_TERM_SCALARS)
      || value
        .target
        .as_deref()
        .is_some_and(|target| !bounded(target, MAX_TERM_SCALARS))
      || match value.policy {
        TerminologyPolicy::Required | TerminologyPolicy::Preferred => value.target.is_none(),
        TerminologyPolicy::Forbidden => value.target.is_some(),
      }
      || !sources.insert(value.source.as_str())
  }) {
    return Err(TranslationResultValidationError::InvalidValue);
  }
  Ok(())
}

fn project_translations(translations: &mut [TurnTranslation], level: ResponseLevel) {
  for translation in translations {
    match level {
      ResponseLevel::Brief => translation.details = None,
      ResponseLevel::Standard => {
        if let Some(details) = &mut translation.details {
          details.aliases.clear();
          details.examples.truncate(1);
          details.usage_notes.truncate(2);
        }
      }
      ResponseLevel::Full => {}
    }
  }
}

const fn visible_at(minimum: ResponseLevel, requested: ResponseLevel) -> bool {
  matches!(
    (minimum, requested),
    (ResponseLevel::Brief, _)
      | (
        ResponseLevel::Standard,
        ResponseLevel::Standard | ResponseLevel::Full
      )
      | (ResponseLevel::Full, ResponseLevel::Full)
  )
}

macro_rules! redacted_debug {
  ($($type:ty),+ $(,)?) => { $(impl std::fmt::Debug for $type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.write_str(concat!(stringify!($type), "(REDACTED)"))
    }
  })+ };
}
redacted_debug!(
  TranslationHistory,
  TranslationTurnRequest,
  TranslationTurn,
  TurnExample,
  LexicalMeaningDraft,
  LexicalTurnDraft,
  TranslationTurnResult,
  TurnTranslation,
  TurnDetails,
  TranslationAnnotation,
  TerminologyDecision,
  ExternalSourceReference,
  CitationReference,
  SegmentTranslationResult,
  ImageRegionTranslationResult,
  ProjectedTranslationResult
);

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn request() -> TranslationTurnRequest {
    TranslationTurnRequest {
      text: Some("C++".to_string()),
      input: None,
      source_language: "en".to_string(),
      target_language: "zh-CN".to_string(),
      response_level: "standard".to_string(),
      history: Vec::new(),
      guidance: None,
    }
  }

  fn history(source_text: impl Into<String>) -> TranslationHistory {
    TranslationHistory {
      source_text: source_text.into(),
      translated_text: "译文".to_string(),
      source_language: "en".to_string(),
      target_language: "zh-CN".to_string(),
    }
  }

  #[test]
  fn accepts_only_the_initial_closed_language_and_response_level_sets() {
    for source in ["auto", "en", "zh-CN"] {
      for target in ["en", "zh-CN"] {
        for level in ["brief", "standard", "full"] {
          let mut input = request();
          input.source_language = source.to_string();
          input.target_language = target.to_string();
          input.response_level = level.to_string();
          assert!(TranslationTurn::new(input).is_ok());
        }
      }
    }

    for source in ["EN", "zh-cn", "fr", "", "en-US"] {
      let mut input = request();
      input.source_language = source.to_string();
      assert_eq!(
        TranslationTurn::new(input).unwrap_err(),
        TurnValidationError::Field("source_language")
      );
    }

    let mut input = request();
    input.target_language = "auto".to_string();
    assert_eq!(
      TranslationTurn::new(input).unwrap_err(),
      TurnValidationError::Field("target_language")
    );
  }

  #[test]
  fn strict_request_and_history_shapes_reject_unknown_fields() {
    assert!(serde_json::from_value::<TranslationTurnRequest>(json!({
      "text": "hello",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "user_id": "private"
    }))
    .is_err());
    assert!(serde_json::from_value::<TranslationTurnRequest>(json!({
      "text": "hello",
      "source_language": "en",
      "target_language": "zh-CN",
      "response_level": "brief",
      "history": [{
        "source_text": "hello",
        "translated_text": "你好",
        "source_language": "en",
        "target_language": "zh-CN",
        "turn_id": "not-allowed"
      }]
    }))
    .is_err());
  }

  #[test]
  fn normalizer_is_versioned_nfc_and_language_aware() {
    let normalizer = TranslationNormalizer::new();
    let normalized = normalizer.normalize(
      "  CAFE\u{301}\tTEST  ",
      SourceLanguage::Known(TurnLanguage::English),
    );
    assert_eq!(normalizer.version(), "translation-lookup-nfc-v1");
    assert_eq!(normalized.primary, "caf\u{e9} test");

    let chinese = normalizer.normalize("术语 C++", SourceLanguage::Known(TurnLanguage::Chinese));
    assert_eq!(chinese.primary, "术语 C++");
  }

  #[test]
  fn normalizer_maps_equivalent_punctuation_and_collapses_whitespace() {
    let normalized = TranslationNormalizer::new().normalize(
      "  \u{201c}Up\u{2014}in\u{2014}the\u{2014}air\u{201d}\u{3000}test  ",
      SourceLanguage::Known(TurnLanguage::English),
    );
    assert_eq!(normalized.primary, "\"up-in-the-air\" test");
    assert_eq!(normalized.derived_forms[1], "up-in-the-air\" test");
    assert!(normalized.derived_forms.len() <= MAX_DERIVED_FORMS);
  }

  #[test]
  fn normalizer_preserves_meaningful_technical_symbols() {
    let normalizer = TranslationNormalizer::new();
    let c = normalizer.normalize("C", SourceLanguage::Known(TurnLanguage::English));
    let cpp = normalizer.normalize(
      "C\u{ff0b}\u{ff0b}",
      SourceLanguage::Known(TurnLanguage::English),
    );
    let csharp = normalizer.normalize("C\u{ff03}", SourceLanguage::Known(TurnLanguage::English));
    assert_eq!(c.primary, "c");
    assert_eq!(cpp.primary, "c++");
    assert_eq!(csharp.primary, "c#");
    assert_ne!(c.primary, cpp.primary);
    assert_ne!(c.primary, csharp.primary);
    assert_ne!(cpp.primary, csharp.primary);
  }

  #[test]
  fn derived_forms_are_bounded_deduplicated_and_keep_symbols() {
    let normalized = TranslationNormalizer::new()
      .normalize("\"C++?!\"", SourceLanguage::Known(TurnLanguage::English));
    assert!(normalized.derived_forms.len() <= MAX_DERIVED_FORMS);
    assert_eq!(normalized.derived_forms[0], "\"c++?!\"");
    assert!(normalized
      .derived_forms
      .iter()
      .any(|form| form.contains("c++")));
    let unique = normalized
      .derived_forms
      .iter()
      .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), normalized.derived_forms.len());
  }

  #[test]
  fn history_has_no_item_cap_inside_the_generation_context_bound() {
    let mut input = request();
    input.history = (0..32)
      .map(|index| history(format!("turn-{index}")))
      .collect();
    let turn = TranslationTurn::new(input).unwrap();
    assert_eq!(turn.history().len(), 32);
  }

  #[test]
  fn oversized_generation_context_fails_with_a_content_free_field() {
    let secret = "private-history-context-991";
    let mut input = request();
    input.history = vec![history(format!(
      "{secret}{}",
      "x".repeat(MAX_GENERATION_CONTEXT_BYTES)
    ))];
    let error = TranslationTurn::new(input).unwrap_err();
    assert_eq!(error, TurnValidationError::Field("generation_context"));
    assert!(!format!("{error:?} {error}").contains(secret));
  }

  #[test]
  fn history_context_and_current_text_use_their_documented_bounds() {
    let mut oversized_history = request();
    oversized_history.history = vec![history("x".repeat(MAX_TURN_BYTES))];
    assert_eq!(
      TranslationTurn::new(oversized_history).unwrap_err(),
      TurnValidationError::Field("generation_context")
    );

    let mut oversized_text = request();
    oversized_text.text = Some("x".repeat(MAX_TURN_BYTES));
    assert_eq!(
      TranslationTurn::new(oversized_text).unwrap_err(),
      TurnValidationError::Field("input.text")
    );
  }

  #[test]
  fn target_text_guidance_is_retained_after_validation() {
    let request: TranslationTurnRequest = serde_json::from_value(json!({
      "input": {"type": "text", "text": "torque"},
      "source_language": "en", "target_language": "zh-CN", "response_level": "standard",
      "guidance": {
        "purpose": "technical", "audience": "specialist", "register": "preserve",
        "terminology": [{"source": "torque", "target": "扭矩", "policy": "required"}],
        "max_alternatives": 0, "annotations": [], "freshness": "offline"
      }
    }))
    .unwrap();
    let turn = TranslationTurn::new(request).unwrap();
    assert_eq!(turn.input_kind(), TranslationInputKind::Text);
    assert_eq!(turn.text(), Some("torque"));
    assert_eq!(turn.guidance().terminology[0].source, "torque");
    assert!(turn.requires_guidance_execution());
    let model_value = serde_json::to_value(&turn).unwrap();
    assert_eq!(model_value["text"], "torque");
    assert!(model_value.get("input").is_none());
    assert!(serde_json::from_value::<TranslationTurnRequest>(json!({
      "input": {"type": "text", "text": "secret", "extra": true},
      "source_language": "en", "target_language": "zh-CN", "response_level": "brief"
    }))
    .is_err());
  }

  #[test]
  fn segment_ranges_and_constraints_fail_closed_before_orchestration() {
    let parse = |ranges: serde_json::Value, terms: serde_json::Value| {
      serde_json::from_value(json!({
        "input": {"type": "segments", "segments": [{
          "segment_id": "s1", "text": "Launch {name}", "role": "title", "format": "plain",
          "protected_ranges": ranges
        }]},
        "source_language": "en", "target_language": "zh-CN", "response_level": "standard",
        "guidance": {"terminology": terms}
      }))
      .unwrap()
    };
    let first = TranslationTurn::new(parse(json!([{"start": 7, "end": 13}]), json!([]))).unwrap();
    assert_eq!(first.input_kind(), TranslationInputKind::Segments);
    assert_eq!(first.text(), None);
    let ordered = TranslationTurn::new(parse(
      json!([{"start": 7, "end": 13}, {"start": 0, "end": 6}]),
      json!([]),
    ))
    .unwrap();
    let TranslationInput::Segments { segments } = ordered.input() else {
      panic!("validated segment input changed shape")
    };
    assert_eq!(segments[0].segment_id, "s1");
    assert_eq!(segments[0].protected_ranges[0].start, 7);
    assert_eq!(segments[0].protected_ranges[1].start, 0);
    assert_eq!(
      TranslationTurn::new(parse(
        json!([{"start": 7, "end": 13}, {"start": 8, "end": 10}]),
        json!([])
      ))
      .unwrap_err(),
      TurnValidationError::Field("input.segments.protected_ranges")
    );
    assert_eq!(
      TranslationTurn::new(parse(
        json!([{"start": 7, "end": 13}]),
        json!([{"source":"{name}","target":"产品", "policy":"required"}])
      ))
      .unwrap_err(),
      TurnValidationError::ConstraintConflict("guidance.terminology")
    );
    assert_eq!(
      TranslationTurn::new(parse(
        json!([{"start": 7, "end": 13}]),
        json!([{"source":"h {name}","target":"产品", "policy":"required"}])
      ))
      .unwrap_err(),
      TurnValidationError::ConstraintConflict("guidance.terminology")
    );
    assert_eq!(
      TranslationTurn::new(parse(
        json!([{"start": 7, "end": 13}]),
        json!([{"source":"Launch","target":"{name}", "policy":"forbidden"}])
      ))
      .unwrap_err(),
      TurnValidationError::ConstraintConflict("guidance.terminology")
    );
  }

  #[test]
  fn image_headers_regions_and_reading_order_are_validated() {
    let mut png = vec![0u8; 24];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[16..20].copy_from_slice(&640u32.to_be_bytes());
    png[20..24].copy_from_slice(&480u32.to_be_bytes());
    let encoded = BASE64.encode(png);
    let request: TranslationTurnRequest = serde_json::from_value(json!({
      "input": {"type":"image_regions", "images":[{"image_id":"p1", "media_type":"image/png", "data":encoded,
        "regions":[{"region_id":"r1","x":0.1,"y":0.2,"width":0.5,"height":0.2}]}], "reading_order":["p1:r1"]},
      "source_language":"auto", "target_language":"en", "response_level":"standard"
    })).unwrap();
    let turn = TranslationTurn::new(request).unwrap();
    assert_eq!(turn.input_kind(), TranslationInputKind::ImageRegions);
    let TranslationInput::ImageRegions {
      images,
      reading_order,
    } = turn.input()
    else {
      panic!("validated image input changed shape")
    };
    assert_eq!(images[0].image_id, "p1");
    assert_eq!(images[0].data, encoded);
    assert_eq!(images[0].regions[0].region_id, "r1");
    assert_eq!(reading_order, &["p1:r1"]);
  }

  #[test]
  fn alternatives_and_conflicting_required_terms_are_rejected_without_content() {
    let mut input = request();
    input.guidance = Some(TranslationGuidance {
      max_alternatives: 1,
      ..Default::default()
    });
    assert_eq!(
      TranslationTurn::new(input).unwrap_err(),
      TurnValidationError::Unsupported("guidance.max_alternatives")
    );
    let mut input = request();
    input.guidance = Some(TranslationGuidance {
      terminology: vec![
        TerminologyConstraint {
          source: "term".into(),
          target: "甲".into(),
          policy: TerminologyPolicy::Required,
        },
        TerminologyConstraint {
          source: "term".into(),
          target: "乙".into(),
          policy: TerminologyPolicy::Required,
        },
      ],
      ..Default::default()
    });
    assert_eq!(
      TranslationTurn::new(input).unwrap_err(),
      TurnValidationError::ConstraintConflict("guidance.terminology")
    );
  }

  #[test]
  fn history_rejects_blank_text_and_non_closed_language_tags() {
    let mut blank = request();
    blank.history = vec![history("  ")];
    assert_eq!(
      TranslationTurn::new(blank).unwrap_err(),
      TurnValidationError::Field("history")
    );

    let mut unsupported = request();
    let mut prior = history("hello");
    prior.source_language = "auto".to_string();
    unsupported.history = vec![prior];
    assert_eq!(
      TranslationTurn::new(unsupported).unwrap_err(),
      TurnValidationError::Field("history")
    );
  }

  #[test]
  fn projection_changes_only_supporting_breadth() {
    let draft = LexicalTurnDraft {
      translations: vec![
        LexicalMeaningDraft {
          text: "热的".to_string(),
          meaning: "high temperature".to_string(),
          part_of_speech: "adjective".to_string(),
          phrase_type: String::new(),
          aliases: vec!["heated".to_string()],
          examples: vec![
            TurnExample {
              source_text: "hot tea".to_string(),
              translated_text: "热茶".to_string(),
            },
            TurnExample {
              source_text: "hot day".to_string(),
              translated_text: "炎热的一天".to_string(),
            },
          ],
          usage_notes: vec![
            "temperature".to_string(),
            "literal".to_string(),
            "common".to_string(),
          ],
        },
        LexicalMeaningDraft {
          text: "热门的".to_string(),
          meaning: "popular".to_string(),
          part_of_speech: "adjective".to_string(),
          phrase_type: String::new(),
          aliases: Vec::new(),
          examples: Vec::new(),
          usage_notes: Vec::new(),
        },
      ],
    };
    assert!(draft.is_valid(TranslationUnit::Word));
    let full = TranslationTurnResult::lexical(
      draft,
      TranslationUnit::Word,
      TurnLanguage::English,
      TurnLanguage::Chinese,
    );
    let brief = serde_json::to_value(full.clone().project(ResponseLevel::Brief)).unwrap();
    let standard = serde_json::to_value(full.clone().project(ResponseLevel::Standard)).unwrap();
    let full = serde_json::to_value(full.project(ResponseLevel::Full)).unwrap();

    for result in [&brief, &standard, &full] {
      assert_eq!(result["translations"].as_array().unwrap().len(), 2);
      assert_eq!(result["translations"][0]["text"], "热的");
      assert_eq!(result["translations"][1]["meaning"], "popular");
    }
    assert!(brief["translations"][0].get("details").is_none());
    assert_eq!(
      standard["translations"][0]["details"]["examples"]
        .as_array()
        .unwrap()
        .len(),
      1
    );
    assert!(standard["translations"][0]["details"]
      .get("aliases")
      .is_none());
    assert_eq!(
      full["translations"][0]["details"]["examples"]
        .as_array()
        .unwrap()
        .len(),
      2
    );
  }

  fn plain_translation(text: &str) -> TurnTranslation {
    TurnTranslation {
      translation_id: "translation_0".into(),
      order: 0,
      text: text.into(),
      language: TurnLanguage::Chinese,
      meaning: None,
      details: None,
    }
  }

  fn annotation(minimum_level: ResponseLevel) -> TranslationAnnotation {
    TranslationAnnotation {
      family: AnnotationFamily::Terminology,
      code: TranslationAnnotationCode::TermSelected,
      message: "The requested term was selected.".into(),
      minimum_level,
      citations: Vec::new(),
    }
  }

  #[test]
  fn structured_projection_preserves_identity_order_translation_constraints_and_review() {
    let review = TranslationReview {
      state: TranslationReviewState::ReviewRecommended,
      issues: vec![TranslationReviewIssue::FormatRisk],
    };
    let superset = TranslationTurnResult::Segment {
      segments: vec![
        SegmentTranslationResult {
          segment_id: "title".into(),
          order: 0,
          detected_source_language: TurnLanguage::English,
          translations: vec![plain_translation("标题 {name}")],
          annotations: vec![
            annotation(ResponseLevel::Brief),
            annotation(ResponseLevel::Full),
            TranslationAnnotation {
              family: AnnotationFamily::Review,
              code: TranslationAnnotationCode::ReviewRequired,
              message: "Formatting requires review.".into(),
              minimum_level: ResponseLevel::Brief,
              citations: Vec::new(),
            },
          ],
          review: review.clone(),
        },
        SegmentTranslationResult {
          segment_id: "body".into(),
          order: 1,
          detected_source_language: TurnLanguage::English,
          translations: vec![plain_translation("正文")],
          annotations: Vec::new(),
          review: TranslationReview::clean(),
        },
      ],
      terminology_decisions: vec![TerminologyDecision {
        source: "launch".into(),
        target: Some("发布".into()),
        policy: TerminologyPolicy::Required,
      }],
    };
    assert_eq!(superset.validate(), Ok(()));
    let brief = serde_json::to_value(superset.clone().project(ResponseLevel::Brief)).unwrap();
    let full = serde_json::to_value(superset.project(ResponseLevel::Full)).unwrap();
    for value in [&brief, &full] {
      assert_eq!(value["unit"], "segment");
      assert_eq!(value["segments"][0]["segment_id"], "title");
      assert_eq!(value["segments"][0]["order"], 0);
      assert_eq!(
        value["segments"][0]["translations"][0]["text"],
        "标题 {name}"
      );
      assert_eq!(
        value["segments"][0]["review"]["state"],
        "review_recommended"
      );
      assert_eq!(value["terminology_decisions"][0]["target"], "发布");
    }
    assert_eq!(
      brief["segments"][0]["annotations"]
        .as_array()
        .unwrap()
        .len(),
      2
    );
    assert_eq!(
      full["segments"][0]["annotations"].as_array().unwrap().len(),
      3
    );
  }

  #[test]
  fn image_results_validate_order_review_and_response_local_citations() {
    let translation = TranslationTurnResult::ImageRegion {
      regions: vec![ImageRegionTranslationResult {
        image_id: "page-1".into(),
        region_id: "region-1".into(),
        order: 0,
        detected_source_language: TurnLanguage::English,
        translations: vec![plain_translation("警告")],
        annotations: vec![TranslationAnnotation {
          family: AnnotationFamily::Culture,
          code: TranslationAnnotationCode::CulturalContext,
          message: "Context comes from the cited source.".into(),
          minimum_level: ResponseLevel::Standard,
          citations: vec![CitationReference {
            source_id: "live_1".into(),
            claim_id: "translation_0".into(),
            fragment_id: Some("fragment_1".into()),
          }],
        }],
        review: TranslationReview::clean(),
      }],
      terminology_decisions: Vec::new(),
    };
    let projected = ProjectedTranslationResult {
      translation,
      metadata: TranslationVersionMetadata {
        schema_version: TRANSLATION_RESULT_SCHEMA_VERSION,
        normalizer_version: NORMALIZER_VERSION,
        projection_version: PROJECTION_VERSION,
        response_level: ResponseLevel::Full,
        model_versions: Vec::new(),
        prompt_versions: Vec::new(),
        inference_profiles: Vec::new(),
        reasoning_escalated: false,
        retrieval_version: Some("live-retrieval-v1".into()),
        content_release: None,
      },
      external_sources: vec![ExternalSourceReference {
        source_id: "live_1".into(),
        title: "Public notice".into(),
        url: "https://example.invalid/notice".into(),
        evidence_state: ExternalEvidenceState::LiveExternal,
      }],
    };
    assert_eq!(projected.validate(), Ok(()));
    let mut invalid = projected;
    invalid.external_sources.clear();
    assert_eq!(
      invalid.validate(),
      Err(TranslationResultValidationError::InvalidValue)
    );
  }

  #[test]
  fn structured_results_reject_order_gaps_before_projection() {
    let invalid = TranslationTurnResult::Segment {
      segments: vec![SegmentTranslationResult {
        segment_id: "segment-1".into(),
        order: 1,
        detected_source_language: TurnLanguage::English,
        translations: vec![plain_translation("译文")],
        annotations: Vec::new(),
        review: TranslationReview::clean(),
      }],
      terminology_decisions: Vec::new(),
    };
    assert_eq!(
      invalid.validate(),
      Err(TranslationResultValidationError::InvalidIdentityOrder)
    );
  }

  #[test]
  fn result_variants_reject_cross_family_shapes_and_noncanonical_ids() {
    let mut passage =
      TranslationTurnResult::passage("译文".into(), TurnLanguage::English, TurnLanguage::Chinese);
    let TranslationTurnResult::Passage { translations, .. } = &mut passage else {
      unreachable!()
    };
    translations[0].translation_id = "caller-picked".into();
    assert_eq!(
      passage.validate(),
      Err(TranslationResultValidationError::InvalidValue)
    );

    let mut passage =
      TranslationTurnResult::passage("译文".into(), TurnLanguage::English, TurnLanguage::Chinese);
    let TranslationTurnResult::Passage { translations, .. } = &mut passage else {
      unreachable!()
    };
    translations.push(plain_translation("第二个译文"));
    translations[1].translation_id = "translation_1".into();
    translations[1].order = 1;
    assert_eq!(
      passage.validate(),
      Err(TranslationResultValidationError::InvalidValue)
    );
  }

  #[test]
  fn request_bound_validation_rejects_substituted_segment_identity() {
    let turn = TranslationTurn::new(
      serde_json::from_value(json!({
        "input":{"type":"segments","segments":[{"segment_id":"expected","text":"Title",
          "role":"title","format":"plain","protected_ranges":[]}]},
        "source_language":"en","target_language":"zh-CN","response_level":"brief","history":[]
      }))
      .unwrap(),
    )
    .unwrap();
    let result = ProjectedTranslationResult {
      translation: TranslationTurnResult::Segment {
        segments: vec![SegmentTranslationResult {
          segment_id: "substituted".into(),
          order: 0,
          detected_source_language: TurnLanguage::English,
          translations: vec![plain_translation("标题")],
          annotations: Vec::new(),
          review: TranslationReview::clean(),
        }],
        terminology_decisions: Vec::new(),
      },
      metadata: metadata(ResponseLevel::Brief),
      external_sources: Vec::new(),
    };
    assert_eq!(
      result.validate_for_turn(&turn),
      Err(TranslationResultValidationError::InvalidIdentityOrder)
    );
  }

  #[test]
  fn projection_removes_sources_whose_annotations_are_not_visible() {
    let translation = TranslationTurnResult::ImageRegion {
      regions: vec![ImageRegionTranslationResult {
        image_id: "page-1".into(),
        region_id: "region-1".into(),
        order: 0,
        detected_source_language: TurnLanguage::English,
        translations: vec![plain_translation("警告")],
        annotations: vec![TranslationAnnotation {
          family: AnnotationFamily::Culture,
          code: TranslationAnnotationCode::CulturalContext,
          message: "Full-only context.".into(),
          minimum_level: ResponseLevel::Full,
          citations: vec![CitationReference {
            source_id: "live_1".into(),
            claim_id: "translation_0".into(),
            fragment_id: None,
          }],
        }],
        review: TranslationReview::clean(),
      }],
      terminology_decisions: Vec::new(),
    };
    let projected = ProjectedTranslationResult {
      translation,
      metadata: metadata(ResponseLevel::Full),
      external_sources: vec![ExternalSourceReference {
        source_id: "live_1".into(),
        title: "Public notice".into(),
        url: "https://example.invalid/notice".into(),
        evidence_state: ExternalEvidenceState::LiveExternal,
      }],
    }
    .project(ResponseLevel::Brief);
    assert!(projected.external_sources.is_empty());
    assert_eq!(projected.validate(), Ok(()));
  }

  #[test]
  fn closed_annotation_review_and_metadata_invariants_fail_closed() {
    let mut translation =
      TranslationTurnResult::passage("译文".into(), TurnLanguage::English, TurnLanguage::Chinese);
    let TranslationTurnResult::Passage { annotations, .. } = &mut translation else {
      unreachable!()
    };
    annotations.push(TranslationAnnotation {
      family: AnnotationFamily::Culture,
      code: TranslationAnnotationCode::TermSelected,
      message: "Mismatched family.".into(),
      minimum_level: ResponseLevel::Brief,
      citations: Vec::new(),
    });
    assert_eq!(
      translation.validate(),
      Err(TranslationResultValidationError::InvalidValue)
    );

    let mut result = ProjectedTranslationResult {
      translation: TranslationTurnResult::passage(
        "译文".into(),
        TurnLanguage::English,
        TurnLanguage::Chinese,
      ),
      metadata: metadata(ResponseLevel::Brief),
      external_sources: Vec::new(),
    };
    result.metadata.schema_version = "translation-result-v0";
    assert_eq!(
      result.validate(),
      Err(TranslationResultValidationError::InvalidValue)
    );
  }

  fn metadata(response_level: ResponseLevel) -> TranslationVersionMetadata {
    TranslationVersionMetadata {
      schema_version: TRANSLATION_RESULT_SCHEMA_VERSION,
      normalizer_version: NORMALIZER_VERSION,
      projection_version: PROJECTION_VERSION,
      response_level,
      model_versions: Vec::new(),
      prompt_versions: Vec::new(),
      inference_profiles: Vec::new(),
      reasoning_escalated: false,
      retrieval_version: None,
      content_release: None,
    }
  }

  #[test]
  fn debug_and_validation_errors_never_contain_request_or_history_text() {
    let secret_text = "current-secret-8172";
    let history_secret = "history-secret-4815";
    let mut input = request();
    input.text = Some(secret_text.to_string());
    input.history = vec![history(history_secret)];
    assert_eq!(format!("{input:?}"), "TranslationTurnRequest(REDACTED)");
    let turn = TranslationTurn::new(input).unwrap();
    let rendered = format!("{turn:?}");
    assert!(!rendered.contains(secret_text));
    assert!(!rendered.contains(history_secret));

    let mut invalid = request();
    invalid.text = Some(secret_text.to_string());
    invalid.history = vec![history("  ")];
    let error = TranslationTurn::new(invalid).unwrap_err().to_string();
    assert!(!error.contains(secret_text));
    assert!(!error.contains(history_secret));

    let structured: TranslationTurnRequest = serde_json::from_value(json!({
      "input":{"type":"segments","segments":[{"segment_id":"private-id-733",
        "text":"private-segment-734","role":"paragraph","format":"plain",
        "protected_ranges":[]}]},
      "source_language":"en","target_language":"zh-CN","response_level":"brief"
    }))
    .unwrap();
    let rendered = format!("{:?}", TranslationTurn::new(structured).unwrap());
    assert!(!rendered.contains("private-id-733"));
    assert!(!rendered.contains("private-segment-734"));

    let structured: TranslationTurnRequest = serde_json::from_value(json!({
      "input":{"type":"segments","segments":[{"segment_id":"private-id-735",
        "text":"private-segment-736","role":"paragraph","format":"plain"}]},
      "source_language":"en","target_language":"zh-CN","response_level":"brief"
    }))
    .unwrap();
    let error = serde_json::to_string(&TranslationTurn::new(structured).unwrap()).unwrap_err();
    let rendered = error.to_string();
    assert!(!rendered.contains("private-id-735"));
    assert!(!rendered.contains("private-segment-736"));
  }
}
