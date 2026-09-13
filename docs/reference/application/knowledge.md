# Knowledge application

中文：[知识 application](../../../docs_cn/reference/application/knowledge_cn.md)

This module owns sense resolution, domain assessment, release-pinned retrieval, relationship ranking, page composition, and deterministic projection for word and established-phrase requests.

## Resolution and assessment

Normalization creates bounded language-aware lookup forms but never serves as canonical identity. Exact canonical and alias matches outrank inflection, spelling, and semantic candidates. Materially plausible meanings remain separate.

Domain assessment uses a bounded published inventory and returns `existing`, `proposed_new`, `general`, or `uncertain`. Inventory failure produces `uncertain`; it cannot prove that a domain is new. Proposals are request-local and appear only where the full response permits them.

## Retrieval and composition

Vector similarity nominates candidates and never establishes a fact. Every displayed canonical node, edge, evidence item, and revision is hydrated from authoritative structured data under the request's release pin. Ranking filters by sense, domain, language, region, period, conditions, and evidence policy without changing identity, direction, provenance, or verification state.

A model may organize supplied facts and write concise explanations, but cannot invent endpoints or change fact metadata. Deterministic validation checks every reference. Empty or weak sections are omitted.

`brief`, `standard`, and `full` are projections of the same validated superset. Response level changes breadth, not fact choice or truth status.

## Verification

Test candidate precedence, ambiguity, closed domain outcomes, unavailable inventory, release consistency, hydration, evidence filtering, graph bounds, hallucinated references, empty sections, and projection monotonicity.
