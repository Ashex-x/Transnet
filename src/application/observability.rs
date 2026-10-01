//! Bounded, response-neutral delivery of closed backend metric events.
//!
//! The dispatcher owns the only asynchronous delivery policy shared by application and HTTP
//! composition. It accepts the closed [`MetricEvent`](crate::domain::observability::MetricEvent)
//! catalog, never arbitrary labels or request data, and deliberately drops telemetry rather than
//! delaying or changing a user-visible operation when its bounded capacity is exhausted.

use std::sync::{
  atomic::{AtomicU64, Ordering},
  Arc,
};

use tokio::sync::Semaphore;

use crate::{
  domain::observability::{MetricEvent, TelemetryDropReason},
  ports::metrics::MetricsRecorder,
};

/// Maximum metric records allowed to be in flight through one dispatcher.
///
/// When all permits are occupied, [`ClosedMetricsDispatcher::dispatch`] drops the new event.
/// This is a deliberate best-effort telemetry boundary rather than a queue that can delay an
/// application response or retain unbounded data.
pub const MAX_IN_FLIGHT_METRIC_RECORDS: usize = 16;

/// Best-effort asynchronous dispatcher for the closed, redacted metric catalog.
///
/// Each accepted event reserves one bounded permit and is delivered from a spawned task. The
/// caller never awaits the recorder. If no Tokio runtime is active or all permits are occupied,
/// the event is dropped without changing the caller's result. Exporter implementations must still
/// keep their own work bounded and must not enrich events with private data.
#[derive(Clone)]
pub struct ClosedMetricsDispatcher {
  recorder: Arc<dyn MetricsRecorder>,
  permits: Arc<Semaphore>,
  dropped_capacity: Arc<AtomicU64>,
  dropped_runtime_unavailable: Arc<AtomicU64>,
}

/// Monotonic local counters for telemetry dropped before recorder delivery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TelemetryDropSnapshot {
  /// Records dropped because bounded delivery capacity was exhausted.
  pub capacity: u64,
  /// Records dropped because no Tokio runtime could schedule delivery.
  pub runtime_unavailable: u64,
}

impl TelemetryDropSnapshot {
  /// Returns the total number of locally observed drops.
  pub const fn total(self) -> u64 {
    self.capacity.saturating_add(self.runtime_unavailable)
  }
}

impl ClosedMetricsDispatcher {
  /// Creates a dispatcher around one closed metric recorder.
  pub fn new(recorder: Arc<dyn MetricsRecorder>) -> Self {
    Self {
      recorder,
      permits: Arc::new(Semaphore::new(MAX_IN_FLIGHT_METRIC_RECORDS)),
      dropped_capacity: Arc::new(AtomicU64::new(0)),
      dropped_runtime_unavailable: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Schedules one closed metric event without awaiting telemetry delivery.
  ///
  /// Events are dropped when the bounded in-flight capacity is full or when no Tokio runtime is
  /// available. A recorder that blocks, fails internally, or is slow therefore cannot delay or
  /// change the caller's result. Delivery order is not an exporter contract: metric events are
  /// categorical observations rather than an audit log.
  pub fn dispatch(&self, event: MetricEvent) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
      self.record_drop(TelemetryDropReason::RuntimeUnavailable);
      return;
    };
    let Ok(permit) = self.permits.clone().try_acquire_owned() else {
      self.record_drop(TelemetryDropReason::Capacity);
      return;
    };
    let recorder = self.recorder.clone();
    runtime.spawn(async move {
      recorder.record(event).await;
      drop(permit);
    });
  }

  /// Returns monotonic local drop counters without awaiting or contacting the recorder.
  pub fn drop_snapshot(&self) -> TelemetryDropSnapshot {
    TelemetryDropSnapshot {
      capacity: self.dropped_capacity.load(Ordering::Relaxed),
      runtime_unavailable: self.dropped_runtime_unavailable.load(Ordering::Relaxed),
    }
  }

  fn record_drop(&self, reason: TelemetryDropReason) {
    match reason {
      TelemetryDropReason::Capacity => {
        self.dropped_capacity.fetch_add(1, Ordering::Relaxed);
      }
      TelemetryDropReason::RuntimeUnavailable => {
        self
          .dropped_runtime_unavailable
          .fetch_add(1, Ordering::Relaxed);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{
    sync::{
      atomic::{AtomicUsize, Ordering},
      Arc,
    },
    time::Duration,
  };

  use async_trait::async_trait;
  use tokio::{
    sync::{Notify, Semaphore},
    time::timeout,
  };

  use super::*;
  use crate::domain::observability::{LookupStage, MetricOutcome};

  #[derive(Clone)]
  struct BlockingRecorder {
    started: Arc<AtomicUsize>,
    started_notify: Arc<Notify>,
    gate: Arc<Semaphore>,
  }

  impl BlockingRecorder {
    fn new() -> Self {
      Self {
        started: Arc::new(AtomicUsize::new(0)),
        started_notify: Arc::new(Notify::new()),
        gate: Arc::new(Semaphore::new(0)),
      }
    }

    async fn wait_for_started(&self, expected: usize) {
      timeout(Duration::from_secs(1), async {
        loop {
          let notified = self.started_notify.notified();
          if self.started.load(Ordering::SeqCst) >= expected {
            return;
          }
          notified.await;
        }
      })
      .await
      .expect("blocking recorder should receive dispatched records");
    }
  }

  #[async_trait]
  impl MetricsRecorder for BlockingRecorder {
    async fn record(&self, _event: MetricEvent) {
      self.started.fetch_add(1, Ordering::SeqCst);
      self.started_notify.notify_waiters();
      let permit = self
        .gate
        .acquire()
        .await
        .expect("blocking recorder gate remains available");
      drop(permit);
    }
  }

  fn event() -> MetricEvent {
    MetricEvent::LookupStage {
      stage: LookupStage::RequestValidation,
      outcome: MetricOutcome::Succeeded,
    }
  }

  #[tokio::test]
  async fn drops_new_events_when_bounded_delivery_is_saturated() {
    let recorder = BlockingRecorder::new();
    let dispatcher = ClosedMetricsDispatcher::new(Arc::new(recorder.clone()));

    for _ in 0..MAX_IN_FLIGHT_METRIC_RECORDS {
      dispatcher.dispatch(event());
    }
    recorder
      .wait_for_started(MAX_IN_FLIGHT_METRIC_RECORDS)
      .await;

    dispatcher.dispatch(event());
    tokio::task::yield_now().await;

    assert_eq!(
      recorder.started.load(Ordering::SeqCst),
      MAX_IN_FLIGHT_METRIC_RECORDS
    );
    assert_eq!(dispatcher.drop_snapshot().capacity, 1);
  }

  #[test]
  fn counts_runtime_unavailable_without_failing_the_caller() {
    let dispatcher = ClosedMetricsDispatcher::new(Arc::new(BlockingRecorder::new()));

    dispatcher.dispatch(event());

    assert_eq!(
      dispatcher.drop_snapshot(),
      TelemetryDropSnapshot {
        capacity: 0,
        runtime_unavailable: 1,
      }
    );
  }
}
