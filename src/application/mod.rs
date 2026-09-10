//! Application use cases.

/// Content-release staging, validation, publication, rollback, and source quarantine.
pub mod content_release;
/// Typed bounded canonical graph reads and neighbor expansion.
pub mod graph;
/// Private learner state and privacy-plan orchestration.
pub mod learner_state;
/// Structured lookup orchestration.
pub mod lookup;
/// Private lookup-job lifecycle and deadline orchestration.
pub mod lookup_job;
/// Canonical hybrid retrieval and lexical-only fallback.
pub mod retrieval;
