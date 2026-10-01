//! Confidential, authenticated continuation cursors for target knowledge views and paths.
//!
//! The payload is independent of the transitional graph cursor. It binds pagination to the full
//! immutable release/projection contract and contains only canonical identifiers, versions, and
//! bounded continuation state—never request text or generated content.

use std::{fmt, sync::Arc};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
  aead::{Aead, KeyInit, Payload},
  XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
  canonical::{CanonicalId, CanonicalReleasePin, LanguageTag},
  knowledge_release::{EdgeCollectionId, NodeCollectionId},
  retrieval_data::RetrievalNodeType,
  translation_turn::ResponseLevel,
};

/// Maximum encoded target knowledge cursor length.
pub const MAX_KNOWLEDGE_CURSOR_LENGTH: usize = 8_192;
/// Minimum secret material accepted for cursor protection.
pub const MIN_KNOWLEDGE_CURSOR_KEY_BYTES: usize = 32;
/// Maximum continuation tokens carried by one cursor.
pub const MAX_KNOWLEDGE_CONTINUATION_TOKENS: usize = 8;

const CURSOR_PREFIX: &str = "k1";
const CURSOR_VERSION: u8 = 1;
const NONCE_BYTES: usize = 24;
const AUTH_TAG_BYTES: usize = 16;
const CURSOR_AAD: &[u8] = b"transnet.knowledge.cursor.k1";
const MAX_SCALAR_CHARS: usize = 256;
const MAX_ORDERING_KEY_CHARS: usize = 512;

/// Validated secret used for target knowledge cursor confidentiality and integrity.
#[derive(Clone)]
pub struct KnowledgeCursorProtectionKey(Arc<[u8]>);

impl KnowledgeCursorProtectionKey {
  /// Normalizes at least 32 bytes of high-entropy secret material into one cipher key.
  ///
  /// # Errors
  ///
  /// Returns an error when the supplied material is too short.
  pub fn new(secret: impl AsRef<[u8]>) -> Result<Self, KnowledgeCursorError> {
    let secret = secret.as_ref();
    if secret.len() < MIN_KNOWLEDGE_CURSOR_KEY_BYTES {
      return Err(KnowledgeCursorError::InvalidKey);
    }
    Ok(Self(Arc::from(Sha256::digest(secret).to_vec())))
  }
}

impl fmt::Debug for KnowledgeCursorProtectionKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("KnowledgeCursorProtectionKey(REDACTED)")
  }
}

/// Canonical typed root bound into one target knowledge pagination flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeCursorRoot {
  family: String,
  id: CanonicalId,
}

impl KnowledgeCursorRoot {
  /// Creates a root from one exact closed node-family wire name and publisher-owned identity.
  ///
  /// # Errors
  ///
  /// Returns an error for an unknown family.
  pub fn new(family: impl Into<String>, id: CanonicalId) -> Result<Self, KnowledgeCursorError> {
    let family = family.into();
    if family != "lexeme" && RetrievalNodeType::new(&family).is_err() {
      return Err(KnowledgeCursorError::InvalidPayload);
    }
    Ok(Self { family, id })
  }

  /// Returns the closed node-family wire name.
  pub fn family(&self) -> &str {
    &self.family
  }

  /// Returns the publisher-owned canonical identity.
  pub fn id(&self) -> &CanonicalId {
    &self.id
  }
}

/// Complete immutable request and projection identity to which a cursor is bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeCursorBinding {
  /// Typed canonical root.
  pub root: KnowledgeCursorRoot,
  /// Closed lens wire name; validated as a bounded token until the lens enum is integrated.
  pub lens: String,
  /// Canonical response language.
  pub language: LanguageTag,
  /// Deterministic response breadth.
  pub response_level: ResponseLevel,
  /// Canonical release and schema version.
  pub canonical_release: CanonicalReleasePin,
  /// Immutable node collection identity.
  pub node_collection_id: NodeCollectionId,
  /// SHA-256 hash of the complete node projection.
  pub node_collection_hash: String,
  /// Immutable edge collection identity.
  pub edge_collection_id: EdgeCollectionId,
  /// SHA-256 hash of the complete edge projection.
  pub edge_collection_hash: String,
  /// Canonical assertion schema version.
  pub assertion_version: String,
  /// Relation-registry version.
  pub registry_version: String,
  /// Projection contract version.
  pub projection_version: String,
  /// Lens-policy version.
  pub lens_policy_version: String,
  /// Stable result-ordering version.
  pub ordering_version: String,
}

