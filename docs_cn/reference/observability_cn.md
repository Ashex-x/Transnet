# 可观测性合同

English: [Observability contract](../../docs/reference/observability.md)

本文定义 Transnet 进程、其 adapter 与离线 publisher 的目标全系统遥测合同，覆盖结构化日志、trace、指标和审计事件。当前 Rust 基础已实现闭合的无内容事件 envelope、严格的内部 `traceparent` 准入与 HTTP 传播、当前 matched HTTP route 的类型化分类、闭合指标维度、有界非阻塞指标分发、本地丢弃计数，以及仓库拥有的逻辑导出策略。更广的 operation instrumentation、exporter adapter、collector transport、审计持久化、保留与部署策略仍属目标工作。

## 目标与失败规则

遥测用于回答请求是否准入、运行了哪条有界路径、时间消耗在哪里、哪个依赖失败、是否发生降级，以及发布状态是否变化，同时不得记录被翻译材料或任何终端用户身份。

在线遥测采用非阻塞、尽力而为策略。缓冲区满、collector 不可用、序列化错误或导出超时只增加本地丢弃事件计数，不得使业务响应失败、延迟、重试或改变。只有显式配置的强制审计 sink 可以阻止离线发布状态转换；它绝不影响在线翻译 readiness。

当前 dispatcher 会在容量耗尽或异步 runtime 不可用时执行本地计数。逻辑导出合同另行区分确定性 sampling、exporter 不可用、导出 timeout、serialization 失败与 exporter shutdown。当前 dispatcher 无法观察 adapter failure，因此这些计数归未来 exporter 所有。本基础不配置外部 collector。

## 逻辑导出合同

仓库拥有的 mode 是闭合的：`disabled` 不创建 exporter boundary，`configured` 则要求单独组合 production exporter。`configured` 刻意不指定 socket、HTTP、gRPC、stdout 或其他 transport。Collector endpoint 命名、认证、framing、batching、acknowledgement 与 retry 规则必须在实现 adapter 前由 deployment 或 collector owner 确认。

一个 configured policy 接受 1 至 4,096 条记录的 queue capacity，以及 1 毫秒至 5 秒的单次 export timeout。这些是仓库拥有的隔离边界，而不是已经启用的 TOML 设置。当前 runtime 不解析 `[telemetry]` table、不构造 exporter，也不声称这项配置已经可用。后续 adapter PR 必须先冻结外部 transport，再在一个纵向切片中把严格配置映射为经过校验的 policy。

Sampling 只应用于结构化 success 与 failure event。两个独立 rate 使用 0 至 10,000（含边界）的整数 basis point，其中 0 表示永不选中，10,000 表示总是选中。选择算法仍属于未来 exporter contract；它必须是确定性的，不得检查请求内容或 identifier，并必须在有界 queue admission 前执行，因此 sampled-out 不计为 capacity exhaustion。Metric record 与 telemetry-drop record 是必需 observation，绝不 sampling，因此 metric counter 与 failure accounting 不会被静默缩减。不采用 adaptive 或 content-dependent sampling。

逻辑 export record 要么是现有闭合 `ObservabilityEvent`，要么是带 timestamp 的 `MetricEvent`；两者使用相同的固定 schema、service identity、package version与闭合 deployment environment。该合同只冻结仓库拥有的数据语义，不冻结 JSON 或网络 framing。它无法携带自由 attribute、message、URL、path、query、body、模型输出、reasoning、canonical material、credential、token 或任意 error/Debug payload。

配置接线存在后，无效 policy value 属于启动配置错误。Runtime exporter 不可用、timeout、serialization failure、queue exhaustion 或 shutdown 仍是 best-effort telemetry failure：它们不得改变业务结果、HTTP response、liveness、readiness 或 dependency availability。有界 queue 是隔离边界，collector outage 不得把无界 backpressure 传播到请求。

## 信号归属

