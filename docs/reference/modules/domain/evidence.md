# Evidence

中文：[证据](../../../../docs_cn/reference/modules/domain/evidence_cn.md)

This page defines rights-aware support and display-safe provenance for canonical content at `src/domain/evidence.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

Evidence records source identity, rights and display policy, support status, scope, and safe provenance. It distinguishes whether a source supports a claim from whether the stored item is canonically verified. Removal and supersession preserve lineage through new immutable revisions rather than mutating history.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

