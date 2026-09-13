# Transnet documentation

中文：[中文文档索引](../docs_cn/documentation-index_cn.md)

## Design and planning

- [System design and architecture](transnet.md): authoritative translation, relationship-page, domain-expansion, canonical-data, and quality semantics.
- [Service behavior](product/service-behavior.md): consumer-visible translation and relationship-centered lookup behavior.
- [Overall plan](todo.md): delivery phases, dependencies, and exit criteria.

## Interfaces

- [Transnet service interface](interfaces/transnet.md): island-port-to-Transnet contract and shared internal UDS transport rules.
- [Island-port SQL endpoints](interfaces/mysql.md): canonical-card and curated-translation storage plus UDS JSON operations under `data/sql/v1`; callers never access MySQL or submit SQL directly.
- [Island-port vector endpoints](interfaces/qdrant.md): UDS JSON operations under `data/vec/v1`; callers never access Qdrant directly.

## Reference

- [Service module reference](reference/modules.md): current and target launcher, transport, API, orchestration, domain, RAG, storage, provider, publication, observability, and shutdown modules.

## Guides

- [Configuration](guides/configuration.md): current listener, routing, and provider settings plus expected infrastructure configuration boundaries.
- [Development](guides/development.md): operational notes supplementing the root README.
- [Content publishing](guides/content-publishing.md): proposed MySQL and Qdrant ingestion, validation, publication, removal, and rollback workflow.
- [Quality assurance](guides/quality-assurance.md): proposed benchmarks, failure tests, release gates, and monitoring.

## Repository rules

- [Repository conventions v1.0.1](../conventions.v1.0.1.md): Rust, documentation, verification, and Git rules.

## 中文

- [Chinese documentation index](../docs_cn/documentation-index_cn.md): Chinese translations of the authoritative interfaces and repository conventions.

The system design is authoritative for product semantics. Interface documents are normative within their stated implementation status; Rust trait details remain in source comments and rustdoc.

## Terminology and status

- **Current runtime:** the executable built from this repository today. It provides transitional loopback translation and model-backed structured lookup; it does not yet expose the target UDS paths or compose production structured and vector data service clients.
- **Target service / target contract:** the intended, versioned service behavior described by the design and interface documents. A target route or schema is not evidence that the current runtime enables it.
- **Canonical content:** reviewed, versioned knowledge that the publication workflow has made authoritative. It is distinct from a model response, a request, or a similarity result.
- **Basic card:** the concise, release-pinned MySQL record for one independently selectable lexical sense. It remains useful when graph retrieval is unavailable.
- **Knowledge root:** the stable canonical node or sense from which a lookup page or bounded graph read starts.
- **Release trio:** one compatible MySQL card release plus its immutable Qdrant node and edge collections. Requests pin all three versions together.
- **MySQL:** the authoritative relational store for canonical cards, deliberately selected translations, release metadata, and publication state in the target architecture.
- **Qdrant:** the rebuildable vector and payload-index projection used to retrieve published canonical nodes and typed edges in the target architecture.
- **Gemma 4 / TranslateGemma:** OpenAI-compatible model providers used by the current runtime. Gemma 4 handles short-text translation and structured lookup; TranslateGemma handles longer translation requests.
- **Verified / inferred / exploratory:** respectively, a published canonical relationship; an evidence-grounded explanation generated only for the current request; and a vector or model candidate. Only verified content is canonical, and the latter two are never persisted as facts.
- **Relationship status fields:** the public page calls that three-way label `evidence_state`. In the Qdrant storage contract, `evidence_state` instead records whether the attached evidence supports an edge (for example, `supported`), while `verification_state` records whether the stored edge is canonical (`verified`) or exploratory. The terms are related but are not interchangeable.
