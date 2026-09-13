# 目标存储目录

English: [Target storage catalog](../../../docs/interfaces/tables/README.md)

本目录将目标 SQL 与向量 endpoint 合同展开为面向实现的表和 collection 目录。Endpoint 合同仍是线上行为的权威来源；本目录负责目标持久化结构、键、可变性，以及规范知识与 island-port 产品数据之间的隔离。

- [MySQL schema](../../../docs/interfaces/tables/sql.sql)：规范发布表、island-port 私有关系判断表和匿名距离投影表的可执行 MySQL 8 DDL；SQL artifact 由中英文文档共同引用，避免产生两个可能分叉的 schema 来源。
- [向量 collection](vec_cn.md)：不可变 Qdrant 节点与边 collection 及其 payload 索引。

任何表或 collection 都不得存储实时翻译文本、查询上下文、请求级历史、provider 输出、凭据或从私有流量生成的向量。
