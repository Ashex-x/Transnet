//! Infrastructure adapters.

/// System and fixed UTC clock implementations.
pub mod clock;
/// Deterministic in-memory implementations of platform ports.
pub mod in_memory;
/// Test-only deterministic canonical repository and vector adapter.
pub mod in_memory_retrieval;
/// OpenAI-compatible structured learning-model client.
pub mod learning_model;
/// ULID-backed and deterministic public-ID implementations.
pub mod public_id;
