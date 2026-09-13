# 知识检索

English: [Knowledge retrieval](../../../../docs/reference/modules/application/knowledge_retrieval.md)

知识检索 application 模块为已解析词义和选定的已有领域构建有界、发布固定的事实 bundle。

状态：基于已有检索、图、发布和内存基础的目标组合。当前 launcher 尚未把生产 island-port 结构化/向量 adapter 接入此流程。

## 职责

模块读取领域知识 profile，只请求有用且可用的事实族，从 Qdrant 获取节点与边候选，再通过权威结构化数据读取补全引用的事实 revision、证据和来源。缺失事实族与部分覆盖保持显式。

候选选择与权威补全是两个独立阶段。短连接路径仅在每一步都是具名且独立满足证据条件的关系时合格；任意深度遍历和相似度链不属于此边界。

## 依赖与不变量

模块依赖面向操作的向量数据与结构化数据读 port，并使用编排器精确的发布三元组和 deadline。Qdrant 是可重建投影；MySQL 或签名发布制品保持权威。

向量相似度绝不证明翻译、同义、分类、因果、共同机理或文化含义。运行时查询文本和临时向量不持久化。Qdrant 故障可产生显式的仅 MySQL 降级结果，不得虚构关系。

## 验证

测试应覆盖依据 profile 选择事实族、发布与 schema 兼容、补全每个展示事实、限制前资格过滤、缺失证据、有界图展开、部分覆盖、Qdrant 降级和 MySQL 故障。注入用例不得绕过发布过滤或伪造来源。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [SQL 数据 endpoint](../../../interfaces/mysql_cn.md)
- [向量数据 endpoint](../../../interfaces/qdrant_cn.md)
- [质量保证](../../../guides/quality-assurance_cn.md)