impl KnowledgeCursorBinding {
  /// Validates every bounded version, lens, hash, and release component.
  ///
  /// # Errors
  ///
  /// Returns an error when a field is blank, untrimmed, oversized, or a hash is not canonical
  /// lowercase SHA-256.
  pub fn validate(&self) -> Result<(), KnowledgeCursorError> {
    if !valid_token(&self.lens)
      || !valid_scalar(
        &self.canonical_release.canonical_schema_version,
        MAX_SCALAR_CHARS,
      )
      || !valid_hash(&self.node_collection_hash)
      || !valid_hash(&self.edge_collection_hash)
      || [
        &self.assertion_version,
        &self.registry_version,
        &self.projection_version,
        &self.lens_policy_version,
        &self.ordering_version,
      ]
      .into_iter()
      .any(|value| !valid_scalar(value, MAX_SCALAR_CHARS))
    {
      return Err(KnowledgeCursorError::InvalidPayload);
    }
    Ok(())
  }
}

/// Validated target knowledge continuation state.
#[derive(Clone, Eq, PartialEq)]
pub struct KnowledgeCursor {
  binding: KnowledgeCursorBinding,
  ordering_key: String,
  continuation_tokens: Vec<String>,
}

impl KnowledgeCursor {
  /// Creates bounded continuation state for an already validated request/projection binding.
  ///
  /// # Errors
  ///
  /// Returns an error for invalid binding, ordering key, or continuation token bounds.
  pub fn new(
    binding: KnowledgeCursorBinding,
    ordering_key: impl Into<String>,
    continuation_tokens: Vec<String>,
  ) -> Result<Self, KnowledgeCursorError> {
    binding.validate()?;
    let ordering_key = ordering_key.into();
    if !valid_scalar(&ordering_key, MAX_ORDERING_KEY_CHARS)
      || continuation_tokens.len() > MAX_KNOWLEDGE_CONTINUATION_TOKENS
      || continuation_tokens
        .iter()
        .any(|value| !valid_scalar(value, MAX_SCALAR_CHARS))
    {
      return Err(KnowledgeCursorError::InvalidPayload);
    }
    Ok(Self {
      binding,
      ordering_key,
      continuation_tokens,
    })
  }

  /// Returns the complete immutable cursor binding.
  pub fn binding(&self) -> &KnowledgeCursorBinding {
    &self.binding
  }

  /// Returns the stable ordering key after which pagination resumes.
  pub fn ordering_key(&self) -> &str {
    &self.ordering_key
  }

  /// Returns bounded opaque dependency continuation tokens.
  pub fn continuation_tokens(&self) -> &[String] {
    &self.continuation_tokens
  }
}

impl fmt::Debug for KnowledgeCursor {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("KnowledgeCursor(REDACTED)")
  }
}

/// Stateless authenticated codec for opaque target knowledge cursors.
pub struct KnowledgeCursorCodec {
  key: KnowledgeCursorProtectionKey,
}

impl KnowledgeCursorCodec {
  /// Creates a codec using stable caller-managed secret material.
  pub fn new(key: KnowledgeCursorProtectionKey) -> Self {
    Self { key }
  }

  /// Encrypts and authenticates one validated cursor.
  ///
  /// # Errors
  ///
  /// Returns a content-free error when encoding, randomness, encryption, or size admission fails.
  pub fn encode(&self, cursor: &KnowledgeCursor) -> Result<String, KnowledgeCursorError> {
    cursor.binding.validate()?;
    let payload = CursorPayload::from(cursor);
    let bytes = serde_json::to_vec(&payload).map_err(|_| KnowledgeCursorError::Encoding)?;
    let cipher =
      XChaCha20Poly1305::new_from_slice(&self.key.0).map_err(|_| KnowledgeCursorError::Encoding)?;
    let mut nonce = [0_u8; NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| KnowledgeCursorError::Encoding)?;
    let ciphertext = cipher
      .encrypt(
        XNonce::from_slice(&nonce),
        Payload {
          msg: &bytes,
          aad: CURSOR_AAD,
        },
      )
      .map_err(|_| KnowledgeCursorError::Encoding)?;
    let encoded = format!(
      "{CURSOR_PREFIX}.{}.{}",
      URL_SAFE_NO_PAD.encode(nonce),
      URL_SAFE_NO_PAD.encode(ciphertext)
    );
    if encoded.len() > MAX_KNOWLEDGE_CURSOR_LENGTH {
      return Err(KnowledgeCursorError::Encoding);
    }
    Ok(encoded)
  }

