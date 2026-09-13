# Logging

中文：[日志](../../../docs_cn/reference/observability/logging_cn.md)

This page defines structured process logging with a strict content-redaction boundary at `src/observability/logging.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: partial current foundation; the target boundary and production composition are not complete.

## Contract

The current `src/logger.rs` initializes one process-wide tracing subscriber, replaces the build-mode log file at startup, honors `RUST_LOG`, and emits JSON or compact text. The target module retains safe operational fields while prohibiting request and canonical content, provider bodies, vectors, capabilities, and credentials. Logging failure at startup is fatal; request paths must not improvise content-bearing diagnostics.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

