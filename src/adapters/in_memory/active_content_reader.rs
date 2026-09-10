//! Fixed in-memory active-content reader for tests and local composition.

use async_trait::async_trait;

use crate::{
  domain::canonical::ActiveContentVersion,
  ports::active_content_reader::{ActiveContentReader, ActiveContentReaderError},
};

/// Immutable process-local active-content selection for tests and local development.
///
/// This adapter is intentionally a fixed read model. It does not stage, publish, roll back, or
/// reconcile content and is not a production active-content pointer.
#[derive(Debug, Clone, Default)]
pub struct InMemoryActiveContentReader {
  content: Option<ActiveContentVersion>,
}

impl InMemoryActiveContentReader {
  /// Creates a reader with the supplied optional active immutable content tuple.
  pub fn new(content: Option<ActiveContentVersion>) -> Self {
    Self { content }
  }

  /// Creates a reader that always returns one supplied active immutable content tuple.
  pub fn with_content(content: ActiveContentVersion) -> Self {
    Self::new(Some(content))
  }
}

#[async_trait]
impl ActiveContentReader for InMemoryActiveContentReader {
  async fn active_content_version(
    &self,
  ) -> Result<Option<ActiveContentVersion>, ActiveContentReaderError> {
    Ok(self.content.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::canonical::CanonicalId;

  #[tokio::test]
  async fn returns_the_fixed_optional_content_tuple_without_mutating_it() {
    let content = ActiveContentVersion {
      release_id: CanonicalId::new("release-1").unwrap(),
      vector_collection_id: CanonicalId::new("vectors-1").unwrap(),
      schema_version: "canonical-v1".to_string(),
      ranking_version: "rank-v1".to_string(),
    };
    let reader = InMemoryActiveContentReader::with_content(content.clone());

    assert_eq!(
      reader.active_content_version().await.unwrap(),
      Some(content)
    );
    assert_eq!(
      InMemoryActiveContentReader::new(None)
        .active_content_version()
        .await
        .unwrap(),
      None
    );
  }
}
