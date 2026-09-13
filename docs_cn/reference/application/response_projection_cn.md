# 响应投影

English: [Response projection](../../../docs/reference/application/response_projection.md)

响应投影 application 模块把一个已校验超集翻译聚合确定性缩减为请求的 `brief`、`standard` 或 `full` 表示。

状态：目标设计。当前路由尚未共享目标超集或此确定性投影器。

## 投影策略

`brief` 保留单元、检测到的源语言、有序译文和避免歧义所需的含义标签。`standard` 添加选定的精简定义或短语用法、每个含义至多一个例句、至多两条篇章提示及仅有实质价值的关系。`full` 添加全部有界合格词汇、领域、证据、分类和强度详情。

投影使用版本化字段与项目 allowlist。空字段和分组会被省略。请求级领域提案仅在 `full` 级别合格。

## 不变量

除 envelope 元数据外，所有级别都是同一发布固定超集的子集。较低级别绝不改变含义顺序、规范 ID、译文、证据或验证状态、降级信号和选定事实。若省略会造成误导，则实质合理含义保持可见。

投影器不执行 provider 调用、检索、重新排序、持久化或线上解析。序列化仍由 API response 模块负责。

## 验证

Golden 与属性测试应从同一聚合比较三个投影，断言严格字段/项目子集行为，强制例句与提示限制，保留歧义并省略空分组。测试应固定投影策略标识符，使策略变化显式且可审核。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [系统设计](../../transnet_cn.md)
- [质量保证](../../guides/quality-assurance_cn.md)
