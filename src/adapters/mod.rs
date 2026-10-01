//! Infrastructure adapters.

/// System and fixed UTC clock implementations.
pub mod clock;
/// Deterministic in-memory implementations of platform ports.
pub mod in_memory;
/// Test-only deterministic canonical repository and vector adapter.
pub mod in_memory_retrieval;
/// Strict outbound island-port canonical-read client.
pub mod island_port;
/// Strict outbound island-port knowledge-publication client.
pub mod island_port_publication;
/// OpenAI-compatible structured lexical-model client.
pub mod learning_model;
/// Provider-neutral OpenAI-compatible generation and ephemeral embedding adapters.
pub mod model_runtime;
/// ULID-backed and deterministic public-ID implementations.
pub mod public_id;
