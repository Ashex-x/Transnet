# Target storage catalog

中文：[目标存储目录](../../../docs_cn/interfaces/tables/README_cn.md)

This directory turns the target SQL and vector endpoint contracts into implementation-oriented schema and collection catalogs. The endpoint contracts remain authoritative for wire behavior; these artifacts own target persistence shape, keys, mutability, and separation between canonical knowledge and island-port product data.

- [MySQL schema](sql.sql): executable MySQL 8 DDL for the optimized hybrid model: eleven canonical tables and five private island-port assessment tables.
- [Vector collections](vec.md): immutable Qdrant node and edge collections and their payload indexes.

No table or collection stores live translation text, lookup context, request-scoped history, provider output, credentials, or vectors derived from private traffic.

The SQL schema promotes stable identity, lifecycle, release membership, relationship endpoints, and common lookup keys to relational columns. Type-specific bounded content uses immutable versioned JSON payloads. New tables require an independent lifecycle, integrity boundary, or demonstrated indexed query; conceptual object count alone is not a reason to add one.

The eleven `transnet_canonical` tables have three responsibilities. `content_release`, `publication_job`, `publication_idempotency`, and `qdrant_projection_outbox` own publication and projection. `canonical_source`, `evidence_revision`, `canonical_entity`, `canonical_entity_revision`, `canonical_relationship`, and `canonical_relationship_revision` own stable reviewed content. `release_member` pins either kind of canonical revision without separate membership tables for every entity family.

The five `island_product` tables preserve necessary lifecycle differences. `relationship_judgment_event` is append-only, `relationship_judgment_moderation` records the separately governed decision, and `relationship_judgment_current` is the replaceable aggregation input. `relationship_aggregate_release` activates one immutable anonymous batch, while `relationship_assessment_projection` combines counts and derived distance because they share the same key and lifecycle.
