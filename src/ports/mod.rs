//! Interfaces between application logic and external systems.

/// Shared cache interface for rebuildable application results.
pub mod cache;
/// Canonical lexical repository interface.
pub mod canonical_repository;
/// UTC time source used by application and infrastructure code.
pub mod clock;
/// Leased durable-work interface for API and worker coordination.
pub mod durable_job;
/// Scoped idempotency reservation and response-storage interface.
pub mod idempotency;
/// Structured learning-model interface.
pub mod learning_model;
/// Opaque, application-generated public identifier interface.
pub mod public_id;
/// Generic record persistence interface.
pub mod repository;
/// Generic ranked-candidate retrieval interface.
pub mod retriever;
/// Versioned vector-retrieval interface.
pub mod vector_retriever;
