//! Runtime configuration for the server, HTTP boundary, model providers, and resilience policy.

use std::{fmt, fs, path::Path, sync::Arc, time::Duration};

use serde::Deserialize;
use thiserror::Error;

use crate::{
  domain::knowledge_cursor::KnowledgeCursorProtectionKey,
  resilience::{ProviderPolicy, ProviderPolicyError},
};

/// Default maximum accepted HTTP request body size in bytes.
pub const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 1_048_576;

/// Complete application configuration loaded from `config/transnet.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
  /// Listener and logging settings.
  pub server: ServerConfig,
  /// HTTP boundary settings.
  #[serde(default)]
  pub http: HttpConfig,
  /// Translation routing and retry settings.
  pub translation: TranslationConfig,
  /// Provider used for short text.
  pub gemma4: ProviderConfig,
  /// Provider used for long text.
  pub translate_gemma: ProviderConfig,
  /// Per-provider timeout, retry, bulkhead, and circuit-breaker policy.
  #[serde(default)]
  pub provider_resilience: ProviderResilienceConfigs,
  /// Optional canonical-only read capability; disabled for local model-only operation.
  #[serde(default)]
  pub canonical: CanonicalRuntimeConfig,
  /// Optional knowledge-view and path runtime; disabled until every dependency is configured.
  #[serde(default)]
  pub knowledge: KnowledgeRuntimeConfig,
}

/// Closed application-configuration loading failures that never reproduce configuration contents.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AppConfigLoadError {
  /// The configured file could not be read as UTF-8 text.
  #[error("application configuration is unavailable")]
  Unavailable,
  /// The configuration did not match the strict application schema.
  #[error("application configuration is invalid")]
  Invalid,
}

impl AppConfig {
  /// Parses strict TOML without retaining or returning source-bearing parser diagnostics.
  ///
  /// # Errors
  ///
  /// Returns a closed error when `source` does not match the application configuration schema.
  pub fn parse_toml(source: &str) -> Result<Self, AppConfigLoadError> {
    toml::from_str(source).map_err(|_| AppConfigLoadError::Invalid)
  }

  /// Loads strict TOML without exposing its path or contents through returned errors.
  ///
  /// # Errors
  ///
  /// Returns a closed unavailable or invalid error without chaining filesystem or parser details.
  pub fn load(path: impl AsRef<Path>) -> Result<Self, AppConfigLoadError> {
    let source = fs::read_to_string(path).map_err(|_| AppConfigLoadError::Unavailable)?;
    Self::parse_toml(&source)
  }
}

/// Opt-in outbound island-port canonical read configuration.
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct CanonicalRuntimeConfig {
  /// Whether the canonical dependency is required for readiness.
  pub enabled: bool,
  /// Absolute Unix socket path; required only when enabled.
  pub socket_path: Option<String>,
  /// Maximum duration of one canonical authority operation, in milliseconds.
  pub timeout_ms: Option<u64>,
}

/// Validated settings for an enabled canonical-only dependency.
#[derive(Clone, PartialEq, Eq)]
pub struct EnabledCanonicalRuntimeConfig {
  /// Absolute Unix socket path, never logged by the runtime.
  pub socket_path: String,
  /// Per-operation timeout, also used as the readiness deadline.
  pub timeout: Duration,
}

/// Closed configuration failure without echoing a socket path.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CanonicalRuntimeConfigError {
  /// An enabled capability did not name a bounded absolute Unix socket path.
  #[error("enabled canonical capability requires a valid absolute Unix socket path")]
  InvalidSocketPath,
  /// An enabled capability did not configure a safe operation timeout.
  #[error("enabled canonical capability requires timeout_ms between 1 and 30000")]
  InvalidTimeout,
}

