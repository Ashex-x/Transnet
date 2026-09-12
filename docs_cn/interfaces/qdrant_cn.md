# Qdrant 适配器接口

English: [Qdrant adapter interface](../../docs/interfaces/qdrant.md)

本合同定义存储于版本化 Qdrant 节点和边集合中的共享规范关系图。JSON 示例表示适配器值和 point payload，不是公开网络 API。

状态：目标合同；当前可执行文件尚未组合此适配器。

## 存储边界

Qdrant 存储规范 Transnet 概念之间的关系。它是可重建的只读投影，MySQL 与经认证的发布工件仍是权威来源。

Qdrant 不包含用户、学习者、账户、画像、偏好、查询、上下文、源段落、历史、保存项目、书签、练习、答案、掌握度、日程、布局、反馈、录音或隐私流程数据。向量只能由已发布规范内容及关系解释生成；运行时请求文本绝不嵌入或存储。

## 发布与集合合同

每个逻辑知识发布包含一个不可变 `knowledge_nodes` 集合和一个不可变 `knowledge_edges` 集合。两者共同钉住发布 ID、嵌入模型、向量维度、稀疏配置、payload schema 与内容哈希，并作为一个单元激活和回滚。

Point ID 必须确定。先构建节点再构建边。发布拒绝缺失端点、跨发布引用、无效方向、重复有类型边、缺失证据、不兼容词义、无支持的语言或领域主张，以及不匹配的嵌入元数据。

发布 manifest 示例：

```json
{
  "release_id": "knowledge-2026-09",
  "collections": {
    "nodes": "knowledge_nodes__knowledge_2026_09",
    "edges": "knowledge_edges__knowledge_2026_09"
  },
  "dense_model": "multilingual-embedding-v4",
  "dense_dimensions": 1536,
  "sparse_model": "lexical-sparse-v2",
  "payload_schema_version": "knowledge-graph-v1",
  "node_content_hash": "sha256:63af5c1e...",
  "edge_content_hash": "sha256:b19d28a7..."
}
```

## 知识节点 point

节点表示一个可独立解释的词义、短语、多语言术语、概念、实体、现象、机理、过程、方程、物理量、材料、仪器、方法、技术、应用、标准、组织、人物、地点、习语、隐喻、语法模式、搭配、误解或规范领域。

