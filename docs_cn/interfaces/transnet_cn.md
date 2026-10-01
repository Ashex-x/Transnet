# Transnet 服务接口

English: [Transnet service interface](../../docs/interfaces/transnet.md)

本合同定义目标 island-port 到 Transnet 接口及内部共享 HTTP/1.1-over-UDS 规则。Island-port 负责互联网传输、认证、用户状态、文件接入、文档重建和最终展示。Transnet 不接收终端用户身份，也不持久化实时请求内容。

状态：修订后的目标 v1 合同。仓库中的可执行文件只通过目标入站 UDS 服务 HTTP/1.1，并实现目标 capability discovery、探针、翻译、BasicCard 与固定发布 sense read，以及按条件组合的 knowledge view/path。翻译边界严格校验 tagged text、结构化 segment、image region、history 与专业 guidance；带 guidance 的 text、结构化 segment 与有界 image region 都会进入当前中立 Gemma VLM orchestrator。缺少完整 search/fetch runtime composition 时，实时检索仍不可用。

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

HTTP boundary 已在过渡期与目标 path 上实现请求上下文基础。若存在 `X-Deadline-At`，它必须是准入时刻之后不超过 120 秒、微秒精度的 UTC RFC 3339 时间戳；省略该 header 时使用 30 秒 deadline。无效或过长 deadline 返回 `400 invalid_deadline`，已耗尽 deadline 返回 `504 deadline_exceeded`。Middleware 存储一个不可变 `RequestContext`，其中包含安全 request ID、绝对 deadline、`transnet-service-v1` schema、剩余预算计算以及供后续 application composition 使用的可选发布 pin。翻译生成现在让每次 fast 或 reasoning 调用都消耗该剩余 deadline，并携带协作式 request-local cancellation signal。

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

视觉输入包含经清理的 inline 图片与归一化矩形。调用方提供的 segment、image 与 region ID 是非空 opaque value，最多 128 个 Unicode scalar；image 与 region ID 不得包含 `:`，因为 `reading_order` 使用无歧义的 `image_id:region_id` 形式。坐标是 0 至 1 的有限十进制值，从左上角测量。每个 region ID 在所属图片内唯一；矩形必须具有正面积且位于边界内；`reading_order` 恰好引用每个 region 一次。若涉及文件或 PDF，island-port 在请求前完成渲染与选页。

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

迁移期间，当前 runtime 同时接受 tagged text shape 与 legacy 顶层 `text` 字段；调用方必须且只能发送其中一种。校验后，请求 domain 仅在本次请求内保留完整 tagged input、guidance 与 history。Segment orchestration 通过中立 generation port 为每个 segment 发送严格的 structure-aware prompt，执行有界并行 fast-profile 调用，并且整个请求最多进行一次 reasoning repair。它确定性恢复调用方顺序与 ID；只有每个 protected scalar range 仍逐字且有序、换行数不变、Markdown delimiter 或 HTML tag 精确匹配时才接受输出。Image-region 请求完成有界结构、media header、decoded byte、尺寸、rectangle 与精确 reading-order 校验。Transnet 在本地解码每张图片，将每个已声明 region 裁剪为有界 attachment，在提交 provider 前丢弃 rectangle 外像素，并通过严格的 attachment-to-region binding 执行一次受 deadline/cancellation 约束的 VLM 调用。输出必须按 reading order 精确匹配每个调用方 image/region ID，并使用请求的 target language；无效或乱序输出 fail closed。图片 byte、crop、prompt material、OCR-like text 与输出随请求丢弃。

`history` 可选并按时间排序。每项只包含先前源文本、译文与语言 tag，不含 turn ID、时间、用户 ID、反馈、模型 metadata 或保存状态。不另设项目数上限，但 history 与 guidance 的 JSON 编码合计最多 8,192 byte，从而保证每个已接受的文本请求都符合 65,536-byte generation-input 合同。超限 aggregate 会在任何 model call 前返回 `422 invalid_translation_request`，并只带不含内容的 `generation_context` field。

## 专业 guidance

