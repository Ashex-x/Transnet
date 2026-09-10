//! Bounded, redacted resilience controls for outbound model providers.

use std::{
  future::Future,
  sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
  },
  time::{Duration, SystemTime},
};

use reqwest::{
  header::{HeaderMap, RETRY_AFTER},
  StatusCode,
};
use thiserror::Error;
use tokio::{
  sync::Semaphore,
  time::{sleep, Instant},
};
use tracing::{info, warn};

/// Name of the counter for logical provider calls that passed circuit admission.
pub const PROVIDER_REQUESTS_STARTED_METRIC: &str = "transnet_provider_requests_started_total";
/// Name of the counter for HTTP attempts made to a provider.
pub const PROVIDER_ATTEMPTS_METRIC: &str = "transnet_provider_attempts_total";
/// Name of the counter for completed logical provider calls by outcome.
pub const PROVIDER_REQUESTS_COMPLETED_METRIC: &str = "transnet_provider_requests_completed_total";
/// Name of the counter for retries scheduled after a classified transient failure.
pub const PROVIDER_RETRIES_METRIC: &str = "transnet_provider_retries_total";
/// Name of the counter for calls rejected because the provider circuit is open.
pub const PROVIDER_CIRCUIT_OPEN_METRIC: &str = "transnet_provider_circuit_open_total";
/// Name of the counter for calls rejected because the provider bulkhead is full.
pub const PROVIDER_BULKHEAD_REJECTED_METRIC: &str = "transnet_provider_bulkhead_rejected_total";

/// Runtime policy for one outbound provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPolicy {
  timeout: Duration,
  max_retries: u32,
  retry_delay: Duration,
  max_retry_delay: Duration,
  max_concurrent_requests: usize,
  circuit_failure_threshold: u32,
  circuit_open_for: Duration,
}

impl ProviderPolicy {
  /// Creates a validated policy for one provider boundary.
  ///
  /// # Errors
  ///
  /// Returns an error when a duration, concurrency bound, or circuit threshold is zero.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    timeout: Duration,
    max_retries: u32,
    retry_delay: Duration,
    max_retry_delay: Duration,
    max_concurrent_requests: usize,
    circuit_failure_threshold: u32,
    circuit_open_for: Duration,
  ) -> Result<Self, ProviderPolicyError> {
    if timeout.is_zero() {
      return Err(ProviderPolicyError::ZeroTimeout);
    }
    if max_concurrent_requests == 0 {
      return Err(ProviderPolicyError::ZeroConcurrency);
    }
    if circuit_failure_threshold == 0 {
      return Err(ProviderPolicyError::ZeroCircuitFailureThreshold);
    }
    if circuit_open_for.is_zero() {
      return Err(ProviderPolicyError::ZeroCircuitOpenDuration);
    }

    Ok(Self {
      timeout,
      max_retries,
      retry_delay,
      max_retry_delay,
      max_concurrent_requests,
      circuit_failure_threshold,
      circuit_open_for,
    })
  }

  /// Returns the deadline for one HTTP provider attempt.
  pub const fn timeout(&self) -> Duration {
    self.timeout
  }
}

/// Invalid provider resilience policy.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum ProviderPolicyError {
  /// The configured per-attempt deadline is zero.
  #[error("provider timeout must be greater than zero")]
  ZeroTimeout,
  /// The configured provider concurrency bound is zero.
  #[error("provider max_concurrent_requests must be greater than zero")]
  ZeroConcurrency,
  /// The circuit cannot open without a positive failure threshold.
  #[error("provider circuit_failure_threshold must be greater than zero")]
  ZeroCircuitFailureThreshold,
  /// The circuit cannot open for a zero-length interval.
  #[error("provider circuit_open_ms must be greater than zero")]
  ZeroCircuitOpenDuration,
}

/// A redacted snapshot of one provider's in-process counters.
///
/// The fields correspond to the metric names exported by the constants in this module. They never
/// include request text, generated text, provider response bodies, credentials, or identity data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProviderMetricsSnapshot {
  /// Number of logical calls admitted by the circuit.
  pub requests_started: u64,
  /// Number of HTTP attempts that entered the provider bulkhead.
  pub attempts: u64,
  /// Number of logical calls that produced a usable response.
  pub succeeded: u64,
  /// Number of logical calls that failed after admission.
  pub failed: u64,
  /// Number of retry delays scheduled after classified transient failures.
  pub retried: u64,
  /// Number of provider attempts that saw a `429` response.
  pub rate_limited: u64,
  /// Number of provider attempts that timed out.
  pub timed_out: u64,
  /// Number of calls rejected while the circuit was open or half-open.
  pub circuit_open: u64,
  /// Number of calls rejected because all bulkhead permits were in use.
  pub bulkhead_rejected: u64,
}

