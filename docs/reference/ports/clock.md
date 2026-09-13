# Clock port

中文：[时钟端口](../../../docs_cn/reference/ports/clock_cn.md)

This page defines UTC time access for deterministic deadlines and tests at `src/ports/clock.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: partial current foundation; the target boundary and production composition are not complete.

## Contract

The checked-in `Clock` trait currently returns a `SystemTime`-backed `UtcTimestamp`, with a system-clock adapter available in the repository. The target orchestration uses the abstraction for deadline accounting rather than reading wall time throughout business logic. It carries no request data.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

