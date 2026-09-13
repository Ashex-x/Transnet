# Knowledge

中文：[知识](../../../../docs_cn/reference/modules/domain/knowledge_cn.md)

This page defines atomic, evidence-addressable canonical facts at `src/domain/knowledge.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

A fact has stable identity, one typed statement, scope and conditions, evidence and provenance references, verification state, immutable revision, and release identity. Structured data or a signed release artifact is authoritative. Vector records are searchable projections and model output is never evidence.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