impl CanonicalRuntimeConfig {
  /// Validates only an enabled capability; disabled local operation needs no island-port settings.
  ///
  /// # Errors
  ///
  /// Returns a closed error for a missing or unsafe socket path or timeout.
  pub fn resolve(
    &self,
  ) -> Result<Option<EnabledCanonicalRuntimeConfig>, CanonicalRuntimeConfigError> {
    if !self.enabled {
      return Ok(None);
    }
    let path = self
      .socket_path
      .as_deref()
      .ok_or(CanonicalRuntimeConfigError::InvalidSocketPath)?;
    if path.len() < 2
      || path.len() > 107
      || !path.starts_with('/')
      || path.chars().any(char::is_whitespace)
      || path.bytes().any(|byte| byte == 0)
      || path.split('/').any(|component| component == "..")
    {
      return Err(CanonicalRuntimeConfigError::InvalidSocketPath);
    }
    let timeout_ms = self
      .timeout_ms
      .filter(|value| (1..=30_000).contains(value))
      .ok_or(CanonicalRuntimeConfigError::InvalidTimeout)?;
    Ok(Some(EnabledCanonicalRuntimeConfig {
      socket_path: path.to_owned(),
      timeout: Duration::from_millis(timeout_ms),
    }))
  }
}

impl fmt::Debug for CanonicalRuntimeConfig {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("CanonicalRuntimeConfig")
      .field("enabled", &self.enabled)
      .field("socket_path", &"[REDACTED]")
      .field("timeout_ms", &self.timeout_ms)
      .finish()
  }
}

impl fmt::Debug for EnabledCanonicalRuntimeConfig {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("EnabledCanonicalRuntimeConfig")
      .field("socket_path", &"[REDACTED]")
      .field("timeout", &self.timeout)
      .finish()
  }
}

/// Opt-in knowledge runtime configuration without inline key material.
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KnowledgeRuntimeConfig {
  /// Whether knowledge views and paths may be composed into the runtime.
  pub enabled: bool,
  /// Whether an enabled knowledge dependency participates in readiness.
  pub required: bool,
  /// Name of the environment variable containing stable cursor-secret bytes.
  pub cursor_secret_env: Option<String>,
  /// Absolute path to a root-owned cursor-secret file unreadable by group or other users.
  pub cursor_secret_file: Option<String>,
}

/// Validated settings for an enabled knowledge runtime.
#[derive(Clone, PartialEq, Eq)]
pub struct EnabledKnowledgeRuntimeConfig {
  /// Whether knowledge dependency failure makes readiness fail.
  pub required: bool,
  /// Existing validated island-port UDS and timeout settings reused by knowledge operations.
  pub canonical: EnabledCanonicalRuntimeConfig,
  /// Stable secret loaded from the configured external source.
  cursor_secret: KnowledgeCursorSecret,
}

/// Stable secret bytes that are never formatted or serialized.
#[derive(Clone, PartialEq, Eq)]
struct KnowledgeCursorSecret(Arc<[u8]>);

impl KnowledgeCursorSecret {
  fn new(bytes: Vec<u8>) -> Result<Self, KnowledgeRuntimeConfigError> {
    if !(32..=4_096).contains(&bytes.len()) {
      return Err(KnowledgeRuntimeConfigError::InvalidSecret);
    }
    Ok(Self(Arc::from(bytes)))
  }
}

/// Closed knowledge-runtime configuration failures without secret references or values.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum KnowledgeRuntimeConfigError {
  /// Required readiness cannot be selected while the capability is disabled.
  #[error("knowledge required policy requires an enabled capability")]
  RequiredWhileDisabled,
  /// Enabled knowledge requires the validated canonical island-port dependency.
  #[error("enabled knowledge capability requires the canonical island-port dependency")]
  MissingCanonicalDependency,
  /// Exactly one external cursor-secret source must be selected.
  #[error("enabled knowledge capability requires exactly one cursor-secret reference")]
  InvalidSecretReference,
  /// The referenced environment value or file could not be loaded safely.
  #[error("knowledge cursor-secret source is unavailable")]
  SecretUnavailable,
  /// Loaded secret material did not satisfy the bounded minimum strength.
  #[error("knowledge cursor-secret material must contain between 32 and 4096 bytes")]
  InvalidSecret,
  /// A secret file was not a regular root-owned file with no group or other permissions.
  #[error("knowledge cursor-secret file must be root-owned and owner-only")]
  UnsafeSecretFile,
}

