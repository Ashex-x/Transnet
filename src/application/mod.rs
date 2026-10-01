//! Application use cases.

/// Request-scoped canonical retrieval and deterministic card assembly without query caching.
pub mod canonical_lookup;
/// Bounded deterministic canonical lookup-card assembly without HTTP or model generation.
pub mod canonical_lookup_card;
/// Request-local production canonical-only read composition.
pub mod canonical_read;
/// Release-pinned bounded canonical sense-detail reads without HTTP or model generation.
pub mod canonical_sense_details;
/// Content-release staging, validation, publication, rollback, and source quarantine.
pub mod content_release;
/// Typed bounded canonical graph reads and neighbor expansion.
pub mod graph;
/// Public bounded graph-topology snapshot caching with version-pinned cache keys.
pub mod graph_topology_cache;
/// Deterministic release-pinned knowledge projection preparation.
pub mod knowledge_projection;
/// Resumable node-first knowledge publication orchestration without activation.
pub mod knowledge_publication;
/// Structured lookup orchestration.
pub mod lookup;
/// Bounded response-neutral delivery of closed metric events.
pub mod observability;
/// Explicit offline publication and release-control composition.
pub mod offline_publication;
/// Canonical hybrid retrieval and lexical-only fallback.
pub mod retrieval;
/// Unified request-local translation orchestration and automatic intent routing.
pub mod translation;
