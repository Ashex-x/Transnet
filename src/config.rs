//! Runtime configuration for the server and model providers.

use serde::Deserialize;

/// Complete application configuration loaded from `config/transnet.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
  /// Listener and logging settings.
  pub server: ServerConfig,
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
