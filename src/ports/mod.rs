//! Interfaces between application logic and external systems.

/// Narrow reader for the active immutable canonical-only release pin.
pub mod active_content_reader;
/// Atomic read-only authority for the active canonical and retrieval projection tuple.
pub mod active_knowledge_release;
/// Shared cache interface for rebuildable application results.
pub mod cache;
/// Request-scoped canonical-only release and read capability.
pub mod canonical_read;
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
/// Outbound-only immutable knowledge-publication capability.
pub mod knowledge_publication;
/// Structured learning-model interface.
pub mod learning_model;
/// Request-local live search, public fetch, and DNS boundaries.
pub mod live_retrieval;
/// Closed, redacted backend metric-event recording interface.
pub mod metrics;
/// Provider-neutral generation and ephemeral embedding operations.
pub mod model_runtime;
/// Opaque, application-generated public identifier interface.
pub mod public_id;
/// Offline-only activation submission and rollback-selection authority.
pub mod release_control;
/// Outbound-only storage-neutral retrieval-data operations.
pub mod retrieval_data;
/// Generic ranked-candidate retrieval interface.
pub mod retriever;
/// Canonical root resolution and request-local lexical query encoding.
pub mod root_retrieval;
/// Request-local connected-text and lexical-draft model operations.
pub mod translation_model;
/// Versioned vector-retrieval interface.
pub mod vector_retriever;
