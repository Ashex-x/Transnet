//! Business types and invariants independent of HTTP and providers.

/// Canonical lexical entities, source permissions, and evidence.
pub mod canonical;
/// Immutable content-release staging, validation, publication, and rollback invariants.
pub mod content_release;
/// Version-pinned private usefulness and accuracy feedback values.
pub mod feedback;
/// Typed, evidence-backed graph topology and traversal limits.
pub mod graph;
/// Private learner-owned profile, history, vocabulary, and privacy-plan values.
pub mod learner;
/// Deterministic hybrid-retrieval values and candidate fusion.
pub mod retrieval;
/// Structured multilingual-to-English translation.
pub mod translation;
