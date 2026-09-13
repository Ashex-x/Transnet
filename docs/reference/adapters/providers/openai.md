# OpenAI-compatible provider adapter

中文：[OpenAI 兼容提供方适配器](../../../../docs_cn/reference/adapters/providers/openai_cn.md)

This page defines shared protocol handling for OpenAI-compatible model endpoints at `src/adapters/providers/openai.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target adapter owns HTTP client construction, authentication, endpoint envelopes, strict structured-output decoding, safe error mapping, and bounded resilience integration. It never logs prompts, source text, provider bodies, credentials, or generated content. Role-specific model selection and prompt/schema policy stay in the Gemma adapters.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

