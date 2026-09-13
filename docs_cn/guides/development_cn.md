# 开发与运维

English: [Development and operations](../../docs/guides/development.md)

根目录 [README](../../README.md) 说明前置条件、构建、校验、运行和探针。仓库修改遵循[约定](../conventions-v1.0.1_cn.md)。

## 运维

Release 二进制为 `target/release/transnet`，应由 Supervisor 运行。保留 `logs/debug/transnet.log` 或 `logs/release/transnet.log`；适用日志文件在每次启动时替换。Ctrl-C 和 Unix 终止信号会触发优雅停机。结构化事件记录请求 ID、路由或适配器边界、结果类别和耗时，不记录请求内容、响应内容、上下文、凭据或身份。

默认可执行文件尚未组合目标生产 MySQL 和 Qdrant 适配器。条件式词义与图 HTTP 切片以及进程内测试适配器不代表生产规范存储已经可用；以[服务接口](../interfaces/port_cn.md)中的状态为准。
