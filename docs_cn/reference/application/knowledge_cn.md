# 知识 application

English: [Knowledge application](../../../docs/reference/application/knowledge.md)

本模块负责单词与固定短语请求的词义解析、领域评估、发布固定检索、关系排序、页面组织与确定性投影。

状态：当前规范查询使用只读检索端口，在请求内组装每张卡片。查询衍生的快照缓存及其公共缓存合同已经移除。图拓扑缓存仍属于发布固定的规范内容缓存。下文的领域评估与完整页面组织属于目标行为，尚未接入默认运行时。

canonical-only 服务从权威端选择一次发布 pin，再通过读取 port 组合已审核翻译候选、确定性排序的词汇候选和无歧义的词义详情。它使用带 canonical-only content pin 的现有 lookup-card 类型，不伪造向量集合，也不把有意的纯词法读取误称为向量故障降级。可执行文件只在显式配置时构造并保存该依赖，用 active-release 只读探针检查就绪，并通过冻结的 BasicCard lookup 与固定发布 sense 路由提供该能力。外部 island-port server 仍需实现匹配的内部合同。请求局部查询形式有界且去重：基线规范化形式最强，谨慎的空白或外围标点变体只是较低优先级的拼写候选。已发布别名、形态、转写和语义归属由权威端确定，不从查询字符串猜测。

公开的固定发布 sense follow-up 使用调用方的 `CanonicalReleasePin` 与 sense ID，不再重新选择 active 内容。现有候选/证据类型携带的 source attribution 来自权威发布的权利审核 metadata，绝不由 source ID 推导。HTTP 边界通过 `POST /api/v1/senses/get` 暴露该 seam；application 模块仍不依赖 transport DTO。

离线 `KnowledgePublicationService` 是建立在 `KnowledgePublicationPort` 上的独立 application 边界。它校验一个不可变、固定发布的 plan，并确定性选择同时满足 256-point 上限与 port 纯 worst-case 1 MiB wire inspection 的最大连续 node/edge 前缀。最终 batch 边界定义 ordinal、hash、fingerprint 与 resume 解释。随后它开始或重放 build intent，把权威 status 作为唯一恢复进度来源，只提交剩余 node batch 后冻结 node，再提交剩余 edge batch 后冻结 edge，最后对冻结工件执行 reconciliation。相同离线边界还提供显式 status 检查与幂等 abort；两者都不保存进度，也不删除不可变工件。只有权威 reconciliation 成功才能返回 `PublicationActivationCandidate`；activation 与 rollback 选择仍由外部认证 publisher/control-plane 通过 island-port 执行，绝不属于在线请求 runtime。

## 解析与评估

规范化产生有界语言感知查询形式，但不充当规范身份。解析采用闭合顺序：精确规范形式 -> 精确已发布别名 -> 有界屈折 -> 有界拼写修正 -> 有界转写 -> 语义提名。分数或向量信号不能把较低类别提升到合格较高类别之上。只有最佳可用类别的候选会保留；一个词义表示已解析，多个不同词义要求澄清或保留歧义，无候选则是显式未找到结果。实质合理的同形词、不同词性、短语级含义与领域特定词义保持分离。

每个非语义权威匹配都必须在排序前针对固定发布候选完成验证。稳定 form ID、精确存储 surface form、active 生命周期、lexeme ownership 与 form role 必须支持所声明的匹配类别。相互矛盾的来源证明会使 canonical-only 读取失败，不能静默变成未找到或获得较弱类别。明确的 canonical-only 操作携带自己的 canonical release pin，与 hybrid 检索显式的 vector-degraded 结果严格区分。

领域评估使用有界已发布清单，并返回 `existing`、`proposed_new`、`general` 或 `uncertain`。清单失败产生 `uncertain`，不能证明领域是新的。提案只在请求内存在，且仅在 full 响应允许的位置出现。

## 检索与组织

向量相似度只提名候选，绝不建立事实。每个展示的规范节点、边、证据项与修订都必须在请求发布固定值下由权威结构化数据补全。排序按词义、领域、语言、地区、时期、条件与证据策略过滤，但不改变身份、方向、来源或验证状态。

模型可组织所提供事实并生成精简解释，但不能虚构 endpoint 或修改事实元数据。确定性校验检查每个引用。空或支持较弱的分区被省略。

规范知识是断言图，而不是存储树。版本化关系注册表定义参与者角色，以及哪些 n 元断言允许生成二元遍历投影。Application 将该图变成有界引导视图：学习、术语、机理、对比与应用 lens 选择已审核根、分区顺序、深度/项目预算与证据策略。因此，树状展示无需复制事实，仍可解释，也不会假装所有关系都是层级。只有每条断言独立满足请求范围时，`knowledge/paths` 才返回短的具名路径。

`brief`、`standard` 与 `full` 是同一个已校验超集的投影。响应级别只改变广度，不改变事实选择或真实性状态。

## 验证

测试候选优先级、歧义、闭合领域结果、清单不可用、发布一致性、补全、关系注册表兼容性、n 元参与者校验、lens 预算、路径证据、循环安全展示、证据过滤、图边界、虚构引用、空分区与投影单调性。
