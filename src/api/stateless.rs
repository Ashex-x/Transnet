//! Rejects end-user state before dispatch and prevents HTTP retention of request results.

use axum::{
  body::HttpBody,
  extract::Request,
  http::{Method, StatusCode},
  middleware::Next,
  response::{IntoResponse, Response},
};

use super::{problem, request_id::RequestId};

/// Admits service-only requests without inspecting or echoing protected header values.
pub(super) async fn admit(request: Request, next: Next) -> Response {
  let has_private_header = request.headers().keys().any(|name| {
    matches!(
      name.as_str(),
      "authorization"
        | "proxy-authorization"
        | "cookie"
        | "cookie2"
        | "x-user-id"
        | "x-learner-id"
        | "x-account-id"
        | "x-owner-id"
        | "x-session-id"
        | "x-authenticated-user"
        | "x-forwarded-user"
        | "remote-user"
        | "x-api-key"
        | "lookup-capability"
    ) || [
      "x-user-",
      "x-learner-",
      "x-account-",
      "x-owner-",
      "x-session-",
    ]
    .iter()
    .any(|prefix| name.as_str().starts_with(prefix))
  });
  // Only the transitional graph routes define query parameters; their typed extractors reject
  // unknown fields. Other routes must not silently accept identity or persistence query strings.
  let graph_query = request.uri().path() == "/v1/graph"
    || (request.uri().path().starts_with("/v1/graph/nodes/")
      && request.uri().path().ends_with("/neighbors"));
  let unexpected_query = request.uri().query().is_some() && !graph_query;
  let unexpected_body = matches!(
    *request.method(),
    Method::GET | Method::HEAD | Method::OPTIONS
  ) && request.body().size_hint().exact() != Some(0);
  if has_private_header || unexpected_query || unexpected_body {
    if let Some(request_id) = request.extensions().get::<RequestId>() {
      return problem::response(
        StatusCode::BAD_REQUEST,
        "invalid_service_request",
        "Invalid service request",
        "End-user credentials, identity headers, and unsupported request parameters are not accepted.",
        request_id,
        false,
        Vec::new(),
      );
    }
    // The outer request-ID middleware normally makes this branch unreachable.
    return problem::no_store(StatusCode::BAD_REQUEST.into_response());
  }
  problem::no_store(next.run(request).await)
}
