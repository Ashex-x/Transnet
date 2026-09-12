# MySQL 适配器接口

English: [MySQL adapter interface](../../docs/interfaces/mysql.md)

本合同定义 MySQL 8 对共享规范单词、短语、词义、领域、证据元数据和不可变内容发布的逻辑操作。JSON 示例表示有类型的适配器值，不是网络协议或存储 JSON 列。

状态：目标合同；当前可执行文件尚未组合此适配器。

## 存储边界

MySQL 是精简结构化词汇内容和发布状态的权威存储。它不包含用户、学习者、账户、画像、偏好、历史、保存项目、书签、练习、答案、掌握度、日程、图布局、反馈、隐私请求或所有权记录；也不保留原始翻译文本、查询文本或消歧上下文。

允许的 Transnet 服务数据：

- 不可变知识发布和兼容性 manifest；
- 规范单词和短语、带语言标签的词形、别名和词义；
- 精简定义、翻译、发音、形态、例句和用法说明；
- 规范领域及其范围定义；
- 指向 Qdrant 知识根和证据记录的稳定引用；
- 发布任务、校验结果、幂等记录和 Qdrant 投影 outbox，且均不含请求文本或用户数据。

使用 `utf8mb4`、UTC 微秒时间、不透明稳定公开 ID、显式外键和不可变已发布修订。凭据和加密密钥置于 MySQL 之外。

## 通用操作 envelope

每个适配器操作携带请求 ID、deadline、预期 schema 版本，并可携带不可变内容发布。发布变更还要求幂等键。读操作返回 `ok`、`not_found`、`version_mismatch`、`unavailable` 或 `timeout`；变更还可返回 `conflict` 或 `invalid`。错误不得暴露 SQL、凭据、请求文本、provider body 或连接内部信息。

请求上下文：

```json
{
  "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
  "deadline_at": "2026-09-12T10:30:05.000000Z",
  "schema_version": "mysql-adapter-v1",
  "content_release": "knowledge-2026-09"
}
```

关闭错误响应：

```json
{
  "outcome": "version_mismatch",
  "error": {
    "code": "content_release_mismatch",
    "message": "The requested content release is not available.",
    "retryable": false
  }
}
```

## resolve_basic_cards

输入规范化属于 Transnet 运行时。适配器只接收有界、排序后的派生形式，绝不接收原始查询、中间变换或上下文。精确规范形式和别名优先于屈折、拼写修正和宽松别名；`C`、`C++`、`C#` 等有意义符号不合并。

请求：

```json
{
  "lookup_forms": [{"form": "sweltering", "match_class": "exact_canonical", "rank": 0}],
  "normalizer_version": "unicode-nfkc-v2",
  "source_language": "en",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "content_release": "knowledge-2026-09",
  "limit": 5
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "matches": [
      {
        "matched_form": "sweltering",
        "match_class": "exact_canonical",
        "card": {
          "card_id": "card_sweltering_en_adj_01",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_form": "sweltering",
          "language": "en",
          "part_of_speech": "adjective",
          "translations": [{"language": "zh-CN", "text": "酷热的"}],
          "definitions": ["uncomfortably hot, especially because of the weather"],
          "knowledge_root_ids": ["node_sweltering_hot_01"],
          "cefr": "B2",
          "domain_ids": ["domain_weather"],
          "revision": 3
        }
      }
    ],
    "alternatives": []
  },
  "content_release": "knowledge-2026-09"
}
```

唯一性由稳定的词形、卡片和词义 ID 及已发布规范形式/别名行维护，不依赖临时规范化检索字符串。最佳适用层级的所有合格冲突均须返回，由服务解析。

## get_sense

返回一个精简规范词义修订；详细关系留在 Qdrant。

请求：