  /// Decodes a cursor and requires its complete immutable binding to match the current request.
  ///
  /// # Errors
  ///
  /// Returns a content-free error for malformed, oversized, tampered, unsupported-version, or
  /// differently bound cursors.
  pub fn decode_for(
    &self,
    value: &str,
    expected: &KnowledgeCursorBinding,
  ) -> Result<KnowledgeCursor, KnowledgeCursorError> {
    expected.validate()?;
    if value.is_empty() || value.len() > MAX_KNOWLEDGE_CURSOR_LENGTH {
      return Err(KnowledgeCursorError::Malformed);
    }
    let mut parts = value.split('.');
    let (Some(prefix), Some(nonce), Some(ciphertext), None) =
      (parts.next(), parts.next(), parts.next(), parts.next())
    else {
      return Err(KnowledgeCursorError::Malformed);
    };
    if prefix != CURSOR_PREFIX {
      return Err(KnowledgeCursorError::Malformed);
    }
    let nonce = URL_SAFE_NO_PAD
      .decode(nonce)
      .map_err(|_| KnowledgeCursorError::Malformed)?;
    let ciphertext = URL_SAFE_NO_PAD
      .decode(ciphertext)
      .map_err(|_| KnowledgeCursorError::Malformed)?;
    if nonce.len() != NONCE_BYTES || ciphertext.len() < AUTH_TAG_BYTES {
      return Err(KnowledgeCursorError::Malformed);
    }
    let cipher = XChaCha20Poly1305::new_from_slice(&self.key.0)
      .map_err(|_| KnowledgeCursorError::Malformed)?;
    let plaintext = cipher
      .decrypt(
        XNonce::from_slice(&nonce),
        Payload {
          msg: &ciphertext,
          aad: CURSOR_AAD,
        },
      )
      .map_err(|_| KnowledgeCursorError::Malformed)?;
    let payload: CursorPayload =
      serde_json::from_slice(&plaintext).map_err(|_| KnowledgeCursorError::Malformed)?;
    let cursor = payload.into_domain()?;
    if cursor.binding() != expected {
      return Err(KnowledgeCursorError::BindingMismatch);
    }
    Ok(cursor)
  }
}

/// Closed cursor failures that never expose secret or continuation content.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KnowledgeCursorError {
  /// Protection-key material was too short.
  #[error("knowledge cursor protection key is invalid")]
  InvalidKey,
  /// Domain state violated a cursor bound or closed catalog.
  #[error("knowledge cursor payload is invalid")]
  InvalidPayload,
  /// Cursor encryption or serialization failed.
  #[error("knowledge cursor encoding failed")]
  Encoding,
  /// The wire value was malformed, oversized, tampered, or used an unsupported version.
  #[error("knowledge cursor is invalid")]
  Malformed,
  /// The cursor belongs to another root, lens, response shape, release, or projection contract.
  #[error("knowledge cursor binding does not match the request")]
  BindingMismatch,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorPayload {
  version: u8,
  root_family: String,
  root_id: String,
  lens: String,
  language: String,
  response_level: String,
  release_id: String,
  canonical_schema_version: String,
  node_collection_id: String,
  node_collection_hash: String,
  edge_collection_id: String,
  edge_collection_hash: String,
  assertion_version: String,
  registry_version: String,
  projection_version: String,
  lens_policy_version: String,
  ordering_version: String,
  ordering_key: String,
  continuation_tokens: Vec<String>,
}

