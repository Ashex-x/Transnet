# Target storage catalog

中文：[目标存储目录](../../../docs_cn/interfaces/tables/README_cn.md)

This directory turns the target SQL and vector endpoint contracts into implementation-oriented table and collection catalogs. The endpoint contracts remain authoritative for wire behavior; these pages own target persistence shape, keys, mutability, and separation between canonical knowledge and island-port product data.

- [MySQL schema](sql.sql): executable MySQL 8 DDL for canonical releases plus private island-port relationship judgments and anonymous distance projections.
- [Vector collections](vec.md): immutable Qdrant node and edge collections and their payload indexes.

No table or collection stores live translation text, lookup context, request-scoped history, provider output, credentials, or vectors derived from private traffic.
