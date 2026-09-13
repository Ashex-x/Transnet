# 传输与 API 边界

English: [Transport and API boundary](../../../docs/reference/transport/server.md)

本模块负责请求准入，以及 HTTP/JSON 与 application 操作之间的映射。路由、body、envelope、标识符、deadline 与错误以 [Transnet 服务接口](../../interfaces/transnet_cn.md)为规范来源。

## 当前运行时

可执行文件当前绑定回环 TCP 并暴露过渡路由。已有版本化 handler 和 problem response 是基础；只有路由注册、组合与合同测试齐全时才表示实现了目标 UDS 合同。

## 目标 server

目标 listener 在一个所属 Unix socket 上提供 HTTP/1.1 JSON。准入在 application 工作开始前强制媒体类型、UTF-8、body 大小、未知字段拒绝、请求 ID、deadline 和并发限制。陈旧 socket 恢复必须区分遗留 inode 与活动 listener。

Middleware 顺序是确定的：识别路由、建立安全请求上下文、应用限制与 deadline、调用 handler、映射失败、记录不含内容的遥测。取消和 permit 必须在所有退出路径释放。

## Handler 规则

探针、翻译、词义和图 handler 必须保持轻薄：解码并校验线上结构，调用一次 application 操作，再编码已记录结果。它们不选择模型、不推断领域、不构造数据库查询、不遍历图，也不持久化请求数据。

## 验证

合同测试覆盖精确路由与 method、严格 JSON、大小边界、未知字段、请求 ID、timeout 与取消、安全错误映射、可选依赖的路由注册、UDS 所有权，以及迁移后不存在 TCP listener。