`guidance` 可选且仅属于当前请求。省略时分别使用 `general`、`general`、`preserve`、零个 alternative、response-level 默认 annotation 与 `offline` freshness。`max_alternatives` 接受零至二。非零值要求可选 relationship-page runtime 与精确解析的 lexical result；否则请求会在 generation 前失败，而不会伪造 alternative。

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

在某个 input family 的 guidance-aware orchestration 完成组合之前，有效但依赖执行的 guidance 返回 `501 translation_capability_unavailable`；runtime 绝不静默忽略已接受的约束。Text 与 segment workflow 执行各自已记录的 guidance。当前 image-region workflow 会在解码或调用 model 前拒绝依赖执行的 guidance。单独显式指定 `offline` freshness 可以接受，因为它维持默认的禁用网络行为。

## 翻译输出

结果由 `unit` 判别：`word`、`phrase`、`passage`、`segment` 或 `image_region`。Word、phrase 与 passage 保留现有有序 `translations`；每个 choice 都有从位置确定的 `translation_id` 与零起始 `order`，并在各投影间保持稳定。Segment 与 image-region 结果保留调用方 ID 及显式的零起始 `order`；每个内层 unit 携带检测语言、主译文、typed annotation 与不变的 review outcome。结构化结果还携带实际执行的有序请求级 `terminology_decisions`。每个 unit 以后可以携带不超过请求数量且实质有用的带标签 alternative；在确定性 evaluator 完成组合前，该 Milestone 5 能力仍不可用。

```json
{
  "translation": {
    "unit": "segment",
    "segments": [
      {
        "segment_id": "seg_title",
        "order": 0,
        "detected_source_language": "en",
        "translations": [{"translation_id": "translation_0", "order": 0, "text": "发布 {product_name}", "language": "zh-CN"}],
        "annotations": [
          {"type": "format", "code": "protected_content_preserved", "message": "Protected content was copied unchanged."}
        ],
        "review": {"state": "clean", "issues": []}
      }
    ],
    "terminology_decisions": []
  }
}
```

图片输出使用 `unit: "image_region"`，其 `regions` 含 `image_id`、`region_id`、零起始阅读 `order`、检测语言、翻译、annotation 与 review。它不返回图片或无限制 OCR transcript。Review state 为 `clean` 或 `review_recommended`；闭合 issue code 包括 `low_confidence`、`source_ambiguous`、`terminology_conflict`、`format_risk`、`protected_content_mismatch`、`visual_order_uncertain` 与 `live_source_incomplete`。

`brief`、`standard` 与 `full` 是一个已验证超集的确定性投影。较低级别移除支持详情，但绝不改变所选含义、译文、protected content、证据状态或 review outcome。空分区省略。

Typed annotation 使用闭合 family `ambiguity`、`terminology`、`register`、`culture`、`format` 与 `review`；闭合 code 为 `ambiguity_detected`、`term_selected`、`protected_content_preserved`、`register_applied`、`cultural_context`、`format_preserved`、`review_required` 与 `live_source_used`。每项 annotation 均包含有界 display message 及可选的响应级 citation reference。`data.external_sources` 如存在，只包含被这些 citation 引用的 source；source、claim 与 fragment ID 只在本次响应内有效，绝不是规范证据。当前 offline 文本路径不返回 external source。结果校验会在序列化前拒绝未知 citation target、重复 source/claim pair、重复 source ID、identity/order 缺口、重复 review issue、与 issue 矛盾的 clean review state，以及空主译文。

HTTP 序列化之前会执行 request-bound validation：structured result 的 ID 与顺序必须精确匹配原始 segment 或 image reading order；每条 translation 必须使用请求的 target language；显式声明的 source language 必须被保留。Translation ID 必须严格为 `translation_<zero-based order>`。Passage、segment 与 image-region unit 只有一条 primary connected-text translation，且不携带 lexical meaning/detail object；word 与 phrase unit 保留有界 meaning label，并且只允许与自身类型一致的 generated exploratory detail shape。Annotation code 只能属于一个闭合 family，review issue 严格排序，且 `review_required` 仅且必须出现在 `review_recommended` unit。Projection 删除最后一条 citation 被移除后不再被引用的 source descriptor。Schema、normalizer、projector、model、prompt、profile、retrieval 与 release metadata 都会接受边界和一致性校验，不会未经检查直接透传。

