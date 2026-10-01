//! Strict W3C trace-context admission and propagation for trusted HTTP boundaries.

use axum::{
  extract::Request,
  http::{HeaderMap, HeaderValue},
  middleware::Next,
  response::Response,
};

use crate::domain::observability::TraceParent;

/// Propagates one valid, unambiguous W3C `traceparent` value through request extensions and the
/// response while discarding malformed, repeated, or unsupported inbound values.
pub(crate) async fn propagate_trace_parent(mut request: Request, next: Next) -> Response {
  let trace_parent = inbound_trace_parent(request.headers());
  if let Some(trace_parent) = trace_parent.clone() {
    request.extensions_mut().insert(trace_parent);
  }

  let mut response = next.run(request).await;
  if let Some(trace_parent) = trace_parent {
    if let Ok(value) = HeaderValue::from_str(trace_parent.as_header_value()) {
      response.headers_mut().insert("traceparent", value);
    }
  }
  response
}

fn inbound_trace_parent(headers: &HeaderMap) -> Option<TraceParent> {
  let mut values = headers.get_all("traceparent").iter();
  let value = values.next()?.to_str().ok()?;
  if values.next().is_some() {
    return None;
  }
  TraceParent::parse(value).ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn admits_one_valid_header_and_rejects_ambiguous_values() {
    let mut headers = HeaderMap::new();
    headers.insert(
      "traceparent",
      "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        .parse()
        .unwrap(),
    );
    assert!(inbound_trace_parent(&headers).is_some());

    headers.append(
      "traceparent",
      "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbb-00"
        .parse()
        .unwrap(),
    );
    assert!(inbound_trace_parent(&headers).is_none());
  }
}
