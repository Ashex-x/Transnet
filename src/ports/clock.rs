//! Clock abstraction for UTC instants.

use std::time::SystemTime;

/// A UTC instant represented by the platform clock.
pub type UtcTimestamp = SystemTime;

/// Supplies the current UTC instant without coupling application logic to the system clock.
pub trait Clock: Send + Sync {
  /// Returns the current UTC instant.
  fn now(&self) -> UtcTimestamp;
}
