# 容错策略

English: [Resilience policies](../../../docs/reference/modules/resilience.md)

`src/resilience.rs` 负责出站 Provider 调用及目标幂等数据读取的有界 Deadline、并发、重试与熔断器。

状态：部分实现。当前模块应用独立 Provider 策略并暴露安全聚合计数器。目标 island-port 结构化与向量读取必须采用相同有界原则，且不得把写入视为可重试操作。

## 策略

每个依赖拥有独立 Bulkhead 与熔断器。一个逻辑请求具有有界的单次超时与重试次数；有效 `Retry-After` 可替代配置延迟，但不得超过上限。仅重试文档明确的瞬时结果，调用方请求 Deadline 始终约束全部尝试与退避。

熔断器打开或 Bulkhead 已满时迅速失败。半开探测仅接纳有界工作。只有幂等操作允许重试；发布写入必须遵守所属工作流的显式幂等合同，且不属于在线组合。

遥测仅记录静态依赖与操作名、尝试次数、结果类别、安全状态码、耗时和重试延迟。不得包含请求文本、历史、Provider Body、规范文本、向量、凭据或身份。

## 验证

使用暂停时间测试覆盖重试预算、`Retry-After`、状态转换、并发 Permit、取消与 Deadline 耗尽。配置细节见[配置指南](../../guides/configuration_cn.md)，失败映射归[问题响应模块](api/problem_cn.md)所有。

