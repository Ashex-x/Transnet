use std::{fs, path::Path};

use anyhow::{Context, Result};
use transnet::api::{app_router, AppState};
use transnet::llm::TranslationService;
use transnet::types::{LlmFileConfig, ServerFileConfig};

#[tokio::main]
async fn main() -> Result<()> {
  let server_config: ServerFileConfig = read_config("config/transnet.toml")?;
  let llm_config: LlmFileConfig = read_config("config/transnet_llm.toml")?;

  init_tracing(&server_config.logging.level, &server_config.logging.format)?;

  let address = format!(
    "{}:{}",
    server_config.server.host, server_config.server.port
  );

  let translation_service = TranslationService::new(llm_config.openai)
    .context("failed to initialize translation service")?;
  let app = app_router(AppState::new(translation_service));

  let listener = tokio::net::TcpListener::bind(&address)
    .await
    .with_context(|| format!("failed to bind to {address}"))?;

  tracing::info!(
    address = %address,
    workers = server_config.server.workers,
    "starting transnet server"
  );

  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("axum server exited with error")?;

  Ok(())
}

fn read_config<T>(path: &str) -> Result<T>
where
  T: serde::de::DeserializeOwned,
{
  let content = fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
  toml::from_str(&content).with_context(|| format!("failed to parse {path}"))
}

fn init_tracing(level: &str, format: &str) -> Result<()> {
  let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

  let is_json = format.eq_ignore_ascii_case("json");
  let subscriber = tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_target(false)
    .with_writer(std::io::stdout);

  if is_json {
    subscriber.json().init();
  } else {
    subscriber.compact().init();
  }

  Ok(())
}

async fn shutdown_signal() {
  let ctrl_c = async {
    let _ = tokio::signal::ctrl_c().await;
  };

  #[cfg(unix)]
  let terminate = async {
    if let Ok(mut signal) =
      tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
      signal.recv().await;
    }
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    _ = ctrl_c => {}
    _ = terminate => {}
  }

  tracing::info!("shutdown signal received");
}

#[allow(dead_code)]
fn config_exists(path: &str) -> bool {
  Path::new(path).exists()
}
