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
#[derive(Clone, Serialize)]
pub struct TranslationTurn {
  text: String,
  source_language: SourceLanguage,
  target_language: TurnLanguage,
  history: Vec<TranslationHistory>,
  guidance: TranslationGuidance,
  #[serde(skip)]
  response_level: ResponseLevel,
}

impl TranslationTurn {
  /// Validates closed selectors and every history item without imposing a history-count cap.
  ///
  /// # Errors
  /// Returns a closed field error or a request-size error before any provider is called.
  pub fn new(request: TranslationTurnRequest) -> Result<Self, TurnValidationError> {
    let input = match (request.text, request.input) {
      (Some(text), None) => TranslationInput::Text { text },
      (None, Some(input)) => input,
      _ => return Err(TurnValidationError::Field("input")),
    };
    let guidance = request.guidance.unwrap_or_default();
    validate_guidance(&guidance)?;
    validate_input(&input, &guidance)?;
    let text = match input {
      TranslationInput::Text { text } => text,
      TranslationInput::Segments { .. } => {
        return Err(TurnValidationError::Unsupported("segments"))
      }
      TranslationInput::ImageRegions { .. } => {
        return Err(TurnValidationError::Unsupported("image_regions"))
      }
    };
    if guidance_requires_execution(&guidance) {
      return Err(TurnValidationError::Unsupported("guidance"));
    }
    let source_language = SourceLanguage::parse(&request.source_language)
      .ok_or(TurnValidationError::Field("source_language"))?;
    let target_language = TurnLanguage::parse(&request.target_language)
      .ok_or(TurnValidationError::Field("target_language"))?;
    let response_level = ResponseLevel::parse(&request.response_level)
      .ok_or(TurnValidationError::Field("response_level"))?;
    // Check raw lengths before serialization to avoid allocating another oversized payload.
    let raw_bytes = request
      .history
      .iter()
      .try_fold(text.len(), |bytes, turn| {
        bytes
          .checked_add(turn.source_text.len())?
          .checked_add(turn.translated_text.len())?
          .checked_add(turn.source_language.len())?
          .checked_add(turn.target_language.len())
      })
      .ok_or(TurnValidationError::TooLarge)?;
    if raw_bytes > MAX_TURN_BYTES {
      return Err(TurnValidationError::TooLarge);
    }
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
      text,
      source_language,
      target_language,
      history: request.history,
      guidance,
      response_level,
    })
  }

  /// Returns the original text without changing paragraph or formatting boundaries.
  pub fn text(&self) -> &str {
    &self.text
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
  /// Returns the projection level, never passed to a generation prompt.
  pub fn response_level(&self) -> ResponseLevel {
    self.response_level
  }
  /// Derives one request-local lookup form while preserving significant symbols such as + and #.
  pub fn lookup_form(&self) -> String {
    TranslationNormalizer::new()
      .normalize(&self.text, self.source_language)
      .primary
  }
  /// Returns whether input is too long or structured to be one lexical unit.
  pub fn requires_passage(&self) -> bool {
    self.text.chars().count() > MAX_LEXICAL_CHARS || self.text.contains(['\n', '\r'])
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
        if segment.segment_id.is_empty()
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
        for term in guidance
          .terminology
          .iter()
          .filter(|term| term.policy == TerminologyPolicy::Required)
        {
          if term.source != term.target
            && term_occurrences(&segment.text, &term.source).any(|(start, end)| {
              protected_ranges
                .iter()
                .any(|&(protected_start, protected_end)| {
                  start < protected_end && end > protected_start
                })
            })
          {
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
    if image.image_id.is_empty() || !image_ids.insert(&image.image_id) {
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
      if region.region_id.is_empty()
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

/// Shared inner translation result used by the unified HTTP response.
#[derive(Clone, Serialize)]
pub struct TranslationTurnResult {
  /// Word, established phrase, or connected passage.
  pub unit: TranslationUnit,
  /// Supported language resolved for the current text.
  pub detected_source_language: TurnLanguage,
  /// Ordered meanings preserved identically across all response levels.
  pub translations: Vec<TurnTranslation>,
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
  pub prompt_versions: Vec<&'static str>,
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
}

/// One translation with optional meaning-specific generated detail.
#[derive(Clone, Serialize)]
pub struct TurnTranslation {
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
  /// Assembles the full lexical superset; callers must validate the model draft first.
  pub fn lexical(
    draft: LexicalTurnDraft,
    unit: TranslationUnit,
    source: TurnLanguage,
    target: TurnLanguage,
  ) -> Self {
    Self {
      unit,
      detected_source_language: source,
      translations: draft
        .translations
        .into_iter()
        .map(|m| TurnTranslation {
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
        .collect(),
    }
  }
  /// Produces a plain connected-text result without inventing optional tips or alternatives.
  pub fn passage(text: String, source: TurnLanguage, target: TurnLanguage) -> Self {
    Self {
      unit: TranslationUnit::Passage,
      detected_source_language: source,
      translations: vec![TurnTranslation {
        text,
        language: target,
        meaning: None,
        details: None,
      }],
    }
  }
  /// Projects an existing superset without changing translation text, meaning count, or rank.
  pub fn project(mut self, level: ResponseLevel) -> Self {
    for translation in &mut self.translations {
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
    self
  }
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
  fn history_has_no_item_cap_inside_the_common_body_bound() {
    let mut input = request();
    input.history = (0..2_048)
      .map(|index| history(format!("turn-{index}")))
      .collect();
    let turn = TranslationTurn::new(input).unwrap();
    assert_eq!(turn.history().len(), 2_048);
  }

  #[test]
  fn history_and_current_text_share_the_one_mebibyte_bound() {
    let mut oversized_history = request();
    oversized_history.history = vec![history("x".repeat(MAX_TURN_BYTES))];
    assert_eq!(
      TranslationTurn::new(oversized_history).unwrap_err(),
      TurnValidationError::TooLarge
    );

    let mut oversized_text = request();
    oversized_text.text = Some("x".repeat(MAX_TURN_BYTES));
    assert_eq!(
      TranslationTurn::new(oversized_text).unwrap_err(),
      TurnValidationError::Field("input.text")
    );
  }

  #[test]
  fn target_text_guidance_is_validated_before_unavailable_execution() {
    let request: TranslationTurnRequest = serde_json::from_value(json!({
      "input": {"type": "text", "text": "torque"},
      "source_language": "en", "target_language": "zh-CN", "response_level": "standard",
      "guidance": {
        "purpose": "technical", "audience": "specialist", "register": "preserve",
        "terminology": [{"source": "torque", "target": "扭矩", "policy": "required"}],
        "max_alternatives": 0, "annotations": ["terminology"], "freshness": "offline"
      }
    }))
    .unwrap();
    assert_eq!(
      TranslationTurn::new(request).unwrap_err(),
      TurnValidationError::Unsupported("guidance")
    );
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
    assert_eq!(
      TranslationTurn::new(parse(json!([{"start": 7, "end": 13}]), json!([]))).unwrap_err(),
      TurnValidationError::Unsupported("segments")
    );
    assert_eq!(
      TranslationTurn::new(parse(
        json!([{"start": 7, "end": 13}, {"start": 0, "end": 6}]),
        json!([])
      ))
      .unwrap_err(),
      TurnValidationError::Unsupported("segments")
    );
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
    assert_eq!(
      TranslationTurn::new(request).unwrap_err(),
      TurnValidationError::Unsupported("image_regions")
    );
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
  }
}
