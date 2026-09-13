# Runtime module

中文：[运行时模块](../../docs_cn/reference/runtime_cn.md)

The runtime module owns process configuration, startup, dependency composition, readiness, listener lifetime, and graceful shutdown. It contains lifecycle wiring, not translation or retrieval policy.

Status: the current executable binds transitional loopback TCP. The target composition uses the UDS boundary defined by the [Transnet service interface](../interfaces/transnet.md).

## Configuration

The process reads config/transnet.toml relative to the Cargo manifest. RUST_LOG may override the configured filter. Parsing rejects unknown fields, and validation checks individual bounds and incompatible combinations before creating clients or listeners.

Provider credentials are secrets. They must never appear in checked-in files, Debug output, logs, metrics, traces, or errors. Code outside configuration and bootstrap receives typed settings rather than rereading files or environment variables. Exact fields, defaults, and target infrastructure settings belong to the [configuration guide](../guides/configuration.md).

## Startup and composition

The current entry point loads configuration, initializes redacted logging and provider clients, builds the transitional router, binds the listener, and waits for shutdown. The default executable composes health, translation, and legacy model-backed lookup. Other checked-in foundations are not necessarily production-composed.

The target bootstrap validates settings before side effects, constructs adapters outside-in, binds the owned Unix socket, registers only routes whose dependencies exist, and reports ready only after required dependencies are usable. Handlers and adapters receive explicit dependencies; they do not create global clients.

## Readiness and shutdown

Liveness reflects process health. Readiness reflects whether the service can safely accept work and may report required dependency failure without treating optional enrichment as fatal.

Shutdown stops admission, drains accepted work within its existing deadlines, closes clients, and removes only the socket inode owned by this process. It never deletes an unresolved path or replaces an active listener.

## Verification

Test defaults, unknown fields, override precedence, redaction, validation boundaries, fatal startup errors, dependency-sensitive route registration, readiness transitions, signals, bounded draining, and safe socket cleanup.
