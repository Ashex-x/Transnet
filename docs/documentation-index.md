# Transnet documentation

中文：[中文文档索引](../docs_cn/documentation-index_cn.md)

## Design and planning

- [System design and architecture](transnet.md): authoritative stateless service behavior, canonical data model, publication, retrieval, and quality scenarios.
- [Service behavior](product/learning-experience.md): consumer-visible translation, lexical lookup, and graph behavior.
- [Overall plan](todo.md): delivery phases, dependencies, and exit criteria.

## Interfaces

- [Transnet service interface](interfaces/port.md): private HTTP boundary and stateless service operations.
- [MySQL adapter interface](interfaces/mysql.md): typed persistence operations and transaction invariants.
- [Qdrant adapter interface](interfaces/qdrant.md): collection, point, retrieval, reconciliation, and publication contract.

## Reference

- [OpenAPI 3.1 contract](reference/transnet-openapi.json): machine-readable target service contract.

The OpenAPI document mirrors the target service interface. Runtime availability remains explicitly marked in the human interface contract.

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
