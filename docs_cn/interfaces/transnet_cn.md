# Transnet 服务接口

English: [Transnet service interface](../../docs/interfaces/transnet.md)

本合同定义目标 island-port 到 Transnet 接口及内部共享 HTTP/1.1-over-UDS 规则。Island-port 负责互联网传输、认证、用户状态、文件接入、文档重建和最终展示。Transnet 不接收终端用户身份，也不持久化实时请求内容。

状态：修订后的目标 v1 合同。仓库中的可执行文件已通过目标入站 UDS 服务 HTTP/1.1，并实现目标 capability discovery 以及 health、liveness 与依赖 readiness probe，另有已记录的过渡期翻译、BasicCard、固定发布 sense 与旧 graph 切片。迁移期间可以通过显式配置保留 loopback listener。结构化 segment、image region、实时检索、引导式知识视图和知识路径，必须等 handler、组合、测试与文档共同落地后才算已实现。

## 目录

- [连接与线上规则](#连接与线上规则)
- [隐私与请求生命周期](#隐私与请求生命周期)
- [Deadline 与调用预算](#deadline-与调用预算)
- [成功与错误 envelope](#成功与错误-envelope)
- [翻译输入](#翻译输入)
- [专业 guidance](#专业-guidance)
- [翻译输出](#翻译输出)
- [实时检索](#实时检索)
- [知识视图](#知识视图)
- [POST /api/v1/capabilities](#post-apiv1capabilities)
- [POST /api/v1/health](#post-apiv1health)
- [POST /api/v1/livez](#post-apiv1livez)
- [POST /api/v1/readyz](#post-apiv1readyz)
- [POST /api/v1/translations](#post-apiv1translations)
- [POST /api/v1/basic-cards/lookup](#post-apiv1basic-cardslookup)
- [POST /api/v1/senses/get](#post-apiv1sensesget)
- [POST /api/v1/knowledge/views](#post-apiv1knowledgeviews)
- [POST /api/v1/knowledge/paths](#post-apiv1knowledgepaths)
- [相关文档](#相关文档)

## 连接与线上规则

Transnet 监听 `/run/transnet/transnet.sock`；Transnet 数据 adapter 调用 `/run/island-port/island-port.sock`。部署可以通过配置迁移 socket，但 endpoint 路径和 payload 不变。Socket owner 创建父目录，只在证明旧 socket 不活动后移除它，以 `0660` 模式绑定，并依赖文件系统 workload identity，而不是转发的用户 header。

每个操作使用 HTTP/1.1、origin-form `/api/v1/...` 路径、`Host: localhost`、UTF-8 JSON 与 `POST`。空输入为 `{}`。Client 发送 `Content-Type: application/json`、`Accept: application/json`、有界 `Content-Length` 和可选 `X-Request-Id`、`X-Deadline-At`。拒绝 query string、chunked request body、multipart body、upgrade 与响应 streaming。Server 拒绝未知 JSON 字段，并返回 `X-Request-Id` 与 `Cache-Control: no-store`。

默认 body 上限为 1 MiB。含 inline 图片的翻译请求可使用该路由专属 12 MiB 编码 body 上限。最多接受四张解码图片，每张不超过 2 MiB 或 4096 × 4096 像素；整个请求最多十六个 region。支持 `image/png`、`image/jpeg` 与 `image/webp`。

ID 是不透明 URL-safe 字符串。时间戳为 UTC RFC 3339 微秒精度。语言值是 capability 公布的规范 BCP 47 tag；`auto` 只允许用于源语言。

## 隐私与请求生命周期

Transnet 不接受用户、学习者、账户、owner、session、cookie、bearer token、画像、偏好、保存状态、掌握状态或持久历史。Island-port 可以发送当前文本、有界 inline 图片、专业 guidance 及按时间排序的最小历史翻译，作为请求级语言上下文。

文本、segment、图片、protected range、术语、历史、规范化形式、chunk plan、provider 输入输出、隐藏 reasoning、实时搜索 query 与结果、推断解释及在线 embedding 只在请求内存在。它们绝不进入 MySQL、Qdrant、日志、trace、指标、cache、持久队列、备份、发布候选或后续训练数据。

规范内容只能通过经认证的离线发布工作流进入存储。实时结果绝不自行发布。产品自有保存、编辑、反馈与文档状态保留在 island-port。

## Deadline 与调用预算

一个调用方 deadline 覆盖完整操作。规范读取、embedding、生成与经许可实时检索获得受剩余时间限制的子 deadline，且不能延长请求。

当前 HTTP boundary 已在过渡期与目标 path 上实现请求上下文基础。若存在 `X-Deadline-At`，它必须是准入时刻之后不超过 120 秒、微秒精度的 UTC RFC 3339 时间戳；省略该 header 时使用 30 秒 deadline。无效或过长 deadline 返回 `400 invalid_deadline`，已耗尽 deadline 返回 `504 deadline_exceeded`。Middleware 存储一个不可变 `RequestContext`，其中包含安全 request ID、绝对 deadline、`transnet-service-v1` schema、剩余预算计算以及供后续 application composition 使用的可选发布 pin。现有 handler 尚不会在预算耗尽时自动取消；下游采用将逐步完成。

足够的规范命中使用零次生成调用。普通翻译、视觉读取、分类与 grounded composition 使用 Gemma4-27B `fast` profile。长输入在同一模型上使用有界语义 chunk、有界并行 fast 调用、一个请求级术语台账与确定性重组。

一个请求最多使用一次 `reasoning` profile，仅用于无法消除的重要歧义、冲突术语或格式约束、fast composition 无法安全表达的已验证多跳解释，或一次无效结构化 fast 结果。输入长度本身绝不触发 reasoning。隐藏 reasoning 绝不返回。精确策略由[模型运行时参考](../reference/model-runtime_cn.md)负责。

## 成功与错误 envelope

成功使用 `data` 与 `meta`。`meta.request_id` 与响应 header 一致。可选 metadata 只在对应组件实际参与时出现。

```json
{
  "data": {},
  "meta": {
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "schema_version": "translation-result-v1",
    "inference_profiles": ["fast"],
    "reasoning_escalated": false
  }
}
```

错误使用 `application/problem+json`，绝不回显私有内容，并拒绝未知字段。

```json
{
  "type": "about:blank",
  "title": "Invalid translation request",
  "status": 422,
  "code": "invalid_translation_request",
  "detail": "One or more translation fields are invalid.",
  "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
  "retryable": false,
  "errors": [{"field": "target_language", "message": "is not supported by this deployment."}]
}
```

公共状态包括：`400` 畸形 JSON 或未知字段、`401` workload 认证失败、`404` 未知规范资源、`409` 固定发布不可用、`413` body 过大、`415` 不支持的图片类型、`422` 无效语义输入、`429` 有界容量、`502` 无效依赖结果、`503` 必需依赖不可用，以及 `504` deadline 过期。

## 翻译输入

`input` 是 tagged union。`text` 是最简单的默认输入。

```json
{
  "input": {"type": "text", "text": "The launch date is still up in the air."},
  "source_language": "auto",
  "target_language": "zh-CN",
  "response_level": "standard"
}
```

结构化文档与 localization 输入使用有序 segment。Segment ID 属于请求级并原样返回。`role` 接受 `title`、`paragraph`、`list_item`、`caption`、`ui` 或 `subtitle`；`format` 接受 `plain`、`markdown` 或 `html`。Protected range 使用从零开始的 Unicode scalar offset，start 包含、end 不包含，不得重叠，并必须精确复现。

```json
{
  "input": {
    "type": "segments",
    "segments": [
      {
        "segment_id": "seg_title",
        "text": "Launch {product_name}",
        "role": "title",
        "format": "plain",
        "protected_ranges": [{"start": 7, "end": 21}]
      }
    ]
  },
  "source_language": "en",
  "target_language": "zh-CN",
  "response_level": "standard"
}
```

视觉输入包含经清理的 inline 图片与归一化矩形。坐标是 0 至 1 的有限十进制值，从左上角测量。每个 region ID 在所属图片内唯一；矩形必须具有正面积且位于边界内；`reading_order` 恰好引用每个 region 一次。若涉及文件或 PDF，island-port 在请求前完成渲染与选页。

```json
{
  "input": {
    "type": "image_regions",
    "images": [
      {
        "image_id": "page_1",
        "media_type": "image/png",
        "data": "<base64-encoded PNG>",
        "regions": [
          {"region_id": "heading", "x": 0.05, "y": 0.06, "width": 0.9, "height": 0.12}
        ]
      }
    ],
    "reading_order": ["page_1:heading"]
  },
  "source_language": "auto",
  "target_language": "en",
  "response_level": "standard"
}
```

文本最多 131,072 个 Unicode scalar。Segment 输入最多 256 项，每项 8,192 scalar，总计 131,072 scalar。每个 segment 最多 128 个 protected range。这些限制仍受编码 body 上限约束。

`history` 可选并按时间排序。每项只包含先前源文本、译文与语言 tag，不含 turn ID、时间、用户 ID、反馈、模型 metadata 或保存状态。公共 body 上限约束 history，不另设项目数上限。

## 专业 guidance

`guidance` 可选且仅属于当前请求。省略时分别使用 `general`、`general`、`preserve`、零个 alternative、response-level 默认 annotation 与 `offline` freshness。`max_alternatives` 是 Milestone 5 的结果组合能力；Milestone 1 在对应 application result 存在前不接受非零值，也不伪造 alternative。

```json
{
  "purpose": "technical",
  "audience": "specialist",
  "register": "preserve",
  "terminology": [
    {"source": "torque", "target": "扭矩", "policy": "required"}
  ],
  "max_alternatives": 1,
  "annotations": ["ambiguity", "terminology", "register", "culture"],
  "freshness": "offline"
}
```

`purpose` 接受 `general`、`publication`、`technical`、`localization` 或 `subtitles`。`audience` 接受 `general`、`professional`、`specialist` 或 `young_reader`。`register` 接受 `preserve`、`neutral`、`formal` 或 `informal`。术语 policy 接受 `required`、`preferred` 或 `forbidden`；最多 128 项，source 与 target 各不超过 256 scalar。`max_alternatives` 为 0 至 2。

Guidance 约束当前结果，但绝不创建画像、翻译记忆或规范术语。互相矛盾的 required term、protected range 或格式规则返回 `422 constraint_conflict`，而不是静默丢弃约束。

## 翻译输出

文本输出保留现有按含义区分的有序翻译列表。Segment 与 image-region 输出保留请求顺序与 ID。每个 unit 有一个主译文及不超过请求数量且实质有用的 alternative。

```json
{
  "translation": {
    "input_type": "segments",
    "detected_source_languages": ["en"],
    "segments": [
      {
        "segment_id": "seg_title",
        "translations": [{"text": "发布 {product_name}", "language": "zh-CN"}],
        "annotations": [
          {"type": "terminology", "code": "protected_content_preserved", "message": "Protected content was copied unchanged."}
        ],
        "review": {"state": "clean", "issues": []}
      }
    ],
    "terminology_decisions": []
  }
}
```

图片输出使用含 `image_id`、`region_id`、检测语言、翻译、annotation 与 review 的 `regions`。它不返回图片或无限制 OCR transcript。Review state 为 `clean` 或 `review_recommended`；闭合 issue code 包括 `low_confidence`、`source_ambiguous`、`terminology_conflict`、`format_risk`、`protected_content_mismatch`、`visual_order_uncertain` 与 `live_source_incomplete`。

`brief`、`standard` 与 `full` 是一个已验证超集的确定性投影。较低级别移除支持详情，但绝不改变所选含义、译文、protected content、证据状态或 review outcome。空分区省略。

## 实时检索

`guidance.freshness` 接受 `offline`、`allowed` 或 `required`。`offline` 是默认值并禁止网络检索。`allowed` 是显式许可，只在确定性分类发现时效性 claim 时检索。`required` 始终尝试检索；若有界操作无法安全完成，返回 `503 live_retrieval_unavailable`。

实时检索是受编排的 search/fetch port，不是无限制模型浏览。它最多执行一轮搜索、选择五个结果、并发抓取三个页面，并遵守配置的子 deadline。Fetcher 只允许公开 HTTP(S)，解析并校验每次 redirect，拒绝 loopback、link-local、私有、保留及 Unix-socket 目标，限制响应 byte，并只接受配置的文本 media type。

抓取内容是不可信数据。它不能修改系统指令、请求其他 URL、泄露凭据、绕过发布 filter 或成为规范证据。Embedding 模型可在内存中排序抓取片段；片段与向量均随请求丢弃。

基于实时检索的 claim 引用响应级 source。实时 source 标记为 `live_external`，而不是 `verified`。

```json
{
  "external_sources": [
    {
      "source_id": "live_1",
      "title": "Example current terminology notice",
      "publisher": "Example standards body",
      "url": "https://example.org/notices/current-term",
      "published_at": "2026-09-20T00:00:00.000000Z",
      "retrieved_at": "2026-10-01T08:00:00.000000Z",
      "evidence_state": "live_external"
    }
  ]
}
```

## 知识视图

规范知识是证据支持的 assertion graph。知识树是通过某个 lens 生成的确定性 root-specific 投影，绝不作为规范 parentage 存储。同一稳定节点可以出现在多个 lens branch，而不会获得第二身份。

闭合 lens 为 `meaning`、`contrast`、`usage`、`form`、`origin`、`domain`、`mechanism` 与 `application`。初始词汇翻译可以返回 `available_lenses`；follow-up 请求选择其中一个。Transnet 根据 lens 与 response level 推导关系族、边界与排序。调用方不提交原始关系 filter、graph depth、node limit、vector selector 或任意 traversal query。

每个展示 item 要么是 root，要么包含到 root 的显式路径、精简 relevance reason、assertion 与证据引用，以及 `verified`、`inferred` 或 `exploratory` 状态。只有补全后的 `verified` assertion 可组成事实连接路径。Inferred 与 exploratory 材料属于请求级并单独展示。

## POST /api/v1/capabilities

返回当前已实现的 BCP 47 语言 selector、输入类型、图片类型、purpose、annotation family、知识 lens、body 与语义限制、实时检索可用性、generation profile 和 schema 版本。空的闭合集明确表示当前 runtime 尚未实现该能力。它不暴露凭据、provider URL、socket 路径、并发状态或私有 feature flag。

Capabilities 遵循整个 interface 的响应策略：每个响应都携带 `Cache-Control: no-store`。调用方可以在需要当前部署信息时重新获取，但合同不承诺 HTTP cache 或 validator 语义。

请求：`{}`

```json
{
  "data": {
    "source_languages": ["auto", "en", "zh-CN"],
    "target_languages": ["en", "zh-CN"],
    "input_types": ["text"],
    "image_media_types": [],
    "purposes": [],
    "annotation_families": [],
    "knowledge_lenses": [],
    "limits": {"max_request_body_bytes": 1048576, "max_translation_bytes": 1048576, "max_lexical_chars": 128, "max_connected_chunk_chars": 8192, "max_connected_chunks": 128},
    "live_retrieval": {"available": false, "default": "offline"},
    "generation_profiles": ["fast"],
    "schema_versions": ["translation-result-v1"]
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/health

进程可应答时返回 `200`，不探测依赖。请求为 `{}`，响应 data 为 `{"status":"ok"}`。

## POST /api/v1/livez

Event loop 与 listener 存活时返回 `200`，不表示 readiness。请求为 `{}`，响应 data 为 `{"status":"alive"}`。

## POST /api/v1/readyz

仅当每个已配置必需依赖都能安全服务新请求时返回 `200`。可选规范、检索、embedding、vision、reasoning 或实时检索 capability 以闭合 component state 报告；只有配置要求时才成为必需依赖。

请求：`{}`

```json
{
  "data": {
    "status": "ready",
    "components": {
      "generation_fast": "available",
      "generation_reasoning": "available",
      "embedding": "available",
      "canonical_data": "disabled",
      "retrieval_data": "disabled",
      "live_retrieval": "disabled"
    }
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/translations

接受上述翻译 input、语言 tag、response level、可选 guidance 与可选 history。它自动选择词汇知识、连续文本、结构化 segment 或视觉 region 处理。调用方绝不选择模型、推理 profile、chunk policy、规范发布、检索策略或 repair policy。

成功 metadata 包含实际参与的 result、normalizer、projection、model 与 prompt 版本；可选 `content_release`、`retrieval_version`、`embedding_version` 与 `live_retrieval` 只在使用时出现。`inference_profiles` 有序且去重。`reasoning_escalated` 报告安全策略结果而不暴露 reasoning 内容。

闭合路由错误还包括 `invalid_translation_request`、`constraint_conflict`、`unsupported_input_type`、`unsupported_language_pair`、`invalid_image`、`invalid_model_output`、`translation_model_unavailable` 与 `live_retrieval_unavailable`。

## POST /api/v1/basic-cards/lookup

执行只使用规范数据、固定发布的词汇查询。请求接受 `query`、`source_language` 与 `target_language`，不接受 release、派生形式、ranker、index 或 vector selector。正常 outcome 为 `resolved`、`clarification_required` 与 `not_found`。Resolved 结果返回规范 root、合格翻译、证据支持定义、coverage、可用知识 lens 与不可变内容 pin。

该路由保留为直接诊断与兼容性读取。新翻译 turn 使用 `/api/v1/translations`。

## POST /api/v1/senses/get

在先前结果返回的准确 `content_release` 与 `canonical_schema_version` 下读取一个规范 sense。它绝不重新选择活动内容或静默升级 pin。请求接受 `sense_id`、`target_language`、`content_release` 与 `canonical_schema_version`。

## POST /api/v1/knowledge/views

为一个规范 root 返回一个引导式 tree-lens 投影。

```json
{
  "root": {"kind": "sense", "id": "sense_sweltering_hot_01"},
  "lens": "contrast",
  "target_language": "zh-CN",
  "response_level": "full",
  "content_release": "knowledge-2026-09",
  "cursor": null
}
```

响应包含 root、lens、有序 branch、稳定 node、显式 path、relevance reason、assertion 与证据引用、evidence state、截断状态及不透明 next cursor。Cursor 绑定 root、lens、语言、response level、release、projection version 与 ordering version，且不含请求文本。

## POST /api/v1/knowledge/paths

在两个规范 root 之间返回最多三条、每条最多三跳且独立验证的路径。Server 选择并强制关系资格，不执行任意深度或 shortest-path inference。

```json
{
  "from": {"kind": "concept", "id": "concept_coriolis_force"},
  "to": {"kind": "concept", "id": "concept_weather_system"},
  "target_language": "en",
  "content_release": "knowledge-2026-09"
}
```

正常 outcome 为 `connected` 与 `no_verified_path`。每个 path step 命名一个已补全 assertion、方向、条件、relevance、证据引用与 release。只基于相似度的候选可以在独立 exploratory 分区返回，但绝不成为 path step。

旧目标草案 `POST /api/v1/graph/get` 与 `POST /api/v1/graph/neighbors` 已从修订目标合同移除。当前可执行文件中的过渡期 `GET /v1/graph...` handler 在迁移或移除前仍是实现兼容行为；它们的存在不使其成为目标 v1 路由。

## 相关文档

- [系统设计](../transnet_cn.md)
- [规范数据接口](canonical-data_cn.md)
- [检索数据接口](retrieval-data_cn.md)
- [模型运行时](../reference/model-runtime_cn.md)
- [持久化边界](../reference/persistence_cn.md)
- [质量保证](../guides/quality-assurance_cn.md)