impl KnowledgeRuntimeConfig {
  /// Resolves the opt-in capability and loads one stable cursor secret from its external source.
  ///
  /// Disabled operation needs neither a canonical dependency nor a secret reference. Enabled
  /// operation reuses an already validated canonical UDS path and timeout rather than accepting a
  /// second transport configuration.
  ///
  /// # Errors
  ///
  /// Returns a closed error for contradictory policy, missing canonical dependency, ambiguous or
  /// unsafe secret references, unavailable secret material, or an invalid secret length.
  pub fn resolve(
    &self,
    canonical: Option<&EnabledCanonicalRuntimeConfig>,
  ) -> Result<Option<EnabledKnowledgeRuntimeConfig>, KnowledgeRuntimeConfigError> {
    self.resolve_with(canonical, load_environment_secret, load_secret_file)
  }

  fn resolve_with<E, F>(
    &self,
    canonical: Option<&EnabledCanonicalRuntimeConfig>,
    environment: E,
    file: F,
  ) -> Result<Option<EnabledKnowledgeRuntimeConfig>, KnowledgeRuntimeConfigError>
  where
    E: FnOnce(&str) -> Result<Vec<u8>, KnowledgeRuntimeConfigError>,
    F: FnOnce(&str) -> Result<Vec<u8>, KnowledgeRuntimeConfigError>,
  {
    if !self.enabled {
      return if self.required {
        Err(KnowledgeRuntimeConfigError::RequiredWhileDisabled)
      } else {
        Ok(None)
      };
    }
    let canonical = canonical
      .cloned()
      .ok_or(KnowledgeRuntimeConfigError::MissingCanonicalDependency)?;
    let secret = match (&self.cursor_secret_env, &self.cursor_secret_file) {
      (Some(name), None) if valid_environment_name(name) => environment(name)?,
      (None, Some(path)) if valid_secret_path(path) => file(path)?,
      _ => return Err(KnowledgeRuntimeConfigError::InvalidSecretReference),
    };
    Ok(Some(EnabledKnowledgeRuntimeConfig {
      required: self.required,
      canonical,
      cursor_secret: KnowledgeCursorSecret::new(secret)?,
    }))
  }
}

impl EnabledKnowledgeRuntimeConfig {
  /// Derives an opaque cursor-protection key without exposing the loaded secret bytes.
  ///
  /// # Errors
  ///
  /// Returns a closed error if the retained secret cannot satisfy the cursor-key contract.
  pub fn cursor_protection_key(
    &self,
  ) -> Result<KnowledgeCursorProtectionKey, KnowledgeRuntimeConfigError> {
    KnowledgeCursorProtectionKey::new(&self.cursor_secret.0)
      .map_err(|_| KnowledgeRuntimeConfigError::InvalidSecret)
  }
}

impl fmt::Debug for KnowledgeRuntimeConfig {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("KnowledgeRuntimeConfig")
      .field("enabled", &self.enabled)
      .field("required", &self.required)
      .field(
        "cursor_secret_env",
        &self.cursor_secret_env.as_ref().map(|_| "[REDACTED]"),
      )
      .field(
        "cursor_secret_file",
        &self.cursor_secret_file.as_ref().map(|_| "[REDACTED]"),
      )
      .finish()
  }
}

impl fmt::Debug for EnabledKnowledgeRuntimeConfig {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("EnabledKnowledgeRuntimeConfig")
      .field("required", &self.required)
      .field("canonical", &self.canonical)
      .field("cursor_secret", &"[REDACTED]")
      .finish()
  }
}

fn valid_environment_name(value: &str) -> bool {
  (1..=128).contains(&value.len())
    && value.bytes().enumerate().all(|(index, byte)| {
      byte.is_ascii_uppercase() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
    })
}

fn valid_secret_path(value: &str) -> bool {
  (2..=4_096).contains(&value.len())
    && value.starts_with('/')
    && !value.chars().any(char::is_whitespace)
    && !value.bytes().any(|byte| byte == 0)
    && !value.split('/').any(|component| component == "..")
}

fn load_environment_secret(name: &str) -> Result<Vec<u8>, KnowledgeRuntimeConfigError> {
  std::env::var(name)
    .map(String::into_bytes)
    .map_err(|_| KnowledgeRuntimeConfigError::SecretUnavailable)
}

