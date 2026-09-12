# Island-port 接口

English: [Island-port interface](../../docs/interfaces/port.md)

本文档定义 Island-port 暴露 Transnet 学习代理的受信内部 HTTP 边界。[系统设计](../transnet_cn.md)对产品语义最权威。

状态：当前运行时已有健康、翻译、结构化查询和 feature-gated 规范图读取。下文书签学习、写作、语音和复习路由是拟议内容，在运行时和 OpenAPI 同步实现前不得宣称可用。

## 共享线上规则

Transnet 绑定回环地址，不终止公网 TLS。Island-port 先完成终端用户认证和学习者资源授权。除单机回环外，内部部署也必须认证；Transnet 不接受终端用户 Cookie 或 Bearer token。

请求与响应默认使用 JSON。每个响应返回 `X-Request-Id`；时间为 UTC RFC 3339 微秒；ID 是不透明 URL-safe 字符串。学习者路由要求 `X-Learner-Id`，变更要求 `Idempotency-Key`。重用键但内容不同返回 `409 idempotency_conflict`。

标准成功响应包含 `data` 和 `meta`。错误包含稳定 `code`、安全 `message`、`request_id`、`retryable` 和可选字段详情，不回显学习者内容、凭据、provider body 或存储细节。

## GET /health

认证：内部部署策略。只返回 HTTP 进程健康状态。

## GET /livez

认证：内部部署策略。事件循环响应时返回成功。

## GET /readyz

认证：内部部署策略。启用路由的必需依赖可用时返回 `200`，可选降级能力放在 metadata。

## POST /translate

认证：内部部署策略，无需学习者 ID。请求包含文本、可选源语言、目标语言、方言和语域。响应以翻译为主，仅对关键歧义、习语、语域或文化语境增加最多两条一句提示。它不创建历史、书签或学习状态。

## POST /v1/lookups

认证：内部部署策略；`X-Learner-Id` 可选且只能影响临时排序。返回查询分析、精简基础卡、选定词义、翻译维基分区、已验证关系、分开的探索关联、证据与发布。结果不是学习卡，不创建持久目标。Qdrant 故障时可明示降级为 MySQL 基础卡。

## GET /v1/knowledge/nodes/{node_id}/neighbors

认证：内部部署策略。查询按关系类、方向、语言、领域、发布和有界 limit 过滤。响应钉住节点/边版本并分开已验证与探索结果。

## POST /v1/bookmarks

状态：拟议。认证：必须 `X-Learner-Id`。幂等请求指定基础卡、词义和内容发布，原子创建书签和第一个冻结学习卡修订。

## PATCH /v1/bookmarks/{bookmark_id}

状态：拟议。认证：必须 `X-Learner-Id`。包含预期修订，可暂停、恢复、重排或明确刷新。刷新创建新的可追溯修订。

## DELETE /v1/bookmarks/{bookmark_id}

状态：拟议。认证：必须 `X-Learner-Id`。幂等停止未来排程，且 `404` 不泄露他人资源。

## GET /v1/learning-cards

状态：拟议。认证：必须 `X-Learner-Id`。分页返回活动、暂停、到期或待再生成卡，包含独立掌握度、时间、版本和优先原因。

## POST /v1/practice/sessions

状态：拟议。认证：必须 `X-Learner-Id`。从到期书签、紧凑误解和直接相关迁移任务创建会话，不自动加入无书签节点。

## POST /v1/practice/sessions/{session_id}/attempts

状态：拟议。认证：必须 `X-Learner-Id`。幂等提交一次冻结练习，返回 `correct`、`needs_revision` 或 `needs_review`、证据、有界反馈、信心、掌握度影响和下一步。只有高信心证据更新已证明技能。

## POST /v1/writing/evaluations

状态：拟议。请求明确受众、目的、媒介、语域、约束和文本。响应保留意图与声音，返回最小修正、可选自然表达、最多两个重点和重试。原始写作不进入策略历史。

## POST /v1/speech/reference

状态：拟议。生成有明确标记的参考语音，支持有界文本、方言、声音、速度和用途。慢速语音保留自然重音和韵律。

## POST /v1/pronunciation/evaluations

状态：拟议。接受有界音频、预期语言和可选目标短语，返回录音质量、对齐信心、最多两个可理解性目标、证据提示和重试。无足够声学证据时返回 `422 unassessable_audio`，不给分。音频默认不保留。

## GET /v1/history

状态：拟议。认证：必须 `X-Learner-Id`。返回过去 30 天内最多 200 个紧凑规范事件，不返回原始学习者内容。

## DELETE /v1/history

状态：拟议。认证：必须 `X-Learner-Id`。立即移除历史对策略的影响，不删除书签。

## 相关文档

- [系统设计](../transnet_cn.md)
- [学习体验](../product/learning-experience_cn.md)
- [MySQL](mysql_cn.md)
- [Qdrant](qdrant_cn.md)
- [当前 OpenAPI 子集](../../docs/reference/transnet-openapi.json)
