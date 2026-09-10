//! Deterministic in-memory recorder for closed metric events.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{domain::observability::MetricEvent, ports::metrics::MetricsRecorder};

/// Shareable recorder that preserves metric-event order for tests and local development.
///
/// It retains events only in process memory and is not an exporter, dashboard, tracing backend, or
/// production retention implementation.
#[derive(Clone, Default)]
pub struct InMemoryMetricsRecorder {
  events: Arc<Mutex<Vec<MetricEvent>>>,
}

impl InMemoryMetricsRecorder {
  /// Creates an empty deterministic recorder.
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns a snapshot of events in the order they were recorded.
  pub async fn events(&self) -> Vec<MetricEvent> {
    self.events.lock().await.clone()
  }

  /// Removes all recorded events.
  pub async fn clear(&self) {
    self.events.lock().await.clear();
  }
}

#[async_trait]
impl MetricsRecorder for InMemoryMetricsRecorder {
  async fn record(&self, event: MetricEvent) {
    self.events.lock().await.push(event);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::observability::{LookupStage, MetricOutcome};

  #[tokio::test]
  async fn retains_closed_events_in_recording_order() {
    let recorder = InMemoryMetricsRecorder::new();
    let first = MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Succeeded,
    };
    let second = MetricEvent::LookupStage {
      stage: LookupStage::CandidateRetrieval,
      outcome: MetricOutcome::Degraded,
    };

    recorder.record(first).await;
    recorder.record(second).await;

    assert_eq!(recorder.events().await, vec![first, second]);

    recorder.clear().await;

    assert!(recorder.events().await.is_empty());
  }
}
