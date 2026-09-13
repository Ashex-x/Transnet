# 数据 port

English: [Data ports](../../../docs/reference/ports/data.md)

本模块定义在线 application service 所需的面向操作的结构化与向量访问。

## 在线读取

结构化读取解析活动发布、规范候选、完整词义详情、领域清单、事实、证据和来源。向量读取检索按发布过滤的节点、边候选及有界浅层图邻域。Port method 表达这些用例，而不是 SQL、Qdrant-native 请求或通用 repository 访问。

每次调用携带请求 deadline 与精确发布标识符。闭合结果区分缺失、不兼容、不可用、无效与权限过滤数据，且不泄露被隐藏内容。

## 变更边界

在线组合不接收 mutation method。独立发布 port 可暂存、校验、投影、激活、隔离、移除与回滚已审核规范内容；这些 port 绝不传给请求 handler。

精确操作与 payload 由 [SQL](../../interfaces/mysql_cn.md) 和[向量](../../interfaces/qdrant_cn.md)合同负责。
