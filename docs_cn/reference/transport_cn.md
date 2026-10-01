# 传输与 API 边界

English: [Transport and API boundary](../../docs/reference/transport.md)

本模块负责请求准入，以及 HTTP/JSON 与 application 操作之间的映射。路由、body、envelope、标识符、deadline 与错误以 [Transnet 服务接口](../interfaces/transnet_cn.md)为规范来源。

## 当前运行时

可执行文件只绑定其所属 Unix socket，并以共享 envelope 与 problem response 基础设施暴露目标 `/api/v1` 操作。TCP、CORS、`POST /translate`、`/v1/lookups`、`/v1/senses/*` 与原始 `/v1/graph*` 路由均不存在。

## 目标 server

目标 listener 在一个所属 Unix socket 上提供 HTTP/1.1 JSON。准入在 application 工作开始前强制媒体类型、UTF-8、body 大小、未知字段拒绝、请求 ID、deadline 和并发限制。陈旧 socket 恢复必须区分遗留 inode 与活动 listener。

Middleware 顺序是确定的：识别路由、建立安全请求上下文、应用限制与 deadline、调用 handler、映射失败、记录不含内容的遥测。取消和 permit 必须在所有退出路径释放。

已实现的准入 middleware 现在会校验或生成一个安全 request ID，并从可选 `X-Deadline-At` 派生一个绝对请求 deadline；默认值为 30 秒，并拒绝超过 120 秒的调用方预算。它在 handler 运行前把公开的请求安全上下文放入 request extension。该上下文只公开关联信息、schema、deadline 预算和可选不可变发布 ID，不包含 request body、身份、凭据或任意 header。可选的 `AppState` knowledge route bundle 会把两个相对 route 原子注册到 `/api/v1` 下，同时激活匹配的 capability 与精确快照 readiness，并创建 request-owned cancellation guard。request 被丢弃时取消其 path 工作，可注入的 runtime signal 也能在 drain 期间取消该工作。当前 executable 尚未注入这个可选 bundle。自动 timeout 取消及向其他所有 application port 传播仍是目标工作。

## Handler 规则

探针、翻译、BasicCard、sense read、knowledge view 与 knowledge path handler 必须保持轻薄：解码并校验线上结构，调用一次 application 操作，再编码已记录结果。它们不选择模型、不推断领域、不构造数据库查询、不遍历图，也不持久化请求数据。

## 验证

合同测试覆盖精确路由与 method、严格 JSON、大小边界、未知字段、请求 ID、timeout 与取消、安全错误映射、可选依赖的路由注册、UDS 所有权，以及迁移后不存在 TCP listener。
