//! Business types and invariants independent of HTTP and providers.

/// Canonical lexical entities, source permissions, and evidence.
pub mod canonical;
/// Bounded canonical lexical details with reviewed factual-evidence lineage.
pub mod canonical_content;
/// Reviewed canonical translation identity, fingerprint, scope, and revision invariants.
pub mod canonical_translation;
/// Content-free declarations of implemented service capabilities.
pub mod capabilities;
/// Immutable content-release staging, validation, publication, and rollback invariants.
pub mod content_release;
/// Release-pinned authoritative material and canonical embedding-input contracts.
pub mod embedding_input;
/// Typed, evidence-backed graph topology and traversal limits.
pub mod graph;
/// Deterministic pre-publication node and edge projection artifacts.
pub mod knowledge_projection;
/// Publication lifecycle, idempotency, manifest hashes, and execution receipts.
pub mod knowledge_publication;
/// Immutable canonical, node-collection, and edge-collection release-trio invariants.
pub mod knowledge_release;
/// Bounded canonical lookup-card presentation values with assertion-level provenance.
pub mod lookup_card;
/// Closed, redacted metric names and categorical event dimensions.
pub mod observability;
/// Request-scoped correlation, deadline, schema, and release context.
pub mod request_context;
/// Deterministic hybrid-retrieval values and candidate fusion.
pub mod retrieval;
/// Structured multilingual-to-English translation.
pub mod translation;
/// Request-local unified translation values, normalization, and response projection.
pub mod translation_turn;
