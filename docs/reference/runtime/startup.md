# Runtime startup

中文：[运行时启动](../../../docs_cn/reference/runtime/startup_cn.md)

This module owns process launch, dependency composition, readiness, listener lifetime, and graceful shutdown.

## Current runtime

[`src/main.rs`](../../../src/main.rs) loads configuration, initializes logging and providers, builds the transitional router, binds loopback TCP, and waits for shutdown. Failure before serving is fatal; Ctrl-C and Unix termination signals start bounded shutdown.

The default executable currently composes health, translation, and legacy model-backed lookup. Checked-in canonical, graph, release, and readiness foundations are not evidence that all target routes are production-composed.

## Target boundary

Bootstrap validates all settings before side effects, constructs adapters outside-in, binds the owned Unix socket, and reports ready only after required dependencies are usable. Shutdown stops admission, drains bounded in-flight work, closes clients, and removes only the socket inode owned by this process.

Startup owns composition, not business policy. Route handlers and adapters receive explicit dependencies; they do not create global clients or read configuration independently.

## Verification

Cover fatal configuration and bind failures, dependency-sensitive route registration, readiness transitions, signal handling, bounded draining, and safe socket cleanup. Configuration details belong to [runtime configuration](configuration.md), while socket rules belong to the [Transnet interface](../../interfaces/transnet.md).
