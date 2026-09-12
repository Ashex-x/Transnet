# Transnet 中文文档

English: [Transnet documentation](../docs/documentation-index.md)

## 设计与规划

- [系统设计与架构](transnet_cn.md)：权威的翻译、关系页面、领域展开、规范数据与质量语义。
- [服务行为](product/service-behavior_cn.md)：消费者可见的翻译与关系型查询行为。
- [交付计划](todo_cn.md)：实现阶段和完成标准。

## 接口

- [Transnet 服务接口](interfaces/port_cn.md)：私有 HTTP 边界和无状态服务操作。
- [MySQL 适配器](interfaces/mysql_cn.md)：有类型持久化操作和事务不变量。
- [Qdrant 适配器](interfaces/qdrant_cn.md)：集合、Point、检索、对账和发布合同。

## 参考

- [OpenAPI 3.1 合同](../docs/reference/transnet-openapi.json)：机器可读的目标服务合同。

OpenAPI 镜像目标服务接口；运行时可用性在人工接口合同中明确标注。

## 指南

- [配置](guides/configuration_cn.md)：当前监听器、路由、Provider 设置及目标基础设施配置边界。
- [开发与运维](guides/development_cn.md)：对根 README 的运维补充。
- [内容发布](guides/content-publishing_cn.md)：拟议的 MySQL/Qdrant 摄取、校验、发布、移除和回滚工作流。
- [质量保证](guides/quality-assurance_cn.md)：拟议的基准、失败测试、发布门禁和监控。

## 仓库规则

- [仓库约定 v1.0.1](conventions-v1.0.1_cn.md)

系统设计对产品语义最权威。接口文档在其明示的实现状态内具有规范性。
