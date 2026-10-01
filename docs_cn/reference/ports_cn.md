# Port 模块

English: [Ports module](../../docs/reference/ports.md)

Port 模块定义 application service 从模型和数据依赖所需的窄操作。Port 使用已校验 domain 类型、显式 deadline、闭合结果和发布标识符，不暴露 provider 或数据库协议。

## 模型操作

生成 port 通过同一已配置 VLM 的 `fast` 或 `reasoning` profile 提供有界翻译、视觉读取与结构化生成。Application 代码选择操作，编排器应用闭合升级策略；两者都不选择 provider 品牌。Embedding port 只暴露请求级候选提名。发布流程把冻结的规范 dense 与 lexical 输入发送给 island-port；只有 island-port 执行并证明发布 embedding 与 lexical encoding。Provider URL、凭据、HTTP envelope、prompt、reasoning 控制、图片编码、JSON Schema 机制、重试 header 与模型专属 payload 属于 adapter。

模型输出绝不是规范证据。调用方必须在使用前校验结构与引用，并在请求结束时丢弃请求内容和生成材料。

[模型运行时参考](model-runtime_cn.md)负责调用预算、升级、视觉与 embedding 生命周期。隐藏 reasoning 绝不是 domain value 或响应字段。

基础 `GenerationPort` 与 `EmbeddingPort` 已实现。两者都接收不可变请求 context 与协作式取消，使用有界脱敏值，并返回闭合错误。`ReasoningBudget` 为编排提供每请求一次的原子 claim；后续翻译工作必须在 application 中拥有符合条件的升级策略，而不是把该策略放入 adapter。

## 数据操作

结构化读取解析活动发布、规范候选、完整词义详情、领域清单、事实、证据与来源。向量读取检索按发布过滤的节点、边候选及有界浅层邻域。Method 表达这些用例，而不是 SQL、Qdrant-native 请求或通用 repository 访问。

每次读取携带请求 deadline 和精确发布标识符。闭合结果区分缺失、不兼容、不可用、无效与权限过滤数据，且不泄露被隐藏内容。

在线组合不接收 mutation method。独立发布 port 可暂存、校验、投影、激活、隔离、移除与回滚已审核规范内容，并绝不传给请求 handler。精确操作由[规范数据](../interfaces/canonical-data_cn.md)与[检索数据](../interfaces/retrieval-data_cn.md)合同负责。

## 验证

使用 fake 测试操作选择、deadline 传播、发布一致性、畸形模型输出、闭合失败映射、过滤数据及在线组合中不存在 mutation 权限。