## 实时检索

`guidance.freshness` 接受 `offline`、`allowed` 或 `required`。`offline` 是默认值并禁止网络检索。当前 claim-bound 实现只为 text 支持非 offline freshness；segment 与 image-region 请求使用 `allowed` 或 `required` 时，会在网络、解码或生成前以 `501 translation_capability_unavailable` 失败。对于 text，`allowed` 是显式许可，只在确定性分类发现时效性 claim 时检索。`required` 始终尝试检索；live sub-deadline 到期或有界依赖操作无法安全完成时返回 `503 live_retrieval_unavailable`，调用方整体 deadline 耗尽仍返回 `504`。

实时检索是受编排的 search/fetch port，不是无限制模型浏览。它最多执行一轮搜索、选择五个结果、并发抓取三个页面，并遵守配置的子 deadline。Fetcher 只允许公开 HTTP(S)，解析并校验每次 redirect，拒绝 loopback、link-local、私有、保留及 Unix-socket 目标，限制响应 byte，并只接受配置的文本 media type。

`required` 始终消耗唯一一次检索轮次，并且只有至少一个安全抓取页面产生可用文本时才成功。`allowed` 只有在确定性分类发现 freshness-sensitive claim 且 canonical material 不足时才消耗该轮次。只要至少一个页面安全，部分抓取成功即可使用。Redirect 上限为三次；每页上限为 512 KiB，全部页面合计上限为 1 MiB，每个保留 fragment 上限为 8,192 个 Unicode scalar。允许的 media type 为 `text/plain`、`text/html` 与 `application/xhtml+xml`。Live sub-deadline 默认为五秒，不得超过十五秒，并始终受请求剩余 budget 限制。

抓取内容是不可信数据。它不能修改系统指令、请求其他 URL、泄露凭据、绕过发布 filter 或成为规范证据。Embedding 模型可在内存中排序抓取片段；片段与向量均随请求丢弃。

基于实时检索的 claim 使用严格的响应级 `{source_id, claim_id}` citation。Lexical claim 使用 `translation_N`；connected-text claim 使用有序重组产生的确定性 `chunk_N` identity。抓取 fragment 是结构化的不可信 prompt 数据，绝不是 instruction。每个实时辅助 claim 都必须引用至少一个已准入 `live_N` source；伪造、重复、缺失、跨 claim citation 或暴露未引用 source 都会失败关闭。Operation 之外只返回已引用 source 的 title 与最终校验过的抓取 URL，每个 descriptor 标记为 `live_external` 而不是 `verified`，metadata 记录 `translation-live-v1`，抓取材料随请求丢弃。无法获得材料或只达到 live sub-deadline 的 `allowed` 尝试会明确退化为建议 review；相同情况下 `required` 映射为 `503 live_retrieval_unavailable`。

