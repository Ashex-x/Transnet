# Development and operations

中文：[开发与运维](../../docs_cn/guides/development_cn.md)

The root [README](../../README.md) owns prerequisites, configuration basics, build commands, verification commands, local startup, and curl examples.

## Operations

Run the release binary under a process supervisor and preserve `logs/release/transnet.log`. The non-blocking logger replaces that file at each process start. The process handles Ctrl-C and Unix termination for graceful shutdown, and records startup, shutdown, request outcomes, provider resilience events, and fatal server errors through `tracing`.

The current executable has no TLS termination and must remain loopback-only behind a gateway or service mesh. It can optionally compose the outbound-only island-port canonical-read client, active-release pinning, canonical readiness, public BasicCard lookup, and release-pinned sense follow-up. This is a client-side capability only: the island-port canonical server, production MySQL migrations and publisher, real activation and rollback, old-release retention verification, and island-port/MySQL end-to-end acceptance remain external work. Qdrant knowledge retrieval and the target inbound Transnet UDS listener are also not implemented. Request bodies are bounded and request IDs are propagated; see the [configuration guide](configuration.md).

Related: [configuration](configuration.md), [design](../transnet.md), and [Transnet service interface](../interfaces/transnet.md).
