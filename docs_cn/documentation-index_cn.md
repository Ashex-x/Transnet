# Transnet 中文文档

English: [Transnet documentation](../docs/documentation-index.md)

## 设计与规划

- [系统设计与架构](transnet_cn.md)：权威的无状态服务行为、规范数据模型、发布、检索和质量场景。
- [服务行为](product/learning-experience_cn.md)：消费者可见的翻译、词汇查询和图行为。
- [交付计划](todo_cn.md)：实现阶段和完成标准。

## 接口

- [Transnet 服务接口](interfaces/port_cn.md)：私有 HTTP 边界和无状态服务操作。
- [MySQL 适配器](interfaces/mysql_cn.md)：规范词汇内容、领域、证据和发布状态。
- [Qdrant 适配器](interfaces/qdrant_cn.md)：节点、边、检索、校验和发布。

## 参考

- [OpenAPI 3.1 合同](../docs/reference/transnet-openapi.json)：机器可读的目标服务合同。

OpenAPI 镜像目标服务接口；运行时可用性在人工接口合同中明确标注。

## 指南

- [配置](guides/configuration_cn.md)
- [开发与运维](guides/development_cn.md)
- [内容发布](guides/content-publishing_cn.md)
- [质量保证](guides/quality-assurance_cn.md)

## 仓库规则

- [仓库约定 v1.0.1](conventions-v1.0.1_cn.md)

系统设计对产品语义最权威。接口文档在其明示的实现状态内具有规范性。
