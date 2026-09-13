# Data ports

中文：[数据 port](../../../docs_cn/reference/ports/data_cn.md)

This module defines operation-focused structured and vector access required by online application services.

## Online reads

Structured reads resolve active releases, canonical candidates, complete sense details, domain inventories, facts, evidence, and provenance. Vector reads retrieve release-filtered node and edge candidates and bounded shallow graph neighborhoods. Port methods express these use cases rather than SQL, Qdrant-native requests, or generic repository access.

Every call carries the request deadline and exact release identifiers. Closed outcomes distinguish missing, incompatible, unavailable, invalid, and permission-filtered data without leaking withheld content.

## Mutation boundary

The online composition receives no mutation methods. Separate publication ports may stage, validate, project, activate, quarantine, remove, and roll back reviewed canonical content. They are never passed to request handlers.

Exact operations and payloads are owned by the [SQL](../../interfaces/mysql.md) and [vector](../../interfaces/qdrant.md) contracts.
