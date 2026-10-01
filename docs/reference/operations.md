# Operations module

中文：[运维模块](../../docs_cn/reference/operations_cn.md)

The operations module owns safe observability, dependency resilience, and offline publication. Online telemetry and resilience participate in every request; publication remains a separate, unreachable composition.

## Observability

The [observability contract](observability.md) owns the versioned event schema, allowed dimensions, prohibited content, sampling, buffering, audit boundaries, and verification. Logs, metrics, and traces use closed content-free fields; audit events are restricted to offline publication and control transitions. Telemetry failure never changes an online result.

Liveness reports process health. Readiness reports whether the service can safely accept work and distinguishes required dependencies from optional enrichment.

## Resilience

All downstream work consumes one caller deadline; individual timeouts cannot extend it. Retries are bounded, jittered, and limited to explicitly retryable idempotent operations. Concurrency limits and circuit breakers are independent per dependency. Permits are released on cancellation and every error path.

## Offline publication

The publisher stages structured content, validates schema and rights, checks evidence and deterministic invariants, writes authoritative revisions, projects immutable vector collections, reconciles exact counts and release identifiers, evaluates quality gates, and activates a compatible release atomically. Quarantine, removal, and rollback preserve auditability.

The publisher is a separate composition root and the only component allowed mutation-capable data ports. Generated candidates are not evidence; they become canonical only after evidence, rights, validation, and review policies succeed.

Live translation and lookup never invoke publication, create durable proposals, or write request text or output to canonical storage. Operational procedures belong to the [content-publishing guide](../guides/content-publishing.md).

## Verification

Test the complete observability contract, deadline budgets, retry classification, cancellation, permit release, circuit transitions, readiness degradation, publication reconciliation, activation gates, quarantine, rollback, and the absence of any online path to mutation.
