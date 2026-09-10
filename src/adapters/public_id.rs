//! Production and deterministic adapters for application-generated public IDs.

use std::{
  collections::VecDeque,
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

use ulid::Generator;

use crate::ports::{
  clock::Clock,
  public_id::{PublicId, PublicIdGenerationError, PublicIdGenerator},
};

/// Monotonic ULID generator whose timestamp comes from an injected UTC clock.
#[derive(Clone)]
pub struct UlidPublicIdGenerator {
  clock: Arc<dyn Clock>,
  generator: Arc<Mutex<Generator>>,
}

impl UlidPublicIdGenerator {
  /// Creates a monotonic generator backed by `clock`.
  pub fn new(clock: Arc<dyn Clock>) -> Self {
    Self {
      clock,
      generator: Arc::new(Mutex::new(Generator::new())),
    }
  }
}

impl fmt::Debug for UlidPublicIdGenerator {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("UlidPublicIdGenerator")
  }
}

impl PublicIdGenerator for UlidPublicIdGenerator {
  fn generate(&self) -> Result<PublicId, PublicIdGenerationError> {
    let now = self.clock.now();
    let mut generator = mutex_lock(&self.generator);
    let id = generator
      .generate_from_datetime(now)
      .map_err(|_| PublicIdGenerationError::MonotonicOverflow)?;
    Ok(PublicId::from(id))
  }
}

/// Deterministic public-ID generator that returns a supplied sequence exactly once.
#[derive(Clone)]
pub struct SequencePublicIdGenerator {
  ids: Arc<Mutex<VecDeque<PublicId>>>,
}

impl SequencePublicIdGenerator {
  /// Creates a generator that returns `ids` in order.
  pub fn new(ids: impl IntoIterator<Item = PublicId>) -> Self {
    Self {
      ids: Arc::new(Mutex::new(ids.into_iter().collect())),
    }
  }
}

impl fmt::Debug for SequencePublicIdGenerator {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("SequencePublicIdGenerator")
  }
}

impl PublicIdGenerator for SequencePublicIdGenerator {
  fn generate(&self) -> Result<PublicId, PublicIdGenerationError> {
    mutex_lock(&self.ids)
      .pop_front()
      .ok_or(PublicIdGenerationError::Exhausted)
  }
}

pub(crate) fn mutex_lock<Value>(lock: &Mutex<Value>) -> MutexGuard<'_, Value> {
  match lock.lock() {
    Ok(guard) => guard,
    Err(error) => error.into_inner(),
  }
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, SystemTime};

  use ulid::Ulid;

  use super::*;
  use crate::adapters::clock::FixedClock;

  #[test]
  fn sequence_generator_returns_ids_in_order() {
    let first = PublicId::from(Ulid::from_parts(1, 1));
    let second = PublicId::from(Ulid::from_parts(1, 2));
    let generator = SequencePublicIdGenerator::new([first.clone(), second.clone()]);

    assert_eq!(generator.generate().unwrap(), first);
    assert_eq!(generator.generate().unwrap(), second);
    assert_eq!(
      generator.generate(),
      Err(PublicIdGenerationError::Exhausted)
    );
  }

  #[test]
  fn ulid_generator_uses_the_injected_clock() {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(42);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(time));
    let generator = UlidPublicIdGenerator::new(clock);

    let first = generator.generate().unwrap();
    let second = generator.generate().unwrap();
    let first_ulid = Ulid::from_string(first.as_str()).unwrap();

    assert_eq!(first_ulid.datetime(), time);
    assert!(first < second);
  }
}