| 信号 | 用途 | 保留与基数 |
|---|---|---|
| 结构化日志 | 可诊断的生命周期与闭合失败事件 | 短期运维保留；无 body |
| Trace | island-port、Transnet、模型与数据边界间的请求及依赖耗时 | 采样；静态 span 名与有界 attribute |
| 指标 | 速率、延迟、饱和度、可用性与质量策略计数 | 仅聚合；闭合低基数 label |
| 审计事件 | 离线发布与安全相关控制转换 | 追加式受限 sink；无请求内容 |

Island-port 创建或校验内部请求与 trace 标识符。Transnet 不接受任意互联网 trace baggage，也绝不把遥测关联当作用户身份。每个依赖 span 都是已准入请求的子 span，并共享其 deadline。

当前 HTTP 边界仅接受一个规范 W3C version-00 `traceparent`，规范化十六进制字符，将已校验值存入请求 extension，并在响应中传播。畸形、重复、不支持版本及全零标识符会被丢弃。系统不接纳 `tracestate` 和任意 baggage。

## 通用事件 Schema

每条结构化日志、trace event 和审计事件都使用版本化 envelope，包含 `event_schema`、`timestamp`、`severity`、`service`、`service_version`、`environment` 与 `event_name`。请求路径事件可增加 `request_id`、`trace_id`、`span_id`、`operation`、静态 `route`、`outcome`、安全 `error_code`、`duration_ms`、`deadline_remaining_bucket`、`request_size_bucket`、`response_size_bucket` 与 `content_release`。

已实现的 envelope 有意从所需公共字段及闭合 route、outcome 和 dependency enum 起步，不包含自由格式 message 或 attribute map。请求标识符、耗时/大小 bucket、内容发布及更广的执行维度暂不加入，等待其所属 request-context 与路由 instrumentation 落地。

只允许以下闭合执行维度：`input_kind`（`text`、`segments` 或 `image_regions`）、`inference_profile`（`none`、`fast` 或 `reasoning`）、`reasoning_escalated`、`retrieval_mode`（`offline`、`allowed` 或 `required`）、`retrieval_used`、`degraded`、`dependency`、`attempt`、`retry_count`，以及断路器或 bulkhead outcome。精确文本长度、图像尺寸、segment 数、引文 URL、模型 token、SQL 文本、规范标签及此处未明确列出的 ID 不是通用遥测字段。指标使用比日志和 trace 更粗的分桶。

准入完成、application 完成、每个依赖逻辑调用完成、reasoning 升级、实时检索完成、降级、取消、停机与发布转换时发出事件。重试是一个逻辑依赖调用的子事件；每个请求的完成事件恰好发出一次。

## 内容禁令

任何信号都不得包含请求或响应 body、文本或子串、分段、保护范围、术语、历史、图像、OCR 文本、译文、备选、prompt、系统指令、provider body、隐藏 reasoning、embedding、向量值、实时搜索查询、结果标题或摘要、抓取页面、引文 URL、规范内容正文、原始 SQL、凭据、授权材料、cookie、socket peer 细节、用户/账户/session 标识符，或从上述内容派生的稳定 fingerprint。

错误使用闭合 code 与脱敏依赖分类。Debug 格式和 panic 路径遵循同一规则。对禁用内容进行 hash 并不会使其安全：确定性 hash、查询 fingerprint 与逐请求内容 digest 同样禁止，因为它们允许关联或字典恢复。

## 日志与 Trace

生产日志采用换行分隔 JSON，写入标准输出或配置的本地 collector。紧凑文本与有界滚动文件仅用于开发。导出使用有界内存队列，绝不使用无界 channel 或请求内容 spool。轮转、压缩、保留与访问控制由部署负责，且必须在生产前验证。

Span 名是 `translation.execute`、`model.generate`、`embedding.search`、`live_retrieval.search`、`canonical.read` 与 `publication.activate` 等静态操作。HTTP request span 通过闭合 `StaticRoute` catalog 分类 Axum matched route template，并且只发出该稳定 identity。未知、已移除或未安装的 optional template 统一成为 `unmatched`；原始 URI、path、query 与请求内容绝不作为 fallback。成功 trace 按已配置低比例采样。失败、reasoning 升级、实时检索与降级可采用更高但有界的采样率，但采样不得检查内容。采样决定与丢弃事件计数作为指标。

