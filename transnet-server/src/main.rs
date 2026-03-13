use std::{env, path::Path};

use anyhow::{Context, Result};
use tower_http::trace::TraceLayer;

use transnet_server::{create_router, AppState, ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
  // Load .env from the project root (parent directory)
  let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
  let project_root = Path::new(&manifest_dir)
    .parent()
    .context("failed to find project root")?;
  let env_path = project_root.join(".env");
  let _ = dotenvy::from_path(&env_path);

  let config: ServerConfig = ServerConfig::from_env().context("failed to load server config")?;

  init_tracing(&config.log_level)?;

  let address = format!("{}:{}", config.host, config.port);

  let backend_host = env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
  let backend_port = env::var("BACKEND_PORT").unwrap_or_else(|_| "35792".to_string());
  let backend_url = format!("http://{}:{}", backend_host, backend_port);
  let app_state = AppState::new(backend_url.clone());

  tracing::info!(
    address = %address,
    backend_url = %backend_url,
    workers = config.workers,
    "starting transnet gateway server"
  );

  let app = create_router()
    .layer(TraceLayer::new_for_http())
    .with_state(app_state);

  let listener = tokio::net::TcpListener::bind(&address)
    .await
    .with_context(|| format!("failed to bind to {address}"))?;

  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("axum server exited with error")?;

  Ok(())
}

fn init_tracing(level: &str) -> Result<()> {
  let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

  let _subscriber = tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_target(false)
    .with_writer(std::io::stdout)
    .compact()
    .init();

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
