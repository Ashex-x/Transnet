# Semantic scale

中文：[语义尺度](../../../../docs_cn/reference/modules/domain/semantic_scale_cn.md)

This page defines first-class ordered intensity or degree dimensions at `src/domain/semantic_scale.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

A scale has stable identity, a named dimension and direction, conditions, domains, evidence, release, and ordered sense-qualified members. Positions express order, not equal numeric distance. Adjacent `lower_degree_than` or `higher_degree_than` edges may be derived for retrieval, but a scale is never a taxonomy or synonym alias.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

