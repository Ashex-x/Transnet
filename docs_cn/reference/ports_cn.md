# Port 模块

English: [Ports module](../../docs/reference/ports.md)

Port 模块定义 application service 从模型和数据依赖所需的窄操作。Port 使用已校验 domain 类型、显式 deadline、闭合结果和发布标识符，不暴露 provider 或数据库协议。

## 模型操作

模型 port 提供有界普通翻译与结构化生成。Application 代码选择操作，而非 provider 品牌。Provider URL、凭据、HTTP envelope、prompt、role message、JSON Schema 机制、重试 header 与模型专属 payload 属于 adapter。

模型输出绝不是规范证据。调用方必须在使用前校验结构与引用，并在请求结束时丢弃请求内容和生成材料。

## 数据操作

结构化读取解析活动发布、规范候选、完整词义详情、领域清单、事实、证据与来源。向量读取检索按发布过滤的节点、边候选及有界浅层邻域。Method 表达这些用例，而不是 SQL、Qdrant-native 请求或通用 repository 访问。

每次读取携带请求 deadline 和精确发布标识符。闭合结果区分缺失、不兼容、不可用、无效与权限过滤数据，且不泄露被隐藏内容。

在线组合不接收 mutation method。独立发布 port 可暂存、校验、投影、激活、隔离、移除与回滚已审核规范内容，并绝不传给请求 handler。精确操作由 [SQL](../interfaces/mysql_cn.md) 与[向量](../interfaces/qdrant_cn.md)合同负责。

## 验证

使用 fake 测试操作选择、deadline 传播、发布一致性、畸形模型输出、闭合失败映射、过滤数据及在线组合中不存在 mutation 权限。
