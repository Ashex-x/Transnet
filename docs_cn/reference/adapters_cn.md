# Adapter 模块

English: [Adapters module](../../docs/reference/adapters.md)

Adapter 模块实现模型与数据 port。它负责外部协议机制，同时保留 domain deadline、发布固定值、闭合结果与隐私规则。

## 模型 provider

共享 OpenAI-compatible client 负责 HTTP 构造、认证、响应大小限制、严格结构化输出解码、安全错误映射与容错集成。Gemma 4 负责短文本及有界结构化组织请求策略；TranslateGemma 负责较长连续文本策略。当前运行时在 translation.long_text_chars 处选择两者。

Provider adapter 绝不记录 prompt、源文本、历史、provider body、凭据或生成内容。错误只暴露闭合依赖与操作分类。

## Island-port 数据访问

Island-port client 把数据 port 操作映射到 island-port 在其所属 Unix socket 上提供的版本化 HTTP/1.1 JSON 调用。它负责连接生命周期、content-type 与 body 限制、schema 版本处理、deadline 和安全传输错误。

Stage 3 client 只实现出站 canonical read：翻译候选、词汇候选解析和 sense details。其私有 strict DTO 重建现有 `CanonicalTranslationRevision`、`CanonicalCandidate` 与 `CanonicalSenseDetails`；ranking、fusion、歧义解析和 coverage 仍是 request-local application 工作。Unix build 提供 production socket transport；测试注入有界 fake transport，不增加入站 listener 或数据库 client。

候选读取现在要求固定发布的权威 source 记录、经过审核的非空 attribution，以及一致的 source/evidence 权限；严格 DTO 映射对缺失或冲突 lineage 闭合失败。结构化的 `content_release_unavailable` 与 `schema_incompatible` 分离，固定发布的 sense 读取将返回的规范 schema 与调用方 pin 复核。错误分类不解析 peer message 文本。

Stage 4 在同一出站 transport 中增加 active canonical release 读取。严格响应映射成 canonical-only release pin，不包含向量集合或本地排序策略版本。Application 只获取一次 pin，并传给后续每个权威读取。原有完整 hybrid content tuple 继续用于向量检索与发布兼容性。

可执行文件可通过经校验的运行配置按需构造此出站 transport 与 canonical-only read service。canonical 依赖的就绪检查只使用只读 active-release 操作。公开 BasicCard lookup 与固定发布 sense follow-up 通过 application 边界使用该服务；没有新增入站 UDS listener 或数据库 client。

Publication adapter 是复用同一可注入 transport 的独立 outbound-only client。私有 strict DTO 把 `KnowledgePublicationPort` operation 映射到 `knowledge-publication-v1` 的 begin、node/edge batch、freeze、reconcile、status 与 abort call。其纯 node/edge batch inspection 与最终 send 使用同一套 DTO/JSON/base64 serializer；inspection 不执行 transport I/O，并为最大合法 request-ID/deadline representation 预留空间，send 则再次执行 1 MiB 防御性校验。它还强制校验 request/deadline/release 回显、256-point bound、256 KiB control/response bound、闭合 outcome/code 组合、精确 execution receipt 和 cross-artifact hash。Fake transport test 在不增加 Qdrant/MySQL driver、island-port server、embedding execution、activation 或 runtime handler wiring 的情况下验证该合同。

对应 island-port server 位于本仓库之外，必须同步实现 `interfaces/mysql.md` 中的当前 delta。在 peer 升级之前，不兼容或不完整响应会 fail closed，且不能声称真实 island-port/MySQL E2E 已验证。

结构化与向量映射保留发布标识符与闭合结果。Island-port 负责 MySQL 和 Qdrant driver、查询、连接池、事务、collection 选择与凭据。Transnet 不暴露 SQL 或 Qdrant-native 请求。文件系统权限认证进程；JSON 绝不转发终端用户身份或凭据。

在线 adapter 只读。单独授权的 publisher 组合使用可变更操作。精确 payload 保留在 [SQL](../interfaces/mysql_cn.md) 与[向量](../interfaces/qdrant_cn.md)接口。

## 验证

测试精确 provider 与 island-port envelope、严格解码、响应边界、脱敏、deadline 传播、发布保留、状态分类、重试资格、断路器行为与本地 socket 失败。
