# Transport and API boundary

中文：[传输与 API 边界](../../docs_cn/reference/transport_cn.md)

This module owns request admission and mapping between HTTP/JSON and application operations. The [Transnet service interface](../interfaces/transnet.md) is the normative source for routes, bodies, envelopes, identifiers, deadlines, and errors.

## Current runtime

The executable currently binds loopback TCP and exposes transitional routes. Existing versioned handlers and problem responses are foundations; they do not implement the target UDS contract unless route registration, composition, and contract tests say so.

## Target server

The target listener serves HTTP/1.1 JSON on one owned Unix socket. Admission enforces media type, UTF-8, body size, unknown-field rejection, request ID, deadline, and concurrency limits before application work starts. Stale-socket recovery must distinguish an abandoned inode from an active listener.

Middleware order is deterministic: identify the route, establish safe request context, apply limits and deadlines, invoke the handler, map failures, and record content-free telemetry. Cancellation and permits must be released on every exit path.

## Handler rule

Probe, translation, sense, and graph handlers are thin. They decode and validate wire shapes, call one application operation, and encode the documented result. They do not select models, infer domains, construct database queries, traverse graphs, or persist request data.

## Verification

Contract tests cover exact routes and methods, strict JSON, boundary sizes, unknown fields, request-ID behavior, timeout and cancellation, safe error mapping, optional dependency registration, UDS ownership, and absence of a TCP listener after migration.
