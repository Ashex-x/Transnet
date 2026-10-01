# 配置

English: [Configuration](../../docs/guides/configuration.md)

进程始终相对于 Cargo Manifest 读取 `config/transnet.toml`，不受 Shell 工作目录影响。

已实现的目标 listener 配置使用 `socket_path = "/run/transnet/transnet.sock"`、`socket_mode = "0660"` 和由运维管理的套接字用户组。路径必须为绝对路径、至多 107 byte，且不得包含空白或 `..` component；mode 必须是 `0600` 至 `0770` 的四位八进制值。Supervisor 必须以预期 owner 与 group 创建真实的父目录。Transnet 拒绝非 socket entry 与活动 socket，仅在连接被拒绝证明旧 socket 已失效后移除它，绑定后应用配置 mode，并在关闭时只移除自己拥有的 socket inode。结构化与向量数据客户端通过 `/run/island-port/island-port.sock` 使用 island-port。Socket 路径是部署设置，API namespace 固定不变。

Transnet 仅通过必填的绝对 `socket_path` 服务 HTTP/1.1，并在关闭时给予已接受连接最多 30 秒完成。旧 `host` 与 `port` 键会被拒绝。`RUST_LOG` 覆盖 `log_level`；`log_format = "json"` 选择换行分隔 JSON，其他值选择紧凑文本。

`[http]` 只配置 `max_request_body_bytes`。请求体限制在缓冲 JSON 前应用，默认 1,048,576 字节。CORS 键会被拒绝，因为浏览器调用产品网关而非 Transnet；终端用户认证仍由 island-port 负责。

`[translation]` 配置 Unicode 字符数路由边界，以及 Provider 单次超时、首次之后重试次数和重试延迟的旧版默认值。Provider 专属覆盖优先。

`[gemma4]` 和 `[translate_gemma]` 分别配置 OpenAI-compatible `base_url`、`model` 和 `api_key`。默认指向 18011 端口的 Gemma 4 和 18007 端口的 TranslateGemma。真实凭据必须在不提交 Git 的情况下提供；解析后的凭据会从 Rust `Debug` 诊断中脱敏，仅用于出站 Provider 请求。

这些配置表描述当前过渡期运行时行为。目标配置用一个生成 endpoint 和一个 embedding endpoint 替代它们。生成设置命名一个 Gemma4-27B 模型及 provider 专属 fast/reasoning 控制；application 策略选择 profile，而不是选择第二个 endpoint。独立有界设置覆盖 fast 推理、reasoning 升级、embedding 调用与可选实时检索。在匹配 Rust 类型与组合存在前，仓库配置不会采用这些目标 key。

Provider client 直接连接所配置的 endpoint，不继承操作系统或环境代理设置。这样可避免回环与私有模型流量（包括 Bearer 凭据）进入无关代理进程。

`[provider_resilience.gemma4]` 和 `[provider_resilience.translate_gemma]` 配置独立容错边界。`timeout_seconds`、`max_retries` 与 `retry_delay_ms` 可覆盖 `[translation]`；`max_retry_delay_ms` 限制 Provider `Retry-After` 延迟；`max_concurrent_requests` 是快速失败 Bulkhead；`circuit_failure_threshold` 是打开熔断器的连续瞬时逻辑调用失败次数；`circuit_open_ms` 是半开探测前的开放时长。省略表时使用 Rust 默认值：8 个并发尝试、阈值 5、开放 30 秒、重试延迟上限 5 秒；仓库中的 TranslateGemma 策略把并发收紧到 4。

仅请求超时、建连失败、`429`、`500`、`502`、`503`、`504` 和不可用的成功 Envelope 会重试。有效 `Retry-After` 代替配置延迟，但受 `max_retry_delay_ms` 限制；其他客户端、服务端或歧义传输失败不再发起请求。熔断器打开或 Bulkhead 已满时，相关请求迅速失败并映射到现有 unavailable 响应。

Provider Trace 只包含静态 Provider 边界、操作名、尝试次数、结果类别、可用时的状态码、耗时和重试延迟。进程通过 Rust 服务 API 暴露内存中的脱敏计数器快照；遥测不含原始查询、上下文、生成答案、Provider Body、凭据或身份。

目标遥测还只记录闭合推理 profile（`fast` 或 `reasoning`）、输入类型分类及是否发生 reasoning 升级。它绝不记录图片数据、prompt、隐藏 reasoning、embedding、实时搜索 query、抓取内容或生成输出。

