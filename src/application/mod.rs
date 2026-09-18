//! Application use cases.

/// Request-scoped canonical retrieval and deterministic card assembly without query caching.
pub mod canonical_lookup;
/// Bounded deterministic canonical lookup-card assembly without HTTP or model generation.
pub mod canonical_lookup_card;
/// Release-pinned bounded canonical sense-detail reads without HTTP or model generation.
pub mod canonical_sense_details;
/// Content-release staging, validation, publication, rollback, and source quarantine.
pub mod content_release;
/// Typed bounded canonical graph reads and neighbor expansion.
pub mod graph;
/// Public bounded graph-topology snapshot caching with version-pinned cache keys.
pub mod graph_topology_cache;
/// Structured lookup orchestration.
pub mod lookup;
/// Bounded response-neutral delivery of closed metric events.
pub mod observability;
/// Canonical hybrid retrieval and lexical-only fallback.
pub mod retrieval;
/// Unified request-local translation orchestration and automatic intent routing.
pub mod translation;