```json
{
  "id": "node_sweltering_hot_01",
  "vectors": {
    "semantic": "<1536-dimensional canonical-content vector>",
    "lexical": {"indices": [1842, 99104], "values": [1.0, 0.62]}
  },
  "payload": {
    "node_id": "node_sweltering_hot_01",
    "node_type": "lexical_sense",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_label": "sweltering",
    "aliases": ["oppressively hot"],
    "translations": [{"language": "zh-CN", "text": "酷热的"}],
    "description": "uncomfortably hot, especially because of the weather",
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "confidence": 0.98,
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

Payload 索引覆盖发布、发布状态、验证状态、节点类型、词义 ID、语言、方言、地区、时期、领域 ID 和证据 ID。

## 知识边 point

边既是有类型连接，也是可检索的关系原因说明。

```json
{
  "id": "edge_sweltering_scorching_01",
  "vectors": {
    "semantic": "<1536-dimensional canonical-relationship vector>",
    "lexical": {"indices": [1842, 77103, 99104], "values": [0.71, 1.0, 0.48]}
  },
  "payload": {
    "edge_id": "edge_sweltering_scorching_01",
    "source_node_id": "node_sweltering_hot_01",
    "target_node_id": "node_scorching_heat_01",
    "relation_type": "higher_degree",
    "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "conditions": ["temperature describes weather or an environment"],
    "restrictions": {"dimension": "temperature_intensity", "register": "general"},
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "evidence_state": "supported",
    "provenance": ["source_dictionary_2026_01"],
    "confidence": 0.96,
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

关系族覆盖词汇命名与翻译等价、分类与整体—部分、同义/反义/对比/明确命名的强度、配价/语法/搭配/固定表达、形态、语域/方言/地区/时期/场景/领域适用性、文化延伸，以及领域机理、因果、依赖、实现、应用、测量、标准化和术语。探索关系保持独立。版本化关系类型注册表定义方向、逆关系、对称性、传递性和因果性；UI 与 LLM 不从措辞猜测。Payload 索引覆盖两端、关系类型、发布与验证状态、发布版本、适用词义、语言、方言、地区、时期、领域和证据 ID。

## search_nodes

结合命名稠密与稀疏检索、精确规范标签、别名、翻译、转写、缩写、公式和领域术语。服务临时创建查询向量，Qdrant 不接收租户或所有者标识。

请求：

```json
{
  "dense_vector": "<1536-dimensional ephemeral query vector>",
  "sparse_vector": {"indices": [1842, 99104], "values": [1.0, 0.55]},
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "verification_states": ["verified"],
    "node_types": ["lexical_sense", "phrase"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "node_id": "node_sweltering_hot_01",
        "score": 0.93,
        "matched_by": ["dense", "sparse", "canonical_label"],
        "payload": {
          "node_type": "lexical_sense",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_label": "sweltering",
          "verification_state": "verified"
        }
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

分数只可在相同模型和发布内比较。向量相似度仅是候选信号，不能证明翻译、同义、层级、因果、共同机制或文化意义。

## search_edges

检索规范关系解释。先应用资格过滤条件再限制数量，已验证和探索性结果必须分开。

请求：

```json
{
  "dense_vector": "<1536-dimensional ephemeral relationship vector>",
  "sparse_vector": {"indices": [77103, 99104], "values": [1.0, 0.6]},
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "relation_types": ["higher_degree", "lower_degree"],
    "verification_states": ["verified"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "score": 0.91,
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree",
        "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
        "verification_state": "verified"
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

## neighbors

通过端点索引读取直接入边和出边，再按 ID 获取另一端节点。它不推断本体语义、不合成边，也不执行事实性多跳遍历。

请求：

```json
{
  "node_id": "node_sweltering_hot_01",
  "direction": "both",
  "relation_types": ["higher_degree", "lower_degree", "collocation"],
  "verification_states": ["verified"],
  "languages": ["en"],
  "domain_ids": ["domain_weather"],
  "release_id": "knowledge-2026-09",
  "limit": 20,
  "cursor": null
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_sweltering_scorching_01",
          "source_node_id": "node_sweltering_hot_01",
          "target_node_id": "node_scorching_heat_01",
          "relation_type": "higher_degree",
          "verification_state": "verified"
        },
        "node": {
          "node_id": "node_scorching_heat_01",
          "node_type": "lexical_sense",
          "canonical_label": "scorching"
        }
      }
    ],
    "next_cursor": null
  },
  "release_id": "knowledge-2026-09"
}
```

扩展始终限制为一次跟随一个选定根，并只返回对该根与请求范围合格的关系。只有每一步都是具名且独立证据合格的边时，服务才可组织短路径。任意深度遍历、基于相似链的路径断言、中心性和可变图事务不属于本合同。

## publish_release_projection

发布向新的不可变集合写入确定性 point，并在激活前校验；不得改写活动集合。

请求：

```json
{
  "manifest": {
    "release_id": "knowledge-2026-10",
    "payload_schema_version": "knowledge-graph-v1",
    "dense_model": "multilingual-embedding-v4",
    "dense_dimensions": 1536,
    "sparse_model": "lexical-sparse-v2",
    "expected_node_count": 184220,
    "expected_edge_count": 612840,
    "node_content_hash": "sha256:dd401f2a...",
    "edge_content_hash": "sha256:98d3a647..."
  },
  "idempotency_key": "publish-qdrant-knowledge-2026-10"
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "release_id": "knowledge-2026-10",
    "node_collection": "knowledge_nodes__knowledge_2026_10",
    "edge_collection": "knowledge_edges__knowledge_2026_10",
    "node_count": 184220,
    "edge_count": 612840,
    "endpoint_coverage": 1.0,
    "validation_state": "ready_for_activation"
  }
}
```

构建对账将数量、端点覆盖、内容哈希、嵌入版本和发布元数据与经认证 manifest 比较。不完整或不匹配的集合对绝不激活。修正创建新不可变发布；回滚选择未变更的保留集合对。

关闭结果为 `ok`、`missing`、`invalid_payload`、`version_mismatch`、`unavailable` 和 `timeout`。日志不得包含凭据、向量、规范源文本、请求内容或原始 Qdrant body。

## 相关文档

- [Transnet 服务接口](port_cn.md)
- [MySQL 接口](mysql_cn.md)
- [内容发布](../guides/content-publishing_cn.md)
- [质量保证](../guides/quality-assurance_cn.md)
