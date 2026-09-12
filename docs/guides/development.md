# Development and operations

中文：[开发与运维](../../docs_cn/guides/development_cn.md)

The root [README](../../README.md) owns prerequisites, configuration basics, build commands, verification commands, local startup, and curl examples.

## Operations

Run the release binary under a process supervisor and preserve `logs/release/transnet.log`. The non-blocking logger replaces that file at each process start. The process handles Ctrl-C and Unix termination for graceful shutdown, and records startup, shutdown, request outcomes, provider resilience events, and fatal server errors through `tracing`.

The current executable has no TLS termination and must remain loopback-only behind Island-port. It does not yet compose the target MySQL cards, Qdrant knowledge graph, scheduler, writing, or speech capabilities. Request bodies are bounded and request IDs are propagated; see the [configuration guide](configuration.md). Implement target capabilities behind typed boundaries and advance their status and contracts in the same change.

Related: [configuration](configuration.md), [design](../transnet.md), and [Island-port interface](../interfaces/port.md).