#[cfg(unix)]
fn load_secret_file(path: &str) -> Result<Vec<u8>, KnowledgeRuntimeConfigError> {
  use std::{
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
  };

  let path = Path::new(path);
  validate_secret_parent(path)?;
  let file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
    .open(path)
    .map_err(|_| KnowledgeRuntimeConfigError::SecretUnavailable)?;
  let metadata = file
    .metadata()
    .map_err(|_| KnowledgeRuntimeConfigError::SecretUnavailable)?;
  if !metadata.is_file() {
    return Err(KnowledgeRuntimeConfigError::UnsafeSecretFile);
  }
  let mode = metadata.permissions().mode() & 0o777;
  if metadata.uid() != 0 || mode & 0o077 != 0 || mode & 0o400 == 0 {
    return Err(KnowledgeRuntimeConfigError::UnsafeSecretFile);
  }
  let mut bytes = Vec::new();
  file
    .take(4_098)
    .read_to_end(&mut bytes)
    .map_err(|_| KnowledgeRuntimeConfigError::SecretUnavailable)?;
  while matches!(bytes.last(), Some(b'\n' | b'\r')) {
    bytes.pop();
  }
  Ok(bytes)
}

#[cfg(unix)]
fn validate_secret_parent(path: &Path) -> Result<(), KnowledgeRuntimeConfigError> {
  use std::os::unix::fs::{MetadataExt, PermissionsExt};

  let parent = path
    .parent()
    .ok_or(KnowledgeRuntimeConfigError::UnsafeSecretFile)?;
  let metadata =
    fs::metadata(parent).map_err(|_| KnowledgeRuntimeConfigError::SecretUnavailable)?;
  let mode = metadata.permissions().mode() & 0o777;
  if !metadata.is_dir() || metadata.uid() != 0 || mode & 0o022 != 0 {
    return Err(KnowledgeRuntimeConfigError::UnsafeSecretFile);
  }
  Ok(())
}

#[cfg(not(unix))]
fn load_secret_file(_path: &str) -> Result<Vec<u8>, KnowledgeRuntimeConfigError> {
  Err(KnowledgeRuntimeConfigError::UnsafeSecretFile)
}

/// Listener and logging settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
  /// Absolute Unix socket path for the only supported listener.
  pub socket_path: String,
  /// Octal Unix socket permissions applied after binding.
  #[serde(default = "default_socket_mode")]
  pub socket_mode: String,
  /// Default tracing filter when `RUST_LOG` is unset.
  pub log_level: String,
  /// `json` for JSON logs; any other value selects compact logs.
  pub log_format: String,
}

fn default_socket_mode() -> String {
  "0660".to_string()
}

/// HTTP request-size and browser-origin settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpConfig {
  /// Maximum accepted request body size in bytes.
  pub max_request_body_bytes: usize,
}

impl Default for HttpConfig {
  fn default() -> Self {
    Self {
      max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
    }
  }
}

impl HttpConfig {
  /// Validates the request-size policy.
  ///
  /// # Errors
  ///
  /// Returns an error when the body limit is zero.
  pub fn validate(&self) -> Result<(), HttpConfigError> {
    if self.max_request_body_bytes == 0 {
      return Err(HttpConfigError::ZeroRequestBodyLimit);
    }
    Ok(())
  }
}

/// Invalid HTTP boundary configuration.
#[derive(Debug, Error)]
pub enum HttpConfigError {
  /// The configured body limit would reject every nonempty request.
  #[error("max_request_body_bytes must be greater than zero")]
  ZeroRequestBodyLimit,
}

/// Translation routing and provider-call settings.
#[derive(Debug, Clone, Deserialize)]
pub struct TranslationConfig {
  /// Maximum Unicode character count routed to Gemma 4.
  pub long_text_chars: usize,
  /// Timeout for one provider request.
  pub timeout_seconds: u64,
  /// Retry count after the initial provider request.
  pub max_retries: u32,
  /// Delay between provider attempts.
  pub retry_delay_ms: u64,
}

/// Per-provider resilience policy overrides.
///
/// Omitted timeout and retry fields inherit the matching value from [`TranslationConfig`]. The
/// remaining fields have safe defaults so existing deployments can opt in without a configuration
/// migration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ProviderResilienceConfigs {
  /// Policy for the short-text Gemma 4 provider and structured lexical-card requests.
  pub gemma4: ProviderResilienceConfig,
  /// Policy for the long-text TranslateGemma provider.
  pub translate_gemma: ProviderResilienceConfig,
}

