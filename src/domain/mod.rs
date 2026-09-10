//! Business types and invariants independent of HTTP and providers.

/// Canonical lexical entities, source permissions, and evidence.
pub mod canonical;
/// Public-only canonical lookup snapshot cache contracts and privacy boundaries.
pub mod canonical_lookup_cache;
/// Immutable content-release staging, validation, publication, and rollback invariants.
pub mod content_release;
/// Version-pinned private usefulness and accuracy feedback values.
pub mod feedback;
/// Typed, evidence-backed graph topology and traversal limits.
pub mod graph;
/// Private saved graph-layout views and bounded presentation-only coordinates.
pub mod graph_view;
/// Private learner-owned profile, history, vocabulary, and privacy-plan values.
pub mod learner;
/// Bounded canonical lookup-card presentation values with assertion-level provenance.
pub mod lookup_card;
/// Closed, redacted metric names and categorical event dimensions.
pub mod observability;
/// Deterministic hybrid-retrieval values and candidate fusion.
pub mod retrieval;
/// Structured multilingual-to-English translation.
pub mod translation;
