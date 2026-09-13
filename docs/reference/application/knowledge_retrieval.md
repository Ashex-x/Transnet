# Knowledge retrieval

中文：[知识检索](../../../docs_cn/reference/application/knowledge_retrieval_cn.md)

The knowledge-retrieval application module builds a bounded, release-pinned fact bundle for an already resolved sense and selected existing domain.

Status: target composition over existing retrieval, graph, release, and in-memory foundations. The current launcher does not wire production island-port structured/vector adapters into this flow.

## Responsibilities

The module reads the domain knowledge profile, requests only useful available fact families, obtains node and edge candidates from Qdrant, and hydrates referenced fact revisions, evidence, and provenance through authoritative structured-data reads. Missing families and partial coverage remain explicit.

Candidate selection and authoritative hydration are separate stages. A short connection path is eligible only when every step is a named, independently evidence-eligible relationship; arbitrary-depth traversal and similarity chains are outside the boundary.

## Dependencies and invariants

The module depends on operation-focused vector-data and structured-data read ports and uses the orchestrator's exact release trio and deadline. Qdrant is a rebuildable projection; MySQL or the signed release artifact remains authoritative.

Vector similarity never proves translation, synonymy, taxonomy, causation, shared mechanism, or cultural meaning. Runtime query text and ephemeral vectors are not persisted. Qdrant failure may yield an explicit MySQL-only degraded result rather than invented relationships.

## Verification

Tests should cover family selection from profiles, release and schema compatibility, hydration of every displayed fact, eligibility filtering before limits, missing evidence, bounded graph expansion, partial coverage, Qdrant degradation, and MySQL failure. Injection cases must not bypass release filters or fabricate provenance.

## Related documents

- [Service module reference](../modules.md)
- [SQL data endpoints](../../interfaces/mysql.md)
- [Vector data endpoints](../../interfaces/qdrant.md)
- [Quality assurance](../../guides/quality-assurance.md)
