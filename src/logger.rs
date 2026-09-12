//! Process-wide structured file logging.

use std::{fs::File, path::Path, sync::OnceLock};

use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Failure to initialize the process-wide tracing subscriber.
#[derive(Debug, Error)]
pub enum LoggerError {
  /// A tracing subscriber or log guard is already installed.
  #[error("logger already initialized for this process")]
  AlreadyInitialized,
  /// The configured fallback tracing filter is invalid.
  #[error("invalid default log filter: {0}")]
  InvalidFilter(#[from] tracing_subscriber::filter::ParseError),
  /// The log directory or file could not be created.
  #[error("failed to create log output: {0}")]
  Io(#[from] std::io::Error),
}

/// Initializes the global tracing subscriber and writes to the build-mode Transnet log.
///
/// Debug builds write `logs/debug/transnet.log`; release builds write
/// `logs/release/transnet.log`. The file is replaced at process startup and written through a
/// non-blocking worker. `RUST_LOG` overrides `default_filter`. A case-insensitive `json` format
/// selects newline-delimited JSON; every other value selects compact text.
///
/// # Errors
///
/// Returns an error if logging was already initialized, the default filter is invalid, or the log
/// output cannot be created.
pub fn init(default_filter: &str, format: &str) -> Result<(), LoggerError> {
  if LOG_GUARD.get().is_some() {
    return Err(LoggerError::AlreadyInitialized);
  }

  let mode = if cfg!(debug_assertions) {
    "debug"
  } else {
    "release"
  };
  let log_dir = Path::new("logs").join(mode);
  std::fs::create_dir_all(&log_dir)?;
  let file = File::create(log_dir.join("transnet.log"))?;
  let (writer, guard) = tracing_appender::non_blocking(file);
  let fallback = EnvFilter::try_new(default_filter)?;
  let filter = EnvFilter::try_from_default_env().unwrap_or(fallback);
  let subscriber = tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_target(true)
    .with_ansi(false)
    .with_writer(writer);

  let result = if format.eq_ignore_ascii_case("json") {
    subscriber.json().try_init()
  } else {
    subscriber.compact().try_init()
  };
  result.map_err(|_| LoggerError::AlreadyInitialized)?;
  LOG_GUARD
    .set(guard)
    .map_err(|_| LoggerError::AlreadyInitialized)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use tracing_subscriber::EnvFilter;

  #[test]
  fn invalid_default_filter_is_rejected() {
    assert!(EnvFilter::try_new("%%%invalid%%%").is_err());
  }
}
