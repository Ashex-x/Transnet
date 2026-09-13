# Vector data port

中文：[向量数据端口](../../../../docs_cn/reference/modules/ports/vector_data_cn.md)

This page defines release-filtered candidate retrieval over knowledge projections at `src/ports/vector_data.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target operations retrieve nodes, fact or edge candidates, bounded neighborhoods, and semantic-scale candidates. Results are pointers and eligibility metadata, not authoritative facts; factual content and evidence must be hydrated through structured data for the same release. The port exposes no generic Qdrant API or online mutation method.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

