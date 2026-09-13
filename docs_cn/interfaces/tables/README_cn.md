# 目标存储目录

English: [Target storage catalog](../../../docs/interfaces/tables/README.md)

本目录将目标 SQL 与向量 endpoint 合同展开为面向实现的 schema 和 collection 目录。Endpoint 合同仍是线上行为的权威来源；这些 artifact 负责目标持久化结构、键、可变性，以及规范知识与 island-port 产品数据之间的隔离。

- [MySQL schema](../../../docs/interfaces/tables/sql.sql)：优化混合模型的可执行 MySQL 8 DDL，包含十一张规范表和五张 island-port 私有评估表；SQL artifact 由中英文文档共同引用，避免产生两个可能分叉的 schema 来源。
- [向量 collection](vec_cn.md)：不可变 Qdrant 节点与边 collection 及其 payload 索引。

任何表或 collection 都不得存储实时翻译文本、查询上下文、请求级历史、provider 输出、凭据或从私有流量生成的向量。

SQL schema 将稳定身份、生命周期、发布成员关系、关系 endpoint 和常用查询键提升为关系列；类型特有的有界内容使用不可变且带版本的 JSON payload。只有存在独立生命周期、完整性边界或已证明需要索引的查询时才新增表；概念对象数量本身不是建表理由。

十一张 `transnet_canonical` 表分为三类职责。`content_release`、`publication_job`、`publication_idempotency` 和 `qdrant_projection_outbox` 负责发布与投影；`canonical_source`、`evidence_revision`、`canonical_entity`、`canonical_entity_revision`、`canonical_relationship` 和 `canonical_relationship_revision` 负责稳定的已审核内容；`release_member` 固定任一种规范修订，无需为每个实体族建立独立成员表。

五张 `island_product` 表保留确有必要的生命周期差异。`relationship_judgment_event` 只追加，`relationship_judgment_moderation` 记录独立治理的决定，`relationship_judgment_current` 是可替换的聚合输入；`relationship_aggregate_release` 激活一个不可变匿名批次，而 `relationship_assessment_projection` 合并计数及派生距离，因为它们共享相同的键与生命周期。
