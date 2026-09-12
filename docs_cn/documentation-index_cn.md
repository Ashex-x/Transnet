# Transnet 中文文档

English: [Transnet documentation](../docs/documentation-index.md)

## 设计与规划

- [系统设计与架构](transnet_cn.md)：权威的目标行为、数据模型、学习循环、代理角色和质量场景。
- [英语学习体验](product/learning-experience_cn.md)：学习者可见行为、书签卡、练习、反馈与隐私。
- [交付计划](todo_cn.md)：实现阶段和完成标准。

## 接口

- [Island-port 接口](interfaces/port_cn.md)：内部 HTTP 边界与当前/拟议操作。
- [MySQL 适配器](interfaces/mysql_cn.md)：基础卡、私有学习卡、有界历史与排程状态。
- [Qdrant 适配器](interfaces/qdrant_cn.md)：节点、边、检索、校验和发布。

## 参考

- [OpenAPI 3.1 合同](../docs/reference/transnet-openapi.json)：当前实现和 feature-gated 兼容 HTTP 表面。

OpenAPI 不是目标产品合同；旧历史、saved-sense、graph-view、preference、privacy 和 practice schema 不得用来推断目标设计。

## 指南

- [配置](guides/configuration_cn.md)
- [开发与运维](guides/development_cn.md)
- [内容发布](guides/content-publishing_cn.md)
- [质量保证](guides/quality-assurance_cn.md)

## 仓库规则

- [仓库约定 v1.0.1](conventions-v1.0.1_cn.md)

系统设计对产品语义最权威。接口文档在其明示的实现状态内具有规范性。
