# 配置

English: [Configuration](../../docs/guides/configuration.md)

进程始终相对于 Cargo Manifest 读取 `config/transnet.toml`，不受 Shell 工作目录影响。

目标 listener 配置使用 `socket_path = "/run/transnet/transnet.sock"`、`socket_mode = "0660"` 和由运维管理的套接字用户组。结构化与向量数据客户端通过 `/run/island-port/island-port.sock` 使用 island-port。套接字路径是部署设置，API namespace 固定不变。当前 `[server] host` 与 `port` 仅配置过渡性回环运行时，并在 UDS 服务实现后移除。`RUST_LOG` 覆盖 `log_level`；`log_format = "json"` 选择换行分隔 JSON，其他值选择紧凑文本。

`[http]` 配置 `max_request_body_bytes` 和过渡性 CORS 字段。请求体限制在缓冲 JSON 前应用，默认 1,048,576 字节。UDS 没有浏览器 Origin，目标运行时忽略并最终移除 CORS 配置；浏览器调用产品网关而非 Transnet。

`[translation]` 配置 Unicode 字符数路由边界，以及 Provider 单次超时、首次之后重试次数和重试延迟的旧版默认值。Provider 专属覆盖优先。

`[gemma4]` 和 `[translate_gemma]` 分别配置 OpenAI-compatible `base_url`、`model` 和 `api_key`。默认指向 18011 端口的 Gemma 4 和 18007 端口的 TranslateGemma。真实凭据必须在不提交 Git 的情况下提供；解析后的凭据会从 Rust `Debug` 诊断中脱敏，仅用于出站 Provider 请求。

`[provider_resilience.gemma4]` 和 `[provider_resilience.translate_gemma]` 配置独立容错边界。`timeout_seconds`、`max_retries` 与 `retry_delay_ms` 可覆盖 `[translation]`；`max_retry_delay_ms` 限制 Provider `Retry-After` 延迟；`max_concurrent_requests` 是快速失败 Bulkhead；`circuit_failure_threshold` 是打开熔断器的连续瞬时逻辑调用失败次数；`circuit_open_ms` 是半开探测前的开放时长。省略表时使用 Rust 默认值：8 个并发尝试、阈值 5、开放 30 秒、重试延迟上限 5 秒；仓库中的 TranslateGemma 策略把并发收紧到 4。

仅请求超时、建连失败、`429`、`500`、`502`、`503`、`504` 和不可用的成功 Envelope 会重试。有效 `Retry-After` 代替配置延迟，但受 `max_retry_delay_ms` 限制；其他客户端、服务端或歧义传输失败不再发起请求。熔断器打开或 Bulkhead 已满时，相关请求迅速失败并映射到现有 unavailable 响应。

Provider Trace 只包含静态 Provider 边界、操作名、尝试次数、结果类别、可用时的状态码、耗时和重试延迟。进程通过 Rust 服务 API 暴露内存中的脱敏计数器快照；遥测不含原始查询、上下文、生成答案、Provider Body、凭据或身份。

结构化 `/v1/lookups` 切片使用 `[gemma4]` Provider，并请求严格 JSON Schema 输出。配置的服务必须支持 OpenAI-compatible `response_format.type = "json_schema"` 请求字段，并在第一条 Assistant Message 中返回 JSON 文本。

当前可执行文件没有 island-port 或规范发布配置。这些目标能力需要分别版本化的发布、嵌入、检索、模型角色和评估设置；见 [SQL endpoint](../interfaces/mysql_cn.md)、[向量 endpoint](../interfaces/qdrant_cn.md)和 [Transnet](../transnet_cn.md)合同。不得把凭据或请求内容写入仓库配置。

相关：[设计](../transnet_cn.md)、[Transnet 服务接口](../interfaces/transnet_cn.md)和[开发](development_cn.md)。
