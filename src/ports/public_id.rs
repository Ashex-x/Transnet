//! Opaque application-generated public identifiers.

use std::{fmt, str::FromStr};

use thiserror::Error;
use ulid::Ulid;

/// A canonical, 26-character ULID used in external contracts.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicId(String);

impl PublicId {
  /// Parses a canonical, uppercase ULID string.
  ///
  /// # Errors
  ///
  /// Returns [`PublicIdError::InvalidFormat`] when `value` is not a canonical ULID.
  pub fn parse(value: &str) -> Result<Self, PublicIdError> {
    let parsed = Ulid::from_string(value).map_err(|_| PublicIdError::InvalidFormat)?;
    if parsed.to_string() != value {
      return Err(PublicIdError::InvalidFormat);
    }

    Ok(Self(value.to_owned()))
  }

  /// Creates a public ID from a validated ULID value.
  pub fn from_ulid(value: Ulid) -> Self {
    Self(value.to_string())
  }

  /// Returns the canonical ULID text used in external contracts.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for PublicId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.debug_tuple("PublicId").field(&self.0).finish()
  }
}

impl fmt::Display for PublicId {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl From<Ulid> for PublicId {
  fn from(value: Ulid) -> Self {
    Self::from_ulid(value)
  }
}

impl FromStr for PublicId {
  type Err = PublicIdError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    Self::parse(value)
  }
}

impl TryFrom<&str> for PublicId {
  type Error = PublicIdError;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Self::parse(value)
  }
}

/// Validation failure for a public ID supplied to application code.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PublicIdError {
  /// The value was not canonical uppercase ULID text.
  #[error("public ID must be a canonical ULID")]
  InvalidFormat,
}

/// Failure returned while creating an application-generated public ID.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PublicIdGenerationError {
  /// A deterministic test generator has no IDs left to return.
  #[error("public ID generator is exhausted")]
  Exhausted,
  /// The monotonic ULID random component cannot be incremented further.
  #[error("public ID generator exhausted its monotonic ULID range")]
  MonotonicOverflow,
}

/// Creates opaque public IDs independently of transport handlers and persistence adapters.
pub trait PublicIdGenerator: Send + Sync {
  /// Generates one new public ID.
  ///
  /// # Errors
  ///
  /// Returns an error only when the generator cannot produce another unique ID.
  fn generate(&self) -> Result<PublicId, PublicIdGenerationError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_canonical_ulids() {
    let id = PublicId::parse("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();

    assert_eq!(id.as_str(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
  }

  #[test]
  fn rejects_noncanonical_ulids() {
    assert_eq!(
      PublicId::parse("01arz3ndektsv4rrffq69g5fav"),
      Err(PublicIdError::InvalidFormat)
    );
  }
}
