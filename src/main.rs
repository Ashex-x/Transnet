//! Transnet process entry point.

use std::{net::IpAddr, path::Path, sync::Arc};

use anyhow::{ensure, Context, Result};
#[cfg(unix)]
use async_trait::async_trait;
use transnet::{
  app_router_with_http_config, application::translation::TranslationOrchestrator, logger,
  AppConfig, AppState, OpenAiGenerationAdapter, OpenAiLearningModel, TranslationService,
};

#[cfg(unix)]
use transnet::{
  adapters::{
    island_port::{IslandPortCanonicalClient, IslandPortTransport, UnixIslandPortTransport},
    island_port_active_knowledge_release::IslandPortActiveKnowledgeReleaseClient,
    island_port_retrieval::IslandPortRetrievalClient,
  },
  api::{CanonicalDependencyReadiness, KnowledgeRouteDependencies},
  application::{
    canonical_read::CanonicalReadService, knowledge_paths::BoundedKnowledgePathService,
    knowledge_views::KnowledgeViewService,
  },
  domain::{
    capabilities::{KnowledgeCapabilityBundle, ServiceCapabilities},
    knowledge_cursor::KnowledgeCursorCodec,
  },
  ports::{
    active_knowledge_release::ActiveKnowledgeReleasePort,
    canonical_read::{CanonicalReadContext, CanonicalReadPort},
    retrieval_data::RetrievalDataPort,
  },
  CompositeKnowledgeReadiness, EnabledCanonicalRuntimeConfig, EnabledKnowledgeRuntimeConfig,
  KnowledgeReadinessComponents, Readiness, ReadinessComponentState, ReadinessReport,
};