```json
{
  "sense_id": "sense_sweltering_hot_01",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "content_release": "knowledge-2026-09"
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [{"language": "zh-CN", "text": "酷热的"}],
    "pronunciations": [{"dialect": "en-US", "ipa": "/ˈswɛltərɪŋ/"}],
    "forms": [{"form": "swelteringly", "label": "adverb"}],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"],
    "revision": 3
  },
  "content_release": "knowledge-2026-09"
}
```

## resolve_domain

领域是规范版本化记录，不是自由标签。先匹配已发布名称和别名；若多个范围均匹配，适配器返回候选，由发布流程消歧。

请求：

```json
{
  "normalized_labels": ["meteorology", "weather"],
  "scope_key": "earth-atmosphere-weather",
  "content_release": "knowledge-2026-09",
  "limit": 5
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "resolution": "matched",
    "domain": {
      "domain_id": "domain_weather",
      "canonical_label": "weather",
      "definition": "Conditions of the atmosphere at a place and time.",
      "inclusion_scope": ["temperature", "precipitation", "wind", "humidity"],
      "exclusion_scope": ["long-term climate classification"],
      "broader_domain_ids": ["domain_earth_science"],
      "revision": 4
    }
  },
  "content_release": "knowledge-2026-09"
}
```

## create_domain_draft

仅发布流程可调用：当精确 MySQL 解析及有界 Qdrant 检索均找不到合适范围时创建领域草稿。运行时查询流量不得创建领域。

请求：

```json
{
  "domain_id": "domain_urban_climatology",
  "canonical_label": "urban climatology",
  "normalized_label": "urban climatology",
  "scope_key": "urban-atmosphere-climate",
  "definition": "Study of atmospheric conditions and climate processes in urban areas.",
  "inclusion_scope": ["urban heat island", "street-canyon airflow"],
  "exclusion_scope": ["general urban planning"],
  "aliases": ["urban climate science"],
  "broader_domain_ids": ["domain_climatology"],
  "related_candidate_ids": ["domain_urban_planning"],
  "generation": {
    "model_version": "domain-curator-2026-09",
    "prompt_version": "domain-draft-v3",
    "confidence": 0.94
  },
  "idempotency_key": "publish-domain-urban-climatology-v1"
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "domain_id": "domain_urban_climatology",
    "revision": 1,
    "publication_state": "draft",
    "outbox_event_id": "outbox_domain_urban_climatology_01"
  }
}
```

规范化名称和范围键上的唯一约束与事务 upsert 防止并发重复。模型生成的相关领域链接在有证据发布验证前均为探索性关系。

## stage_card_revision

暂存不可变的单词或短语修订及其 Qdrant 根引用。暂存校验所有结构化字段，但不会让内容从活动发布中读取。

请求：

```json
{
  "card": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [{"language": "zh-CN", "text": "酷热的"}],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"]
  },
  "target_release": "knowledge-2026-10",
  "source_revision": 3,
  "evidence_ids": ["evidence_dictionary_1042"],
  "idempotency_key": "stage-card-sweltering-r4"
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "revision": 4,
    "publication_state": "staged",
    "target_release": "knowledge-2026-10"
  }
}
```

## activate_release

激活是原子的，必须引用兼容的不可变 Qdrant 节点/边发布。若任一卡片根、领域、证据记录、内容哈希或 Qdrant manifest 缺失或不兼容，激活失败。

请求：

```json
{
  "release_id": "knowledge-2026-10",
  "expected_active_release": "knowledge-2026-09",
  "mysql_content_hash": "sha256:9c49d7f6...",
  "qdrant_manifest_hash": "sha256:2e17a054...",
  "idempotency_key": "activate-knowledge-2026-10"
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "active_release": "knowledge-2026-10",
    "previous_release": "knowledge-2026-09",
    "activated_at": "2026-10-01T00:00:00.000000Z"
  }
}
```

隔离、撤回和修正会创建新的发布状态或发布，绝不静默重写已发布行。

## 相关文档

- [Transnet 服务接口](port_cn.md)
- [Qdrant 接口](qdrant_cn.md)
- [内容发布](../guides/content-publishing_cn.md)
