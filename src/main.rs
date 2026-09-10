//! Transnet process entry point.

use std::{fs, path::Path, sync::Arc};

use anyhow::{Context, Result};
use transnet::{
  app_router_with_http_config, AppConfig, AppState, OpenAiLearningModel, TranslationService,
};

#[tokio::main]
async fn main() -> Result<()> {
  let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/transnet.toml");
  let config: AppConfig = toml::from_str(
    &fs::read_to_string(&config_path)
      .with_context(|| format!("failed to read {}", config_path.display()))?,
  )
  .with_context(|| format!("failed to parse {}", config_path.display()))?;

  init_tracing(&config.server.log_level, &config.server.log_format)?;
  let address = format!("{}:{}", config.server.host, config.server.port);
  let gemma4_policy = config
    .provider_resilience
    .gemma4
    .resolve(&config.translation)
    .context("invalid Gemma 4 provider resilience policy")?;
  let translate_gemma_policy = config
    .provider_resilience
    .translate_gemma
    .resolve(&config.translation)
    .context("invalid TranslateGemma provider resilience policy")?;
  let learning_model =
    OpenAiLearningModel::with_provider_policy(config.gemma4.clone(), gemma4_policy.clone())?;
  let service = TranslationService::with_provider_policies(
    config.translation,
    config.gemma4,
    gemma4_policy,
    config.translate_gemma,
    translate_gemma_policy,
  )?;
  let listener = tokio::net::TcpListener::bind(&address)
    .await
    .with_context(|| format!("failed to bind to {address}"))?;

  tracing::info!(address = %address, "starting transnet");
  let router = app_router_with_http_config(
    AppState::new(service).with_learning_model(Arc::new(learning_model)),
    &config.http,
  )
  .context("invalid HTTP configuration")?;
  axum::serve(listener, router)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("transnet server failed")?;
  Ok(())
}

fn init_tracing(level: &str, format: &str) -> Result<()> {
  let filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
  let subscriber = tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_target(false)
    .with_writer(std::io::stdout);

  if format.eq_ignore_ascii_case("json") {
    subscriber
      .json()
      .try_init()
      .map_err(|error| anyhow::anyhow!(error.to_string()))?;
  } else {
    subscriber
      .compact()
      .try_init()
      .map_err(|error| anyhow::anyhow!(error.to_string()))?;
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
}
