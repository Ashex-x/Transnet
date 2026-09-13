# Transnet 中文文档

English: [Transnet documentation](../docs/documentation-index.md)

## 设计与规划

- [系统设计与架构](transnet_cn.md)：权威的翻译、关系页面、领域展开、规范数据与质量语义。
- [服务行为](product/service-behavior_cn.md)：消费者可见的翻译与关系型查询行为。
- [交付计划](todo_cn.md)：实现阶段和完成标准。

## 接口

- [接口目录](interfaces/README_cn.md)：本目录的目录、边界与建议阅读顺序。
- [Transnet 服务接口](interfaces/transnet_cn.md)：island-port 到 Transnet 的合同及共享内部 UDS 传输规则。
- [Island-port SQL endpoint](interfaces/mysql_cn.md)：规范卡片和精选翻译存储，以及 `data/sql/v1` 下的 UDS JSON 操作；调用方不得直接访问 MySQL 或提交 SQL。
- [Island-port 向量 endpoint](interfaces/qdrant_cn.md)：`data/vec/v1` 下的 UDS JSON 操作；调用方不得直接访问 Qdrant。

## 参考

- [服务模块目录](reference/modules_cn.md)：各份当前/目标启动器、传输、API、application、domain、port、adapter、发布、可观测性与停机模块参考的导航与共享依赖规则。

## 指南

- [配置](guides/configuration_cn.md)：当前监听器、路由、Provider 设置及目标基础设施配置边界。
- [开发与运维](guides/development_cn.md)：对根 README 的运维补充。
- [内容发布](guides/content-publishing_cn.md)：拟议的 MySQL/Qdrant 摄取、校验、发布、移除和回滚工作流。
- [质量保证](guides/quality-assurance_cn.md)：拟议的基准、失败测试、发布门禁和监控。

## 仓库规则

- [仓库约定 v1.0.1](conventions-v1.0.1_cn.md)

## 术语与实现状态

- **当前运行时：**今天可由本仓库构建的可执行文件。它提供回环地址上的翻译和模型驱动的结构化查询；尚未组合生产级 MySQL 或 Qdrant 适配器。
- **目标服务 / 目标合同：**设计和接口文档所定义的预期且版本化的服务行为。目标路由或 Schema 并不表示当前运行时已经启用它。
- **规范内容：**经发布工作流审查、版本化并确认为权威的知识。它不同于模型响应、请求内容或相似度结果。
- **基础卡（`BasicCard`）：**一个可独立选择的词汇词义对应的精简、发布版本固定的 MySQL 记录；即使图检索不可用，它仍应有用。
- **知识根：**查询页面或有界图读取的起点，即稳定的规范节点或词义。
- **发布三元组：**一份兼容的 MySQL 卡片发布版本，加上配对的不可变 Qdrant 节点集合和边集合。每个请求同时固定这三个版本。
- **MySQL：**目标架构中用于规范卡片、经审慎选择的翻译、发布元数据和发布状态的权威关系型存储。
- **Qdrant：**目标架构中用于检索已发布规范节点和有类型边的、可重建的向量与 Payload 索引投影。
- **Gemma 4 / TranslateGemma：**当前运行时使用的 OpenAI-compatible 模型 Provider。Gemma 4 处理短文本翻译和结构化查询；TranslateGemma 处理较长的翻译请求。
- **`verified` / `inferred` / `exploratory`：**分别是已发布的规范关系、仅限当前请求的有证据推断说明，以及向量或模型候选。只有 `verified` 是规范事实；后两者绝不作为事实持久化。
- **关系状态字段：**公共页面将上述三种标签命名为 `evidence_state`。在 Qdrant 存储合同中，`evidence_state` 表示附带证据是否支持某条边（例如 `supported`），而 `verification_state` 表示存储的边是否为规范边（`verified`）或探索性边。它们相关，但不可互换。

系统设计对产品语义最权威。接口文档在其明示的实现状态内具有规范性。
