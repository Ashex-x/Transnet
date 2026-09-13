# Transnet 服务 HTTP 接口

English: [Transnet service HTTP interface](../../docs/interfaces/port.md)

本文档是 Transnet 的主接口合同。Transnet 是共享、无用户状态的翻译与关系知识服务：翻译连续文本，并围绕一个已解析词义或领域概念构建有界的关系型页面。调用产品负责其用户、私有状态和展示工作流。

状态：目标服务合同。当前运行时以过渡性线上格式实现了其中一部分，规范词义和图读取受 feature gate 控制。仓库中的 OpenAPI 镜像本目标合同；运行时可用性以本文档说明为准，不能从机器合同推断。

## 服务边界

Transnet 不接受用户 ID、学习者 ID、账户 ID、Cookie、终端用户 Bearer token、画像、偏好集合、保存项目状态、掌握度状态或个人历史。学习画像、课程、练习、掌握度、复习日程、辅导、进度追踪、写作评估、语音和发音都不是 Transnet 模块。

源文本、查询文本和可选消歧上下文是请求载荷，不是用户记录。它们只能在有界请求生命周期内存在于内存中，绝不能写入 MySQL、Qdrant、日志、指标、trace、缓存或持久队列。需要个性化的调用方只能传递请求级语言选项，并自行保存响应与终端用户的关联。

Transnet 绑定私有地址且不终止公网 TLS。部署认证识别调用服务而非终端用户。除单机回环部署外，网关或服务网格必须认证调用方。

## 共享线上规则

请求和响应均使用 JSON。每个响应返回 `X-Request-Id`。时间为 UTC RFC 3339 微秒精度；ID 为不透明 URL-safe 字符串，客户端不得推断类型或顺序。未知请求字段会被拒绝。

成功应用响应使用 `data` 和 `meta`。`meta.request_id` 与响应头一致；规范读取还会返回用于应答的不可变内容发布。

```json
{
  "data": {},
  "meta": {
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "content_release": "knowledge-2026-09"
  }
}
```

错误使用统一安全 envelope，绝不回显请求文本、上下文、凭据、provider body、向量或存储内部信息。

```json
{
  "error": {
    "code": "invalid_request",
    "message": "The request is invalid.",
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "retryable": false,
    "fields": [
      {
        "field": "target_language",
        "message": "A valid BCP 47 language tag is required."
      }
    ]
  }
}
```

常见状态码为：`400` JSON 格式错误或未知字段，`401` 部署认证失败，`404` 未知规范资源，`409` 发布冲突，`413` 请求体过大，`422` 语义输入无效，`429` 有界容量限制，`502` provider 结果无效，`503` 必需依赖不可用，`504` 超时。

下面 GET 示例中的请求 JSON 仅为路径和查询参数的文档表示；GET 请求没有 JSON body。

## GET /health

返回进程健康，不探测依赖，也不泄露配置。

请求参数：

```json
{}
```

响应 `200`：

```json
{
  "data": {"status": "ok"},
  "meta": {"request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX"}
}
```

## GET /livez

进程事件循环可响应时返回成功。

请求参数：

```json
{}
```

响应 `200`：

```json
{
  "data": {"status": "ok"},
  "meta": {"request_id": "req_01K4Z8Q8X2A6B7C4D9E0F3G5HJ"}
}
```

## GET /readyz

仅在已启用路由所需依赖就绪时返回 `200`。可选能力可降级而不使进程变为未就绪。

请求参数：

```json
{}
```

响应 `200`：

```json
{
  "data": {
    "status": "ok",
    "dependencies": {
      "mysql": "ready",
      "qdrant": "ready",
      "translation_provider": "ready"
    },
    "capabilities": {
      "translation": "available",
      "canonical_lookup": "available",
      "relationship_pages": "available"
    }
  },
  "meta": {"request_id": "req_01K4Z8R4CX7E2J6K1M9N3P5Q8S"}
}
```

`503` 使用标准错误 envelope，代码为 `not_ready`。它可说明依赖类别，但不得暴露主机、凭据、集合名称或 provider 响应。

## POST /translate

翻译有界文本。`source_language` 可为 `auto`；其余选项仅是本次请求的语言指令。方言、领域、上下文、受众、目的和语域只影响本响应，不创建历史、翻译记忆、规范事实或可复用画像。

