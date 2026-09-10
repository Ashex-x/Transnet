//! Application use cases.

/// Canonical lookup composition through public snapshots and deterministic card assembly.
pub mod canonical_lookup;
/// Public canonical lookup snapshot caching with private-input bypasses.
pub mod canonical_lookup_cache;
/// Bounded deterministic canonical lookup-card assembly without HTTP or model generation.
pub mod canonical_lookup_card;
/// Release-pinned bounded canonical sense-detail reads without HTTP or model generation.
pub mod canonical_sense_details;
/// Content-release staging, validation, publication, rollback, and source quarantine.
pub mod content_release;
/// Private graph-feedback validation and projection orchestration.
pub mod feedback;
/// Typed bounded canonical graph reads and neighbor expansion.
pub mod graph;
/// Private saved graph-layout view orchestration.
pub mod graph_view;
/// Private learner state and privacy-plan orchestration.
pub mod learner_state;
/// Structured lookup orchestration.
pub mod lookup;
/// Private lookup-job lifecycle and deadline orchestration.
pub mod lookup_job;
/// Canonical hybrid retrieval and lexical-only fallback.
pub mod retrieval;
