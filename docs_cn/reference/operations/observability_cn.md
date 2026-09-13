# 可观测性与容错

English: [Observability and resilience](../../../docs/reference/operations/observability.md)

本模块负责安全日志、指标、trace、timeout、重试、并发限制与断路器。

## 安全遥测

遥测可包含操作名称、匹配路由模板、安全状态分类、延迟、有界重试次数、断路器状态与粗粒度 payload 大小分桶。不得包含请求文本、历史、译文、prompt、模型输出、凭据、provider body、规范内容正文、capability 或可稳定关联用户的标识符。指标标签必须闭合且低基数。

## 容错

所有下游工作消耗一个调用方 deadline。单项 timeout 不能延长它。重试必须有界、带抖动，且仅适用于明确可重试的幂等操作。每个依赖拥有独立并发限制和断路器；取消及所有错误路径都必须释放 permit。

Liveness 报告进程健康。Readiness 报告服务能否安全接收工作，并反映必需依赖状态，而不把可选增强能力当作致命依赖。

## 验证

测试结构性脱敏、闭合标签、匹配路由命名、timeout 预算、重试分类、取消、permit 释放、断路器转换、half-open 探针与就绪降级。
