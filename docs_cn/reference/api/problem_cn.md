# API 问题响应

English: [API problem responses](../../../docs/reference/api/problem.md)

`src/api/problem.rs` 负责版本化路由的封闭、展示安全失败响应。

状态：现有基础，并非默认目标合同。当前模块为部分版本化路由发出 RFC 9457 风格 `application/problem+json`，而目标 Transnet 接口当前规定 `error` Envelope。实现目标 Handler 时必须解决此形状差异；任一文档均不能视为运行时可用性的证明。

## 映射规则

问题映射把已知验证、容量、依赖、发布、Provider 输出、未找到与 Deadline 失败转换为稳定的状态与 Code 组合。字段错误仅命名安全 Wire 字段。可重试性必须显式且保守。

响应包含请求 ID 与 no-store 策略。Detail 绝不回显当前文本或历史，也不揭示 Provider Body、向量、规范文本、凭据、套接字路径、Peer 详情、SQL、Collection 名称或内部错误链。

意外失败映射为统一通用内部结果，仅在脱敏服务端遥测中保留诊断上下文。领域与应用错误在到达此边界前保持强类型。

## 验证

测试应穷举各封闭映射、Content Type、请求 ID 传播、缓存策略、可重试性与脱敏。目标状态列表与 Envelope 以[Transnet 接口](../../interfaces/transnet_cn.md)为规范。

