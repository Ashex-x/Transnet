//! Opt-in canonical dependency composition and unchanged readiness/translation HTTP boundaries.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
  body::{to_bytes, Body},
  http::{Request, StatusCode},
  response::Response,
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  adapters::island_port::{IslandPortCanonicalClient, IslandPortClientError, IslandPortTransport},
  api::CanonicalDependencyReadiness,
  app_router,
  application::canonical_read::CanonicalReadService,
  config::{CanonicalRuntimeConfig, CanonicalRuntimeConfigError},
  domain::{
    canonical::{CanonicalId, CanonicalReleasePin},
    canonical_content::CanonicalSenseDetails,
    canonical_translation::CanonicalTranslationRevision,
    retrieval::RepositoryMatch,
  },
  ports::canonical_read::{
    CanonicalCandidateQuery, CanonicalReadContext, CanonicalReadError, CanonicalReadPort,
    CanonicalSenseQuery, CanonicalTranslationQuery,
  },
  AppConfig, AppState, ProviderConfig, TranslationConfig, TranslationService,
};

#[derive(Clone, Copy)]
enum ProbeOutcome {
  Healthy,
  Unavailable,
  VersionMismatch,
  Timeout,
}

struct FakeAuthority(ProbeOutcome);

struct ActiveReleaseTransport {
  schema_version: &'static str,
}

#[async_trait]
impl IslandPortTransport for ActiveReleaseTransport {
  async fn post_json(
    &self,
    path: &'static str,
    body: Vec<u8>,
    timeout: Duration,
  ) -> Result<Vec<u8>, IslandPortClientError> {
    assert_eq!(path, "/api/v1/releases/active");
    assert!(!timeout.is_zero());
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert!(body["context"].get("content_release").is_none());
    assert_eq!(body["input"], serde_json::json!({}));
    Ok(
      serde_json::json!({
        "request_id": body["context"]["request_id"],
        "schema_version": self.schema_version,
        "outcome": "ok",
        "value": {
          "content_release": "release-runtime-test",
          "canonical_schema_version": "canonical-v1"
        }
      })
      .to_string()
      .into_bytes(),
    )
  }
}

#[async_trait]
impl CanonicalReadPort for FakeAuthority {
  async fn active_release(
    &self,
    context: &CanonicalReadContext,
  ) -> Result<Option<CanonicalReleasePin>, CanonicalReadError> {
    assert!(!context.request_id.is_empty());
    assert!(!context.deadline_at.is_empty());
    match self.0 {
      ProbeOutcome::Healthy => Ok(Some(
        CanonicalReleasePin::new(
          CanonicalId::new("release-runtime-test").unwrap(),
          "canonical-v1".into(),
        )
        .unwrap(),
      )),
      ProbeOutcome::Unavailable => Err(CanonicalReadError::Unavailable),
      ProbeOutcome::VersionMismatch => Err(CanonicalReadError::SchemaIncompatible),
      ProbeOutcome::Timeout => std::future::pending().await,
    }
  }

  async fn translations(
    &self,
    _: &CanonicalReadContext,
    _: &CanonicalReleasePin,
    _: CanonicalTranslationQuery,
  ) -> Result<Vec<CanonicalTranslationRevision>, CanonicalReadError> {
    panic!("readiness must not resolve translations")
  }

  async fn candidates(
    &self,
    _: &CanonicalReadContext,
    _: &CanonicalReleasePin,
    _: CanonicalCandidateQuery,
  ) -> Result<Vec<RepositoryMatch>, CanonicalReadError> {
    panic!("readiness must not resolve candidates")
  }

  async fn sense(
    &self,
    _: &CanonicalReadContext,
    _: &CanonicalReleasePin,
    _: CanonicalSenseQuery,
  ) -> Result<CanonicalSenseDetails, CanonicalReadError> {
    panic!("readiness must not load sense details")
  }
}

