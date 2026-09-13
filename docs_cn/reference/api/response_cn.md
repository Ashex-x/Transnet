# API 响应模型

English: [API response models](../../../docs/reference/api/response.md)

`src/api/response.rs` 是成功元数据及 `TranslationResult`、含义专属详情、规范词义读取与图读取 Wire 序列化的目标所有者。

状态：目标模块设计。当前旧版响应类型与现有规范基础并不构成共享目标响应合同或默认生产组合。

## 合同

成功应用响应使用 `data` 与 `meta`；`meta.request_id` 与 `X-Request-Id` 一致，规范读取标识所使用的不可变内容发布。翻译成功在 `data.translation` 放置一个结果，并使用有序 `translations` 数组区分实质上合理的含义。

`brief`、`standard` 与 `full` 序列化同一已验证 Superset 结果的确定性投影。投影可以移除支持字段及过量低价值项目，但不得改变事实身份、关系方向、证据状态、发布身份，也不能在会误导时隐藏实质上合理的含义。

规范标记仅用于命名发布中的已审核内容。推断与探索性材料保持请求级并显式标注。响应模型绝不序列化请求历史、Provider 详情、存储内部信息、凭据或用户所属状态。

## 验证

Golden JSON 测试应覆盖各响应级别、多重含义、规范与降级读取、图边界、发布元数据、省略可选字段及未知 Enum 防护。规范形状见[Transnet 接口](../../interfaces/transnet_cn.md)。

