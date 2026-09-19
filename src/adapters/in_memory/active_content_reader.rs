//! Fixed in-memory active canonical release reader for tests and local composition.

use async_trait::async_trait;

use crate::{
  domain::canonical::CanonicalReleasePin,
  ports::active_content_reader::{ActiveContentReader, ActiveContentReaderError},
};

/// Immutable process-local canonical release selection for tests and local development.
///
/// This adapter is intentionally a fixed read model. It does not stage, publish, roll back, or
/// reconcile content and is not a production active-content pointer.
#[derive(Debug, Clone, Default)]
pub struct InMemoryActiveContentReader {
  content: Option<CanonicalReleasePin>,
}

impl InMemoryActiveContentReader {
  /// Creates a reader with the supplied optional active immutable content tuple.
  pub fn new(content: Option<CanonicalReleasePin>) -> Self {
    Self { content }
  }

  /// Creates a reader that always returns one supplied active immutable release pin.
  pub fn with_content(content: CanonicalReleasePin) -> Self {
    Self::new(Some(content))
  }
}

#[async_trait]
impl ActiveContentReader for InMemoryActiveContentReader {
  async fn active_release_pin(
    &self,
  ) -> Result<Option<CanonicalReleasePin>, ActiveContentReaderError> {
    Ok(self.content.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::CanonicalId;

  #[tokio::test]
  async fn returns_the_fixed_optional_content_tuple_without_mutating_it() {
    let content = CanonicalReleasePin::new(
      CanonicalId::new("release-1").unwrap(),
      "canonical-v1".to_string(),
    )
    .unwrap();
    let reader = InMemoryActiveContentReader::with_content(content.clone());

    assert_eq!(reader.active_release_pin().await.unwrap(), Some(content));
    assert_eq!(
      InMemoryActiveContentReader::new(None)
        .active_release_pin()
        .await
        .unwrap(),
      None
    );
  }
}