```json
{
  "external_sources": [
    {
      "source_id": "live_1",
      "title": "Example current terminology notice",
      "url": "https://example.org/notices/current-term",
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

安装 translation orchestrator 会原子地公布 `segments`、`image_regions`、三种已接受图片媒体类型与 `format` annotation family；缺少该依赖的 state 不公布其中任何一项。扁平的 v1 capability shape 无法表达按 input 区分的 guidance 支持，因此在所有已公布 input 都执行相同 purpose 集之前，`purposes` 保持为空。替换调用方提供的 capability 声明不能部分移除或虚构这个原子集合。

实时检索 capability 还会声明具备完整 claim-bound attribution 的精确 `input_types`。当前已组合实现只报告 `text`，并增加用于 `live_external` attribution 的 `review` annotation family；不可用 deployment 会同时报告空 input 列表、没有 live review family 与 `available: false`。

知识 lens 的激活是原子的。`AppState` 只接受一个经过验证且不可拆分的 knowledge route dependency bundle，其中 view 与 path service 必须共享同一个完整不可变 projection expectation；安装它时也会同时安装与该快照匹配的 active-release readiness。未提供 bundle 时两个 route 都不存在。只有安装该 bundle 时，runtime 才公布 `meaning`、`contrast`、`usage`、`form`、`origin` 和 `domain`，即使调用方提供了陈旧 capability 声明也不例外。`mechanism` 与 `application` 仍不公布，因为其显式技术关系策略尚不可执行。只有可选 knowledge 配置与完整 active release trio 校验成功后，executable 才构造该 bundle。

Capabilities 遵循整个 interface 的响应策略：每个响应都携带 `Cache-Control: no-store`。调用方可以在需要当前部署信息时重新获取，但合同不承诺 HTTP cache 或 validator 语义。

请求：`{}`

```json
{
  "data": {
    "source_languages": ["auto", "en", "zh-CN"],
    "target_languages": ["en", "zh-CN"],
    "input_types": ["text", "segments", "image_regions"],
    "image_media_types": ["image/png", "image/jpeg", "image/webp"],
    "purposes": [],
    "annotation_families": ["format"],
    "knowledge_lenses": [],
    "limits": {"max_request_body_bytes": 1048576, "max_translation_bytes": 1048576, "max_generation_context_bytes": 8192, "max_lexical_chars": 128, "max_connected_chunk_chars": 8192, "max_connected_chunks": 128},
    "live_retrieval": {"available": false, "default": "offline", "input_types": []},
    "generation_profiles": ["fast", "reasoning"],
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

仅当每个已配置必需依赖都能安全服务新请求时返回 `200`。知识 bundle 只报告 `canonical_data`、`retrieval_data` 与 `knowledge_projection`，其闭合值为 `available`、`unavailable` 或 `disabled`；响应不暴露 release ID、collection ID、hash 或 endpoint。只有活动 trio authority 返回 view service 所期望的准确完整规范 pin 与不可变 node/edge projection tuple 时，已配置 bundle 才可用。任何缺失、无效或不一致的 tuple 都使整个原子 bundle 不可用。未配置的 bundle 把三个 component 都报告为 disabled，且不影响 readiness。

请求：`{}`

```json
{
  "data": {
    "status": "ready",
    "components": {
      "canonical_data": "disabled",
      "retrieval_data": "disabled",
      "knowledge_projection": "disabled"
    }
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/translations

接受上述翻译 input、语言 tag、response level、可选 guidance 与可选 history。它自动选择词汇知识、连续文本、结构化 segment 或视觉 region 处理。调用方绝不选择模型、推理 profile、chunk policy、规范发布、检索策略或 repair policy。

成功 metadata 包含实际参与的 result、normalizer、projection、model 与 prompt 版本；可选 `content_release`、`retrieval_version`、`embedding_version` 与 `live_retrieval` 只在使用时出现。`inference_profiles` 有序且去重。`reasoning_escalated` 报告安全策略结果而不暴露 reasoning 内容。

只有已解析的 word 与 established-phrase 结果可以嵌入 relationship-page object。Passage、segment 与 image-region 结果绝不包含它。页面先给出绑定完整 release pin 的 BasicCard 或 concept-summary authority receipt，再对受支持 direct group、完整 semantic scale、可选 verified short path、带标签的 generated example 与 inferred explanation，以及视觉上独立的 exploratory section 应用确定性 progressive disclosure。Verified group 保留精确的第一步 hydration proof，且只接纳闭合 grouping policy 声明的 relation；不受支持的 mechanism 与 application claim 会 fail closed。完整 semantic scale 与被接纳的 taxonomy group 原子化包含。显式请求的 labeled alternative 每个 lexical unit 最多两个，绑定 stable translation ID 与 order，并说明改变的维度、实际后果与 usefulness reason；它们不是 alias 或 normalized lookup form。request-local relationship-gap nomination 不携带规范 endpoint、relation 或 evidence authority，不进入在线 response，只作为独立 offline review workflow 的输入。系统不新增公开 relationship-page route。只有原子配置的 material authority 提供匹配 root、目标语言、response level、完整 release pin 与已校验 material 时，response 才嵌入页面。存在 verified group、path 或 evidence-grounded explanation 时状态为 `complete`；只有这些 relationship section 均为空时才是 `canonical_only`。仅在 translation orchestrator 与该 authority 都已安装时，capabilities 才包含 `relationship-page-v1`，builder 顺序不能公布部分 runtime。Authority deadline 与 cancellation 保留标准闭合 problem 语义；请求 labeled alternative 的结构化输入在 generation 前失败。默认 executable 因生产 root-resolution 与 canonical/retrieval composition 尚未完成而省略 material authority。

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

响应包含 root、lens、有序 branch、稳定 node、显式 path、relevance reason、assertion 与证据引用、evidence state、截断状态及不透明 next cursor。有序 branch 精确划分每个非 root item；truncation 为 true 当且仅当存在 next cursor。响应宽度始终使用 request 绑定的 level。Mechanism 与 application 在规范 registry 提供显式技术 traversal family 之前保持不可用；弱主题关联不能建立这两个 lens。每个事实 path step 都从响应完整内容 pin 下的精确规范 assertion 补全结果构建，并沿选定 registry traversal 从 source 前往 target；反向 traversal 需要单独声明的 inverse。Cursor 绑定 root、闭合 lens、语言、response level、release、projection version 与 ordering version，且不含请求文本。

已实现的 target cursor foundation 使用不透明 `k1.<nonce>.<ciphertext>` 格式，以 XChaCha20-Poly1305 保护，并采用与过渡期 graph cursor 不同的 authenticated context。其加密 payload 绑定有类型 root family/ID、八个闭合 lens 之一、语言、response level、完整 canonical release/schema pin、不可变 node/edge collection ID 与 SHA-256 hash、assertion/registry/projection/lens-policy/ordering version、稳定 ordering key，以及最多八个有界 dependency continuation token。解码要求与当前请求 binding 精确一致，并拒绝未知 lens 或 cursor 版本、超长值、畸形 base64、篡改、不同密钥或变化后的 release/projection 数据。Payload 不包含 query text、生成 prose、evidence excerpt、credential、user identity 或 durable history。该 foundation 本身不公开 route，也不加入 application state。

仓库中的 handler bundle 现已同时实现该 route 与 path route，但尚未把两者加入默认 runtime composition。它严格拒绝未知 JSON field，把共享 request context 映射到标准 success envelope，并为成功、problem 与错误 method 响应统一设置 `Cache-Control: no-store`。格式正确但并非 bundle 已配置不可变 release 的 pin 返回 `409 content_release_unavailable`；格式错误的字段返回 `422`。Handler 针对当前请求与不可变 execution binding 精确解码公开 `k1` cursor，只把稳定的末项 ordering key 传给 application，再为响应加密下一个 ordering key。它绝不把公开 cursor 转发到 retrieval-data，也不暴露 dependency continuation token。Application 会耗尽有界 dependency pagination，先物化并确定性排序完整合格 superset，再选择 response-level page，因此恢复后的页面不会遗漏 child frontier，也不会因 dependency page boundary 改变顺序。每个序列化 verified step 都包含 release ID 与 canonical schema version；若 relation 没有声明 projection wire name，则失败关闭。

成功响应使用共享 envelope，并采用以下 route-specific shape：

```json
{
  "data": {
    "root": {"kind": "lexical_sense", "id": "sense_sweltering_hot_01"},
    "lens": "contrast",
    "content_release": "knowledge-2026-09",
    "canonical_schema_version": "canonical-v1",
    "branches": [
      {
        "order": 1,
        "reason": "contrast",
        "item_ids": [{"kind": "lexical_sense", "id": "sense_cool_01"}]
      }
    ],
    "items": [
      {
        "node": {"kind": "lexical_sense", "id": "sense_sweltering_hot_01"},
        "order": 1,
        "relevance_reason": "contrast",
        "evidence_state": "verified",
        "path_to_root": null
      }
    ],
    "truncated": false,
    "next_cursor": null
  },
  "meta": {
    "request_id": "01JKNOWLEDGEVIEW0000000000",
    "schema_version": "knowledge-view-result-v1",
    "content_release": "knowledge-2026-09"
  }
}
```

## POST /api/v1/knowledge/paths

在两个规范 root 之间返回最多三条、每条最多三跳且独立验证的路径。Server 选择并强制关系资格，不执行任意深度或 shortest-path inference。

```json
{
  "from": {"kind": "concept", "id": "concept_coriolis_force"},
  "to": {"kind": "concept", "id": "concept_weather_system"},
  "target_language": "en",
  "content_release": "knowledge-2026-09",
  "canonical_schema_version": "canonical-v1"
}
```

request body 与两个嵌套 node reference 都拒绝未知字段。Node kind 使用闭合 canonical family catalog；lexeme root、虚构 family name 与超过 256 个 Unicode scalar 的 node identifier 会被拒绝。`target_language` 是严格 BCP-47 tag，不可变 release 与 canonical schema 组成共享 request context 使用的完整 pin。格式正确但不可用的 pin 返回 `409 content_release_unavailable`，并与格式错误的输入区分。

```json
{
  "data": {
    "outcome": "connected",
    "from": {"kind": "concept", "id": "concept_coriolis_force"},
    "to": {"kind": "concept", "id": "concept_weather_system"},
    "target_language": "en",
    "paths": [{
      "order": 1,
      "steps": [{
        "edge_id": "edge_weather_17",
        "relationship_revision": 2,
        "assertion_id": "assertion_weather_17",
        "assertion_revision": 3,
        "traversal_id": "traversal_cause_effect",
        "relation": "has_subtype",
        "relation_registry_revision": 1,
        "direction": "forward",
        "source": {"kind": "concept", "id": "concept_coriolis_force"},
        "target": {"kind": "concept", "id": "concept_weather_system"},
        "conditions": [],
        "evidence_ids": ["evidence_weather_4"],
        "content_release": "knowledge-2026-09",
        "canonical_schema_version": "canonical-v1"
      }]
    }]
  },
  "meta": {
    "request_id": "01JPATHREQUEST0000000000000",
    "schema_version": "knowledge-path-result-v1",
    "content_release": "knowledge-2026-09"
  }
}
```

正常 outcome 为 `connected` 与 `no_verified_path`；后者在相同 success envelope 中返回空 `paths` array。`no_verified_path` 表示有界搜索没有找到每条 edge 都能在固定发布中成功补全精确合格事实修订的路径；它不证明搜索边界之外、其他发布或未发布知识中不存在关系。每个 path step 命名一个已补全 assertion、已声明 forward direction、结构化 condition、relation relevance、证据引用与完整 release pin。只基于相似度的候选绝不成为 path step。

畸形 JSON 或 content type 返回 `400 invalid_json`；无效字段返回 `422 invalid_knowledge_path_request`；退役 pin 返回 `409 content_release_unavailable`；dependency 与 incomplete-search failure 返回可重试 `503` problem；deadline 耗尽返回可重试 `504 deadline_exceeded`；矛盾不可变 proof 返回 `502 invalid_knowledge_proof`。每个 success 与 problem response 都使用 `Cache-Control: no-store`；错误绝不回显 node ID 或 dependency payload。

旧目标草案 `POST /api/v1/graph/get`、`POST /api/v1/graph/neighbors` 与过渡期 `GET /v1/graph...` handler 均已移除。引导式 view/path 是唯一公共 relationship traversal 表面。

## 相关文档

- [系统设计](../transnet_cn.md)
- [规范数据接口](canonical-data_cn.md)
- [检索数据接口](retrieval-data_cn.md)
- [模型运行时](../reference/model-runtime_cn.md)
- [持久化边界](../reference/persistence_cn.md)
- [质量保证](../guides/quality-assurance_cn.md)
