//! Interfaces between application logic and external systems.

/// Narrow reader for the active immutable canonical-content tuple.
pub mod active_content_reader;
/// Shared cache interface for rebuildable application results.
pub mod cache;
/// Canonical lexical repository interface.
pub mod canonical_repository;
/// Release-pinned canonical sense-details repository interface.
pub mod canonical_sense_details_repository;
/// UTC time source used by application and infrastructure code.
pub mod clock;
/// Atomic content-release staging, publication, rollback, and source-quarantine interface.
pub mod content_release_repository;
/// Read-only canonical graph topology interface.
pub mod graph_repository;
/// Structured learning-model interface.
pub mod learning_model;
/// Closed, redacted backend metric-event recording interface.
pub mod metrics;
/// Opaque, application-generated public identifier interface.
pub mod public_id;
/// Generic ranked-candidate retrieval interface.
pub mod retriever;
/// Request-local connected-text and lexical-draft model operations.
pub mod translation_model;
/// Versioned vector-retrieval interface.
pub mod vector_retriever;
