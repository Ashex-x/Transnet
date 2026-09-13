# Publication module

中文：[发布模块](../../../docs_cn/reference/publication/overview_cn.md)

This page defines reusable offline staging, validation, projection, reconciliation, activation, quarantine, and rollback logic at `src/publication/mod.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

This target library is the only composition allowed mutation-capable structured and vector ports. It builds authoritative MySQL content first, Qdrant projections second, reconciles identities and hashes, evaluates the exact release trio, and activates atomically. Generated candidates start quarantined; model provenance is not evidence, and rights review, deterministic validation, evidence or approved editorial policy, and reviewer approval precede activation.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

