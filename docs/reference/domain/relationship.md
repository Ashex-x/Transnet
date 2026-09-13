# Relationship

中文：[关系](../../../docs_cn/reference/domain/relationship_cn.md)

This page defines typed and directed relationships between sense-qualified knowledge nodes at `src/domain/relationship.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The registry defines each relation's direction, inverse, symmetry, transitivity, and causality instead of inferring properties from labels. Taxonomy uses child-to-parent `is_a` and inverse `has_subtype`; contrasts, grammar, terminology, technical facts, and derived degree comparisons remain distinct families. Every presented relationship retains applicable sense, domain, conditions, evidence state, provenance, and release.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

