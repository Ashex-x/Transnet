# Target storage catalog

中文：[目标存储目录](../../../docs_cn/interfaces/tables/README_cn.md)

This directory turns the target SQL and vector endpoint contracts into implementation-oriented schema and collection catalogs. The endpoint contracts remain authoritative for wire behavior; these artifacts own target persistence shape, keys, mutability, and separation between canonical knowledge and island-port product data.

- [MySQL implementation](mysql.sql): reviewed MySQL 8 target snapshot for the optimized hybrid model: sixteen canonical tables and six private island-port tables.
- [MySQL migration policy](migrations.md): immutable migration identity, expand/migrate/switch/contract rollout, online DDL, backfill, compatibility, and rollback rules.
- [Qdrant implementation](qdrant.md): immutable Qdrant node and edge collections and their payload indexes.

No table or collection stores live translation text, lookup context, request-scoped history, provider output, credentials, or vectors derived from private traffic.

The SQL schema promotes stable identity, lifecycle, release membership, relationship endpoints, and common lookup keys to relational columns. Type-specific bounded content uses immutable versioned JSON payloads. New tables require an independent lifecycle, integrity boundary, or demonstrated indexed query; conceptual object count alone is not a reason to add one.

The sixteen `transnet_canonical` tables have four responsibilities. `schema_migration` records physical evolution. `content_release`, `publication_job`, `publication_idempotency`, and `qdrant_projection_outbox` own publication and projection. `canonical_source`, `canonical_source_revision`, `evidence_revision`, `entity_type_revision`, `canonical_entity`, `canonical_entity_revision`, `relation_type_revision`, `canonical_assertion_participant`, `canonical_relationship`, and `canonical_relationship_revision` own stable reviewed content, extensible registries, and typed projections. `release_member` pins exact source, evidence, registry, entity, and relationship revisions without separate membership tables for every family.

The six `island_product` tables preserve an independent migration ledger and necessary lifecycle differences. `relationship_judgment_event` is append-only, `relationship_judgment_moderation` records the separately governed decision, and `relationship_judgment_current` is the replaceable aggregation input. `relationship_aggregate_release` activates one immutable anonymous batch, while `relationship_assessment_projection` combines counts and derived distance because they share the same key and lifecycle. Algorithm versions and privacy thresholds are row data so a new policy builds a new aggregate rather than requiring DDL.
