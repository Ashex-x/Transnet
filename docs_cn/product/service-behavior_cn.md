# Transnet 服务行为

English: [Transnet service behavior](../../docs/product/service-behavior.md)

本指南描述无状态目标服务的 consumer-visible 行为。产品 application 负责用户、文件接入、保存、文档状态与展示。当前运行时覆盖窄于该目标，并由[服务接口](../interfaces/transnet_cn.md)明确标识。

## 翻译

`POST /api/v1/translations` 是唯一新 turn 入口。简单路径只要求文本、源语言、目标语言及 `brief`、`standard` 或 `full`。专业调用方也可以提供有序文档/localization segment 或有界 image region，并附带可选 purpose、audience、register、术语、alternative、annotation 与 freshness guidance。每个高级值只属于请求，不创建画像或翻译记忆。

Transnet 自动选择词汇、连续文本、结构化 segment 或视觉 region 处理。规范精确结果可能完全不需要生成。普通工作使用唯一 Gemma4-27B VLM 的 fast profile；长内容在同一模型上使用有界 chunk。Reasoning 只可按闭合歧义、约束、已验证路径或无效结构策略升级一次。

文本结果返回按含义区分的有序译文。Segment 与图片结果保留请求 ID、顺序、protected content 与格式约束。有类型 annotation 只解释实质歧义、术语、语域、文化、格式风险或 review 需求。Response level 改变支持详情广度，但不改变所选译文、证据状态、约束或 review outcome。

## 视觉与文档

Island-port 上传并校验文件、渲染 PDF、选择页面并重建输出文档。Transnet 只接收有界 inline PNG、JPEG 或 WebP 及归一化 region，或已提取的结构化 segment。它绝不下载调用方 URL，也不存储文件 byte、OCR-like 输出或 layout state。

Protected range 必须原样 round-trip。互相矛盾的 required 术语、protected content 或格式规则显式失败，而非静默忽略。Island-port 把超大文档分成有界同步请求；请求级术语 guidance 提供连续性，而不创建持久任务。

## 当前信息

互联网检索默认禁用。`allowed` 与 `required` freshness 是显式 opt-in，因为派生 query 可能向外部搜索 provider 泄露请求材料。检索是一次有界 search/fetch 操作，具有严格公开网络、redirect、media type、byte 与 deadline 控制。

实时页面是不可信数据，不能改变指令或成为规范事实。当前 claim 引用响应级 `live_external` source，并在请求后丢弃。实时结果绝不自动进入发布。

## 词汇与概念详情

单词、固定短语与专业术语解析为规范 sense 或 concept。同形词、词性、短语级含义与领域特定 sense 保持分离。初始结果公布相关知识 lens，而不是暴露 graph 控制。

`knowledge/views` 为 meaning、contrast、usage、form、origin、domain、mechanism 或 application 展示引导式 root-specific tree 投影。`knowledge/paths` 只返回规范 root 间短且独立验证的路径。调用方不选择原始关系 filter、depth、node limit、vector selector 或任意 traversal。

规范来源是 assertion graph，而不是严格树。稳定节点可以出现在多个 lens 下，但每个 item 保留单一身份、到 root 的显式路径、relevance reason、适用条件、证据、来源和发布。Verified、inferred、exploratory 与 live-external 材料保持显式分离。

## 内容与降级

规范数据读取对事实、证据、翻译与发布具有权威性。检索数据读取通过 embedding 投影提名节点和关系。相似度绝不建立翻译、同义、分类、因果、机制、文化含义或真值。

检索不可用时可以显式降级为规范基础卡。权威内容缺失、发布不兼容或必需实时检索失败绝不伪装为空结果。实时请求不写入 alias、card、fact、domain、assertion、vector 或 release。

## 相关文档

- [系统设计](../transnet_cn.md)
- [Transnet 服务接口](../interfaces/transnet_cn.md)
- [规范数据接口](../interfaces/canonical-data_cn.md)
- [检索数据接口](../interfaces/retrieval-data_cn.md)
- [模型运行时](../reference/model-runtime_cn.md)
