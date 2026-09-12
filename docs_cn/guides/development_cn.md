# 开发与运维

English: [development and operations](../../docs/guides/development.md)

根目录 [README](../../README.md) 负责前置条件、配置基础、构建、验证、本地启动和 curl 示例。

## 运维

在进程 supervisor 下运行发布二进制，并保留 `logs/release/transnet.log`。非阻塞 logger 每次启动都会替换该文件。进程处理 Ctrl-C 和 Unix termination，以优雅关闭，并通过 `tracing` 记录启动、关闭、请求结果、提供商容错事件和致命服务器错误。

当前可执行文件不终止 TLS，必须在 Island-port 后保持仅回环访问。MySQL 和 Qdrant 接线属于 Island-port；请求体有界且请求 ID 会传播。详见[配置指南](configuration_cn.md)。

相关：[配置](configuration_cn.md)、[设计](../transnet_cn.md)、[Island-port 接口](../interfaces/port_cn.md)。
