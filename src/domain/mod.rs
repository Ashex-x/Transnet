//! Business types and invariants independent of HTTP and providers.

/// Canonical lexical entities, source permissions, and evidence.
pub mod canonical;
/// Bounded canonical lexical details with reviewed factual-evidence lineage.
pub mod canonical_content;
/// Immutable content-release staging, validation, publication, and rollback invariants.
pub mod content_release;
/// Typed, evidence-backed graph topology and traversal limits.
pub mod graph;
/// Bounded canonical lookup-card presentation values with assertion-level provenance.
pub mod lookup_card;
/// Closed, redacted metric names and categorical event dimensions.
pub mod observability;
/// Deterministic hybrid-retrieval values and candidate fusion.
pub mod retrieval;
/// Structured multilingual-to-English translation.
pub mod translation;
