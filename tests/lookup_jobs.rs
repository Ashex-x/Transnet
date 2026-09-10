//! Lookup-job polling contract tests.

use std::{
  num::NonZeroU32,
  sync::Arc,
  time::{Duration, SystemTime},
};

use axum::{
  body::{to_bytes, Body},
  http::{header, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use transnet::{
  adapters::{
    clock::FixedClock,
    in_memory::{InMemoryDurableJobQueue, InMemoryLookupJobStore},
    public_id::SequencePublicIdGenerator,
  },
  app_router,
  ports::{
    durable_job::{DurableJobQueue, JobKind, JobSubmission, WorkerId},
    lookup_job::{
      LookupJobAccess, LookupJobCapability, LookupJobCreation, LookupJobFailure, LookupJobOwner,
      LookupJobResult, LookupJobStore,
    },
    public_id::{PublicId, PublicIdGenerator},
  },
  AppState, AuthenticatedLookupJobOwner, ProviderConfig, TranslationConfig, TranslationService,
};
use ulid::Ulid;

fn service() -> TranslationService {
  let provider = ProviderConfig {
    base_url: "http://127.0.0.1:1/v1".to_string(),
    model: "unused".to_string(),
    api_key: "unused".to_string(),
  };
  TranslationService::new(
    TranslationConfig {
      long_text_chars: 4_000,
      timeout_seconds: 1,
      max_retries: 0,
      retry_delay_ms: 0,
    },
    provider.clone(),
    provider,
  )
  .unwrap()
}

fn id(random: u128) -> PublicId {
  PublicId::from(Ulid::from_parts(1_700_000_000_000, random))
}

fn capability() -> LookupJobCapability {
  LookupJobCapability::parse("aB_1-23456789012345678901234567890").unwrap()
}

fn owner() -> LookupJobOwner {
  LookupJobOwner::new("owner-equality-token").unwrap()
}

async fn json(response: axum::response::Response) -> Value {
  serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn route_is_absent_without_an_injected_lookup_job_store() {
  let response = app_router(AppState::new(service()))
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{}", id(1)))
        .header("lookup-capability", capability().as_str())
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(json(response).await["code"], "not_found");
}

#[tokio::test]
async fn capability_poll_returns_pending_without_leaking_the_bearer_value() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
  let clock = Arc::new(FixedClock::new(now));
  let store = Arc::new(InMemoryLookupJobStore::new(clock));
  let job_id = id(2);
  let capability = capability();
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::capability(capability.clone()),
        now + Duration::from_secs(60),
      ),
    )
    .await
    .unwrap();

  let response = app_router(AppState::new(service()).with_lookup_job_store(store))
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{job_id}"))
        .header("lookup-capability", capability.as_str())
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::ACCEPTED);
  assert_eq!(response.headers()[header::RETRY_AFTER], "1");
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  let body = json(response).await;
  assert_eq!(
    body,
    serde_json::json!({"schema_version": "1.0", "status": "queued"})
  );
  assert!(!body.to_string().contains(capability.as_str()));
}

#[tokio::test]
async fn unauthorized_poll_is_indistinguishable_from_an_absent_job() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000);
  let clock = Arc::new(FixedClock::new(now));
  let store = Arc::new(InMemoryLookupJobStore::new(clock));
  let job_id = id(3);
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::capability(capability()),
        now + Duration::from_secs(60),
      ),
    )
    .await
    .unwrap();
  let router = app_router(AppState::new(service()).with_lookup_job_store(store));

  let unauthorized = router
    .clone()
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{job_id}"))
        .header(
          "lookup-capability",
          "different_23456789012345678901234567890",
        )
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  let absent = router
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{}", id(999)))
        .header("lookup-capability", capability().as_str())
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(unauthorized.status(), StatusCode::NOT_FOUND);
  assert_eq!(absent.status(), StatusCode::NOT_FOUND);
  assert_eq!(json(unauthorized).await["code"], json(absent).await["code"]);
}

