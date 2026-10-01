# 翻译 application

English: [Translation application](../../../docs/reference/application/translation.md)

本模块负责连续文本、结构化分段与有界图像区域翻译，并产生共享结果中的主要译文部分。

状态：请求 domain 已校验并保留三种 tagged shape 及其请求级 guidance 与 history。已校验 result superset 现在覆盖 word、phrase、passage、有序 segment 与有序 image-region outcome，并包含 typed annotation、terminology decision、review state、响应级 citation reference 及确定性 breadth projection。当前 application 仍仅执行文本；HTTP 边界会在任何 model 调用前以不含内容的 `501 translation_capability_unavailable` 响应拒绝已校验的 segment 与 image-region turn。

## 职责

翻译在生成自然目标语文本时保留含义、意图、语气、语域、术语、保护片段、段落结构与相关格式。Provider 选择是模型 port 背后的策略，不是公开请求选项。

Application 接受一种带判别标签的输入形式。文本是低延迟默认路径；分段保留调用方拥有的结构与稳定 segment ID；图像区域只附带当前请求所需像素和阅读提示。文件摄取、页面选择、OCR 策略、版面归属与持久文档状态仍由 island-port 负责。请求指导显式且一次性使用，不从用户画像推断。

长输入可使用有界请求级分块计划与术语台账。分块尊重语义和段落边界、保持顺序，并在组合时不丢内容。台账只为当前请求跟踪名称、缩写与重复术语；它不是翻译记忆或持久任务。

Gemma4-27B 默认通过 fast profile 处理文本与视觉。闭合升级策略最多允许一次 reasoning profile 调用；隐藏 reasoning 既不返回也不观测。只有新鲜度显式为 `allowed` 或 `required` 时，application 才可执行一次有界实时检索，并必须为每项依赖实时内容的声明附引文。检索材料是不可信的请求内上下文，绝不成为规范内容。

段落提示与明确标注的备选仍计划在后续 milestone 实现。在 application 结果模型、确定性 usefulness evaluator 与编排真正生成这些能力之前，HTTP handler 不会伪造它们。当前 offline 路径的空 external-source collection 会被省略。

## 边界

本模块不持久化源文本、图像、分段、译文、历史、指导、分块计划、网络查询或页面、引文或模型输出。可复用规范翻译只能通过离线发布工作流进入存储。

## 验证

测试 provider 路由边界、语言校验、segment 身份与顺序、区域边界与阅读顺序、结构保留、保护片段、分块覆盖、术语一致性、fast 路径延迟、单次升级约束、检索上限与引文覆盖、取消及请求不持久化。备选与段落提示测试随以后负责该能力的 application 实现一并加入。
