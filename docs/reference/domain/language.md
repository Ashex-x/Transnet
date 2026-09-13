# Language

中文：[语言](../../../docs_cn/reference/domain/language_cn.md)

This page defines the closed product language selectors and canonical internal language-tag mapping at `src/domain/language.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

It accepts automatic source detection plus English and Simplified Chinese in the initial target contract. It normalizes display aliases at the boundary, while downstream domain values use `auto`, `en`, or `zh-CN` exactly. It does not perform detection or translation.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

