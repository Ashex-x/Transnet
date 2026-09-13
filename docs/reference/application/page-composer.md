# Relationship page composer

中文：[关系页面组织器](../../../docs_cn/reference/application/page-composer_cn.md)

The page-composer application module organizes meaning-specific lexical and domain material into the target superset result before response-level projection.

Status: target design. The current legacy structured lookup uses a learning-specific model schema and is not this shared, release-grounded composer.

## Responsibilities

The composer starts from one selected sense or domain concept and arranges concise definitions, usage, examples, lexical groups, domain facts, semantic scales, provenance, and short connection explanations. It omits empty or weakly supported sections and may reorder eligible groups by usefulness.

The model is a composer, not the source of truth. It may summarize or connect supplied facts within bounded structured output, but it cannot change fact identity, direction, applicable scope, evidence, provenance, verification state, or release.

## Dependencies and invariants

The module receives resolved meanings and ranked, hydrated relationships. A bounded structured composition operation may be called through the model port; deterministic code checks all references against the supplied bundle.

Every section remains attached to its meaning root. Verified, inferred, and exploratory material stays visibly distinct, and every displayed connection path contains only named independently eligible steps. Explanations and examples are request-local unless already part of canonical content.

## Verification

Tests should cover multiple meanings, root attachment, omission of empty sections, concise explanations, fact-reference validation, path integrity, and separation of evidence states. Invalid model additions, altered IDs, unsupported claims, and cross-meaning leakage must be rejected or safely omitted.

## Related documents

- [Service module reference](../modules.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [System design](../../transnet.md)
- [Quality assurance](../../guides/quality-assurance.md)