#[tokio::main]
async fn main() -> Result<()> {
  let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/transnet.toml");
  let config = AppConfig::load(&config_path).context("failed to load application configuration")?;

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
  let knowledge = config
    .knowledge
    .resolve(canonical.as_ref())
    .context("invalid knowledge runtime configuration")?;
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
    TranslationOrchestrator::new(Arc::new(OpenAiGenerationAdapter::new(service.clone())));
  let state = AppState::new(service)
    .with_learning_model(Arc::new(learning_model))
    .with_translation_orchestrator(Arc::new(orchestrator));
  let state = match canonical {
    Some(canonical) => {
      configure_canonical(
        state,
        canonical,
        knowledge,
        config.http.max_request_body_bytes,
      )
      .await?
    }
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
async fn configure_canonical(
  state: AppState,
  canonical: EnabledCanonicalRuntimeConfig,
  knowledge: Option<EnabledKnowledgeRuntimeConfig>,
  max_request_body_bytes: usize,
) -> Result<AppState> {
  let transport = Arc::new(
    UnixIslandPortTransport::new(&canonical.socket_path)
      .context("could not construct canonical island-port transport")?,
  );
  let shared_transport: Arc<dyn IslandPortTransport> = transport;
  let authority: Arc<dyn CanonicalReadPort> =
    Arc::new(IslandPortCanonicalClient::new(shared_transport.clone()));
  let canonical_readiness: Arc<dyn Readiness> = Arc::new(CanonicalDependencyReadiness::new(
    authority.clone(),
    canonical.timeout,
  ));
  let state = state.with_canonical_read_service_timeout(
    Arc::new(CanonicalReadService::new(authority.clone())),
    canonical.timeout,
  );
  let Some(knowledge) = knowledge else {
    return Ok(state.with_readiness(canonical_readiness));
  };

  let active_authority: Arc<dyn ActiveKnowledgeReleasePort> = Arc::new(
    IslandPortActiveKnowledgeReleaseClient::new(shared_transport.clone()),
  );
  let execution =
    match select_active_execution(&active_authority, knowledge.canonical.timeout).await {
      Ok(execution) => execution,
      Err(error) if knowledge.required => return Err(error),
      Err(_) => {
        return Ok(
          state.with_readiness(Arc::new(RuntimeKnowledgeReadiness::unavailable(
            canonical_readiness,
          ))),
        )
      }
    };
  let (state, knowledge_readiness) = configure_knowledge(
    state,
    authority,
    shared_transport,
    active_authority,
    execution,
    &knowledge,
    max_request_body_bytes,
  )?;
  Ok(
    state.with_readiness(Arc::new(RuntimeKnowledgeReadiness::configured(
      canonical_readiness,
      knowledge_readiness,
      knowledge.required,
    ))),
  )
}

#[cfg(unix)]
async fn select_active_execution(
  active_authority: &Arc<dyn ActiveKnowledgeReleasePort>,
  timeout: std::time::Duration,
) -> Result<transnet::domain::retrieval_data::NeighborProjectionExecutionExpectation> {
  let execution = active_authority
    .active_knowledge_release(&startup_canonical_context(timeout)?)
    .await
    .context("active knowledge release selection failed")?
    .context("enabled knowledge runtime requires one complete active release trio")?;
  execution
    .validate()
    .context("active knowledge release tuple is incompatible")?;
  Ok(execution)
}

#[cfg(unix)]
fn configure_knowledge(
  state: AppState,
  canonical: Arc<dyn CanonicalReadPort>,
  transport: Arc<dyn IslandPortTransport>,
  active_authority: Arc<dyn ActiveKnowledgeReleasePort>,
  execution: transnet::domain::retrieval_data::NeighborProjectionExecutionExpectation,
  knowledge: &EnabledKnowledgeRuntimeConfig,
  max_request_body_bytes: usize,
) -> Result<(AppState, Arc<CompositeKnowledgeReadiness>)> {
  let retrieval: Arc<dyn RetrievalDataPort> = Arc::new(IslandPortRetrievalClient::new(transport));
  let views = Arc::new(KnowledgeViewService::new(
    retrieval.clone(),
    canonical.clone(),
    execution.clone(),
  ));
  let paths = Arc::new(BoundedKnowledgePathService::new(
    retrieval,
    canonical,
    execution.clone(),
  ));
  let routes = KnowledgeRouteDependencies::new(
    views,
    Arc::new(KnowledgeCursorCodec::new(
      knowledge
        .cursor_protection_key()
        .context("knowledge cursor key could not be constructed")?,
    )),
    paths,
    active_authority.clone(),
    knowledge.canonical.timeout,
  )
  .context("knowledge route dependencies are inconsistent")?;
  let readiness = Arc::new(CompositeKnowledgeReadiness::configured(
    active_authority,
    execution,
    knowledge.canonical.timeout,
  ));
  let capabilities = ServiceCapabilities::current(max_request_body_bytes)
    .with_translation_orchestrator(true)
    .with_knowledge_bundle(KnowledgeCapabilityBundle::FullyConfigured);
  Ok((
    state
      .with_knowledge_routes(routes)
      .with_capabilities(capabilities),
    readiness,
  ))
}

#[cfg(unix)]
fn startup_canonical_context(timeout: std::time::Duration) -> Result<CanonicalReadContext> {
  let deadline = time::OffsetDateTime::now_utc()
    + time::Duration::try_from(timeout).context("knowledge startup timeout is invalid")?;
  Ok(CanonicalReadContext {
    request_id: ulid::Ulid::new().to_string(),
    deadline_at: deadline
      .format(&time::format_description::well_known::Rfc3339)
      .context("knowledge startup deadline could not be formatted")?,
    timeout,
  })
}

#[cfg(not(unix))]
async fn configure_canonical(
  _state: AppState,
  _canonical: EnabledCanonicalRuntimeConfig,
  _knowledge: Option<EnabledKnowledgeRuntimeConfig>,
  _max_request_body_bytes: usize,
) -> Result<AppState> {
  anyhow::bail!("enabled canonical capability requires Unix domain sockets")
}

#[cfg(unix)]
struct RuntimeKnowledgeReadiness {
  canonical: Arc<dyn Readiness>,
  knowledge: Option<Arc<CompositeKnowledgeReadiness>>,
  required: bool,
}

#[cfg(unix)]
impl RuntimeKnowledgeReadiness {
  fn configured(
    canonical: Arc<dyn Readiness>,
    knowledge: Arc<CompositeKnowledgeReadiness>,
    required: bool,
  ) -> Self {
    Self {
      canonical,
      knowledge: Some(knowledge),
      required,
    }
  }

  fn unavailable(canonical: Arc<dyn Readiness>) -> Self {
    Self {
      canonical,
      knowledge: None,
      required: false,
    }
  }
}

#[cfg(unix)]
#[async_trait]
impl Readiness for RuntimeKnowledgeReadiness {
  async fn is_ready(&self) -> bool {
    self.report().await.is_ready()
  }

  async fn report(&self) -> ReadinessReport {
    let canonical_ready = self.canonical.is_ready().await;
    let knowledge_report = match &self.knowledge {
      Some(knowledge) => knowledge.report().await,
      None => ReadinessReport::new(
        false,
        KnowledgeReadinessComponents {
          canonical_data: ReadinessComponentState::Unavailable,
          retrieval_data: ReadinessComponentState::Unavailable,
          knowledge_projection: ReadinessComponentState::Unavailable,
        },
      ),
    };
    let knowledge_ready = knowledge_report.is_ready();
    let knowledge_state = if canonical_ready && knowledge_ready {
      ReadinessComponentState::Available
    } else {
      ReadinessComponentState::Unavailable
    };
    ReadinessReport::new(
      canonical_ready && (!self.required || knowledge_ready),
      KnowledgeReadinessComponents {
        canonical_data: if canonical_ready {
          ReadinessComponentState::Available
        } else {
          ReadinessComponentState::Unavailable
        },
        retrieval_data: knowledge_state,
        knowledge_projection: knowledge_state,
      },
    )
  }
}

#[cfg(all(test, unix))]
mod tests {
  use super::*;

  struct FixedReadiness(bool);

  #[async_trait]
  impl Readiness for FixedReadiness {
    async fn is_ready(&self) -> bool {
      self.0
    }
  }

  #[tokio::test]
  async fn optional_unavailable_knowledge_preserves_canonical_readiness() {
    let readiness = RuntimeKnowledgeReadiness::unavailable(Arc::new(FixedReadiness(true)));
    let report = readiness.report().await;
    assert!(report.is_ready());
    assert_eq!(
      report.components,
      KnowledgeReadinessComponents {
        canonical_data: ReadinessComponentState::Available,
        retrieval_data: ReadinessComponentState::Unavailable,
        knowledge_projection: ReadinessComponentState::Unavailable,
      }
    );

    let readiness = RuntimeKnowledgeReadiness::unavailable(Arc::new(FixedReadiness(false)));
    assert!(!readiness.report().await.is_ready());
  }
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
