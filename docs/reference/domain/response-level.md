# Response level

中文：[响应级别](../../../docs_cn/reference/domain/response-level_cn.md)

This page defines closed output breadth choices and deterministic projection policy identity at `src/domain/response_level.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The public values are `brief`, `standard`, and `full`. Projection removes fields and lower-value items from one validated superset through deterministic allowlists; it never changes selected facts, truth state, or release, and it must not hide a materially plausible meaning when that would mislead.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