impl From<&KnowledgeCursor> for CursorPayload {
  fn from(value: &KnowledgeCursor) -> Self {
    let binding = value.binding();
    Self {
      version: CURSOR_VERSION,
      root_family: binding.root.family.clone(),
      root_id: binding.root.id.to_string(),
      lens: binding.lens.clone(),
      language: binding.language.as_str().to_string(),
      response_level: response_level_name(binding.response_level).to_string(),
      release_id: binding.canonical_release.release_id.to_string(),
      canonical_schema_version: binding.canonical_release.canonical_schema_version.clone(),
      node_collection_id: binding.node_collection_id.as_str().to_string(),
      node_collection_hash: binding.node_collection_hash.clone(),
      edge_collection_id: binding.edge_collection_id.as_str().to_string(),
      edge_collection_hash: binding.edge_collection_hash.clone(),
      assertion_version: binding.assertion_version.clone(),
      registry_version: binding.registry_version.clone(),
      projection_version: binding.projection_version.clone(),
      lens_policy_version: binding.lens_policy_version.clone(),
      ordering_version: binding.ordering_version.clone(),
      ordering_key: value.ordering_key.clone(),
      continuation_tokens: value.continuation_tokens.clone(),
    }
  }
}

impl CursorPayload {
  fn into_domain(self) -> Result<KnowledgeCursor, KnowledgeCursorError> {
    if self.version != CURSOR_VERSION {
      return Err(KnowledgeCursorError::Malformed);
    }
    let binding = KnowledgeCursorBinding {
      root: KnowledgeCursorRoot::new(self.root_family, parse_id(self.root_id)?)?,
      lens: self.lens,
      language: LanguageTag::parse(&self.language).map_err(|_| KnowledgeCursorError::Malformed)?,
      response_level: ResponseLevel::parse(&self.response_level)
        .ok_or(KnowledgeCursorError::Malformed)?,
      canonical_release: CanonicalReleasePin::new(
        parse_id(self.release_id)?,
        self.canonical_schema_version,
      )
      .ok_or(KnowledgeCursorError::Malformed)?,
      node_collection_id: NodeCollectionId::parse(self.node_collection_id)
        .map_err(|_| KnowledgeCursorError::Malformed)?,
      node_collection_hash: self.node_collection_hash,
      edge_collection_id: EdgeCollectionId::parse(self.edge_collection_id)
        .map_err(|_| KnowledgeCursorError::Malformed)?,
      edge_collection_hash: self.edge_collection_hash,
      assertion_version: self.assertion_version,
      registry_version: self.registry_version,
      projection_version: self.projection_version,
      lens_policy_version: self.lens_policy_version,
      ordering_version: self.ordering_version,
    };
    KnowledgeCursor::new(binding, self.ordering_key, self.continuation_tokens)
      .map_err(|_| KnowledgeCursorError::Malformed)
  }
}

fn parse_id(value: String) -> Result<CanonicalId, KnowledgeCursorError> {
  CanonicalId::new(value).map_err(|_| KnowledgeCursorError::Malformed)
}

const fn response_level_name(value: ResponseLevel) -> &'static str {
  match value {
    ResponseLevel::Brief => "brief",
    ResponseLevel::Standard => "standard",
    ResponseLevel::Full => "full",
  }
}

fn valid_scalar(value: &str, max_chars: usize) -> bool {
  value.trim() == value
    && !value.is_empty()
    && value.chars().count() <= max_chars
    && !value.chars().any(char::is_control)
}

