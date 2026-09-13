# Resilience policies

中文：[容错策略](../../docs_cn/reference/resilience_cn.md)

`src/resilience.rs` owns bounded deadlines, concurrency, retries, and circuit breakers for outbound provider calls and target idempotent data reads.

Status: partially implemented. The current module applies independent provider policies and exposes safe aggregate counters. Target island-port structured and vector reads must adopt the same bounded principles without treating mutation as retryable.

## Policy

Each dependency has an independent bulkhead and circuit. A logical request receives a bounded attempt timeout and retry count; a valid `Retry-After` may replace the configured delay but cannot exceed its cap. Only documented transient outcomes are retried, and the caller's request deadline always bounds all attempts and backoff.

Open circuits and full bulkheads fail promptly. Half-open probing admits only bounded work. Retries are allowed only for idempotent operations; publication mutations require their owning workflow's explicit idempotency contract and are outside the online composition.

Telemetry records static dependency and operation names, attempt number, outcome class, status when safe, elapsed time, and retry delay. It excludes request text, history, provider bodies, canonical text, vectors, credentials, and identity.

## Verification

Use paused-time tests for retry budgets, `Retry-After`, state transitions, concurrency permits, cancellation, and deadline exhaustion. Configuration details are in the [configuration guide](../guides/configuration.md), and failure mapping belongs to the [problem module](api/problem.md).