/// Thread-safe metrics facade for a single provider boundary.
#[derive(Clone, Default)]
pub struct ProviderMetrics {
  counters: Arc<ProviderMetricCounters>,
}

impl ProviderMetrics {
  /// Returns a point-in-time, redacted copy of the provider counters.
  pub fn snapshot(&self) -> ProviderMetricsSnapshot {
    ProviderMetricsSnapshot {
      requests_started: self.counters.requests_started.load(Ordering::Relaxed),
      attempts: self.counters.attempts.load(Ordering::Relaxed),
      succeeded: self.counters.succeeded.load(Ordering::Relaxed),
      failed: self.counters.failed.load(Ordering::Relaxed),
      retried: self.counters.retried.load(Ordering::Relaxed),
      rate_limited: self.counters.rate_limited.load(Ordering::Relaxed),
      timed_out: self.counters.timed_out.load(Ordering::Relaxed),
      circuit_open: self.counters.circuit_open.load(Ordering::Relaxed),
      bulkhead_rejected: self.counters.bulkhead_rejected.load(Ordering::Relaxed),
    }
  }
}

#[derive(Default)]
struct ProviderMetricCounters {
  requests_started: AtomicU64,
  attempts: AtomicU64,
  succeeded: AtomicU64,
  failed: AtomicU64,
  retried: AtomicU64,
  rate_limited: AtomicU64,
  timed_out: AtomicU64,
  circuit_open: AtomicU64,
  bulkhead_rejected: AtomicU64,
}

/// A provider response or transport outcome safe to classify for resilience behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderAttemptError {
  /// The HTTP client timed out while sending or reading the response.
  Timeout,
  /// The provider explicitly rate limited the request.
  RateLimited {
    /// Provider-supplied delay, when a valid `Retry-After` header was present.
    retry_after: Option<Duration>,
  },
  /// A selected transient server status was returned.
  ServerError {
    /// HTTP status code from the provider.
    status: u16,
    /// Provider-supplied delay, when a valid `Retry-After` header was present.
    retry_after: Option<Duration>,
  },
  /// The provider returned a successful status with no usable expected envelope.
  InvalidEnvelope,
  /// Connection establishment failed before a provider request was sent.
  Connect,
  /// A transport failure that was not a timeout.
  Transport,
  /// A non-retryable HTTP status from the provider.
  NonRetryableStatus {
    /// HTTP status code from the provider.
    status: u16,
  },
}

impl ProviderAttemptError {
  fn is_retryable(self) -> bool {
    matches!(
      self,
      Self::Timeout
        | Self::RateLimited { .. }
        | Self::ServerError { .. }
        | Self::InvalidEnvelope
        | Self::Connect
    )
  }

  fn counts_toward_circuit(self) -> bool {
    self.is_retryable()
  }

  fn retry_after(self) -> Option<Duration> {
    match self {
      Self::RateLimited { retry_after } | Self::ServerError { retry_after, .. } => retry_after,
      Self::Timeout
      | Self::InvalidEnvelope
      | Self::Connect
      | Self::Transport
      | Self::NonRetryableStatus { .. } => None,
    }
  }

  fn outcome(self) -> &'static str {
    match self {
      Self::Timeout => "timeout",
      Self::RateLimited { .. } => "rate_limited",
      Self::ServerError { .. } => "server_error",
      Self::InvalidEnvelope => "invalid_envelope",
      Self::Connect => "connect",
      Self::Transport => "transport",
      Self::NonRetryableStatus { .. } => "non_retryable_status",
    }
  }

  fn status(self) -> Option<u16> {
    match self {
      Self::ServerError { status, .. } | Self::NonRetryableStatus { status } => Some(status),
      Self::Timeout
      | Self::RateLimited { .. }
      | Self::InvalidEnvelope
      | Self::Connect
      | Self::Transport => None,
    }
  }
}

