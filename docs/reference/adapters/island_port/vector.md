# Island-port vector adapter

中文：[Island-port 向量适配器](../../../../docs_cn/reference/adapters/island_port/vector_cn.md)

This page defines vector-data port mapping onto island-port's versioned vector endpoints at `src/adapters/island_port/vector.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target runtime adapter sends release- and eligibility-filtered retrieval operations under `data/vec/v1`. Island-port owns collection selection, query construction, pooling, and Qdrant credentials. Returned candidates remain non-authoritative until same-release structured hydration; mutation is confined to the offline publisher composition.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

