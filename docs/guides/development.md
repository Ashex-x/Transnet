# Development and operations

中文：[开发与运维](../../docs_cn/guides/development_cn.md)

The root [README](../../README.md) owns prerequisites, configuration basics, build commands, verification commands, local startup, and curl examples.

Validate fenced JSON examples, local Markdown links, and English/Chinese heading parity with `python3 tools/validate_docs.py`. Run the validator together with the Rust verification commands before submitting documentation changes.

## Operations

Run the release binary under a process supervisor and preserve `logs/release/transnet.log`. The non-blocking logger replaces that file at each process start. The process handles Ctrl-C and Unix termination for graceful shutdown, and records startup, shutdown, request outcomes, provider resilience events, and fatal server errors through `tracing`.

The executable accepts HTTP/1.1 only through its owned Unix socket and can optionally compose outbound island-port canonical and knowledge clients. The island-port server, production MySQL/Qdrant execution, migrations and publisher, real activation/rollback, old-release retention, and production end-to-end acceptance remain external work. Request bodies are bounded and request IDs are propagated.

Related: [configuration](configuration.md), [design](../transnet.md), and [Transnet service interface](../interfaces/transnet.md).