/// Result of a rejected or exhausted provider call.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub(crate) enum ProviderCallError {
  /// The provider circuit is open or permits only a different half-open probe.
  #[error("provider circuit is open")]
  CircuitOpen,
  /// The provider's bounded-concurrency bulkhead is full.
  #[error("provider bulkhead is full")]
  BulkheadFull,
  /// The provider did not produce a usable response.
  #[error("provider request failed")]
  RequestFailed,
}

/// Shared circuit, bulkhead, retry, trace, and counter state for one provider.
#[derive(Clone)]
pub(crate) struct ProviderResilience {
  provider: &'static str,
  policy: ProviderPolicy,
  bulkhead: Arc<Semaphore>,
  circuit: Arc<Mutex<CircuitState>>,
  metrics: ProviderMetrics,
}

impl ProviderResilience {
  /// Creates independent resilience state for one named provider boundary.
  pub(crate) fn new(provider: &'static str, policy: ProviderPolicy) -> Self {
    Self {
      provider,
      bulkhead: Arc::new(Semaphore::new(policy.max_concurrent_requests)),
      circuit: Arc::new(Mutex::new(CircuitState::Closed {
        consecutive_failures: 0,
      })),
      policy,
      metrics: ProviderMetrics::default(),
    }
  }

  /// Returns the redacted metrics facade for this provider.
  pub(crate) fn metrics(&self) -> ProviderMetrics {
    self.metrics.clone()
  }

  /// Executes one logical provider call without exposing request or response data to telemetry.
  pub(crate) async fn execute<T, F, Fut>(
    &self,
    operation: &'static str,
    mut attempt: F,
  ) -> Result<T, ProviderCallError>
  where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ProviderAttemptError>>,
  {
    let mut admission = match self.admit() {
      Some(admission) => admission,
      None => {
        self
          .metrics
          .counters
          .circuit_open
          .fetch_add(1, Ordering::Relaxed);
        warn!(
          provider = self.provider,
          operation,
          outcome = "circuit_open",
          "provider call rejected by circuit"
        );
        return Err(ProviderCallError::CircuitOpen);
      }
    };
    self
      .metrics
      .counters
      .requests_started
      .fetch_add(1, Ordering::Relaxed);

    for attempt_number in 0..=self.policy.max_retries {
      let permit = match Arc::clone(&self.bulkhead).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
          self
            .metrics
            .counters
            .bulkhead_rejected
            .fetch_add(1, Ordering::Relaxed);
          admission.abandon();
          warn!(
            provider = self.provider,
            operation,
            outcome = "bulkhead_full",
            "provider call rejected by bulkhead"
          );
          return Err(ProviderCallError::BulkheadFull);
        }
      };
      self
        .metrics
        .counters
        .attempts
        .fetch_add(1, Ordering::Relaxed);
      let started = Instant::now();
      let result = attempt().await;
      drop(permit);
      let elapsed_ms = duration_millis(started.elapsed());

      match result {
        Ok(value) => {
          admission.succeed();
          self
            .metrics
            .counters
            .succeeded
            .fetch_add(1, Ordering::Relaxed);
          info!(
            provider = self.provider,
            operation,
            attempt = attempt_number,
            outcome = "success",
            elapsed_ms,
            "provider call completed"
          );
          return Ok(value);
        }
        Err(error) => {
          if matches!(error, ProviderAttemptError::RateLimited { .. }) {
            self
              .metrics
              .counters
              .rate_limited
              .fetch_add(1, Ordering::Relaxed);
          }
          if matches!(error, ProviderAttemptError::Timeout) {
            self
              .metrics
              .counters
              .timed_out
              .fetch_add(1, Ordering::Relaxed);
          }
          let retryable = error.is_retryable();
          if retryable && attempt_number < self.policy.max_retries {
            let delay = self.retry_delay(error);
            self
              .metrics
              .counters
              .retried
              .fetch_add(1, Ordering::Relaxed);
            warn!(
              provider = self.provider,
              operation,
              attempt = attempt_number,
              outcome = error.outcome(),
              status = error.status().unwrap_or_default(),
              retry_in_ms = duration_millis(delay),
              elapsed_ms,
              "provider attempt scheduled for retry"
            );
            sleep(delay).await;
            continue;
          }

          if error.counts_toward_circuit() {
            admission.fail();
          } else {
            admission.complete_without_circuit_failure();
          }
          self.metrics.counters.failed.fetch_add(1, Ordering::Relaxed);
          warn!(
            provider = self.provider,
            operation,
            attempt = attempt_number,
            outcome = error.outcome(),
            status = error.status().unwrap_or_default(),
            retryable,
            elapsed_ms,
            "provider call failed"
          );
          return Err(ProviderCallError::RequestFailed);
        }
      }
    }

    unreachable!("retry loop always returns a provider call result")
  }

  fn admit(&self) -> Option<CircuitAdmission> {
    let mut state = lock_recovering_poison(&self.circuit);
    let now = Instant::now();
    match *state {
      CircuitState::Closed { .. } => Some(CircuitAdmission::new(
        Arc::clone(&self.circuit),
        self.policy.circuit_failure_threshold,
        self.policy.circuit_open_for,
        false,
      )),
      CircuitState::Open { retry_at } if now >= retry_at => {
        *state = CircuitState::HalfOpen;
        Some(CircuitAdmission::new(
          Arc::clone(&self.circuit),
          self.policy.circuit_failure_threshold,
          self.policy.circuit_open_for,
          true,
        ))
      }
      CircuitState::Open { .. } | CircuitState::HalfOpen => None,
    }
  }

  fn retry_delay(&self, error: ProviderAttemptError) -> Duration {
    error
      .retry_after()
      .unwrap_or(self.policy.retry_delay)
      .min(self.policy.max_retry_delay)
  }
}

