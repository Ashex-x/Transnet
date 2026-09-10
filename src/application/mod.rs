//! Application use cases.

/// Typed bounded canonical graph reads and neighbor expansion.
pub mod graph;
/// Structured lookup orchestration.
pub mod lookup;
/// Private lookup-job lifecycle and deadline orchestration.
pub mod lookup_job;
/// Canonical hybrid retrieval and lexical-only fallback.
pub mod retrieval;
