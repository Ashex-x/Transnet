# Structured data port

中文：[结构化数据端口](../../../../docs_cn/reference/modules/ports/structured_data_cn.md)

This page defines read-only application operations over authoritative canonical content at `src/ports/structured_data.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target runtime port groups operations for canonical translations, cards, senses, domain inventory and profiles, fact and evidence hydration, semantic scales, and active releases. Calls carry derived lookup forms or canonical IDs plus a release pin, never user identity or raw history. Mutation-capable publication interfaces are separate and unavailable to online composition.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