目标 `[telemetry]` 设置选择 NDJSON 标准输出或本地 collector、有界队列容量、导出 timeout、固定成功/失败采样分类及可选的仅开发滚动文件。指标 label 与事件 attribute 是代码编译的闭合集合，而不是任意配置。审计 sink 设置属于离线 publisher 组合，不得使在线翻译 readiness 依赖 exporter。详见[可观测性合同](../reference/observability_cn.md)；在对应类型化实现存在前，不向仓库配置增加这些目标 key。

目标翻译操作使用中立 generation boundary。Provider 专属配置节在 generation adapter 完成合并前仍是内部兼容配置；它们不会启用旧公共路由。

`[canonical]` 是可选启用的生产 canonical-only 读取依赖。仓库配置为 `enabled = false`：进程不构造 island-port client，模型翻译保持原行为，readiness 也不声称 canonical 可用。在 Unix 主机启用时，设置 `enabled = true`、island-port 的绝对 `socket_path`（例如 `/run/island-port/island-port.sock`），以及 1 至 30,000 的 `timeout_ms`。Unix socket 路径不得超过 107 字节，不得含空白或 `..` 路径组件；配置 Debug 与错误不会输出该路径。启用时缺失或无效配置会拒绝启动；非 Unix 主机启用会因没有生产 UDS transport 而拒绝启动。

启用后，进程构造严格的出站 island-port client 与请求局部的 `CanonicalReadService`，供 `POST /api/v1/basic-cards/lookup` 和固定发布的 `POST /api/v1/senses/get` 使用。`POST /api/v1/readyz` 执行有界的 active-release 读取：没有 active release、transport 故障、schema 不兼容、畸形响应或超时都表示未就绪。island-port 服务端缺失或尚未升级时，进程可以继续存活，但目标 readiness 返回 `503`；不得把这种情况当作 canonical miss 或可用能力。socket 不是 MySQL 直连，也不会捏造 vector/ranking 版本。Adapter wire schema 和响应大小边界保持固定严格，不开放调用方配置伪版本。

`[knowledge]` 是 guided view 与 bounded path 的独立 opt-in 组合。仓库配置为 `enabled = false` 与 `required = false`；禁用时无需 secret，并保留 canonical-only runtime。`required = true` 仅在 `enabled = true` 时有效。启用 knowledge 会为 canonical、retrieval 与 active-trio client 复用已成功校验的 `[canonical]` island-port Unix socket 与 timeout；它不能配置竞争 transport 或数据库直连。启动时只读取并校验一次完整的活动 canonical/node/edge tuple，并把该准确 snapshot 的 clone 交给两个 application service、原子 route bundle 与 readiness。缺失或不兼容的活动数据会阻止 required 启动；optional knowledge 则不安装 knowledge route 或 lens，保持 canonical readiness 为权威，并把 retrieval/projection 报告为 unavailable。

启用的 knowledge runtime 必须且只能选择一个稳定 cursor-secret reference：`cursor_secret_env = "TRANSNET_KNOWLEDGE_CURSOR_SECRET"` 或 `cursor_secret_file = "/run/secrets/transnet-knowledge-cursor"`。环境变量名只能包含大写 ASCII 字母、非首位数字与下划线。file path 必须为绝对路径、不得为 symlink，并且必须指向 root-owned regular file，其 group/other permission bit 全部清零，例如 mode `0400` 或 `0600`；其直接 parent directory 也必须由 root 拥有，且 group/other 不可写。Transnet 只以禁止跟随末端 symlink 的方式打开文件一次，校验该已打开 descriptor，并从同一 descriptor 读取。读取文件时会移除惯例性的尾随 CR/LF byte。加载后的 material 必须为 32 至 4,096 byte。绝不能把 secret value 本身放进 TOML；configuration Debug、enabled-setting Debug、加载错误与解析错误都会遮蔽 reference 与 value。

Transnet-side canonical 公开交付已由上述两个路由实现，包括发布/schema 失败映射和 attribution/证据投影。island-port canonical server、生产 MySQL migration、publisher/write 操作、真实发布、激活、回滚与隔离、不可变旧发布服务验证，以及 island-port/MySQL 端到端验收仍属外部目标能力。Knowledge embedding 配置会保持缺席，直到 artifact identity、dimension、input contract、endpoint、timeout、credential 与 redaction 可以一起校验。见[规范数据 endpoint](../interfaces/canonical-data_cn.md)、[检索数据 endpoint](../interfaces/retrieval-data_cn.md)和 [Transnet](../transnet_cn.md)合同。不得把凭据或请求内容写入仓库配置。

相关：[设计](../transnet_cn.md)、[Transnet 服务接口](../interfaces/transnet_cn.md)、[可观测性](../reference/observability_cn.md)和[开发](development_cn.md)。
