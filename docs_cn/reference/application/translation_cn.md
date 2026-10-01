# 翻译 application

English: [Translation application](../../../docs/reference/application/translation.md)

本模块负责连续文本、结构化分段与有界图像区域翻译，并产生共享结果中的主要译文部分。

状态：请求 domain 已校验并保留三种 tagged shape 及其请求级 guidance 与 history。已校验 result superset 覆盖 word、phrase、passage、有序 segment 与有序 image-region outcome，并包含 typed annotation、terminology decision、review state、响应级 citation reference 及确定性 breadth projection。Application 执行带 guidance 的 text、结构化 segment 与有界 image-region turn；它确定性检查 required/forbidden terminology 与段落结构，并最多允许一次 reasoning-profile repair。Preferred terminology 是 prompt preference，而不是硬 postcondition。Image execution 会拒绝自身无法执行的 execution-dependent guidance，在 blocking pool 上执行有界 decode/crop/encode 并在各 unit 之间检查 deadline 与 cancellation，只为每个声明 region 提交一个有界 crop，执行一次有界 fast-profile 调用，并把 attachment index、精确 reading-order ID 与检测语言严格映射到 image-region result。

## 职责

翻译在生成自然目标语文本时保留含义、意图、语气、语域、术语、保护片段、段落结构与相关格式。Provider 选择是模型 port 背后的策略，不是公开请求选项。

Application 接受一种带判别标签的输入形式。文本是低延迟默认路径；分段保留调用方拥有的结构与稳定 segment ID；图像区域只附带当前请求所需像素和阅读提示。文件摄取、页面选择、OCR 策略、版面归属与持久文档状态仍由 island-port 负责。请求指导显式且一次性使用，不从用户画像推断。

长输入可使用有界请求级分块计划与术语台账。分块尊重语义和段落边界、保持顺序，并在组合时不丢内容。台账只为当前请求跟踪名称、缩写与重复术语；它不是翻译记忆或持久任务。

结构化 segment 使用有界并行 fast call，同时在组装结果中保持确定性请求顺序。每个 prompt 绑定 segment role、format、source/target language、protected scalar range、请求 guidance 与 prompt contract；调用方 ID 不会发送给 model。Required 与 forbidden terminology 使用适合文字体系的匹配进行检查，包括嵌在未分词 CJK 文本中的术语；结果记录完整且有序的 terminology decision 列表。整个请求共享一次 reasoning-repair budget。确定性 postcondition 会拒绝缺失、重排或重复的 protected value，以及改变的段落换行、Markdown 结构 delimiter 或 HTML tag。成功结果在返回前针对请求完成校验，并使用调用方 segment ID、零起始 segment order、`translation_0` 及可选闭合 format annotation；完成后不保留任何 segment 内容。

Gemma4-27B 默认通过 fast profile 处理文本。闭合升级策略对 invalid、ambiguous、违反 guidance 或 citation 无效的输出最多允许一次 reasoning profile 调用；隐藏 reasoning 既不返回也不观测。`offline` 不发起 search 或 fetch。`allowed` 仅在确定性判断输入对时效敏感且 canonical material 不足时消耗唯一一轮；若被许可的轮次不可用，响应保留翻译并以 `review_recommended` 与 `live_source_incomplete` 明确标记，而不会静默声称时效性。`required` 始终尝试检索；没有安全可用 source 时在生成前显式失败。

实时材料只作为结构化 `live_material` 数组进入 generation；固定 application instruction 将其中 fragment 标记为不可信数据。材料参与时，模型输出必须引用一个或多个已准入的 `live_N` identifier；材料未参与时不得引用。伪造、重复或缺失的 live citation 都不满足有界输出契约。Application 仅暴露已引用的 title/URL descriptor，附加 response-local citation reference，把 `translation-live-v1` 记录为 retrieval version，并随请求丢弃 query、抓取 fragment 与 vector。只有 search、安全 fetch 和本 translation orchestrator 作为一个 runtime unit 完整组合时，capability discovery 才报告 live retrieval；默认 executable 仍没有 production search authority，因此继续报告不可用。

段落提示与明确标注的备选仍计划在后续 milestone 实现。在 application 结果模型、确定性 usefulness evaluator 与编排真正生成这些能力之前，HTTP handler 不会伪造它们。当前 offline 路径的空 external-source collection 会被省略。

## 边界

本模块不持久化源文本、图像、分段、译文、历史、指导、分块计划、网络查询或页面、引文或模型输出。可复用规范翻译只能通过离线发布工作流进入存储。

## 验证

测试 provider 路由边界、语言校验、segment 身份与顺序、区域边界与阅读顺序、结构保留、保护片段、分块覆盖、术语一致性、fast 路径延迟、单次升级约束、检索上限与引文覆盖、取消及请求不持久化。备选与段落提示测试随以后负责该能力的 application 实现一并加入。
