# Release

中文：[发布](../../../../docs_cn/reference/modules/domain/release_cn.md)

This page defines one compatible immutable canonical-data snapshot and its degradation state at `src/domain/release.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target release identity pins one MySQL card release and the matching immutable Qdrant node and edge collections. A request uses one compatible trio throughout orchestration. Qdrant loss may degrade to MySQL-only content; absent or incompatible authoritative structured data fails canonical reads safely.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

