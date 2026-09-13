# Gemma 4 provider adapter

中文：[Gemma 4 提供方适配器](../../../../docs_cn/reference/adapters/providers/gemma4_cn.md)

This page defines short-text translation and bounded structured-composition policy for Gemma 4 at `src/adapters/providers/gemma4.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: partial current foundation; the target boundary and production composition are not complete.

## Contract

The current provider module routes text at or below the configured threshold to Gemma 4 and the current legacy lookup uses Gemma 4 strict JSON Schema output. The target adapter implements the translation-model port with versioned prompts and schemas, validates every response, and delegates wire mechanics to the shared OpenAI-compatible client.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

