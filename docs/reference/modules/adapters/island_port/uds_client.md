# Island-port UDS client

中文：[Island-port UDS 客户端](../../../../../docs_cn/reference/modules/adapters/island_port/uds_client_cn.md)

This page defines bounded HTTP/1.1 JSON transport to island-port's owned Unix socket at `src/adapters/island_port/uds_client.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target client owns socket connection lifecycle, common request context, deadlines, body limits, schema-version outcomes, and safe transport errors. Filesystem credentials authorize callers; JSON must not carry forwarded user credentials. It does not expose SQL, Qdrant-native requests, or direct database drivers.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../../modules.md)
- [System design](../../../../transnet.md)
- [Transnet service interface](../../../../interfaces/transnet.md)
- [Content publishing](../../../../guides/content-publishing.md)

