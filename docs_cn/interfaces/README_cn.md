# 接口目录

English: [Interface catalog](../../docs/interfaces/README.md)

本目录定义 Transnet 与 island-port 之间的目标合同。它们在已声明的目标状态内具有规范性；并不表示当前可执行文件已经暴露每一条路由。

| 文档 | 边界 | 适用内容 |
| --- | --- | --- |
| [Transnet 服务接口](transnet_cn.md) | island-port → Transnet | 面向服务的翻译操作、请求级历史、响应级别、结果 envelope 与后续规范读取。 |
| [SQL 数据 endpoint 接口](mysql_cn.md) | Transnet / publisher → island-port → MySQL | 固定发布版本的规范翻译、卡片、领域、事实、语义尺度、暂存与激活。 |
| [向量数据 endpoint 接口](qdrant_cn.md) | Transnet / publisher → island-port → Qdrant | 候选节点、关系和尺度的检索，以及不可变投影发布。 |

## 建议阅读顺序

1. 从 [Transnet](transnet_cn.md) 开始，了解用户和服务的请求/响应合同。
2. 阅读 [SQL](mysql_cn.md)，了解哪些内容具有权威性并可以作为事实展示。
3. 阅读 [向量](qdrant_cn.md)，了解候选如何检索；向量结果是指针，不是规范事实。

三份文档均使用 [Transnet](transnet_cn.md) 中定义的共享 UDS JSON 传输。MySQL 是权威来源；Qdrant 是可重建、按发布版本固定的检索投影。实时请求文本和请求级历史绝不会发送给这些数据合同。