请求：

```json
{
  "text": "That plan is still up in the air.",
  "source_language": "en",
  "target_language": "zh-CN",
  "dialect": "en-US",
  "domain": "general",
  "audience": "general",
  "purpose": "inform",
  "preserve_formatting": true,
  "register": "neutral"
}
```

响应 `200`：

```json
{
  "data": {
    "translation": "那个计划仍然悬而未决。",
    "detected_source_language": "en",
    "tips": [
      {
        "kind": "idiom",
        "message": "“up in the air” means undecided rather than physically airborne."
      }
    ]
  },
  "meta": {
    "request_id": "req_01K4Z8S1AK6C8D2F0G4H7J9M3N",
    "model_version": "translate-2026-09"
  }
}
```

`tips` 最多两条，每条一句；没有实质价值时省略。上下文不足时，`alternative` 可包含一个明确标注的译文和理由，否则省略。按请求保留受保护片段、段落结构与格式。长文处理使用的分块计划或术语台账随请求丢弃。

## POST /v1/lookups

将单词、术语、习语、短语动词或固定短语解析为一个选定规范词义或领域概念，再围绕该根组织精简翻译维基页面。歧义返回排序候选或澄清结果。`context`、`domain`、`audience`、`purpose`、`register` 和 `dialect` 只影响本次请求的词义选择、排序和解释；它们不改变规范身份或关系事实。`detail` 只控制响应大小，不用于个性化。

请求：

```json
{
  "query": "sweltering",
  "source_language": "en",
  "explanation_language": "zh-CN",
  "dialect": "en-US",
  "domain": "weather",
  "context": "a sweltering afternoon",
  "audience": "general",
  "purpose": "translation",
  "register": "neutral",
  "detail": "full",
  "include": ["meaning", "degree", "contrasts", "collocations", "usage"]
}
```

响应 `200`：

```json
{
  "data": {
    "query_analysis": {
      "normalized_form": "sweltering",
      "detected_language": "en",
      "match_class": "exact_canonical"
    },
    "result_mode": "lookup",
    "selected_root": {
      "kind": "sense",
      "sense_id": "sense_sweltering_hot_01",
      "node_id": "node_sweltering_hot_01"
    },
    "domain_assessment": {
      "classification": "general",
      "candidate_domain_ids": ["domain_weather"],
      "reason": "The selected sense describes uncomfortable atmospheric heat."
    },
    "page": {
      "basic_card": {
          "card_id": "card_sweltering_en_adj_01",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_form": "sweltering",
          "part_of_speech": "adjective",
          "definitions": ["uncomfortably hot, especially because of the weather"],
          "translations": ["酷热的", "闷热难耐的"],
          "domain_ids": ["domain_weather"]
      },
      "pronunciations": [{"dialect": "en-US", "ipa": "/ˈswɛltərɪŋ/"}],
      "examples": [
          {
            "text": "We waited until evening to leave the sweltering house.",
            "translation": "我们一直等到傍晚才离开闷热难耐的房子。"
          }
        ],
      "relationship_sections": [
        {
          "kind": "degree",
          "title": "Intensity",
          "items": [
          {
            "edge_id": "edge_sweltering_scorching_01",
            "relation_type": "higher_degree",
            "target_node_id": "node_scorching_heat_01",
            "target_label": "scorching",
            "explanation": "Scorching usually expresses a stronger degree of heat.",
            "restrictions": {"dimension": "temperature_intensity"},
            "evidence_state": "verified",
            "confidence": 0.96,
            "provenance": ["evidence_dictionary_1042"]
          }
          ]
        }
      ],
      "connection_paths": [],
      "exploratory_sections": []
    },
    "alternatives": []
  },
  "meta": {
    "request_id": "req_01K4Z8T5BN2P6Q9R1S3V7W0XYZ",
    "content_release": "knowledge-2026-09",
    "degraded": false
  }
}
```

`relationship_sections` 按用途排序，并省略空或证据不足的分组。项目使用 `verified`、`inferred` 或 `exploratory` 证据状态；推断和探索内容只存在于请求内，不能静默表述为规范事实。每条连接路径都很短，且每一步都有命名关系和独立合格证据。若 Qdrant 不可用但 MySQL 已解析基础卡，Transnet 返回关系与路径分区为空且 `meta.degraded: true` 的卡片，不得编造替代关系。

