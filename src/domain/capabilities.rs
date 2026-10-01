//! Closed, content-free declarations of the capabilities implemented by this runtime.

use serde::Serialize;

use crate::{
  application::translation::{MAX_CONNECTED_CHUNKS, MAX_CONNECTED_CHUNK_CHARS},
  domain::translation_turn::{MAX_GENERATION_CONTEXT_BYTES, MAX_LEXICAL_CHARS, MAX_TURN_BYTES},
};

/// Current runtime capabilities safe to expose to an internal caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServiceCapabilities {
  /// Accepted source-language selectors.
  pub source_languages: Vec<SourceLanguageCapability>,
  /// Accepted target language tags.
  pub target_languages: Vec<TargetLanguageCapability>,
  /// Translation input shapes with an implemented handler and application path.
  pub input_types: Vec<InputTypeCapability>,
  /// Image media types accepted by implemented input shapes.
  pub image_media_types: Vec<ImageMediaTypeCapability>,
  /// Request-scoped purposes accepted by the implemented translation schema.
  pub purposes: Vec<PurposeCapability>,
  /// Annotation families emitted by the implemented result schema.
  pub annotation_families: Vec<AnnotationFamilyCapability>,
  /// Knowledge lenses backed by implemented target view handlers.
  pub knowledge_lenses: Vec<KnowledgeLensCapability>,
  /// Closed semantic and transport limits.
  pub limits: CapabilityLimits,
  /// Live-retrieval availability and default behavior.
  pub live_retrieval: LiveRetrievalCapability,
  /// Generation profiles reachable through the current application path.
  pub generation_profiles: Vec<GenerationProfileCapability>,
  /// Result schemas emitted by implemented handlers.
  pub schema_versions: Vec<SchemaVersionCapability>,
}

impl ServiceCapabilities {
  /// Builds a declaration from implemented closed sets and the configured HTTP body limit.
  pub fn current(max_request_body_bytes: usize) -> Self {
    Self {
      source_languages: vec![
        SourceLanguageCapability::Auto,
        SourceLanguageCapability::English,
        SourceLanguageCapability::Chinese,
      ],
      target_languages: vec![
        TargetLanguageCapability::English,
        TargetLanguageCapability::Chinese,
      ],
      input_types: vec![InputTypeCapability::Text],
      image_media_types: Vec::new(),
      purposes: Vec::new(),
      annotation_families: Vec::new(),
      knowledge_lenses: Vec::new(),
      limits: CapabilityLimits {
        max_request_body_bytes,
        max_translation_bytes: MAX_TURN_BYTES,
        max_generation_context_bytes: MAX_GENERATION_CONTEXT_BYTES,
        max_lexical_chars: MAX_LEXICAL_CHARS,
        max_connected_chunk_chars: MAX_CONNECTED_CHUNK_CHARS,
        max_connected_chunks: MAX_CONNECTED_CHUNKS,
      },
      live_retrieval: LiveRetrievalCapability {
        available: false,
        default: LiveRetrievalDefault::Offline,
      },
      generation_profiles: vec![
        GenerationProfileCapability::Fast,
        GenerationProfileCapability::Reasoning,
      ],
      schema_versions: vec![SchemaVersionCapability::TranslationResultV1],
    }
  }

  /// Activates only the lenses backed by one fully composed canonical/retrieval/view bundle.
  pub fn with_knowledge_bundle(mut self, bundle: KnowledgeCapabilityBundle) -> Self {
    if bundle == KnowledgeCapabilityBundle::FullyConfigured {
      self.knowledge_lenses = vec![
        KnowledgeLensCapability::Meaning,
        KnowledgeLensCapability::Contrast,
        KnowledgeLensCapability::Usage,
        KnowledgeLensCapability::Form,
        KnowledgeLensCapability::Origin,
        KnowledgeLensCapability::Domain,
      ];
    }
    self
  }

  /// Rebinds the configured HTTP body limit without changing activated capabilities.
  pub fn with_max_request_body_bytes(mut self, max_request_body_bytes: usize) -> Self {
    self.limits.max_request_body_bytes = max_request_body_bytes;
    self
  }
}

