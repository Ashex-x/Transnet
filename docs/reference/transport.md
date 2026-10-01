# Transport and API boundary

中文：[传输与 API 边界](../../docs_cn/reference/transport_cn.md)

This module owns request admission and mapping between HTTP/JSON and application operations. The [Transnet service interface](../interfaces/transnet.md) is the normative source for routes, bodies, envelopes, identifiers, deadlines, and errors.

## Current runtime

The executable binds only its owned Unix socket and exposes the target `/api/v1` operations with shared envelope and problem-response infrastructure. TCP, CORS, `POST /translate`, `/v1/lookups`, `/v1/senses/*`, and raw `/v1/graph*` routes are absent.

## Target server

The target listener serves HTTP/1.1 JSON on one owned Unix socket. Admission enforces media type, UTF-8, body size, unknown-field rejection, request ID, deadline, and concurrency limits before application work starts. Stale-socket recovery must distinguish an abandoned inode from an active listener.

Middleware order is deterministic: identify the route, establish safe request context, apply limits and deadlines, invoke the handler, map failures, and record content-free telemetry. Cancellation and permits must be released on every exit path.

The implemented admission middleware validates or generates one safe request ID and derives one absolute request deadline from optional `X-Deadline-At`, using a 30-second default and rejecting caller budgets beyond 120 seconds. It inserts the public request-safe context into request extensions before a handler runs. The context exposes only correlation, schema, deadline budget, and an optional immutable release ID; it contains no request body, identity, credentials, or arbitrary headers. The optional `AppState` knowledge-route bundle atomically registers both relative routes under `/api/v1`, activates matching capabilities and exact-snapshot readiness, and creates a request-owned cancellation guard. Dropping the request or draining the runtime cancels in-flight work. The executable injects this bundle only after optional knowledge configuration and the active release trio validate successfully.

## Handler rule

Probe, translation, BasicCard, sense-read, knowledge-view, and knowledge-path handlers are thin. They decode and validate wire shapes, call one application operation, and encode the documented result. They do not select models, infer domains, construct database queries, traverse graphs, or persist request data.

## Verification

Contract tests cover exact routes and methods, strict JSON, boundary sizes, unknown fields, request-ID behavior, timeout and cancellation, safe error mapping, optional dependency registration, UDS ownership, and absence of a TCP listener after migration.