/// Configurable resilience bounds for one outbound provider.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProviderResilienceConfig {
  /// Optional per-attempt deadline override in seconds.
  pub timeout_seconds: Option<u64>,
  /// Optional retry-count override after the first request.
  pub max_retries: Option<u32>,
  /// Optional retry-delay override in milliseconds when no `Retry-After` is supplied.
  pub retry_delay_ms: Option<u64>,
  /// Largest accepted retry delay in milliseconds, including a provider `Retry-After` value.
  pub max_retry_delay_ms: u64,
  /// Maximum concurrent HTTP attempts allowed for this provider.
  pub max_concurrent_requests: usize,
  /// Consecutive transient logical-call failures that open this provider's circuit.
  pub circuit_failure_threshold: u32,
  /// Time in milliseconds that an opened circuit rejects calls before one half-open probe.
  pub circuit_open_ms: u64,
}

impl Default for ProviderResilienceConfig {
  fn default() -> Self {
    Self {
      timeout_seconds: None,
      max_retries: None,
      retry_delay_ms: None,
      max_retry_delay_ms: 5_000,
      max_concurrent_requests: 8,
      circuit_failure_threshold: 5,
      circuit_open_ms: 30_000,
    }
  }
}

impl ProviderResilienceConfig {
  /// Resolves explicit overrides and legacy translation defaults into one validated provider policy.
  ///
  /// # Errors
  ///
  /// Returns an error when a resolved timeout, concurrency bound, or circuit setting is invalid.
  pub fn resolve(
    &self,
    translation: &TranslationConfig,
  ) -> Result<ProviderPolicy, ProviderPolicyError> {
    ProviderPolicy::new(
      Duration::from_secs(self.timeout_seconds.unwrap_or(translation.timeout_seconds)),
      self.max_retries.unwrap_or(translation.max_retries),
      Duration::from_millis(self.retry_delay_ms.unwrap_or(translation.retry_delay_ms)),
      Duration::from_millis(self.max_retry_delay_ms),
      self.max_concurrent_requests,
      self.circuit_failure_threshold,
      Duration::from_millis(self.circuit_open_ms),
    )
  }
}

/// One OpenAI-compatible provider endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
  /// Base URL containing the provider's `/v1` API root.
  pub base_url: String,
  /// Model identifier sent in requests.
  pub model: String,
  /// Redacted bearer credential sent only to the provider.
  pub api_key: ProviderApiKey,
}

/// A bearer credential retained exclusively for outbound provider authentication.
///
/// This type deliberately does not implement `Display` or `Serialize`. Its `Debug` output is
/// always redacted, including when it is nested in [`ProviderConfig`] or [`AppConfig`].
#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct ProviderApiKey(String);

impl ProviderApiKey {
  /// Creates a provider credential for programmatic configuration.
  pub fn new(value: impl Into<String>) -> Self {
    Self(value.into())
  }

  /// Returns the credential only for crate-local outbound bearer authentication.
  pub(crate) fn bearer_token(&self) -> &str {
    &self.0
  }
}

impl From<String> for ProviderApiKey {
  fn from(value: String) -> Self {
    Self::new(value)
  }
}

impl From<&str> for ProviderApiKey {
  fn from(value: &str) -> Self {
    Self::new(value)
  }
}

impl fmt::Debug for ProviderApiKey {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("ProviderApiKey([REDACTED])")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn provider_resilience_overrides_only_the_selected_provider_values() {
    let translation = TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 60,
      max_retries: 3,
      retry_delay_ms: 250,
    };
    let config = ProviderResilienceConfig {
      timeout_seconds: Some(5),
      max_retries: None,
      retry_delay_ms: Some(10),
      max_retry_delay_ms: 500,
      max_concurrent_requests: 2,
      circuit_failure_threshold: 3,
      circuit_open_ms: 1_000,
    };