隐藏模型 reasoning 是不透明的 provider 执行。遥测可记录选择了 reasoning profile 及其总耗时；不得请求、解码、保留或导出思维链。

## 指标

必需指标族覆盖准入/拒绝请求、完成 outcome、延迟、进行中工作、deadline 耗尽、模型 profile 调用、reasoning 升级、embedding 调用、实时检索使用与失败、依赖 attempt、重试、断路器/bulkhead outcome、降级、响应校验失败、遥测丢弃及发布转换。Histogram 使用固定 bucket。Label 在代码中枚举，不得包含 request ID、发布版本、语言标签、模型字符串、领域、关系类型或错误消息。

特定发布诊断使用有界日志或 trace，而不是无界指标 label。质量评估指标由离线许可测试数据集产生，不通过记录生产请求内容或模型响应产生。

## 审计事件

审计事件仅用于离线发布与控制面动作：stage 创建、校验、审核决定、隔离、激活、回滚、移除、配置接受及被拒绝的安全边界事件。它们可标识服务 actor 或 job、发布/schema 版本、转换、校验 outcome、聚合数量、manifest hash、封闭 reason code、append-only audit sequence 与时间戳。严格 release-control client 要求 authority receipt 回显精确 sequence，并在 gap 或 conflict 时闭合失败。

审计事件绝不包含来源 body、生成候选、证据文本、prompt、查询数据或审核者自由文本。部署支持时，publisher 原子写入审计事件与状态转换；否则激活在可见前闭合失败。

## 验证

合同测试捕获每个信号 sink，并在成功、校验失败、依赖失败、timeout、取消、reasoning、视觉、实时检索与 panic-safe 路径中搜索植入的 secret 和请求片段。测试还强制闭合指标 label、静态 span 名、单一完成事件、trace parent 连续性、队列边界、丢弃行为、审计/状态顺序，以及遥测失败不得改变在线响应。

当前仓库测试证明严格的 trace-parent 解析与传播、每条当前 matched route 的闭合分类、unmatched 闭合处理、trace-context 与 provider reasoning Debug 脱敏、无内容 envelope 构造、闭合指标 label、有界分发，以及单调的容量/runtime 丢弃计数。BasicCard lookup 与 lexical structured model validation 是最早接入 instrumentation 的 operation slice：安装可选 internal dispatcher 时，BasicCard 发出 HTTP request-validation 与 application 拥有的 resolution completion；lexical validation 则为每个逻辑 lexical response validation 恰好发出一个终态 `Accepted`、`Repaired` 或 `Rejected`。Provider call/attempt/retry、reasoning escalation、connected/chunk/image validation、pinned sense、knowledge-view/path、relationship-page、readiness 与 publication operation instrumentation 仍属后续工作。当前仍未安装 production exporter、collector transport 或 telemetry runtime configuration。一个 scoped 合成 sentinel harness 在不安装全局 subscriber 的前提下覆盖成功与拒绝的 HTTP 工作、结构化文本与图像输入、provider 输出失败、私有 header 拒绝、原始 unmatched path 与 query、请求级 live value，以及测试中植入 sentinel 的精确 SHA-256 指纹。Provider 遥测、collector、基础设施日志、retention、备份、数据库与 vector store 无法由仓库测试观察，仍属于更广的生产验收目标。

部署验收验证 collector 传输、访问控制、轮转、保留、备份行为、provider 侧遥测与删除策略。仅靠仓库测试无法证明这些外部控制。

## 相关文档

- [运维模块](operations_cn.md)
- [请求数据流](application/request-dataflow_cn.md)
- [模型运行时](model-runtime_cn.md)
- [配置](../guides/configuration_cn.md)
- [质量保证](../guides/quality-assurance_cn.md)
