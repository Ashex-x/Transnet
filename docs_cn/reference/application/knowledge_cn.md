# 知识 application

English: [Knowledge application](../../../docs/reference/application/knowledge.md)

本模块负责单词与固定短语请求的词义解析、领域评估、发布固定检索、关系排序、页面组织与确定性投影。

状态：当前规范查询使用只读检索端口，在请求内组装每张卡片。查询衍生的快照缓存及其公共缓存合同已经移除。图拓扑缓存仍属于发布固定的规范内容缓存。有界领域评估基础与严格出站 inventory client 已实现，但尚未由默认在线 composition 暴露。新的有界 root-retrieval application boundary 已组合注入式 canonical root/hydration、ephemeral embedding、query lexical encoding 与严格 retrieval-data port；其窄 root/hydration seam 当前只有 fake 实现，仍等待 canonical-data adapter，且尚未暴露公开 route。下文完整页面组织仍属于目标行为。

canonical-only 服务从权威端选择一次发布 pin，再通过读取 port 组合已审核翻译候选、确定性排序的词汇候选和无歧义的词义详情。它使用带 canonical-only content pin 的现有 lookup-card 类型，不伪造向量集合，也不把有意的纯词法读取误称为向量故障降级。可执行文件只在显式配置时构造并保存该依赖，用 active-release 只读探针检查就绪，并通过冻结的 BasicCard lookup 与固定发布 sense 路由提供该能力。外部 island-port server 仍需实现匹配的内部合同。请求局部查询形式有界且去重：基线规范化形式最强，谨慎的空白或外围标点变体只是较低优先级的拼写候选。已发布别名、形态、转写和语义归属由权威端确定，不从查询字符串猜测。

公开的固定发布 sense follow-up 使用调用方的 `CanonicalReleasePin` 与 sense ID，不再重新选择 active 内容。现有候选/证据类型携带的 source attribution 来自权威发布的权利审核 metadata，绝不由 source ID 推导。HTTP 边界通过 `POST /api/v1/senses/get` 暴露该 seam；application 模块仍不依赖 transport DTO。

离线 `KnowledgePublicationService` 是建立在 `KnowledgePublicationPort` 上的独立 application 边界。它校验一个不可变、固定发布的 plan，并确定性选择同时满足 256-point 上限与 port 纯 worst-case 1 MiB wire inspection 的最大连续 node/edge 前缀。最终 batch 边界定义 ordinal、hash、fingerprint 与 resume 解释。随后它开始或重放 build intent，把权威 status 作为唯一恢复进度来源，只提交剩余 node batch 后冻结 node，再提交剩余 edge batch 后冻结 edge，最后对冻结工件执行 reconciliation。相同离线边界还提供显式 status 检查与幂等 abort；两者都不保存进度，也不删除不可变工件。只有权威 reconciliation 成功才能返回 `PublicationActivationCandidate`；activation 与 rollback 选择仍由外部认证 publisher/control-plane 通过 island-port 执行，绝不属于在线请求 runtime。

## 解析与评估

规范化产生有界语言感知查询形式，但不充当规范身份。解析采用闭合顺序：精确规范形式 -> 精确已发布别名 -> 有界屈折 -> 有界拼写修正 -> 有界转写 -> 语义提名。分数或向量信号不能把较低类别提升到合格较高类别之上。只有最佳可用类别的候选会保留；一个词义表示已解析，多个不同词义要求澄清或保留歧义，无候选则是显式未找到结果。实质合理的同形词、不同词性、短语级含义与领域特定词义保持分离。

每个非语义权威匹配都必须在排序前针对固定发布候选完成验证。稳定 form ID、精确存储 surface form、active 生命周期、lexeme ownership 与 form role 必须支持所声明的匹配类别。相互矛盾的来源证明会使 canonical-only 读取失败，不能静默变成未找到或获得较弱类别。明确的 canonical-only 操作携带自己的 canonical release pin，与 hybrid 检索显式的 vector-degraded 结果严格区分。

领域评估在调用方单一不可变发布 pin 下使用有界已发布清单，并返回 `existing`、`proposed_new`、`general` 或 `uncertain`。权威端提供多语言 label、alias 与 definition、inclusion/exclusion scope、规范 broader ID，以及包含 fact family、language、verified count 和 `seed`、`partial` 或 `curated` coverage 的 knowledge profile。确定性校验只接受该精确 allowlist 中的稳定 ID。

任何 inventory 调用失败都产生 `uncertain`，不能证明领域是新的。成功但不完整的 catalog 同样不能产生 `proposed_new`。只有明确 complete 且严格排序的 catalog 才允许结构化提案；其 broader ID 必须属于所提供 allowlist，且精确的规范 language-plus-label pair 不得与所提供 label 或 alias 冲突。提案没有稳定 identity，仅在请求内存在，不由该流程持久化，且只在后续 full 响应允许的位置出现。Application 不暴露 live domain-creation capability。

## 检索与组织

向量相似度只提名候选，绝不建立事实。每个展示的规范节点、边、证据项与修订都必须在请求发布固定值下由权威结构化数据补全。排序按词义、领域、语言、地区、时期、条件与证据策略过滤，但不改变身份、方向、来源或验证状态。

Root retrieval 始终先完成 canonical resolution。Not-found 与同优先级歧义会在 embedding、lexical encoding 或访问 retrieval-data 前停止。单一已解析 root 为所有后续 operation 冻结 release；调用方 pin、projection echo 或 hydrated record 若来自其他 release，则闭合失败。Transnet 临时生成 1,024 维 dense signal 与 `query-lexical-input-v1` sparse signal；retrieval-data 只接收 vector 与结构化 filter，不接收 query text。

每个被提名的 projection pointer 必须先通过 canonical-data 批量补全，才能进入已校验超集。缺失补全会显式产生 partial-publication coverage；identity、label、sense 或 release 异常则闭合失败。确定性重排使用有界整数 score、闭合 nomination mechanism，并以 canonical node identity 作为最终 tie-breaker。Canonical root 标记为 `verified`；相似度提名即使成功补全 node identity 仍保持 `exploratory`，且该流程不会创建 `inferred` claim。Vector retrieval 不可用时，outcome 显式标记为 canonical/MySQL-only，不编造 relationship，也不隐瞒 vector 失败。

模型可组织所提供事实并生成精简解释，但不能虚构 endpoint 或修改事实元数据。确定性校验检查每个引用。空或支持较弱的分区被省略。

规范知识是断言图，而不是存储树。版本化关系注册表定义参与者角色，以及哪些 n 元断言允许生成二元遍历投影。Application 将该图变成有界引导视图：学习、术语、机理、对比与应用 lens 选择已审核根、分区顺序、深度/项目预算与证据策略。因此，树状展示无需复制事实，仍可解释，也不会假装所有关系都是层级。只有每条断言独立满足请求范围时，`knowledge/paths` 才返回短的具名路径。

`brief`、`standard` 与 `full` 是同一个已校验超集的投影。响应级别只改变广度，不改变事实选择或真实性状态。

## 验证

测试候选优先级、歧义、闭合领域结果、清单不可用、发布一致性、补全、关系注册表兼容性、n 元参与者校验、lens 预算、路径证据、循环安全展示、证据过滤、图边界、虚构引用、空分区与投影单调性。
