# 模型运行时

English: [Model runtime](../../docs/reference/model-runtime.md)

本子系统负责单个已配置 Gemma4-27B 视觉语言模型与独立 embedding 模型的目标推理策略。它不通过服务接口暴露 provider 品牌、reasoning 控制、prompt 或 embedding payload。

状态：目标设计。当前可执行文件仍把短输入路由到 Gemma 4、把长输入路由到 TranslateGemma。该过渡拆分会一直保持已实现状态，直到后续运行时变更替换它；目标文档不得把 TranslateGemma 描述为必需部署依赖。

## 生成 profile

生成 port 暴露由同一 Gemma4-27B endpoint 与模型身份支持的 `fast` 和 `reasoning` 调用 profile。Fast 是翻译、结构化提取、视觉读取、分类和 grounded composition 的默认选择。Reasoning 是内部有界升级，而不是调用方选择的模型，也不是独立 provider。

规范精确命中不需要生成调用。普通文本、segment 与 image-region 翻译使用 fast 推理。输入长度绝不选择另一个模型：翻译 application 把已接受长输入拆成有界语义 chunk，以有界并发执行互相独立的 chunk，携带请求级术语台账，并确定性重组。

一个请求最多升级 reasoning 一次，且仅限确定性工作识别出无法消除的重要词义歧义、冲突的术语或格式约束、fast profile 无法安全表述的已验证多跳解释，或一次无效 fast 结构化结果。可选 reasoning 失败时，有效 fast 结果可带缩减详情 metadata 继续返回。若正确性依赖 reasoning 结果，操作应返回澄清或闭合依赖失败，而不是猜测。

Prompt 与隐藏 reasoning 都不是响应数据。服务可以返回精简结论、安全 decision code 和证据引用，但绝不返回 chain-of-thought、草稿、reasoning token 或 provider-native reasoning 字段。

## Embedding 操作

Embedding port 有两类有界用途。离线发布把已审核规范节点与关系说明嵌入不可变发布投影。在线请求可以在内存中嵌入 query 或经明确许可的实时检索片段，以提名候选。在线向量随请求丢弃，绝不进入日志、trace、指标、cache、规范存储或后续发布。

向量相似度仅是排序信号。它不能建立翻译等价、同义、分类、因果、机制、文化含义、证据或真值。

## Deadline 与调用预算

每个模型与 embedding 操作都消耗调用方 deadline。配置为 fast、reasoning、embedding 与可选实时检索提供独立子 deadline，且均受剩余请求时间限制。依赖允许时，互相独立的规范读取与向量读取并发执行。

默认调用预算是：足够的规范命中使用零次生成调用，普通输入使用一次 fast 逻辑操作，长结构化输入使用有界并行 fast chunk 操作，整个请求最多一次 reasoning 升级。Application 不为每个响应分区或关系分别调用模型。

重试仅适用于 transport-level 可重试 attempt，在已有有效结果后不得产生第二次语义生成。可选 enrichment 超过其子 deadline 时显式降级，且不延长调用方 deadline。必需生成或 embedding 失败使用闭合结果。

## 视觉边界

VLM 只接受来自 Transnet 请求合同、经过校验的 inline PNG、JPEG 或 WebP 数据及有界 image region。Island-port 负责文件上传、PDF 渲染、解压、恶意内容检查、页面选择和文档重建。Transnet 绝不抓取调用方提供的图片 URL，也不持久化图片 byte 或视觉中间输出。

## 验证

测试零调用规范答案、fast 路径选择、闭合升级触发条件、单次升级限制、长输入 chunk 覆盖与顺序、术语一致性、图片边界、结构化输出修复、deadline 记账、取消、并发、脱敏，以及文本、图片、reasoning 输出和在线向量的丢弃。
