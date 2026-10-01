//! Safe HTTP request correlation IDs.

pub(crate) use crate::domain::request_context::RequestId;
use axum::{
  extract::Request,
  http::{HeaderMap, HeaderValue},
  middleware::Next,
  response::Response,
};

/// Adds a safe request ID to extensions and propagates it to the response.
pub(crate) async fn propagate_request_id(mut request: Request, next: Next) -> Response {
  let request_id = inbound_request_id(request.headers()).unwrap_or_else(RequestId::generate);
  request.extensions_mut().insert(request_id.clone());

  let mut response = next.run(request).await;
  if let Ok(value) = HeaderValue::from_str(request_id.as_str()) {
    response.headers_mut().insert("x-request-id", value);
  }
  response
}

fn inbound_request_id(headers: &HeaderMap) -> Option<RequestId> {
  let mut values = headers.get_all("x-request-id").iter();
  let value = values.next()?.to_str().ok()?;
  if values.next().is_some() || !is_safe_request_id(value) {
    return None;
  }
  RequestId::new(value.to_string()).ok()
}

fn is_safe_request_id(value: &str) -> bool {
  RequestId::new(value.to_string()).is_ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_bounded_header_safe_identifiers() {
    assert!(is_safe_request_id("edge-42.request_id"));
    assert!(is_safe_request_id(&"a".repeat(128)));
  }

  #[test]
  fn rejects_unsafe_or_ambiguous_identifier_values() {
    for value in [
      "",
      "with space",
      "line\nfeed",
      "slash/value",
      &"a".repeat(129),
    ] {
      assert!(
        !is_safe_request_id(value),
        "expected `{value}` to be unsafe"
      );
    }
  }
}
