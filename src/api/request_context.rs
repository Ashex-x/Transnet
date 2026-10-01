//! HTTP admission for one bounded target request context.

use std::time::Duration;

use axum::{extract::Request, middleware::Next, response::Response};
use time::{
  format_description::well_known::Rfc3339, Duration as TimeDuration, OffsetDateTime, UtcOffset,
};

use crate::domain::request_context::RequestContext;

use super::{problem, request_id::RequestId};

/// Transport schema attached to current inbound service operations.
pub(crate) const TRANSNET_SCHEMA_VERSION: &str = "transnet-service-v1";
/// Default total budget assigned when a caller omits its absolute deadline.
pub(crate) const DEFAULT_REQUEST_BUDGET: Duration = Duration::from_secs(30);
/// Largest total budget accepted from an inbound caller.
pub(crate) const MAX_REQUEST_BUDGET: Duration = Duration::from_secs(120);

/// Establishes one immutable request context before handlers perform application work.
pub(crate) async fn establish(mut request: Request, next: Next) -> Response {
  let request_id = request
    .extensions()
    .get::<RequestId>()
    .cloned()
    .unwrap_or_else(RequestId::generate);
  let now = truncate_to_microseconds(OffsetDateTime::now_utc());
  let mut deadline_headers = request.headers().get_all("x-deadline-at").iter();
  let first_deadline = deadline_headers.next();
  let duplicate_deadline = deadline_headers.next().is_some();
  let deadline_at = match first_deadline {
    None => now + TimeDuration::try_from(DEFAULT_REQUEST_BUDGET).unwrap_or(TimeDuration::ZERO),
    Some(value) => match parse_deadline(value.to_str().ok(), duplicate_deadline, now) {
      Ok(deadline) => deadline,
      Err(DeadlineError::Invalid) => return problem::response(
        axum::http::StatusCode::BAD_REQUEST,
        "invalid_deadline",
        "Invalid request deadline",
        "X-Deadline-At must be a future UTC RFC 3339 timestamp within the maximum request budget.",
        &request_id,
        false,
        Vec::new(),
      ),
      Err(DeadlineError::Expired) => {
        return problem::response(
          axum::http::StatusCode::GATEWAY_TIMEOUT,
          "deadline_exceeded",
          "Request deadline exceeded",
          "The caller deadline was exhausted before request admission.",
          &request_id,
          false,
          Vec::new(),
        )
      }
    },
  };
  let context = RequestContext::new(request_id, deadline_at, TRANSNET_SCHEMA_VERSION, None)
    .expect("middleware constants and normalized deadlines are valid");
  request.extensions_mut().insert(context);
  next.run(request).await
}

fn parse_deadline(
  value: Option<&str>,
  duplicate: bool,
  now: OffsetDateTime,
) -> Result<OffsetDateTime, DeadlineError> {
  let value = value.ok_or(DeadlineError::Invalid)?;
  if duplicate || value.len() > 64 {
    return Err(DeadlineError::Invalid);
  }
  let deadline = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| DeadlineError::Invalid)?;
  if deadline.offset() != UtcOffset::UTC || !deadline.nanosecond().is_multiple_of(1_000) {
    return Err(DeadlineError::Invalid);
  }
  if deadline <= now {
    return Err(DeadlineError::Expired);
  }
  let maximum = TimeDuration::try_from(MAX_REQUEST_BUDGET).map_err(|_| DeadlineError::Invalid)?;
  if deadline - now > maximum {
    return Err(DeadlineError::Invalid);
  }
  Ok(deadline)
}

fn truncate_to_microseconds(value: OffsetDateTime) -> OffsetDateTime {
  value
    .replace_nanosecond(value.nanosecond() / 1_000 * 1_000)
    .unwrap_or(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeadlineError {
  Invalid,
  Expired,
}

#[cfg(test)]
mod tests {
  use axum::{
    body::{to_bytes, Body},
    extract::Extension,
    http::Request,
    middleware,
    routing::get,
    Router,
  };
  use tower::ServiceExt;

  use super::*;

  #[test]
  fn accepts_only_future_bounded_utc_microsecond_deadlines() {
    let now = OffsetDateTime::parse("2026-10-01T12:00:00Z", &Rfc3339).unwrap();
    assert!(parse_deadline(Some("2026-10-01T12:00:30.123456Z"), false, now).is_ok());
    assert_eq!(
      parse_deadline(Some("2026-10-01T12:00:00Z"), false, now),
      Err(DeadlineError::Expired)
    );
    assert_eq!(
      parse_deadline(Some("2026-10-01T12:03:00Z"), false, now),
      Err(DeadlineError::Invalid)
    );
    assert_eq!(
      parse_deadline(Some("2026-10-01T20:00:30+08:00"), false, now),
      Err(DeadlineError::Invalid)
    );
    assert_eq!(
      parse_deadline(Some("2026-10-01T12:00:30Z"), true, now),
      Err(DeadlineError::Invalid)
    );
  }

  #[tokio::test]
  async fn middleware_makes_one_safe_default_context_available() {
    async fn inspect(Extension(context): Extension<RequestContext>) -> String {
      format!(
        "{}:{}:{}",
        context.request_id().as_str(),
        context.schema_version(),
        context.content_release().is_none()
      )
    }

    let app = Router::new()
      .route("/inspect", get(inspect))
      .layer(middleware::from_fn(establish))
      .layer(middleware::from_fn(
        super::super::request_id::propagate_request_id,
      ));
    let response = app
      .oneshot(
        Request::get("/inspect")
          .header("x-request-id", "context-test-1")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    let body = to_bytes(response.into_body(), 1_024).await.unwrap();

    assert_eq!(&body[..], b"context-test-1:transnet-service-v1:true");
  }
}
