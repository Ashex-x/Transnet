//! Deterministic in-memory ranked retriever.

use std::{
  collections::HashMap,
  hash::Hash,
  sync::{Arc, RwLock},
};

use async_trait::async_trait;

use crate::{
  adapters::clock::{read_lock, write_lock},
  ports::retriever::{RetrievalError, Retriever},
};

/// Shareable retriever that returns preconfigured candidate order for exact test queries.
#[derive(Clone)]
pub struct InMemoryRetriever<Query, Candidate> {
  candidates: Arc<RwLock<HashMap<Query, Vec<Candidate>>>>,
}

impl<Query, Candidate> InMemoryRetriever<Query, Candidate> {
  /// Creates an empty deterministic retriever.
  pub fn new() -> Self {
    Self {
      candidates: Arc::new(RwLock::new(HashMap::new())),
    }
  }
}

impl<Query, Candidate> InMemoryRetriever<Query, Candidate>
where
  Query: Eq + Hash,
{
  /// Replaces the ranked candidates returned for `query`.
  pub fn set(&self, query: Query, candidates: Vec<Candidate>) {
    write_lock(&self.candidates).insert(query, candidates);
  }
}

impl<Query, Candidate> Default for InMemoryRetriever<Query, Candidate> {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<Query, Candidate> Retriever<Query, Candidate> for InMemoryRetriever<Query, Candidate>
where
  Query: Eq + Hash + Send + Sync + 'static,
  Candidate: Clone + Send + Sync + 'static,
{
  async fn retrieve(&self, query: &Query) -> Result<Vec<Candidate>, RetrievalError> {
    Ok(
      read_lock(&self.candidates)
        .get(query)
        .cloned()
        .unwrap_or_default(),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn preserves_configured_candidate_order() {
    let retriever = InMemoryRetriever::new();
    retriever.set("hola", vec!["hello", "hi"]);

    assert_eq!(
      retriever.retrieve(&"hola").await.unwrap(),
      vec!["hello", "hi"]
    );
    assert!(retriever.retrieve(&"absent").await.unwrap().is_empty());
  }
}