/// Atomic runtime composition state for the executable guided-view dependency bundle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KnowledgeCapabilityBundle {
  /// One or more canonical, retrieval, projection-authority, view-service, or route pieces are absent.
  #[default]
  Disabled,
  /// Canonical and retrieval ports, active-trio authority, view service, cursor key, and route exist.
  FullyConfigured,
}

/// Supported source-language selectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceLanguageCapability {
  /// Request-local English or Chinese detection.
  #[serde(rename = "auto")]
  Auto,
  /// English input.
  #[serde(rename = "en")]
  English,
  /// Simplified Chinese input.
  #[serde(rename = "zh-CN")]
  Chinese,
}

/// Supported target languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TargetLanguageCapability {
  /// English output.
  #[serde(rename = "en")]
  English,
  /// Simplified Chinese output.
  #[serde(rename = "zh-CN")]
  Chinese,
}

/// Implemented translation input shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputTypeCapability {
  /// One connected or lexical text input.
  Text,
}

/// Image media types; the enum is intentionally uninhabited until vision input is implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ImageMediaTypeCapability {}

/// Request purposes; the enum is intentionally uninhabited until guidance is implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PurposeCapability {}

/// Result annotation families; intentionally uninhabited until typed annotations are implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AnnotationFamilyCapability {}

/// Guided-view lenses with an executable relation policy in the current implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLensCapability {
  /// Definitions, equivalence, and taxonomy around the selected meaning.
  Meaning,
  /// Explicit opposition, near-equivalence, and reviewed confusion.
  Contrast,
  /// Reviewed construction, collocation, and suitability relationships.
  Usage,
  /// Inflectional and derivational form relationships.
  Form,
  /// Reviewed historical derivation relationships.
  Origin,
  /// Explicit domain membership and domain-scoped relationships.
  Domain,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn knowledge_lenses_activate_only_as_one_complete_bundle() {
    assert!(ServiceCapabilities::current(1_024)
      .with_knowledge_bundle(KnowledgeCapabilityBundle::Disabled)
      .knowledge_lenses
      .is_empty());
    assert_eq!(
      ServiceCapabilities::current(1_024)
        .with_knowledge_bundle(KnowledgeCapabilityBundle::FullyConfigured)
        .knowledge_lenses,
      vec![
        KnowledgeLensCapability::Meaning,
        KnowledgeLensCapability::Contrast,
        KnowledgeLensCapability::Usage,
        KnowledgeLensCapability::Form,
        KnowledgeLensCapability::Origin,
        KnowledgeLensCapability::Domain,
      ]
    );
  }
}

/// Numeric bounds enforced by the current translation and HTTP paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CapabilityLimits {
  /// Configured maximum serialized HTTP request size in bytes.
  pub max_request_body_bytes: usize,
  /// Maximum serialized translation turn size in bytes.
  pub max_translation_bytes: usize,
  /// Maximum encoded request-local history and guidance passed into generation.
  pub max_generation_context_bytes: usize,
  /// Largest input considered for lexical orchestration, in Unicode scalar values.
  pub max_lexical_chars: usize,
  /// Maximum connected-text chunk size, in Unicode scalar values.
  pub max_connected_chunk_chars: usize,
  /// Maximum number of connected-text chunks.
  pub max_connected_chunks: usize,
}

/// Live-retrieval declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LiveRetrievalCapability {
  /// Whether the runtime can perform live retrieval.
  pub available: bool,
  /// Default policy applied when callers omit retrieval guidance.
  pub default: LiveRetrievalDefault,
}

/// Closed default live-retrieval policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveRetrievalDefault {
  /// No live network retrieval occurs.
  Offline,
}

/// Generation profiles reachable through the current translation implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProfileCapability {
  /// Ordinary bounded generation without reasoning escalation.
  Fast,
  /// Single policy-owned repair escalation for invalid or explicitly ambiguous fast output.
  Reasoning,
}

/// Result schema versions emitted by current target handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SchemaVersionCapability {
  /// Unified translation result schema.
  #[serde(rename = "translation-result-v1")]
  TranslationResultV1,
}