fn legacy_service() -> TranslationService {
  let provider = |model: &str| ProviderConfig {
    base_url: "http://127.0.0.1:9/v1".into(),
    model: model.into(),
    api_key: "unused-test-credential".into(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider("short"),
    provider("long"),
  )
  .unwrap()
}

async fn json(response: Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[test]
fn canonical_configuration_is_optional_but_strict_when_enabled() {
  let checked_in: AppConfig = toml::from_str(include_str!("../config/transnet.toml")).unwrap();
  assert!(checked_in.canonical.resolve().unwrap().is_none());
  assert!(CanonicalRuntimeConfig::default()
    .resolve()
    .unwrap()
    .is_none());
  let disabled = CanonicalRuntimeConfig {
    enabled: false,
    socket_path: Some("PRIVATE_SOCKET_PATH".into()),
    timeout_ms: None,
  };
  assert!(disabled.resolve().unwrap().is_none());
  let missing = CanonicalRuntimeConfig {
    enabled: true,
    ..CanonicalRuntimeConfig::default()
  };
  assert_eq!(
    missing.resolve().unwrap_err(),
    CanonicalRuntimeConfigError::InvalidSocketPath
  );
  let invalid = CanonicalRuntimeConfig {
    enabled: true,
    socket_path: Some("relative.sock".into()),
    timeout_ms: Some(100),
  };
  assert_eq!(
    invalid.resolve().unwrap_err(),
    CanonicalRuntimeConfigError::InvalidSocketPath
  );
  let valid = CanonicalRuntimeConfig {
    enabled: true,
    socket_path: Some("/run/island-port/island-port.sock".into()),
    timeout_ms: Some(500),
  };
  assert_eq!(
    valid.resolve().unwrap().unwrap().timeout,
    Duration::from_millis(500)
  );
  for timeout_ms in [None, Some(0), Some(30_001)] {
    assert_eq!(
      CanonicalRuntimeConfig {
        timeout_ms,
        ..valid.clone()
      }
      .resolve()
      .unwrap_err(),
      CanonicalRuntimeConfigError::InvalidTimeout
    );
  }
  assert!(!format!("{valid:?}").contains("/run/island-port"));
  assert!(!format!("{:?}", valid.resolve().unwrap().unwrap()).contains("/run/island-port"));
  assert!(!format!("{:?}", invalid.resolve().unwrap_err()).contains("relative.sock"));
}

#[tokio::test]
async fn strict_outbound_client_can_back_opt_in_service_and_readiness() {
  for (schema_version, expected) in [
    ("mysql-adapter-v1", StatusCode::OK),
    ("obsolete-schema", StatusCode::SERVICE_UNAVAILABLE),
  ] {
    let transport = Arc::new(ActiveReleaseTransport { schema_version });
    let authority: Arc<dyn CanonicalReadPort> = Arc::new(IslandPortCanonicalClient::new(transport));
    let state = AppState::new(legacy_service())
      .with_canonical_read_service(Arc::new(CanonicalReadService::new(authority.clone())))
      .with_readiness(Arc::new(CanonicalDependencyReadiness::new(
        authority,
        Duration::from_secs(1),
      )));
    assert!(state.canonical_read_service().is_some());
    let response = app_router(state)
      .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), expected);
  }
}

#[tokio::test]
async fn disabled_runtime_keeps_model_only_readiness_and_no_canonical_dependency() {
  let state = AppState::new(legacy_service());
  assert!(state.canonical_read_service().is_none());
  let response = app_router(state)
    .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(json(response).await, serde_json::json!({"status":"ok"}));
}

#[tokio::test]
async fn enabled_runtime_reuses_one_authority_for_service_and_read_only_probe() {
  for (outcome, expected) in [
    (ProbeOutcome::Healthy, StatusCode::OK),
    (ProbeOutcome::Unavailable, StatusCode::SERVICE_UNAVAILABLE),
    (
      ProbeOutcome::VersionMismatch,
      StatusCode::SERVICE_UNAVAILABLE,
    ),
    (ProbeOutcome::Timeout, StatusCode::SERVICE_UNAVAILABLE),
  ] {
    let authority: Arc<dyn CanonicalReadPort> = Arc::new(FakeAuthority(outcome));
    let timeout = Duration::from_millis(10);
    let state = AppState::new(legacy_service())
      .with_canonical_read_service(Arc::new(CanonicalReadService::new(authority.clone())))
      .with_readiness(Arc::new(CanonicalDependencyReadiness::new(
        authority, timeout,
      )));
    assert!(state.canonical_read_service().is_some());
    let router = app_router(state);
    let response = router
      .clone()
      .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), expected);
    let response = router.oneshot(Request::post("/api/v1/translations")
      .header("content-type", "application/json")
      .body(Body::from(r#"{"text":"hello","source_language":"en","target_language":"zh-CN","response_level":"brief"}"#))
      .unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
      json(response).await["code"],
      "translation_model_unavailable"
    );
  }
}
