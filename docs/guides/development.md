# Development and operations

The root [README](../../README.md) owns prerequisites, configuration basics, build commands, verification commands, local startup, and curl examples.

## Operations

Run the release binary under a process supervisor and preserve `logs/release/transnet.log`. The non-blocking logger replaces that file at each process start. The process handles Ctrl-C and Unix termination for graceful shutdown, and records startup, shutdown, request outcomes, provider resilience events, and fatal server errors through `tracing`.

The current executable has no TLS termination and must remain loopback-only behind Island-port. MySQL and Qdrant wiring belongs to Island-port, not a future Transnet composition. Request bodies are bounded and request IDs are propagated; see the [configuration guide](configuration.md).

Related: [configuration](configuration.md), [design](../transnet.md), and [Island-port interface](../interfaces/port.md).