    assert_eq!(
      config.resolve(&translation).unwrap(),
      ProviderPolicy::new(
        Duration::from_secs(5),
        3,
        Duration::from_millis(10),
        Duration::from_millis(500),
        2,
        3,
        Duration::from_secs(1),
      )
      .unwrap()
    );
  }

  #[test]
  fn provider_resilience_rejects_zero_critical_bounds() {
    let translation = TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 60,
      max_retries: 0,
      retry_delay_ms: 0,
    };
    let config = ProviderResilienceConfig {
      max_concurrent_requests: 0,
      ..ProviderResilienceConfig::default()
    };

    assert_eq!(
      config.resolve(&translation),
      Err(ProviderPolicyError::ZeroConcurrency)
    );
  }

  #[test]
  fn provider_api_keys_deserialize_without_debug_exposure() {
    let config: AppConfig = toml::from_str(
      r#"
[server]
socket_path = "/run/transnet/transnet.sock"
socket_mode = "0660"
log_level = "info"
log_format = "compact"

[translation]
long_text_chars = 4000
timeout_seconds = 2
max_retries = 0
retry_delay_ms = 0

[gemma4]
base_url = "http://127.0.0.1:18011/v1"
model = "Gemma4"
api_key = "GEMMA4_CREDENTIAL_SECRET"

[translate_gemma]
base_url = "http://127.0.0.1:18007/v1"
model = "TranslateGemma"
api_key = "TRANSLATE_GEMMA_CREDENTIAL_SECRET"
"#,
    )
    .unwrap();

    let app_debug = format!("{config:?}");
    let provider_debug = format!("{:?}", config.gemma4);
    for credential in [
      "GEMMA4_CREDENTIAL_SECRET",
      "TRANSLATE_GEMMA_CREDENTIAL_SECRET",
    ] {
      assert!(!app_debug.contains(credential));
      assert!(!provider_debug.contains(credential));
    }
    assert!(app_debug.contains("ProviderApiKey([REDACTED])"));
  }

  #[test]
  fn disabled_knowledge_needs_no_dependency_or_secret() {
    let config = KnowledgeRuntimeConfig::default();
    assert!(config
      .resolve_with(None, |_| unreachable!(), |_| unreachable!())
      .unwrap()
      .is_none());

    let contradictory = KnowledgeRuntimeConfig {
      required: true,
      ..KnowledgeRuntimeConfig::default()
    };
    assert_eq!(
      contradictory.resolve_with(None, |_| unreachable!(), |_| unreachable!()),
      Err(KnowledgeRuntimeConfigError::RequiredWhileDisabled)
    );
  }

  #[test]
  fn enabled_knowledge_reuses_validated_canonical_transport() {
    let canonical = enabled_canonical();
    let config = KnowledgeRuntimeConfig {
      enabled: true,
      required: true,
      cursor_secret_env: Some("TRANSNET_CURSOR_SECRET".into()),
      cursor_secret_file: None,
    };
    let enabled = config
      .resolve_with(
        Some(&canonical),
        |name| {
          assert_eq!(name, "TRANSNET_CURSOR_SECRET");
          Ok(vec![7; 32])
        },
        |_| unreachable!(),
      )
      .unwrap()
      .unwrap();

    assert!(enabled.required);
    assert_eq!(enabled.canonical, canonical);
    let key = enabled.cursor_protection_key().unwrap();
    assert_eq!(format!("{key:?}"), "KnowledgeCursorProtectionKey(REDACTED)");
  }

  #[test]
  fn enabled_knowledge_requires_one_safe_external_reference() {
    let canonical = enabled_canonical();
    for config in [
      KnowledgeRuntimeConfig {
        enabled: true,
        ..KnowledgeRuntimeConfig::default()
      },
      KnowledgeRuntimeConfig {
        enabled: true,
        cursor_secret_env: Some("CURSOR_SECRET".into()),
        cursor_secret_file: Some("/run/secrets/transnet-cursor".into()),
        ..KnowledgeRuntimeConfig::default()
      },
      KnowledgeRuntimeConfig {
        enabled: true,
        cursor_secret_env: Some("unsafe-name".into()),
        ..KnowledgeRuntimeConfig::default()
      },
      KnowledgeRuntimeConfig {
        enabled: true,
        cursor_secret_file: Some("relative/secret".into()),
        ..KnowledgeRuntimeConfig::default()
      },
    ] {
      assert_eq!(
        config.resolve_with(Some(&canonical), |_| unreachable!(), |_| unreachable!()),
        Err(KnowledgeRuntimeConfigError::InvalidSecretReference)
      );
    }
    let missing_dependency = KnowledgeRuntimeConfig {
      enabled: true,
      cursor_secret_env: Some("CURSOR_SECRET".into()),
      ..KnowledgeRuntimeConfig::default()
    };
    assert_eq!(
      missing_dependency.resolve_with(None, |_| Ok(vec![0; 32]), |_| unreachable!()),
      Err(KnowledgeRuntimeConfigError::MissingCanonicalDependency)
    );
  }

  #[test]
  fn cursor_secret_strength_and_file_policy_fail_closed() {
    let canonical = enabled_canonical();
    let environment = KnowledgeRuntimeConfig {
      enabled: true,
      cursor_secret_env: Some("CURSOR_SECRET".into()),
      ..KnowledgeRuntimeConfig::default()
    };
    assert_eq!(
      environment.resolve_with(Some(&canonical), |_| Ok(vec![1; 31]), |_| unreachable!()),
      Err(KnowledgeRuntimeConfigError::InvalidSecret)
    );

    let file = KnowledgeRuntimeConfig {
      enabled: true,
      cursor_secret_file: Some("/run/secrets/transnet-cursor".into()),
      ..KnowledgeRuntimeConfig::default()
    };
    assert_eq!(
      file.resolve_with(
        Some(&canonical),
        |_| unreachable!(),
        |_| Err(KnowledgeRuntimeConfigError::UnsafeSecretFile),
      ),
      Err(KnowledgeRuntimeConfigError::UnsafeSecretFile)
    );
  }

  #[test]
  fn knowledge_debug_and_errors_never_expose_references_or_values() {
    let config = KnowledgeRuntimeConfig {
      enabled: true,
      required: false,
      cursor_secret_env: Some("PRIVATE_CURSOR_ENV".into()),
      cursor_secret_file: None,
    };
    let enabled = config
      .resolve_with(
        Some(&enabled_canonical()),
        |_| Ok(b"PRIVATE_CURSOR_SECRET_VALUE_123456".to_vec()),
        |_| unreachable!(),
      )
      .unwrap()
      .unwrap();
    let diagnostics = format!("{config:?} {enabled:?}");
    assert!(!diagnostics.contains("PRIVATE_CURSOR_ENV"));
    assert!(!diagnostics.contains("PRIVATE_CURSOR_SECRET_VALUE"));
    assert!(diagnostics.contains("[REDACTED]"));
    assert!(!KnowledgeRuntimeConfigError::SecretUnavailable
      .to_string()
      .contains("PRIVATE"));
  }

  #[test]
  fn knowledge_configuration_rejects_inline_or_unknown_fields() {
    let result = toml::from_str::<KnowledgeRuntimeConfig>(
      r#"
enabled = true
required = false
cursor_secret = "INLINE_SECRET_MUST_NEVER_BE_ACCEPTED"
"#,
    );

    let error = result.unwrap_err().to_string();
    assert!(error.contains("unknown field"));
  }

  #[test]
  fn application_config_parser_never_echoes_rejected_inline_secrets() {
    let source = include_str!("../config/transnet.toml").replace(
      "required = false",
      "required = false\ncursor_secret = \"INLINE_SECRET_MUST_NEVER_BE_ECHOED\"",
    );

    let error = AppConfig::parse_toml(&source).unwrap_err();
    let diagnostics = format!("{error} {error:?}");
    assert_eq!(error, AppConfigLoadError::Invalid);
    assert!(!diagnostics.contains("INLINE_SECRET_MUST_NEVER_BE_ECHOED"));
    assert!(!diagnostics.contains("cursor_secret"));
  }

  #[test]
  fn listener_and_http_configuration_reject_transitional_keys() {
    let server = toml::from_str::<ServerConfig>(
      r#"socket_path = "/run/transnet/transnet.sock"
socket_mode = "0660"
log_level = "info"
log_format = "compact"
host = "127.0.0.1"
port = 16002"#,
    );
    assert!(server.unwrap_err().to_string().contains("unknown field"));

    let http = toml::from_str::<HttpConfig>(
      r#"max_request_body_bytes = 1048576
allowed_origins = ["http://localhost"]
allow_credentials = false"#,
    );
    assert!(http.unwrap_err().to_string().contains("unknown field"));
  }

  fn enabled_canonical() -> EnabledCanonicalRuntimeConfig {
    EnabledCanonicalRuntimeConfig {
      socket_path: "/run/island-port/island-port.sock".into(),
      timeout: Duration::from_secs(2),
    }
  }
}
