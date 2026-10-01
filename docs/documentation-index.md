# Transnet documentation

中文：[中文文档索引](../docs_cn/documentation-index_cn.md)

## Design and planning

- [System design and architecture](transnet.md): authoritative translation, relationship-page, domain-expansion, canonical-data, and quality semantics.
- [Service behavior](product/service-behavior.md): consumer-visible translation and relationship-centered lookup behavior.
- [Overall plan](todo.md): delivery phases, dependencies, and exit criteria.

## Interfaces

- [Interface catalog](interfaces/README.md): table of contents, boundaries, and recommended reading order for this directory.
- [Transnet service interface](interfaces/transnet.md): island-port-to-Transnet contract and shared internal UDS transport rules.
- [Canonical-data endpoints](interfaces/canonical-data.md): storage-neutral canonical-card, translation, fact, release, and publication operations over UDS JSON.
- [Retrieval-data endpoints](interfaces/retrieval-data.md): storage-neutral candidate node, relationship, scale, and projection operations over UDS JSON.
- [MySQL migration policy](interfaces/tables/migrations.md): physical-schema compatibility, online rollout, backfill, rollback, and migration-ledger rules.

## Reference

- [Module reference](reference/modules.md): searchable catalog of module-owned runtime, transport, application, domain, port, adapter, persistence, observability, and operations pages.
- [Observability contract](reference/observability.md): whole-system structured logs, traces, metrics, audit events, privacy exclusions, and failure behavior.

## Guides

- [Configuration](guides/configuration.md): current listener, routing, and provider settings plus expected infrastructure configuration boundaries.
- [Development](guides/development.md): operational notes supplementing the root README.
- [Deployment](guides/deployment.md): GitHub Actions, GPU-server systemd setup, secrets, and release checks.
- [Content publishing](guides/content-publishing.md): proposed MySQL and Qdrant ingestion, validation, publication, removal, and rollback workflow.
- [Quality assurance](guides/quality-assurance.md): proposed benchmarks, failure tests, release gates, and monitoring.

## Repository rules

- [Repository conventions v1.0.1](../conventions.v1.0.1.md): Rust, documentation, verification, and Git rules.

The system design is authoritative for product semantics. Interface documents are normative within their stated implementation status; Rust trait details remain in source comments and rustdoc.

## Terminology and status

- **Current runtime:** the executable built from this repository today. It serves the target `/api/v1` surface over one owned UDS and supports translation plus opt-in canonical and knowledge reads through an outbound island-port UDS client. Legacy TCP, CORS, lookup, sense-detail, and raw graph routes are absent. The island-port canonical server, production MySQL migrations and publication workflow, real old-release retention, end-to-end MySQL acceptance, and production vector execution remain externally owned or unverified here.
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
