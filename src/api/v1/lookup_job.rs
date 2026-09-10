//! `GET /v1/lookup-jobs/{job_id}` authorized polling contract.

use axum::{
  extract::{Extension, Path, State},
  http::{header, HeaderMap, HeaderValue, StatusCode},
  response::{IntoResponse, Response},
  Json,
};
use serde::Serialize;

use crate::{
  api::{problem, request_id::RequestId, AppState, AuthenticatedLookupJobOwner},
  ports::{
    lookup_job::{LookupJobAccess, LookupJobCapability, LookupJobPoll},
    public_id::PublicId,
  },
};

const LOOKUP_CAPABILITY_HEADER: &str = "lookup-capability";

/// Polls one private lookup job using an authenticated owner extension or anonymous capability.
pub(crate) async fn poll(
  State(state): State<AppState>,
  Path(job_id): Path<String>,
  Extension(request_id): Extension<RequestId>,
  headers: HeaderMap,
  owner: Option<Extension<AuthenticatedLookupJobOwner>>,
) -> Response {
  let Ok(job_id) = PublicId::parse(&job_id) else {
    return private_not_found(&request_id);
  };
  let Some(access) = request_access(owner, &headers) else {
    return private_not_found(&request_id);
  };
  let Some(service) = state.lookup_job_service() else {
    return problem::response(
      StatusCode::SERVICE_UNAVAILABLE,
      "lookup_jobs_unavailable",
      "Lookup jobs unavailable",
      "Lookup-job polling is not configured.",
      &request_id,
      true,
      Vec::new(),
    );
  };

  match service.poll(&job_id, &access).await {
    Ok(None) => private_not_found(&request_id),
    Ok(Some(LookupJobPoll::Pending { state, retry_after })) => {
      pending_response(state.as_str(), retry_after)
    }
    Ok(Some(LookupJobPoll::Completed(result))) => {
      problem::no_store((StatusCode::OK, Json(result.into_value())).into_response())
    }
    Ok(Some(LookupJobPoll::Failed(failure))) => problem::no_store(
      (
        StatusCode::OK,
        Json(LookupJobFailureResponse {
          schema_version: "1.0",
          status: "failed",
          failure: FailureResponse {
            code: failure.code().as_str().to_string(),
          },
        }),
      )
        .into_response(),
    ),
    Ok(Some(LookupJobPoll::Expired)) => problem::response(
      StatusCode::GONE,
      "lookup_job_expired",
      "Lookup job expired",
      "The lookup job is no longer retained.",
      &request_id,
      false,
      Vec::new(),
    ),
    Err(_) => {
      tracing::warn!("lookup-job polling dependency could not serve a request");
      problem::response(
        StatusCode::SERVICE_UNAVAILABLE,
        "lookup_jobs_unavailable",
        "Lookup jobs unavailable",
        "Lookup-job polling is temporarily unavailable.",
        &request_id,
        true,
        Vec::new(),
      )
    }
  }
}

fn request_access(
  owner: Option<Extension<AuthenticatedLookupJobOwner>>,
  headers: &HeaderMap,
) -> Option<LookupJobAccess> {
  if let Some(Extension(owner)) = owner {
    return Some(LookupJobAccess::owner(owner.owner().clone()));
  }

  let capability = headers
    .get(LOOKUP_CAPABILITY_HEADER)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| LookupJobCapability::parse(value).ok())?;
  Some(LookupJobAccess::capability(capability))
}

fn pending_response(status: &'static str, retry_after: std::time::Duration) -> Response {
  let mut response = problem::no_store(
    (
      StatusCode::ACCEPTED,
      Json(LookupJobPendingResponse {
        schema_version: "1.0",
        status,
      }),
    )
      .into_response(),
  );
  let retry_after = retry_after.as_secs().max(1).to_string();
  if let Ok(value) = HeaderValue::from_str(&retry_after) {
    response.headers_mut().insert(header::RETRY_AFTER, value);
  }
  response
}

fn private_not_found(request_id: &RequestId) -> Response {
  problem::response(
    StatusCode::NOT_FOUND,
    "not_found",
    "Versioned API route not found",
    "The requested versioned API route does not exist.",
    request_id,
    false,
    Vec::new(),
  )
}

#[derive(Debug, Serialize)]
struct LookupJobPendingResponse {
  schema_version: &'static str,
  status: &'static str,
}

#[derive(Debug, Serialize)]
struct LookupJobFailureResponse {
  schema_version: &'static str,
  status: &'static str,
  failure: FailureResponse,
}

#[derive(Debug, Serialize)]
struct FailureResponse {
  code: String,
}
