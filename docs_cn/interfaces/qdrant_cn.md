# Qdrant 适配器接口

English: [Qdrant adapter interface](../../docs/interfaces/qdrant.md)

Qdrant 是可重建的派生索引；MySQL 保存规范事实和活动的 `(release, collection, schema, ranker)` 版本组。集合按发布版本和嵌入配置不可变创建。

## 集合与点

```json
{
  "operation": "collection.create",
  "request_id": "01JREQUEST",
  "input": {
    "collection": "transnet_sense_01JRELEASE_e5_v3",
    "release_id": "01JRELEASE",
    "vector": {"size": 1024, "distance": "Cosine", "model": "embed-v5"},
    "payload_indexes": ["release_id", "entity_kind", "status"]
  }
}
```

## 检索

```json
{
  "operation": "vector.search",
  "request_id": "01JREQUEST",
  "content_version": {"release_id": "01JRELEASE", "collection": "transnet_sense_01JRELEASE_e5_v3", "ranking_version": "lookup-v1"},
  "input": {"purpose": "canonical_definition", "content_language": "es", "query_vector": [0.125, -0.25, 0.5], "limit": 30}
}
```

检索必须绑定物理集合，先应用版本、状态和来源过滤，再执行 1–100 的限制。分数只在同一模型和集合版本内可比较；候选 ID 必须回到 MySQL 做权限和证据校验。

## 校准与生命周期

构建流程是新建集合、幂等写入点、校验身份与内容哈希、在 MySQL 记录就绪，再发布活动版本。来源隔离先在 MySQL 生效，然后删除匹配点并再次校验。集合删除必须确认没有活动版本或保留回滚版本引用。
