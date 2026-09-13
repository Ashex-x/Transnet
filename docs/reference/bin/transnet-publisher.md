# Transnet publisher launcher

中文：[Transnet 发布器启动器](../../../docs_cn/reference/bin/transnet-publisher_cn.md)

This page defines separate offline process composition for canonical content publication at `src/bin/transnet-publisher.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

If owned by this repository, target `src/bin/transnet-publisher.rs` validates publisher configuration and authorization, constructs mutation-capable island-port adapters, invokes the publication library, reports safe stage outcomes, and chooses process exit status. It is never linked into the online service bootstrap and never accepts live request text or history. The launcher and publication pipeline are not implemented in the current runtime.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

