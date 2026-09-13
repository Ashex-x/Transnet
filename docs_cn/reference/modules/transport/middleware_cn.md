# 传输中间件

English: [Transport middleware](../../../../docs/reference/modules/transport/middleware.md)

`src/transport/middleware.rs` 是 UDS Listener 与版本化 Handler 之间的目标请求控制层。

状态：目标模块设计。当前 API 包含部分请求 ID 与 Body 限制行为，但完整 UDS 中间件栈尚未实现或组合。

## 顺序与安全

中间件建立或验证不透明请求 ID，应用服务器 Deadline，获取有界并发 Permit，分派静态路由，记录安全聚合结果，并把未捕获失败映射到封闭问题 Envelope。每个响应都返回 `X-Request-Id`。

请求 Deadline 绝不在下游延长。容量耗尽时迅速失败，取消会释放 Permit，Panic 或内部错误不披露存储、Provider、文件系统或请求详情。

在策略允许处，Metric 可包含请求 ID、静态 Endpoint 模板、结果类别、字节数和耗时。不得包含带标识符的原始路径、Body、当前文本、历史、规范文本、向量、凭据或套接字 Peer 详情。

## 验证

测试应证明中间件顺序、请求 ID 传播、Deadline 取消、Permit 释放、安全失败映射及遥测脱敏。另见[问题响应模块](../api/problem_cn.md)与[Transnet 接口](../../../interfaces/transnet_cn.md)。

