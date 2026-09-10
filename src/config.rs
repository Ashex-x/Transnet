//! Runtime configuration for the server, HTTP boundary, and model providers.

use axum::http::{HeaderValue, Uri};
use serde::Deserialize;
use thiserror::Error;

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

/// One OpenAI-compatible provider endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
  /// Base URL containing the provider's `/v1` API root.
  pub base_url: String,
  /// Model identifier sent in requests.
  pub model: String,
  /// Bearer credential sent to the provider.
  pub api_key: String,
}
