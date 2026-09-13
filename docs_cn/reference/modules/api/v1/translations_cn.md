# 版本 1 翻译

English: [Version 1 translations](../../../../../docs/reference/modules/api/v1/translations.md)

`src/api/v1/translations.rs` 是 `POST /transnet/v1/translations` 的目标薄 Handler，也是开始新翻译轮次的唯一入口。

状态：目标路由。当前可执行文件暴露过渡性旧版翻译路由，且未组合目标自动编排、发布固定检索或共享响应。

## Handler 边界

Handler 严格解码简单请求，应用请求上下文与 Deadline，调用请求编排器一次，并把已验证 Aggregate 映射为成功或问题 Envelope。它绝不要求调用方选择模式、领域、方言、受众、目的、检索 Filter 或模型。

可选历史作为请求级语言上下文传递，并在完成后丢弃。Handler 绝不写入请求内容、发布模型输出或创建用户所属状态。规范读取固定到编排器选择的唯一兼容发布三元组。

应用层决定词、短语或段落处理、词义解析、领域评估、检索、Provider 选择、关系组合及响应投影。Handler 不执行这些策略。

## 验证

合同测试应覆盖严格解码、所有响应级别、实质歧义含义、历史丢弃、Deadline 与依赖失败、安全错误及不存在写入。Wire 合同见[Transnet 接口](../../../../interfaces/transnet_cn.md)。

