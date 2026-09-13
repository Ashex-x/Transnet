# Transport middleware

中文：[传输中间件](../../../docs_cn/reference/transport/middleware_cn.md)

`src/transport/middleware.rs` is the target request-control layer between the UDS listener and versioned handlers.

Status: target module design. The current API contains some request-ID and body-limit behavior, but the complete UDS middleware stack is not implemented or composed.

## Order and safety

Middleware establishes or validates the opaque request ID, applies the server deadline, acquires a bounded concurrency permit, dispatches the static route, records a safe aggregate outcome, and maps uncaught failures to the closed problem envelope. Every response returns `X-Request-Id`.

The request deadline is never extended downstream. Capacity exhaustion fails promptly, cancellation releases permits, and panic or internal errors disclose no storage, provider, filesystem, or request details.

Metrics may contain request ID where policy permits, static endpoint template, outcome class, byte counts, and elapsed time. They must not contain raw paths with identifiers, bodies, current text, history, canonical text, vectors, credentials, or socket peer details.

## Verification

Tests should prove middleware ordering, request-ID propagation, deadline cancellation, permit release, safe failure mapping, and redacted telemetry. See the [problem module](../api/problem.md) and [Transnet interface](../../interfaces/transnet.md).

