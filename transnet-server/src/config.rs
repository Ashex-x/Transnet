//! Environment-backed gateway configuration.

use anyhow::{ensure, Context, Result};
use serde::Deserialize;

/// Network and runtime settings for the HTTP gateway.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
  /// Interface or host name on which the gateway listens.
  pub host: String,
  /// TCP port on which the gateway listens.
  pub port: u16,
  /// Requested Tokio worker count, retained for deployment configuration compatibility.
  pub workers: usize,
  /// Default tracing filter when `RUST_LOG` does not contain a valid filter.
  pub log_level: String,
}

impl ServerConfig {
  /// Loads configuration from process environment variables with local defaults.
  ///
  /// # Errors
  ///
  /// Returns an error when `GATEWAY_PORT` or `WORKERS` is not a valid integer.
  pub fn from_env() -> Result<Self> {
    Self::from_lookup(|key| std::env::var(key).ok())
  }

  fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self> {
    let host = lookup("GATEWAY_HOST").unwrap_or_else(|| "0.0.0.0".to_string());
    let port = lookup("GATEWAY_PORT")
      .unwrap_or_else(|| "8080".to_string())
      .parse()
      .context("GATEWAY_PORT must be an integer between 0 and 65535")?;
    let workers: usize = lookup("WORKERS")
      .unwrap_or_else(|| "4".to_string())
      .parse()
      .context("WORKERS must be a positive integer")?;
    ensure!(workers > 0, "WORKERS must be greater than zero");
    let log_level = lookup("RUST_LOG").unwrap_or_else(|| "info".to_string());

    Ok(Self {
      host,
      port,
      workers,
      log_level,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn test_from_lookup_with_no_values_uses_defaults() {
    let config = ServerConfig::from_lookup(|_| None).expect("defaults should be valid");
    assert_eq!(
      config,
      ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        workers: 4,
        log_level: "info".to_string(),
      }
    );
  }

  #[test]
  fn test_from_lookup_with_invalid_port_returns_contextual_error() {
    let values = HashMap::from([("GATEWAY_PORT", "invalid")]);
    let error = ServerConfig::from_lookup(|key| values.get(key).map(ToString::to_string))
      .expect_err("invalid port should fail");
    assert!(error.to_string().contains("GATEWAY_PORT"));
  }

  #[test]
  fn test_from_lookup_with_zero_workers_returns_contextual_error() {
    let values = HashMap::from([("WORKERS", "0")]);
    let error = ServerConfig::from_lookup(|key| values.get(key).map(ToString::to_string))
      .expect_err("zero workers should fail");
    assert!(error.to_string().contains("WORKERS"));
  }
}
