# 请求编排器

English: [Request orchestrator](../../../docs/reference/application/request-orchestrator.md)

请求编排器在 API 完成线上结构解码与校验后，负责一个目标翻译 turn。它协调 application service，不把传输、provider、检索或展示策略塞进 handler。

状态：目标设计。当前可执行文件尚未组合此模块；回环 handler 直接连接现有翻译与旧结构化查询路径。

## 职责与流程

编排器接收已校验请求、请求 ID、deadline 和兼容的活动发布固定值。它调用单元与意图路由，再进入连续文本或词汇/领域路径，最后投影并校验一个超集结果。该 turn 中每次规范与向量读取都使用同一个发布三元组。

它负责整个 turn 的取消和降级决策。它可使用有界备用 provider；Qdrant 不可用时可返回仅含 MySQL 内容的词汇结果；共享 deadline 耗尽时停止下游工作。它不会把存储或 provider 故障解释成规范事实。

## 依赖与不变量

依赖是窄 application service，以及时间、规范结构化数据、向量候选、翻译/组织模型和安全聚合指标的只读 port。在线组合绝不接收具备变更能力的发布 port。

当前文本、历史、模型 body 与请求级提案只存在于请求拥有的内存中，并在返回前丢弃。缓存键、指标、trace、队列和持久 adapter 均不得保留这些内容。响应级别只改变投影广度，不选择另一发布、不重新执行事实选择，也不改变真实性状态。

## 验证

测试应覆盖两条路由分支、所有读取共用一个发布固定值、deadline 取消、有界 provider fallback、Qdrant 降级、依赖不兼容及不存在持久写入。合同测试应证明所有成功路径都经过响应投影与校验，且安全错误不泄露请求或依赖内容。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [系统设计](../../transnet_cn.md)
- [质量保证](../../guides/quality-assurance_cn.md)
