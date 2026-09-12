# 配置

English: [configuration](../../docs/guides/configuration.md)

进程从 Cargo manifest 相对路径读取 `config/transnet.toml`，不受 shell 当前目录影响。

`[server]` 配置 `host`、`port`、`log_level` 和 `log_format`。`host` 必须是 `127.0.0.1` 或 `::1` 等回环 IP；公共边缘部署由 Island-port 负责。`RUST_LOG` 覆盖 `log_level`；`log_format = "json"` 使用逐行 JSON。调试和发布日志分别写入 `logs/debug/transnet.log` 与 `logs/release/transnet.log`，启动时替换对应文件。

`[http]` 配置 `max_request_body_bytes`、`allowed_origins` 和 `allow_credentials`。默认请求体上限为 1,048,576 字节，并在缓冲 JSON 前检查。生产环境应保持 `allowed_origins` 为空；通配符来源会被拒绝。

`[translation]` 配置 Unicode 字符路由边界，以及提供商单次超时、首次之后的重试次数和重试延迟。提供商专属配置优先。

`[gemma4]` 与 `[translate_gemma]` 配置 OpenAI 兼容的 `base_url`、`model` 和 `api_key`。默认端口分别为 18011 和 18007。真实凭据不得提交到 Git；Rust `Debug` 输出会脱敏凭据。

`[provider_resilience.gemma4]` 和 `[provider_resilience.translate_gemma]` 独立配置 `timeout_seconds`、`max_retries`、`retry_delay_ms`、`max_retry_delay_ms`、`max_concurrent_requests`、`circuit_failure_threshold` 和 `circuit_open_ms`。默认并发为 8、阈值为 5、熔断窗口为 30 秒、重试延迟上限为 5 秒；TranslateGemma 的已提交策略将并发收紧为 4。

只有超时、建连失败、429、500、502、503、504 以及不可用的成功响应封装会重试。合法的 `Retry-After` 会在上限内替代配置延迟；其他失败不重复请求。打开的熔断器和已满的 bulkhead 会快速返回现有的 unavailable 响应。

提供商 trace 只包含静态边界、操作名、尝试次数、结果类别、可用的状态码、耗时和重试延迟。不会包含查询、上下文、生成答案、正文、凭据或身份。

结构化 `/v1/lookups` 使用 `[gemma4]`，并要求服务器支持 `response_format.type = "json_schema"`，且第一条 assistant 消息返回 JSON 文本。

MySQL、Qdrant、加密、共享缓存、遥测导出器和 worker 配置不属于 Transnet。参见 [MySQL](../interfaces/mysql_cn.md)、[Qdrant](../interfaces/qdrant_cn.md) 和 [Island-port 接口](../interfaces/port_cn.md)。

相关：[设计](../transnet_cn.md)、[Island-port 接口](../interfaces/port_cn.md)、[开发](development_cn.md)。