#[derive(Debug, Clone, Copy)]
enum CircuitState {
  Closed { consecutive_failures: u32 },
  Open { retry_at: Instant },
  HalfOpen,
}

struct CircuitAdmission {
  circuit: Arc<Mutex<CircuitState>>,
  failure_threshold: u32,
  open_for: Duration,
  half_open_probe: bool,
  completed: bool,
}

impl CircuitAdmission {
  fn new(
    circuit: Arc<Mutex<CircuitState>>,
    failure_threshold: u32,
    open_for: Duration,
    half_open_probe: bool,
  ) -> Self {
    Self {
      circuit,
      failure_threshold,
      open_for,
      half_open_probe,
      completed: false,
    }
  }

  fn succeed(&mut self) {
    let mut state = lock_recovering_poison(&self.circuit);
    if self.half_open_probe || matches!(*state, CircuitState::Closed { .. }) {
      *state = CircuitState::Closed {
        consecutive_failures: 0,
      };
    }
    self.completed = true;
  }

  fn fail(&mut self) {
    let mut state = lock_recovering_poison(&self.circuit);
    if self.half_open_probe {
      *state = CircuitState::Open {
        retry_at: Instant::now() + self.open_for,
      };
    } else if let CircuitState::Closed {
      consecutive_failures,
    } = *state
    {
      let failures = consecutive_failures.saturating_add(1);
      *state = if failures >= self.failure_threshold {
        CircuitState::Open {
          retry_at: Instant::now() + self.open_for,
        }
      } else {
        CircuitState::Closed {
          consecutive_failures: failures,
        }
      };
    }
    self.completed = true;
  }

  fn complete_without_circuit_failure(&mut self) {
    let mut state = lock_recovering_poison(&self.circuit);
    if self.half_open_probe {
      *state = CircuitState::Closed {
        consecutive_failures: 0,
      };
    }
    self.completed = true;
  }

  fn abandon(&mut self) {
    let mut state = lock_recovering_poison(&self.circuit);
    if self.half_open_probe {
      *state = CircuitState::Open {
        retry_at: Instant::now() + self.open_for,
      };
    }
    self.completed = true;
  }
}

impl Drop for CircuitAdmission {
  fn drop(&mut self) {
    if self.completed || !self.half_open_probe {
      return;
    }

    let mut state = lock_recovering_poison(&self.circuit);
    if matches!(*state, CircuitState::HalfOpen) {
      *state = CircuitState::Open {
        retry_at: Instant::now() + self.open_for,
      };
    }
  }
}

pub(crate) fn status_failure(status: StatusCode, headers: &HeaderMap) -> ProviderAttemptError {
  let retry_after = retry_after(headers);
  match status.as_u16() {
    429 => ProviderAttemptError::RateLimited { retry_after },
    500 | 502 | 503 | 504 => ProviderAttemptError::ServerError {
      status: status.as_u16(),
      retry_after,
    },
    _ => ProviderAttemptError::NonRetryableStatus {
      status: status.as_u16(),
    },
  }
}

