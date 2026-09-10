//! Port for recording closed, redacted backend metric events.
//!
//! The port accepts only [`MetricEvent`](crate::domain::observability::MetricEvent) values from
//! the closed domain catalog. It does not accept arbitrary attribute maps, raw request content,
//! identity fields, credentials, model tokens, or numeric payloads. Exporter configuration, trace
//! propagation, retention, sampling, alerts, and dashboards remain outside this foundation.

use async_trait::async_trait;

use crate::domain::observability::MetricEvent;

/// Records one closed, redacted metric event without affecting a user-visible operation.
///
/// Implementations must preserve the catalog's fixed label boundary. They may aggregate, buffer,
/// or drop telemetry according to a separately configured reliability policy, but must not enrich
/// an event with private application data.
#[async_trait]
pub trait MetricsRecorder: Send + Sync {
  /// Records one categorical backend event.
  ///
  /// This operation intentionally has no error result so callers need not change a successful
  /// user-visible outcome when telemetry is unavailable. Production exporters are responsible for
  /// their own bounded failure handling.
  async fn record(&self, event: MetricEvent);
}
