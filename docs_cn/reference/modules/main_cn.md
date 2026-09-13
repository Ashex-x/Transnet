# 主启动器

English: [Main launcher](../../../docs/reference/modules/main.md)

`src/main.rs` 负责在线进程入口。运维人员与贡献者可通过本文了解 bootstrap 接管控制前允许发生的工作。

状态：部分实现。当前启动器读取配置、初始化日志、组合仅模型服务、绑定回环 TCP，并处理 Ctrl-C 或终止信号。目标启动器把组合委托给 `bootstrap`，在不暴露秘密的情况下报告致命启动失败，并选择进程退出状态。

## 边界

Main 不解析请求或业务输入，也不包含翻译、检索、持久化或发布策略。它为在线服务调用唯一 bootstrap 路径；若本仓库拥有离线发布器，则使用单独的二进制与组合。

目标成功路径为 `main -> bootstrap -> 有界 accept loop -> 优雅关闭`。启动错误保留运维上下文，但排除凭据、请求文本、Provider Body 和规范内容。

## 验证

除仓库检查外，启动器变更还需覆盖启动失败和信号关闭。当前行为位于 [`src/main.rs`](../../../src/main.rs)；目标组合由[服务模块参考](../modules_cn.md)与[系统设计](../../transnet_cn.md)定义。

