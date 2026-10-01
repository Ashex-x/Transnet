# 目标向量 collection

English: [Target vector collections](../../../docs/interfaces/tables/vec.md)

本目录定义[向量数据 endpoint](../qdrant_cn.md)背后的目标 Qdrant collection。Qdrant 是可重建且绑定发布版本的投影；MySQL 规范修订和签名发布 artifact 仍是权威来源。

状态：目标 schema；当前 Transnet 可执行文件不会创建或激活这些 collection。

## Collection 集合

每个内容发布只创建两个不可变物理 collection：

- `knowledge_nodes__<release>`：规范节点和语义尺度投影。
- `knowledge_edges__<release>`：规范类型化关系投影。

稳定 alias `knowledge_nodes__active` 和 `knowledge_edges__active` 仅在核对完成后共同切换。Collection 元数据固定发布 ID、payload schema、稠密与稀疏模型版本、向量维数、point 数量、内容 hash 和构建时间。Point ID 在其族和发布内是确定性的。

不存在用户反馈、判断事件、判断聚合或距离 collection。这些可变值属于 island-port SQL 存储，并在规范检索后 join，从而避免投票进入 embedding，也避免修改活动知识 collection。

## knowledge_nodes payload

必需索引字段为 `release_id`、`publication_state`、`verification_state`、`node_type`、`node_id`、`sense_id`、`language`、`dialect`、`region`、`period`、`domain_ids` 和 `evidence_ids`。可选展示字段包括规范标签、别名、翻译、精简描述及绑定发布版本的领域知识 profile。

语义尺度 point 使用 `node_type: semantic_scale`，并额外包含尺度 ID、维度、方向、条件、有序词义限定成员、证据 ID 和事实 ID。成员位置只表达顺序。

每个 point 都有命名稠密 `semantic` 向量和命名稀疏 `lexical` 向量，两者只能由已发布规范内容生成。运行时查询向量是临时数据，绝不存储。

## knowledge_edges payload

必需索引字段为 `release_id`、`publication_state`、`verification_state`、`edge_id`、`relation_version`、`fact_id`、`fact_revision`、`source_node_id`、`target_node_id`、`relation_type`、`language`、`dialect`、`region`、`period`、`domain_ids`、`applicable_sense_ids` 和 `evidence_ids`。

Payload 还携带完整规范关系解释、方向、条件、限制、证据状态、来源引用、置信度及 `assessment_enabled`。`relation_version` 必须与 MySQL `release_member` 中的关系条目一致。`assessment_enabled` 只声明目标资格；Qdrant 不存储判断、计数、社区分数、距离调整、用户 ID 或聚合版本。

稠密向量嵌入完整的“源—关系—目标”解释。稀疏向量索引规范 endpoint 标签、关系术语和已审核别名。相似度只能提出候选，不能建立或否定关系。

## 检索与距离 join

Island-port 首先解析活动内容发布和不可变节点/边 alias。Qdrant 返回合格规范候选后，再按 `(content_release, edge_id, relation_version)` join 活动 `relationship_assessment_projection`。该优化表将匿名计数及其派生距离保存在同一条不可变聚合记录中，因为两者共享一个键和生命周期。投影缺失或低于门槛时调整为零。Transnet 随后按有效距离进行排序或布局，并在响应元数据和 cursor 中同时固定内容发布与聚合版本。

该 join 不能添加 Qdrant 未返回的边、绕过验证过滤、改变 endpoint 或关系类型，也不能替代来自 MySQL 的证据 hydration。聚合依赖不可用时，图读取使用基础距离，在公开合同要求时将响应标为 degraded，并且绝不复用其他关系版本或发布的投影。

## 发布与重建

先发布节点再发布边；根据暂存节点 manifest 校验每条边的 endpoint，并拒绝跨发布引用。核对检查 point 数量、endpoint 覆盖、schema 与模型版本、向量维数、hash、证据覆盖，以及与 MySQL 的关系版本一致性。两个 alias 必须与兼容 SQL 发布一起原子激活。

修正会构建新的物理 collection；回滚选择保留的不可变 collection 对。重新嵌入规范内容时，绝不把私有判断或距离数据复制到 Qdrant。

## 相关文档

- [向量 endpoint 合同](../qdrant_cn.md)
- [MySQL schema](../../../docs/interfaces/tables/sql.sql)
- [关系评估合同](../transnet_cn.md#关系评估元数据)
- [内容发布](../../guides/content-publishing_cn.md)