## GET /v1/senses/{sense_id}

读取一个规范词义及其精简 MySQL 卡片。服务不保存访问或已保存项目记录。

请求参数：

```json
{
  "path": {"sense_id": "sense_sweltering_hot_01"},
  "query": {"explanation_language": "zh-CN", "release": "knowledge-2026-09"}
}
```

响应 `200`：

```json
{
  "data": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "aliases": ["oppressively hot"],
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [{"language": "zh-CN", "text": "酷热的"}],
    "forms": [{"form": "swelteringly", "label": "adverb"}],
    "examples": [{"text": "We waited until evening to leave the sweltering house.", "translation": "我们一直等到傍晚才离开闷热难耐的房子。"}],
    "usage_notes": ["Usually describes weather or an uncomfortably hot place."],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"]
  },
  "meta": {
    "request_id": "req_01K4Z8V2DE5F7G9H1J3K6M8NPQ",
    "content_release": "knowledge-2026-09"
  }
}
```

## GET /v1/graph

读取以一个词义、概念节点或领域为根的有界规范子图。`depth` 受配置的浅层最大值限制。结果保持有根、有类型且经过范围过滤；本端点不是通用图查询语言或无限制邻居倾倒接口。

请求参数：

```json
{
  "query": {
    "root_kind": "sense",
    "root_id": "sense_sweltering_hot_01",
    "depth": 1,
    "relation_types": ["lower_degree", "higher_degree", "collocation"],
    "verification_state": "verified",
    "node_limit": 20,
    "edge_limit": 30,
    "release": "knowledge-2026-09"
  }
}
```

响应 `200`：

```json
{
  "data": {
    "root": {"kind": "sense", "id": "sense_sweltering_hot_01", "node_id": "node_sweltering_hot_01"},
    "nodes": [
      {"node_id": "node_sweltering_hot_01", "node_type": "lexical_sense", "label": "sweltering"},
      {"node_id": "node_scorching_heat_01", "node_type": "lexical_sense", "label": "scorching"}
    ],
    "edges": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree",
        "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
        "restrictions": {"dimension": "temperature_intensity"},
        "evidence_state": "verified",
        "confidence": 0.96,
        "provenance": ["evidence_dictionary_1042"],
        "verification_state": "verified"
      }
    ],
    "truncated": false
  },
  "meta": {
    "request_id": "req_01K4Z8W9RS2T4V6X0Y1Z3A5BCD",
    "content_release": "knowledge-2026-09"
  }
}
```

## GET /v1/graph/nodes/{kind}/{id}/neighbors

分页读取一个规范节点的直接入边和出边。cursor 绑定根、过滤条件和发布，且不得包含请求文本。

请求参数：

```json
{
  "path": {"kind": "knowledge_node", "id": "node_sweltering_hot_01"},
  "query": {
    "direction": "both",
    "relation_types": ["lower_degree", "higher_degree"],
    "verification_state": "verified",
    "limit": 10,
    "cursor": null,
    "release": "knowledge-2026-09"
  }
}
```

响应 `200`：

```json
{
  "data": {
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_hot_sweltering_01",
          "source_node_id": "node_hot_temperature_01",
          "target_node_id": "node_sweltering_hot_01",
          "relation_type": "higher_degree",
          "explanation": "Sweltering expresses a more uncomfortable degree of heat than hot.",
          "restrictions": {"dimension": "temperature_intensity"},
          "evidence_state": "verified",
          "confidence": 0.96,
          "provenance": ["evidence_dictionary_1042"],
          "verification_state": "verified"
        },
        "node": {"node_id": "node_hot_temperature_01", "node_type": "lexical_sense", "label": "hot"}
      }
    ],
    "next_cursor": null
  },
  "meta": {
    "request_id": "req_01K4Z8X6FG1H3J5K7M9N2P4QRS",
    "content_release": "knowledge-2026-09"
  }
}
```

## 相关文档

- [系统设计](../transnet_cn.md)
- [MySQL 接口](mysql_cn.md)
- [Qdrant 接口](qdrant_cn.md)
- [当前 OpenAPI 快照](../../docs/reference/transnet-openapi.json)
