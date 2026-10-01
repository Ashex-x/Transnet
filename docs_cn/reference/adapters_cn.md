# Adapter 模块

English: [Adapters module](../../docs/reference/adapters.md)

Adapter 模块实现模型与数据 port。它负责外部协议机制，同时保留 domain deadline、发布固定值、闭合结果与隐私规则。

## 模型 provider

目标 OpenAI-compatible 生成 adapter 负责单个 Gemma4-27B VLM 的 HTTP 构造、认证、响应大小限制、图片编码、严格结构化输出解码、安全错误映射与容错集成。它把 domain-neutral `fast` 与 `reasoning` profile 映射到 provider 设置，而不暴露 provider-native reasoning 字段。独立 embedding adapter 只服务在线临时候选提名。发布 embedding 与 lexical encoding 是 island-port 的执行职责，并通过 publication adapter 返回经证明的 receipt。

当前运行时仍在 `translation.long_text_chars` 处选择 Gemma 4 或 TranslateGemma。这是过渡期已实现行为，不是目标 provider 拓扑。后续运行时切片将移除第二个生成模型，并通过 application 自有分块在同一 VLM 上处理长输入。

已实现的兼容 generation adapter 对两个中立 profile 都调用现有 resilient Gemma endpoint，以剩余请求 deadline 限制调用，观察协作式取消，并校验有界输出和版本 metadata。已实现的 OpenAI-compatible embedding adapter 校验严格的单向量响应、有限固定 dimension 和所配置的 artifact version。Runtime composition 尚未使用这些 adapter；legacy provider adapter 会一直保留到翻译与检索编排迁移。

Provider adapter 绝不记录 prompt、源文本、历史、provider body、凭据或生成内容。错误只暴露闭合依赖与操作分类。

## Island-port 数据访问

Island-port client 把数据 port 操作映射到 island-port 在其所属 Unix socket 上提供的版本化 HTTP/1.1 JSON 调用。它负责连接生命周期、content-type 与 body 限制、schema 版本处理、deadline 和安全传输错误。

Canonical-data client 只实现出站 `canonical-data-v1` read：active-release 选择、翻译候选、词汇候选解析和 sense details。其私有 strict DTO 重建现有 `CanonicalTranslationRevision`、`CanonicalCandidate` 与 `CanonicalSenseDetails`；ranking、fusion、歧义解析和 coverage 仍是 request-local application 工作。Unix build 提供 production socket transport；测试注入有界 fake transport，不增加入站 listener 或数据库 client。

候选读取现在要求固定发布的权威 source 记录、经过审核的非空 attribution，以及一致的 source/evidence 权限；严格 DTO 映射对缺失或冲突 lineage 闭合失败。结构化的 `content_release_unavailable` 与 `schema_incompatible` 分离，固定发布的 sense 读取将返回的规范 schema 与调用方 pin 复核。错误分类不解析 peer message 文本。

Stage 4 在同一出站 transport 中增加 active canonical release 读取。严格响应映射成 canonical-only release pin，不包含向量集合或本地排序策略版本。Application 只获取一次 pin，并传给后续每个权威读取。原有完整 hybrid content tuple 继续用于向量检索与发布兼容性。

可执行文件可通过经校验的运行配置按需构造此出站 transport 与 canonical-only read service。canonical 依赖的就绪检查只使用只读 active-release 操作。公开 BasicCard lookup 与固定发布 sense follow-up 通过 application 边界使用该服务；没有新增入站 UDS listener 或数据库 client。

Publication adapter 是复用同一可注入 transport 的独立 outbound-only client。私有 strict DTO 把 `KnowledgePublicationPort` operation 映射到 `knowledge-publication-v1` 的 begin、node/edge batch、freeze、reconcile、status 与 abort call。其纯 node/edge batch inspection 与最终 send 使用同一套 DTO/JSON/base64 serializer；inspection 不执行 transport I/O，并为最大合法 request-ID/deadline representation 预留空间，send 则再次执行 1 MiB 防御性校验。它还强制校验 request/deadline/release 回显、256-point bound、256 KiB control/response bound、闭合 outcome/code 组合、精确 execution receipt 和 cross-artifact hash。第二个仅离线 adapter 将显式 candidate submission 与 rollback selection 映射到 `release-control-v1`，校验 proof 与 audit-sequence echo，并保持不进入 runtime routing。Fake transport test 验证两个合同，且不增加 Qdrant/MySQL driver、island-port server、authority mutation、embedding execution 或在线 handler wiring。

对应 island-port server 位于本仓库之外，必须同步实现 `interfaces/canonical-data.md` 中的当前 delta。在 peer 升级之前，不兼容或不完整响应会 fail closed，且不能声称真实 island-port/MySQL E2E 已验证。

结构化与向量映射保留发布标识符与闭合结果。Island-port 负责 MySQL 和 Qdrant driver、查询、连接池、事务、collection 选择与凭据。Transnet 不暴露 SQL 或 Qdrant-native 请求。文件系统权限认证进程；JSON 绝不转发终端用户身份或凭据。

在线 adapter 只读。单独授权的 publisher 组合使用可变更操作。精确 payload 保留在[规范数据](../interfaces/canonical-data_cn.md)与[检索数据](../interfaces/retrieval-data_cn.md)接口。

## 验证

测试精确 provider 与 island-port envelope、严格解码、响应边界、脱敏、deadline 传播、发布保留、状态分类、重试资格、断路器行为与本地 socket 失败。
