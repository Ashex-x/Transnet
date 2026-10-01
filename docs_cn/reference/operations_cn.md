# 运维模块

English: [Operations module](../../docs/reference/operations.md)

运维模块负责安全可观测性、依赖容错与离线发布。在线遥测和容错参与每个请求；发布保持为独立且不可到达的组合。

## 可观测性

[可观测性合同](observability_cn.md)负责版本化事件 Schema、允许维度、禁止内容、采样、缓冲、审计边界与验证。日志、指标和 trace 使用闭合且不含内容的字段；审计事件限于离线发布与控制转换。遥测失败绝不改变在线结果。

Liveness 报告进程健康。Readiness 报告服务能否安全接收工作，并区分必需依赖与可选增强。

## 容错

所有下游工作消耗一个调用方 deadline；单项 timeout 不能延长它。重试必须有界、带抖动，且仅适用于明确可重试的幂等操作。每个依赖有独立并发限制和断路器。取消与所有错误路径都必须释放 permit。

## 离线发布

已实现的 Transnet slice 准备 projection、通过 publication port 完成 reconciliation，并提供显式且仅离线的 release control，用于 candidate submission 与 retained rollback selection。Island-port 仍负责权威写入、collection mutation、持久 audit state 与原子 active-pointer transaction。Client 校验 idempotency、proof echo、封闭 outcome 与 append-only audit sequence receipt；它绝不接入在线 request handling。

Publisher 是单独组合根，也是唯一允许接收可变更数据 port 的组件。生成候选不是证据；只有证据、权利、校验与审核策略通过后才成为规范内容。

实时翻译与查询绝不调用发布、创建持久提案，也不把请求文本或输出写入规范存储。操作步骤由[内容发布指南](../guides/content-publishing_cn.md)负责。

## 验证

测试完整可观测性合同、deadline 预算、重试分类、取消、permit 释放、断路器转换、就绪降级、发布核对、激活门禁、隔离、回滚及在线路径无法访问 mutation。
