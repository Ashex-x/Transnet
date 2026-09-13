# TranslateGemma provider adapter

中文：[TranslateGemma 提供方适配器](../../../../docs_cn/reference/adapters/providers/translate_gemma_cn.md)

This page defines longer connected-text translation policy for TranslateGemma at `src/adapters/providers/translate_gemma.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: partial current foundation; the target boundary and production composition are not complete.

## Contract

The current provider module selects TranslateGemma only when character count exceeds `translation.long_text_chars` and sends its structured message form. The target adapter keeps that role-specific request construction behind the translation-model port. Chunk planning and terminology ledgers belong to request-local application orchestration, not this adapter.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

