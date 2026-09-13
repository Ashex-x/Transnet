# Operations module

中文：[运维模块](../../../docs_cn/reference/operations/operations_cn.md)

The operations module owns safe observability, dependency resilience, and offline publication. Online telemetry and resilience participate in every request; publication remains a separate, unreachable composition.

## Observability

Logs, metrics, and traces may contain operation names, matched route templates, safe status classes, latency, bounded retry counts, circuit state, and coarse payload-size buckets. They must not contain request text, history, translations, prompts, model output, credentials, provider bodies, canonical content bodies, capabilities, or stable user-correlatable identifiers. Metric labels are closed and low-cardinality.

Liveness reports process health. Readiness reports whether the service can safely accept work and distinguishes required dependencies from optional enrichment.

## Resilience

All downstream work consumes one caller deadline; individual timeouts cannot extend it. Retries are bounded, jittered, and limited to explicitly retryable idempotent operations. Concurrency limits and circuit breakers are independent per dependency. Permits are released on cancellation and every error path.

## Offline publication

The publisher stages structured content, validates schema and rights, checks evidence and deterministic invariants, writes authoritative revisions, projects immutable vector collections, reconciles exact counts and release identifiers, evaluates quality gates, and activates a compatible release atomically. Quarantine, removal, and rollback preserve auditability.

The publisher is a separate composition root and the only component allowed mutation-capable data ports. Generated candidates are not evidence; they become canonical only after evidence, rights, validation, and review policies succeed.

Live translation and lookup never invoke publication, create durable proposals, or write request text or output to canonical storage. Operational procedures belong to the [content-publishing guide](../../guides/content-publishing.md).

## Verification

Test telemetry redaction, closed labels, deadline budgets, retry classification, cancellation, permit release, circuit transitions, readiness degradation, publication reconciliation, activation gates, quarantine, rollback, and the absence of any online path to mutation.