pub(crate) fn transport_failure(error: &reqwest::Error) -> ProviderAttemptError {
  if error.is_timeout() {
    ProviderAttemptError::Timeout
  } else if error.is_connect() {
    ProviderAttemptError::Connect
  } else {
    ProviderAttemptError::Transport
  }
}

pub(crate) fn response_failure(error: &reqwest::Error) -> ProviderAttemptError {
  if error.is_timeout() {
    ProviderAttemptError::Timeout
  } else {
    ProviderAttemptError::InvalidEnvelope
  }
}

fn retry_after(headers: &HeaderMap) -> Option<Duration> {
  let value = headers.get(RETRY_AFTER)?.to_str().ok()?.trim();
  if let Ok(seconds) = value.parse::<u64>() {
    return Some(Duration::from_secs(seconds));
  }

  let retry_at = httpdate::parse_http_date(value).ok()?;
  Some(
    retry_at
      .duration_since(SystemTime::now())
      .unwrap_or_default(),
  )
}

fn duration_millis(duration: Duration) -> u64 {
  duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn lock_recovering_poison<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
  match mutex.lock() {
    Ok(guard) => guard,
    Err(poisoned) => poisoned.into_inner(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn policy() -> ProviderPolicy {
    ProviderPolicy::new(
      Duration::from_secs(1),
      1,
      Duration::from_millis(25),
      Duration::from_secs(1),
      1,
      2,
      Duration::from_millis(25),
    )
    .unwrap()
  }

  #[test]
  fn classifies_only_selected_transient_statuses_for_retries() {
    let headers = HeaderMap::new();
    assert!(status_failure(StatusCode::TOO_MANY_REQUESTS, &headers).is_retryable());
    assert!(status_failure(StatusCode::INTERNAL_SERVER_ERROR, &headers).is_retryable());
    assert!(status_failure(StatusCode::BAD_GATEWAY, &headers).is_retryable());
    assert!(status_failure(StatusCode::SERVICE_UNAVAILABLE, &headers).is_retryable());
    assert!(status_failure(StatusCode::GATEWAY_TIMEOUT, &headers).is_retryable());
    assert!(!status_failure(StatusCode::BAD_REQUEST, &headers).is_retryable());
    assert!(!status_failure(StatusCode::NOT_IMPLEMENTED, &headers).is_retryable());
  }

  #[test]
  fn retry_after_accepts_seconds_and_http_dates() {
    let mut seconds = HeaderMap::new();
    seconds.insert(RETRY_AFTER, "3".parse().unwrap());
    assert_eq!(retry_after(&seconds), Some(Duration::from_secs(3)));

    let mut past = HeaderMap::new();
    past.insert(
      RETRY_AFTER,
      "Sun, 06 Nov 1994 08:49:37 GMT".parse().unwrap(),
    );
    assert_eq!(retry_after(&past), Some(Duration::ZERO));
  }

  #[tokio::test]
  async fn circuit_opens_after_configured_transient_failures() {
    let resilience = ProviderResilience::new("test_provider", policy());
    for _ in 0..2 {
      assert_eq!(
        resilience
          .execute("test", || async {
            Err::<(), _>(ProviderAttemptError::Timeout)
          })
          .await,
        Err(ProviderCallError::RequestFailed)
      );
    }

    assert_eq!(
      resilience
        .execute("test", || async { Ok::<(), ProviderAttemptError>(()) })
        .await,
      Err(ProviderCallError::CircuitOpen)
    );
    assert_eq!(resilience.metrics().snapshot().circuit_open, 1);
  }

  #[tokio::test]
  async fn successful_half_open_probe_closes_the_circuit() {
    let policy = ProviderPolicy::new(
      Duration::from_secs(1),
      0,
      Duration::ZERO,
      Duration::from_secs(1),
      1,
      1,
      Duration::from_millis(10),
    )
    .unwrap();
    let resilience = ProviderResilience::new("test_provider", policy);
    assert_eq!(
      resilience
        .execute("test", || async {
          Err::<(), _>(ProviderAttemptError::Timeout)
        })
        .await,
      Err(ProviderCallError::RequestFailed)
    );

    sleep(Duration::from_millis(15)).await;
    assert_eq!(
      resilience
        .execute("test", || async { Ok::<(), ProviderAttemptError>(()) })
        .await,
      Ok(())
    );
    assert_eq!(
      resilience
        .execute("test", || async { Ok::<(), ProviderAttemptError>(()) })
        .await,
      Ok(())
    );
  }
}
