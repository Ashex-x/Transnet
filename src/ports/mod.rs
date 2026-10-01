//! Interfaces between application logic and external systems.

/// Atomic read-only authority for the active canonical and retrieval projection tuple.
pub mod active_knowledge_release;
/// Request-scoped canonical-only release and read capability.
pub mod canonical_read;
/// UTC time source used by application and infrastructure code.
pub mod clock;
/// Outbound-only immutable knowledge-publication capability.
pub mod knowledge_publication;
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
/// Canonical root resolution and request-local lexical query encoding.
pub mod root_retrieval;
