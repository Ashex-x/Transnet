# Sense resolution

中文：[词义解析](../../../docs_cn/reference/application/sense_resolution_cn.md)

The sense-resolution application module maps a word or established phrase to release-pinned canonical candidates while preserving material ambiguity.

Status: target orchestration over existing foundations. Canonical lookup, card, and sense components exist in the repository but are not composed into the current executable's default translation path.

## Responsibilities

The module derives bounded lookup forms through the versioned normalizer and ranks exact canonical, alias, inflection, spelling, and semantic candidates in that order of evidentiary strength. It compares exact stored source text where the storage contract requires it; a fingerprint alone is not identity.

Stable sense IDs, not normalized strings, identify meanings. Current text and history may rank candidates, and several materially plausible meanings remain in the superset. The module never creates a card, alias, or sense as a lookup side effect.

## Dependencies and invariants

Resolution uses release-pinned structured-data reads and may use vector retrieval only to propose semantic candidates. Similarity never establishes translation equivalence or sense identity. Meaningful symbols such as `C`, `C++`, and `C#` remain distinct.

Returned definitions, examples, and relationships must stay attached to their applicable sense. History can disambiguate but cannot rewrite canonical identity, fact direction, or evidence.

## Verification

Tests should cover normalization versions, exact and alias precedence, inflections, spelling suggestions, multilingual aliases, symbols, ambiguous parts of speech, several retained meanings, release mismatch, and degraded exact-card reads. Adversarial tests should ensure semantic proximity is never promoted to a canonical match.

## Related documents

- [Service module reference](../modules.md)
- [SQL data endpoints](../../interfaces/mysql.md)
- [Vector data endpoints](../../interfaces/qdrant.md)
- [System design](../../transnet.md)
