# 开发与运维

English: [Development and operations](../../docs/guides/development.md)

根目录 [README](../../README.md) 说明前置条件、构建、校验、运行和探针。仓库修改遵循[约定](../conventions-v1.0.1_cn.md)。

使用 `python3 tools/validate_docs.py` 校验 fenced JSON 示例、本地 Markdown 链接及中英文标题层级一致性。提交文档变更前，将此校验器与 Rust 校验命令一同运行。

## 运维

Release 二进制为 `target/release/transnet`，应由 Supervisor 运行。保留 `logs/debug/transnet.log` 或 `logs/release/transnet.log`；适用日志文件在每次启动时替换。Ctrl-C 和 Unix 终止信号会触发优雅停机。结构化事件记录请求 ID、路由或适配器边界、结果类别和耗时，不记录请求内容、响应内容、上下文、凭据或身份。

可执行文件只通过所属 Unix socket 接受 HTTP/1.1，并可按配置组合出站 island-port canonical 与 knowledge client。Island-port server、生产 MySQL/Qdrant 执行、migration 与 publisher、真实 activation/rollback、旧发布保留及生产端到端验收仍属外部工作。请求 body 有界并传播 request ID。
