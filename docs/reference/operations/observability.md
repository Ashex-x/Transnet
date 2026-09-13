# Observability and resilience

中文：[可观测性与容错](../../../docs_cn/reference/operations/observability_cn.md)

This module owns safe logs, metrics, traces, timeouts, retries, concurrency limits, and circuit breakers.

## Safe telemetry

Telemetry may contain operation names, matched route templates, safe status classes, latency, bounded retry counts, circuit state, and coarse payload-size buckets. It must not contain request text, history, translations, prompts, model output, credentials, provider bodies, canonical content bodies, capabilities, or stable user-correlatable identifiers. Metric labels are closed and low-cardinality.

## Resilience

All downstream work consumes one caller deadline. Individual timeouts cannot extend it. Retries are bounded, jittered, and limited to explicitly retryable idempotent operations. Concurrency limits and circuit breakers are independent per dependency; permits are released on cancellation and every error path.

Liveness reports process health. Readiness reports whether the service can safely accept work and reflects required dependency state without treating optional enrichment as fatal.

## Verification

Test redaction by construction, closed labels, matched-route naming, timeout budgets, retry classification, cancellation, permit release, circuit transitions, half-open probes, and readiness degradation.
