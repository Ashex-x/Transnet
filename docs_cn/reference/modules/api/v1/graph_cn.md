# 版本 1 图读取

English: [Version 1 graph reads](../../../../../docs/reference/modules/api/v1/graph.md)

`src/api/v1/graph.rs` 是以翻译返回的规范 ID 为根的有界图与邻居读取目标薄 Handler 所有者。

状态：目标路由。现有图服务、分页保护与测试是旧版路由后的基础；默认启动器未在 UDS 上组合 `POST /transnet/v1/graph/get` 或 `POST /transnet/v1/graph/neighbors`。

## Handler 边界

Handler 严格解码接口定义的根、发布、关系 Filter 与边界，保留请求 Deadline 与发布固定，调用图应用服务，并序列化节点、有类型有方向 Edge、证据标签及分页状态。它们不包含排序、拓扑、检索或持久化决策。

图读取保持准确方向：`is_a` 指向子词义 -> 父类别，`has_subtype` 是其逆向。语义尺度顺序独立于分类。向量相似度可提出候选，但不能创建返回的规范事实。

路由不接受学习者、用户、保存视图、布局或实体标签所有权输入。Cursor 不透明且受完整性保护，不得泄露公开结果之外的内部 Filter 或标识符。

## 验证

合同测试应覆盖节点与 Edge 边界、关系方向、发布固定、不透明分页、未知字段、未找到、依赖降级及排除私有状态。准确 Schema 见[Transnet 接口](../../../../interfaces/transnet_cn.md)。