fn valid_token(value: &str) -> bool {
  valid_scalar(value, 64)
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_hash(value: &str) -> bool {
  value.len() == 71
    && value.starts_with("sha256:")
    && value[7..]
      .bytes()
      .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
  }

  fn binding() -> KnowledgeCursorBinding {
    KnowledgeCursorBinding {
      root: KnowledgeCursorRoot::new("lexical_sense", id("sense-root")).unwrap(),
      lens: "usage".into(),
      language: LanguageTag::parse("zh-CN").unwrap(),
      response_level: ResponseLevel::Standard,
      canonical_release: CanonicalReleasePin::new(id("release-1"), "canonical-v1".into()).unwrap(),
      node_collection_id: NodeCollectionId::parse("nodes-1").unwrap(),
      node_collection_hash: format!("sha256:{}", "a".repeat(64)),
      edge_collection_id: EdgeCollectionId::parse("edges-1").unwrap(),
      edge_collection_hash: format!("sha256:{}", "b".repeat(64)),
      assertion_version: "canonical-assertions-v1".into(),
      registry_version: "relations-v1".into(),
      projection_version: "knowledge-projection-v1".into(),
      lens_policy_version: "knowledge-lenses-v1".into(),
      ordering_version: "knowledge-order-v1".into(),
    }
  }

  fn codec() -> KnowledgeCursorCodec {
    KnowledgeCursorCodec::new(KnowledgeCursorProtectionKey::new([7_u8; 32]).unwrap())
  }

  #[test]
  fn round_trip_preserves_every_binding_and_continuation_field() {
    let binding = binding();
    let cursor = KnowledgeCursor::new(
      binding.clone(),
      "0007:node-a",
      vec!["nodes:opaque-a".into(), "edges:opaque-b".into()],
    )
    .unwrap();
    let encoded = codec().encode(&cursor).unwrap();
    assert!(encoded.starts_with("k1."));
    assert!(!encoded.contains("sense-root"));
    assert_eq!(codec().decode_for(&encoded, &binding).unwrap(), cursor);
  }

  #[test]
  fn tamper_wrong_key_and_version_fail_closed() {
    let binding = binding();
    let cursor = KnowledgeCursor::new(binding.clone(), "key", vec![]).unwrap();
    let mut encoded = codec().encode(&cursor).unwrap().into_bytes();
    let last = encoded.len() - 1;
    encoded[last] = if encoded[last] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(encoded).unwrap();
    assert_eq!(
      codec().decode_for(&tampered, &binding),
      Err(KnowledgeCursorError::Malformed)
    );
    let other = KnowledgeCursorCodec::new(KnowledgeCursorProtectionKey::new([8_u8; 32]).unwrap());
    let valid = codec().encode(&cursor).unwrap();
    assert_eq!(
      other.decode_for(&valid, &binding),
      Err(KnowledgeCursorError::Malformed)
    );
    assert_eq!(
      codec().decode_for(&valid.replacen("k1", "k2", 1), &binding),
      Err(KnowledgeCursorError::Malformed)
    );
  }

  #[test]
  fn every_request_and_projection_binding_change_is_rejected() {
    let original = binding();
    let encoded = codec()
      .encode(&KnowledgeCursor::new(original.clone(), "key", vec![]).unwrap())
      .unwrap();
    let mut changes = Vec::new();
    let mut changed = original.clone();
    changed.lens = "taxonomy".into();
    changes.push(changed);
    let mut changed = original.clone();
    changed.response_level = ResponseLevel::Full;
    changes.push(changed);
    let mut changed = original.clone();
    changed.canonical_release =
      CanonicalReleasePin::new(id("release-2"), "canonical-v1".into()).unwrap();
    changes.push(changed);
    let mut changed = original.clone();
    changed.node_collection_hash = format!("sha256:{}", "c".repeat(64));
    changes.push(changed);
    let mut changed = original.clone();
    changed.edge_collection_id = EdgeCollectionId::parse("edges-2").unwrap();
    changes.push(changed);
    let mut changed = original.clone();
    changed.assertion_version = "canonical-assertions-v2".into();
    changes.push(changed);
    let mut changed = original.clone();
    changed.ordering_version = "knowledge-order-v2".into();
    changes.push(changed);
    for changed in changes {
      assert_eq!(
        codec().decode_for(&encoded, &changed),
        Err(KnowledgeCursorError::BindingMismatch)
      );
    }
  }

  #[test]
  fn bounds_and_debug_output_do_not_leak_private_values() {
    assert!(matches!(
      KnowledgeCursorProtectionKey::new([1_u8; 31]),
      Err(KnowledgeCursorError::InvalidKey)
    ));
    let tokens = vec!["token".to_string(); MAX_KNOWLEDGE_CONTINUATION_TOKENS + 1];
    assert_eq!(
      KnowledgeCursor::new(binding(), "key", tokens),
      Err(KnowledgeCursorError::InvalidPayload)
    );
    let cursor =
      KnowledgeCursor::new(binding(), "private-ordering", vec!["private-token".into()]).unwrap();
    let debug = format!(
      "{cursor:?} {:?}",
      KnowledgeCursorProtectionKey::new([9_u8; 32]).unwrap()
    );
    assert!(!debug.contains("private"));
    assert!(!debug.contains('9'));
  }
}
