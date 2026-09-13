# Relationship ranker

中文：[关系排序器](../../../docs_cn/reference/application/relationship_ranker_cn.md)

The relationship-ranker application module orders eligible canonical and clearly labeled request-local relationships for usefulness around one selected meaning root.

Status: target design. Existing graph foundations contain related invariants, but this target ranker is not composed in the current executable.

## Responsibilities

The ranker accepts a hydrated fact bundle and keeps only facts applicable to the selected sense, domain, language, region, period, conditions, and evidence policy. It groups and orders useful relationships without changing their identifiers, direction, provenance, verification state, or evidence state.

Taxonomy, part-whole structure, intensity, synonymy, contrast, grammar, terminology, technical facts, and exploratory association remain distinct. `is_a` points child sense to parent category; `has_subtype` is its inverse. A semantic scale is a separately named ordered dimension, not taxonomy or equal numeric spacing.

## Dependencies and invariants

Ranking is deterministic for the same release, policy version, root, and eligible inputs. Model-produced explanations may assist composition but cannot make a candidate eligible or convert `inferred` or `exploratory` content into `verified` content.

Every retained item has an explicit useful path to its meaning root. Similarity affects candidate ordering only and never establishes a fact.

## Verification

Tests should cover applicability filters, stable ordering, evidence thresholds, inverse direction, taxonomy cycles, complete eligible semantic scales, separate relationship families, and ties. Adversarial suites should reject promotion of vector proximity into factual relationship types.

## Related documents

- [Service module reference](../modules.md)
- [Vector data endpoints](../../interfaces/qdrant.md)
- [System design](../../transnet.md)
- [Quality assurance](../../guides/quality-assurance.md)
