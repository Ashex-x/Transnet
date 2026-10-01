//! Transnet process entry point.

use std::{fs, net::IpAddr, path::Path, sync::Arc};

use anyhow::{ensure, Context, Result};
use transnet::{
  app_router_with_http_config, application::translation::TranslationOrchestrator, logger,
  AppConfig, AppState, OpenAiLearningModel, TranslationService,
};

#[cfg(unix)]
use transnet::{
  adapters::island_port::{IslandPortCanonicalClient, UnixIslandPortTransport},
  api::CanonicalDependencyReadiness,
  application::canonical_read::CanonicalReadService,
  ports::canonical_read::CanonicalReadPort,
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
  let canonical = config
    .canonical
    .resolve()
    .context("invalid canonical runtime configuration")?;
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
  let orchestrator =
    TranslationOrchestrator::new(Arc::new(service.clone()), Arc::new(learning_model.clone()));
  let state = AppState::new(service)
    .with_learning_model(Arc::new(learning_model))
    .with_translation_orchestrator(Arc::new(orchestrator));
  let state = match canonical {
    Some(canonical) => configure_canonical(state, canonical)?,
    None => state,
  };
  let router =
    app_router_with_http_config(state, &config.http).context("invalid HTTP configuration")?;
  if let Some(socket_path) = config.server.socket_path.as_deref() {
    #[cfg(unix)]
    {
      let socket =
        transnet::server::UnixListenerConfig::new(socket_path, &config.server.socket_mode)?;
      let listener = transnet::server::OwnedUnixListener::bind(&socket).await?;
      tracing::info!("starting transnet Unix listener");
      listener.serve(router, shutdown_signal()).await?;
    }
    #[cfg(not(unix))]
    anyhow::bail!("server.socket_path requires Unix domain socket support");
  } else {
    let listener = tokio::net::TcpListener::bind(address)
      .await
      .with_context(|| format!("failed to bind to {address}"))?;
    tracing::info!(address = %address, "starting transitional transnet TCP listener");
    axum::serve(listener, router)
      .with_graceful_shutdown(shutdown_signal())
      .await
      .context("transnet server failed")?;
  }
  Ok(())
}

#[cfg(unix)]
fn configure_canonical(
  state: AppState,
  canonical: transnet::config::EnabledCanonicalRuntimeConfig,
) -> Result<AppState> {
  let transport = Arc::new(
    UnixIslandPortTransport::new(&canonical.socket_path)
      .context("could not construct canonical island-port transport")?,
  );
  let authority: Arc<dyn CanonicalReadPort> = Arc::new(IslandPortCanonicalClient::new(transport));
  Ok(
    state
      .with_canonical_read_service_timeout(
        Arc::new(CanonicalReadService::new(authority.clone())),
        canonical.timeout,
      )
      .with_readiness(Arc::new(CanonicalDependencyReadiness::new(
        authority,
        canonical.timeout,
      ))),
  )
}

#[cfg(not(unix))]
fn configure_canonical(
  _state: AppState,
  _canonical: transnet::config::EnabledCanonicalRuntimeConfig,
) -> Result<AppState> {
  anyhow::bail!("enabled canonical capability requires Unix domain sockets")
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
