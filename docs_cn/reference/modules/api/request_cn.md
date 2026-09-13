# API 请求模型

English: [API request models](../../../../docs/reference/modules/api/request.md)

`src/api/request.rs` 是准确入站 Wire 请求的目标所有者，包括翻译轮次及其可选最小历史项。

状态：目标模块设计。当前运行时使用旧版请求类型与路径，并未实现目标 `/transnet/v1` 请求合同。

## 形状与验证

翻译请求仅包含 `text`、`source_language`、`target_language`、`response_level` 与可选按时间顺序排列的 `history`。初始语言 Selector 在合同允许处为 `auto`、`en` 与 `zh-CN`；响应级别为 `brief`、`standard` 或 `full`。拒绝未知字段。

每个历史项仅包含先前源文本、译文及其语言。历史没有独立条目数限制，但受通用 Body 限制约束。它不携带用户、轮次、时间、反馈、偏好、领域、模型或保存项元数据，并随请求丢弃。

请求模型执行结构与封闭值验证，然后映射为领域输入。它们不选择意图、词义、领域、检索、Provider 或持久化。

## 验证

往返与拒绝测试应覆盖所有封闭值、未知字段、按时间顺序的历史形状、空或无效文本及 Body 边界交互。规范 Schema 见[Transnet 接口](../../../interfaces/transnet_cn.md)。

