# Island-port SQL adapter

中文：[Island-port SQL 适配器](../../../../docs_cn/reference/adapters/island_port/sql_cn.md)

This page defines structured-data port mapping onto island-port's versioned SQL data endpoints at `src/adapters/island_port/sql.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target runtime adapter performs operation-specific read calls under `data/sql/v1` and preserves closed outcomes, release pins, and authoritative hydration semantics. Island-port owns MySQL queries, transactions, pooling, and credentials. Publication uses a separately authorized mutation-capable composition, never the online runtime port.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

