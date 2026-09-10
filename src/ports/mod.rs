//! Interfaces between application logic and external systems.

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
/// Leased durable-work interface for API and worker coordination.
pub mod durable_job;
/// Canonical graph-feedback eligibility and private-feedback persistence interfaces.
pub mod graph_feedback;
/// Read-only canonical graph topology interface.
pub mod graph_repository;
/// Private saved graph-layout view persistence interface.
pub mod graph_view;
/// Scoped idempotency reservation and response-storage interface.
pub mod idempotency;
/// Private learner-profile, history, saved-vocabulary, and inventory interfaces.
pub mod learner_state;
/// Structured learning-model interface.
pub mod learning_model;
/// Private lookup-job lifecycle and polling interface.
pub mod lookup_job;
/// Closed, redacted backend metric-event recording interface.
pub mod metrics;
/// Atomic private scheduler-neutral practice-state persistence interface.
pub mod practice_state;
/// Opaque, application-generated public identifier interface.
pub mod public_id;
/// Generic record persistence interface.
pub mod repository;
/// Generic ranked-candidate retrieval interface.
pub mod retriever;
/// Versioned vector-retrieval interface.
pub mod vector_retriever;
