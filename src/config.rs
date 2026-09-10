//! Runtime configuration for the server, HTTP boundary, model providers, and resilience policy.

use std::{fmt, time::Duration};

use axum::http::{HeaderValue, Uri};
use serde::Deserialize;
use thiserror::Error;

use crate::resilience::{ProviderPolicy, ProviderPolicyError};

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
}

/// Listener and logging settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  /// Listener host.
  pub host: String,
  /// Listener port.
  pub port: u16,
  /// Default tracing filter when `RUST_LOG` is unset.
  pub log_level: String,
  /// `json` for JSON logs; any other value selects compact logs.
  pub log_format: String,
}

/// HTTP request-size and browser-origin settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
  /// Maximum accepted request body size in bytes.
  pub max_request_body_bytes: usize,
  /// Exact browser origins allowed to make cross-origin requests.
  pub allowed_origins: Vec<String>,
  /// Whether allowed browser origins may include credentials.
  pub allow_credentials: bool,
}

impl Default for HttpConfig {
  fn default() -> Self {
    Self {
      max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
      allowed_origins: Vec::new(),
      allow_credentials: false,
    }
  }
}

impl HttpConfig {
  /// Validates the request-size and exact-origin policy.
  ///
  /// # Errors
  ///
  /// Returns an error when the body limit is zero or an origin is not a single, explicit origin.
  pub fn validate(&self) -> Result<(), HttpConfigError> {
    if self.max_request_body_bytes == 0 {
      return Err(HttpConfigError::ZeroRequestBodyLimit);
    }

    for origin in &self.allowed_origins {
      validate_origin(origin)?;
    }
    Ok(())
  }

  pub(crate) fn origin_header_values(&self) -> Result<Vec<HeaderValue>, HttpConfigError> {
    self.validate()?;
    self
      .allowed_origins
      .iter()
      .map(|origin| {
        HeaderValue::from_str(origin)
          .map_err(|_| HttpConfigError::InvalidOrigin(origin.to_string()))
      })
      .collect()
  }
}

/// Invalid HTTP boundary configuration.
#[derive(Debug, Error)]
pub enum HttpConfigError {
  /// The configured body limit would reject every nonempty request.
  #[error("max_request_body_bytes must be greater than zero")]
  ZeroRequestBodyLimit,
  /// An origin is wildcarded, malformed, or includes a path, query, or fragment.
  #[error("allowed origin `{0}` must be one exact http or https origin without a path")]
  InvalidOrigin(String),
}

fn validate_origin(origin: &str) -> Result<(), HttpConfigError> {
  if origin == "*" || origin.ends_with('/') {
    return Err(HttpConfigError::InvalidOrigin(origin.to_string()));
  }

  let uri = origin
    .parse::<Uri>()
    .map_err(|_| HttpConfigError::InvalidOrigin(origin.to_string()))?;
  if !matches!(uri.scheme_str(), Some("http" | "https"))
    || uri.authority().is_none()
    || uri
      .authority()
      .is_some_and(|authority| authority.as_str().contains('@'))
    || uri
      .path_and_query()
      .is_some_and(|path_and_query| path_and_query.as_str() != "/")
  {
    return Err(HttpConfigError::InvalidOrigin(origin.to_string()));
  }

  HeaderValue::from_str(origin).map_err(|_| HttpConfigError::InvalidOrigin(origin.to_string()))?;
  Ok(())
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
  /// Policy for the short-text Gemma 4 provider and structured learning-card requests.
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
host = "127.0.0.1"
port = 3000
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
}
