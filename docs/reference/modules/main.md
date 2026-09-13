# Main launcher

中文：[主启动器](../../../docs_cn/reference/modules/main_cn.md)

`src/main.rs` owns the online process entry point. Operators and contributors should use this page to understand what may happen before bootstrap receives control.

Status: partially implemented. The current launcher reads configuration, initializes logging, composes model-only services, binds loopback TCP, and handles Ctrl-C or termination. The target launcher delegates composition to `bootstrap`, reports fatal startup failure without exposing secrets, and selects the process exit status.

## Boundary

Main parses no request or business input and contains no translation, retrieval, persistence, or publication policy. It invokes one bootstrap path for the online service; an offline publisher, if owned here, uses a separate binary and composition.

The target success path is `main -> bootstrap -> bounded accept loop -> graceful shutdown`. Startup errors retain operational context while excluding credentials, request text, provider bodies, and canonical content.

## Verification

Launcher changes require startup-failure and signal-shutdown coverage in addition to the repository checks. The current behavior lives in [`src/main.rs`](../../../src/main.rs); target composition is defined by the [service module reference](../modules.md) and [system design](../../transnet.md).

