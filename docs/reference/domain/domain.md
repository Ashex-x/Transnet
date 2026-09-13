# Domain

中文：[领域](../../../docs_cn/reference/domain/domain_cn.md)

This page defines canonical domain identity, scope, resolution outcome, and knowledge coverage at `src/domain/domain.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

Domain assessment may resolve to an allowlisted existing domain, a request-local `proposed_new` result, `general`, or `uncertain`. A proposal is not persisted or treated as canonical. Coverage profiles describe available fact families and honest `seed`, `partial`, or `curated` coverage for the pinned release.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

