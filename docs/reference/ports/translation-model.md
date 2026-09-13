# Translation model port

中文：[翻译模型端口](../../../docs_cn/reference/ports/translation-model_cn.md)

This page defines application-facing connected-text translation and bounded structured composition operations at `src/ports/translation_model.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

Operations are use-case specific and accept only bounded, validated inputs. Implementations return candidates that application validation must check; they cannot publish facts or canonical translations. Provider protocol envelopes, prompts, retries, and credentials remain adapter concerns.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