#[tokio::test]
async fn authenticated_owner_can_poll_completed_result_without_an_owner_header() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3_000);
  let clock = Arc::new(FixedClock::new(now));
  let store = Arc::new(InMemoryLookupJobStore::new(clock));
  let job_id = id(4);
  let owner = owner();
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::owner(owner.clone()),
        now + Duration::from_secs(60),
      ),
    )
    .await
    .unwrap();
  store.start(&job_id).await.unwrap();
  store
    .complete(
      &job_id,
      LookupJobResult::new(serde_json::json!({
        "schema_version": "1.0",
        "query": {"normalized": "caliente"},
      })),
    )
    .await
    .unwrap();
  let mut request = Request::get(format!("/v1/lookup-jobs/{job_id}"))
    .body(Body::empty())
    .unwrap();
  request
    .extensions_mut()
    .insert(AuthenticatedLookupJobOwner::new(owner));

  let response = app_router(AppState::new(service()).with_lookup_job_store(store))
    .oneshot(request)
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
  assert_eq!(
    json(response).await,
    serde_json::json!({
      "schema_version": "1.0",
      "query": {"normalized": "caliente"},
    })
  );
}

#[tokio::test]
async fn expired_poll_returns_a_typed_gone_problem() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000);
  let clock = Arc::new(FixedClock::new(now));
  let store = Arc::new(InMemoryLookupJobStore::new(clock.clone()));
  let job_id = id(5);
  let capability = capability();
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::capability(capability.clone()),
        now + Duration::from_secs(1),
      ),
    )
    .await
    .unwrap();
  clock.advance(Duration::from_secs(1));

  let response = app_router(AppState::new(service()).with_lookup_job_store(store))
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{job_id}"))
        .header("lookup-capability", capability.as_str())
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::GONE);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "application/problem+json"
  );
  assert_eq!(json(response).await["code"], "lookup_job_expired");
}

#[tokio::test]
async fn terminal_failures_are_typed_and_redacted() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(5_000);
  let clock = Arc::new(FixedClock::new(now));
  let store = Arc::new(InMemoryLookupJobStore::new(clock));
  let job_id = id(6);
  let capability = capability();
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::capability(capability.clone()),
        now + Duration::from_secs(60),
      ),
    )
    .await
    .unwrap();
  store.start(&job_id).await.unwrap();
  store
    .fail(
      &job_id,
      LookupJobFailure::new(
        transnet::ports::durable_job::JobFailureCode::new("provider_unavailable").unwrap(),
      ),
    )
    .await
    .unwrap();

  let response = app_router(AppState::new(service()).with_lookup_job_store(store))
    .oneshot(
      Request::get(format!("/v1/lookup-jobs/{job_id}"))
        .header("lookup-capability", capability.as_str())
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    json(response).await,
    serde_json::json!({
      "schema_version": "1.0",
      "status": "failed",
      "failure": {"code": "provider_unavailable"},
    })
  );
}

#[tokio::test]
async fn reopened_test_queue_can_resume_a_matching_lifecycle_record() {
  let now = SystemTime::UNIX_EPOCH + Duration::from_secs(6_000);
  let clock = Arc::new(FixedClock::new(now));
  let queue_ids: Arc<dyn PublicIdGenerator> =
    Arc::new(SequencePublicIdGenerator::new([id(7), id(8)]));
  let queue = InMemoryDurableJobQueue::new(clock.clone(), queue_ids);
  let store = Arc::new(InMemoryLookupJobStore::new(clock));
  let capability = capability();
  let job_id = queue
    .enqueue(JobSubmission::new(
      JobKind::new("lookup.generate").unwrap(),
      b"opaque lookup payload".to_vec(),
      1,
      now,
      NonZeroU32::new(1).unwrap(),
    ))
    .await
    .unwrap();
  store
    .create(
      job_id.clone(),
      LookupJobCreation::new(
        LookupJobAccess::capability(capability.clone()),
        now + Duration::from_secs(60),
      ),
    )
    .await
    .unwrap();

  let reopened_queue = queue.reopen();
  let claim = reopened_queue
    .claim(
      &WorkerId::new("lookup-worker-a").unwrap(),
      Duration::from_secs(5),
    )
    .await
    .unwrap()
    .unwrap();
  assert_eq!(claim.id(), &job_id);
  store.start(claim.id()).await.unwrap();

  assert!(matches!(
    store
      .poll(&job_id, &LookupJobAccess::capability(capability))
      .await
      .unwrap(),
    Some(transnet::ports::lookup_job::LookupJobPoll::Pending {
      state: transnet::ports::lookup_job::LookupJobPendingState::Running,
      ..
    })
  ));
}
