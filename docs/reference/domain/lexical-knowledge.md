# Lexical and knowledge domain

中文：[词汇与知识 domain](../../../docs_cn/reference/domain/lexical-knowledge_cn.md)

This module owns stable lexical identity, canonical knowledge, typed relationships, evidence, and semantic scales.

## Lexical identity

A sense or established phrase has a stable canonical ID independent of spelling normalization. Forms, aliases, pronunciation, definitions, grammar, register, morphology, examples, collocations, and usage restrictions remain attached to the applicable meaning. Multiple parts of speech or materially different meanings are not collapsed.

## Knowledge and relationships

Canonical knowledge consists of immutable, release-scoped nodes and atomic facts. Relationships have explicit type, direction, endpoints, applicability, conditions, provenance, evidence, verification state, and revision. Symmetry, inverse projection, and transitivity are properties of a relationship type, never guesses from wording.

Semantic scales are named ordered dimensions separate from taxonomy and synonymy. Member positions express order, not equal numeric distance. Derived adjacent-degree edges remain distinguishable from stored canonical edges.

`verified` content is published canonical knowledge. `inferred` explanations and `exploratory` candidates exist only for the request and are never persisted as facts.

## Invariants

Every displayed node is the selected root or has a useful explicit path back to it. Similarity cannot prove translation, synonymy, hierarchy, causality, mechanism, or cultural meaning. Exact persisted payloads belong to the [SQL](../../interfaces/mysql.md) and [vector](../../interfaces/qdrant.md) interfaces.
