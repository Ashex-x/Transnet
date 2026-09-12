//! Transnet process entry point.

use std::{fs, net::IpAddr, path::Path, sync::Arc};

use anyhow::{ensure, Context, Result};
use transnet::{
  app_router_with_http_config, logger, AppConfig, AppState, OpenAiLearningModel, TranslationService,
};

#[tokio::main]
async fn main() -> Result<()> {
  let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/transnet.toml");
  let config: AppConfig = toml::from_str(
    &fs::read_to_string(&config_path)
      .with_context(|| format!("failed to read {}", config_path.display()))?,
  )
  .with_context(|| format!("failed to parse {}", config_path.display()))?;

  logger::init(&config.server.log_level, &config.server.log_format)?;
  if let Err(error) = run(config).await {
    tracing::error!(error = %error, "transnet stopped with an error");
    return Err(error);
  }
  tracing::info!("transnet stopped");
  Ok(())
}

async fn run(config: AppConfig) -> Result<()> {
  let host = config
    .server
    .host
    .parse::<IpAddr>()
    .context("server.host must be a loopback IP address")?;
  ensure!(
    host.is_loopback(),
    "server.host must be loopback; public exposure belongs to Island-port"
  );
  let address = std::net::SocketAddr::from((host, config.server.port));
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
  let listener = tokio::net::TcpListener::bind(address)
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
    _ = ctrl_c => tracing::info!(signal = "ctrl_c", "graceful shutdown requested"),
    _ = terminate => tracing::info!(signal = "terminate", "graceful shutdown requested"),
  }
}
