//! System and deterministic UTC clock adapters.

use std::{
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
  time::{Duration, SystemTime},
};

use crate::ports::clock::{Clock, UtcTimestamp};

/// Clock adapter backed by the host's UTC system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
  fn now(&self) -> UtcTimestamp {
    SystemTime::now()
  }
}

/// Shareable clock whose UTC instant can be advanced deterministically in tests.
#[derive(Debug, Clone)]
pub struct FixedClock {
  now: Arc<RwLock<UtcTimestamp>>,
}

impl FixedClock {
  /// Creates a fixed clock at `now`.
  pub fn new(now: UtcTimestamp) -> Self {
    Self {
      now: Arc::new(RwLock::new(now)),
    }
  }

  /// Replaces the current UTC instant.
  pub fn set(&self, now: UtcTimestamp) {
    *write_lock(&self.now) = now;
  }

  /// Advances the clock and returns the new UTC instant, if it is representable.
  pub fn advance(&self, duration: Duration) -> Option<UtcTimestamp> {
    let mut now = write_lock(&self.now);
    let advanced = now.checked_add(duration)?;
    *now = advanced;
    Some(advanced)
  }
}

impl Clock for FixedClock {
  fn now(&self) -> UtcTimestamp {
    *read_lock(&self.now)
  }
}

pub(crate) fn read_lock<Value>(lock: &RwLock<Value>) -> RwLockReadGuard<'_, Value> {
  match lock.read() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

pub(crate) fn write_lock<Value>(lock: &RwLock<Value>) -> RwLockWriteGuard<'_, Value> {
  match lock.write() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fixed_clock_advances_without_waiting() {
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let clock = FixedClock::new(start);

    assert_eq!(
      clock.advance(Duration::from_secs(5)),
      Some(start + Duration::from_secs(5))
    );
    assert_eq!(clock.now(), start + Duration::from_secs(5));
  }
}
