# Application validation

中文：[应用校验](../../../docs_cn/reference/application/validation_cn.md)

The application-validation module enforces semantic invariants on target requests, provider candidates, the superset aggregate, and the final projected result. Wire decoding and safe HTTP problem mapping remain API responsibilities.

Status: target design assembled from contract and existing domain invariants. The current executable validates its transitional routes but does not compose this complete target validation boundary.

## Responsibilities

Input checks enforce supported languages, source/target compatibility, text and body limits, chronological minimal history shape, and closed response levels. Provider and composition checks reject malformed structured output, unknown references, unsupported claims, and unsafe size growth.

Aggregate checks enforce meaning attachment, stable canonical IDs, one compatible release trio, relationship direction and applicability, graph bounds, evidence and verification labels, semantic-scale rules, degradation consistency, and final response limits.

## Dependencies and invariants

Validation is deterministic and has no storage mutation or provider side effect. It receives versioned registries and limits through validated configuration or domain policy, not caller-controlled relationship semantics.

Failures map to closed safe application errors without echoing request text, history, provider bodies, credentials, vectors, or storage internals. Validation never repairs a response by inventing facts; bounded schema repair, where allowed, remains a provider-adapter policy followed by full revalidation.

## Verification

Tests should exercise every closed value and bound, unknown fields at the API boundary, multi-meaning consistency, release mismatches, taxonomy direction and cycles, semantic scales, evidence labels, graph depth and size, projection limits, invalid provider output, and secret/content-safe errors. Fuzz and injection tests should treat all external text as untrusted.

## Related documents

- [Service module reference](../modules.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [SQL data endpoints](../../interfaces/mysql.md)
- [Vector data endpoints](../../interfaces/qdrant.md)
- [Quality assurance](../../guides/quality-assurance.md)
