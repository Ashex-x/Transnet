# Request

中文：[请求](../../../../docs_cn/reference/modules/domain/request_cn.md)

This page defines the request-lifetime translation input and its privacy boundary at `src/domain/request.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The aggregate carries current text, requested source and target languages, response level, and an optional chronological list of minimal prior translation turns. History is immutable request context, oldest first, and has no independent item-count limit; the transport body limit bounds it. All allocations are discarded with the request and must never reach durable storage, logs, metrics, traces, vectors, or cache keys.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

