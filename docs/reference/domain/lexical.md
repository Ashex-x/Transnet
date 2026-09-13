# Lexical

中文：[词汇](../../../docs_cn/reference/domain/lexical_cn.md)

This page defines sense-qualified lexical concepts used by lookup and response composition at `src/domain/lexical.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: target module design; no target source file is checked in.

## Contract

The target types cover lexemes, established phrases, stable senses, definitions, pronunciation, morphology, examples, usage, and aliases. Sense identity is required wherever a form is ambiguous; aliases and spelling or inflection matches help resolution but do not establish equivalence by themselves.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../modules.md)
- [System design](../../transnet.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [Content publishing](../../guides/content-publishing.md)

