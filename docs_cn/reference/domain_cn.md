# Domain 模块

English: [Domain module](../../docs/reference/domain.md)

Domain 模块负责与传输无关的翻译词汇、词汇与规范知识身份、关系语义、发布兼容性及降级结果不变量。它不包含 HTTP、provider、数据库、日志或进程生命周期代码。

## 翻译值

翻译请求包含源文本、源语言与目标语言选择器、响应级别及可选的按时间排序最小历史。语言标签会规范化并限制边界。历史是语言上下文，不是身份或持久状态；domain 值绝不携带用户 ID、持久化策略、provider 选择或存储指令。

翻译结果将主要译文与可选歧义、语域、术语或文化说明分开。Brief、standard 与 full 在完整结果组装后控制确定性广度。精确公开结构与限制由 [Transnet 服务接口](../interfaces/transnet_cn.md)负责。

## 词汇身份

词义或固定短语具有独立于拼写规范化的稳定规范 ID。词形、别名、发音、定义、语法、语域、形态、例句、搭配与限制保持附属于适用含义。不同词性或实质不同含义不得合并。

规范公共 ID 遵循带版本的 `canonical-id-v1` 策略。实体族前缀区分翻译、卡片、概念根与领域，其余不透明值由发布工作流分配，不能由查询、规范化形式、定义、译文、数据库行号或内容 hash 派生。修正保留稳定实体 ID，并创建下一个正数不可变修订；已发布修订的内容绝不原地修改。

一条已审核翻译具有稳定翻译 ID、发布成员关系、源语言与目标语言、一个正数修订、证据引用，以及显式词汇含义范围或段落范围。词汇范围保留 lexeme ID、可独立选择的 sense ID、词性、组合式或短语级状态，以及有序且有界的规范领域 ID。即使规范化表面形式相同，这些值也会保持同形词、不同词性、固定短语含义与领域特定词义相互分离。

带版本的 `translation-source-v1` fingerprint 是基于 NFC 源文、源语言和 fingerprint 合同版本的私有候选选择键。它保留有意义符号与大小写，因此 `C`、`C++` 与 `C#` 始终是不同候选。Fingerprint 命中绝不等同于身份或相等证明；application 代码采用候选前，必须按同一规范化合同比较返回的已存源文和请求源文。

## 知识与关系

规范知识由不可变、发布范围内的节点与原子事实组成。关系具有显式类型、方向、endpoint、适用性、条件、来源、证据、验证状态与修订。对称、逆投影和传递性是关系类型的声明属性，绝不能从措辞猜测。

语义尺度是与分类和同义分离的命名有序维度。成员位置表达顺序，不表达相等数值距离。派生相邻程度边与存储规范边保持可区分。

Verified 内容是已发布规范知识；inferred 解释与 exploratory 候选只在请求内存在，绝不作为事实持久化。每个展示节点必须是所选根或通过有用显式路径连接到根；相似度不能证明翻译、同义、层级、因果、机理或文化含义。

## 发布与降级

一个内容视图由 MySQL 卡片发布及配对的不可变 Qdrant 节点与边 collection 构成。请求只固定三元组一次，每次结构化与向量读取都使用它。MySQL 或签名发布工件是权威来源；Qdrant 是可重建投影，其候选需要同一发布补全。

已实现的 M3 foundation 通过 `KnowledgeReleaseTrio` 表示该激活候选：现有 canonical-only `CanonicalReleasePin`、一个强类型不可变节点 collection manifest、一个强类型不可变边 collection manifest，以及共享的 dense/sparse embedding 修订。边 manifest 绑定已验证节点内容哈希并携带完整端点数量。`ActiveContentVersion` 只作为确定性 match 建模所用的 domain-level compatibility shape 保留；任何可执行 runtime service 都不会选择它，它也不是发布权威。其中单个 `vector_collection_id` 绝不能代替两个 M3 collection。

已实现的关系 registry 冻结 retrieval-data 合同中的全部 v1 wire 名与 inverse 对。只有 taxonomy 对 `is_a` / `has_subtype` 具有传递性；当前所有 v1 关系均声明因果性不适用，所有非 taxonomy 关系均声明传递性不适用。发布声明与这些 registry 语义冲突时闭合失败。Assertion domain 为文档规定的词汇、短语、术语、概念、实体、领域、科学、语言及语义尺度 family 提供闭合目录。`CanonicalNodeId` 将 publisher 提供的不透明 ID 与其 family 一并保留，绝不生成 ID。当前 graph read 与 node projection model 仍仅支持 `Sense`、`Lexeme`、`Construction` 和 `Scale`；其他 family 在具备权威 publisher mapping 前保持不可投影。

`CanonicalAssertion` 是已实现且与 transport 无关的 n-ary 权威模型。一个不可变 publisher identity 与 revision 固定精确 relation-registry entry、有序且有角色名的 entity 或 typed-literal participant、已排序规范 `DomainId` scope、registry 自有结构化 condition、精确 evidence ID 与已解析 lineage、verification state 及 release。Domain、condition 与 condition-parameter ID 必须通过固定到同一发布的 registry record 解析；仅结构合法的 ID 不足以通过。Registry admission 会拒绝未知 role、错误 entity family、entity/literal 不匹配、缺失或过多 role cardinality、重复或不连续 role ordinal、不支持的 condition type 或 parameter 集、未排序 parameter/domain ID、未解析 evidence，以及跨发布或不允许 embedding 的 evidence。

N-ary assertion 不会被改写为猜测的 pairwise fact。`BinaryTraversalRule` 必须显式命名两个必填 singleton entity role 与一个现有 graph relation。Publisher 的 binary record 独立提供 assertion identity 与 revision；将 assertion 与从自身复制的值比较不构成 admission。`BinaryAssertionProjection` 会在 relationship 进入 projection 前证明该独立 linkage、role 到 endpoint 的 family/ID 相等、release 相等、完整 canonical evidence-lineage 相等，并执行现有 graph registry/evidence 规则。生成的 edge artifact 与 content hash 保留 assertion identity/revision、registry type/revision 与 traversal declaration。当前 edge schema 没有结构化 domain/condition payload，因此 scoped assertion 会闭合失败而非静默丢字段；只有空结构化 scope 能在此路径无损投影。缺少 declaration、未知或跨发布 registry record，以及任何不匹配都会闭合失败。

`PublishedRelationship` 是已实现的 Stage 2 admission aggregate。它将 stored relation 与两个端点的发布所有权、精确声明的 wire 方向和 inverse、经审核的 M2 evidence lineage 以及 verified lifecycle state 绑定。稳定 `PublishedEdgeIdentity` 包含 canonical relationship ID 与 revision，并排除 evidence revision。Evidence 变更通过新的不可变 relationship revision、精确 release membership、evidence content hash 与 projection/content hash 绑定。批量校验先按稳定 identity 排序，再验证候选，并使用独立的 scope-aware semantic key 拒绝重复 typed assertion，不受输入顺序或所提供 edge ID 影响。

Stage 3 preparation foundation 将 active 的权威 lexeme 与 sense 转换为确定性 node 工件，然后仅在两个端点都能从精确 node set 解析后，把已 admission 的 relationship 转换为 edge 工件。规范 lemma 现在携带有界、有序的 `lemma_evidence_ids`；这些引用支持 lemma assertion，不参与 lexeme identity，并且必须解析为同发布且允许 embedding 的 lineage。Projection admission 从 lexeme、可选且归属一致的 sense、active word form、localized gloss、经审核的词汇 translation 与精确 evidence lineage 构造一个有界权威 aggregate。缺失、重复、冲突、dangling、跨发布、未经审核或无权限的 material 均闭合失败；passage translation 以及 query/model 派生 material 被排除。

稳定 point ID 与内容 hash 使用带版本的 `knowledge-projection-hash-v1` 长度前缀规范序列化，而不依赖 JSON map 顺序或 Debug 输出。独立的 `node-dense-input-v1`、`node-lexical-input-v1`、`edge-dense-input-v1` 和 `edge-lexical-input-v1` 合同使用 UTF-8 NFC、保留大小写与技术符号、编码冻结字段顺序/tag、字节长度、presence marker 及确定性 list 顺序，并为 dense 与 lexical 使用不同 hash domain。Edge input 绑定冻结的 source/target node input hash、typed wire relation、已接纳 scope 与 verified evidence metadata，不包含生成式 relationship prose。Execution contract 将 dense family 固定为 1,024 维的 `Qwen/Qwen3-Embedding-0.6B`，其精确不可变 artifact revision 仍由部署提供；sparse 侧固定为非神经的 `transnet-lexical-bm25-v1` encoder。闭合 compatibility registry 必须精确匹配 artifact/encoder revision、dimensions、vector name 以及 node/edge input-spec version。这些仍是不含 collection ID、也不声明 production `Verified` 的预发布值。Transnet 现在已有严格的出站 publication/release-control client 与离线 orchestration，但仍没有 embedding execution、Qdrant mutation、island-port server、持久化 production reconciliation 或 active-pointer transaction。

`knowledge_publication` 定义与 transport 无关的 node-first lifecycle、确定性 build/request/batch identity、精确 retry classification、无碰撞 lexical dictionary proof、persisted collection/publication manifest hash 与 dense/lexical execution receipt。Raw vector、request ID、clock、random value 与 storage-generated identity 不能进入 canonical hash。Receipt validation 将独立观测的 execution metadata 与一个精确 registry entry 比较，并在 build、revision、dimensions、encoder、dictionary、input-spec 或 count 不匹配时闭合失败。`KnowledgePublicationService` 通过 `KnowledgePublicationPort` 驱动 begin、基于权威 status 的恢复、有界 node batch、node freeze、有界 edge batch、edge freeze 与 reconciliation，且不保留本地进度。成功 reconciliation 只返回 typed activation candidate；该服务不执行 embedding、不修改 collection、不持久化权威状态，也不激活或回滚 release。

向量失败时可返回带显式降级的 MySQL 基础卡，但不得虚构关系或隐藏缺失知识族。权威内容缺失或发布不兼容必须安全失败。实时请求不能创建别名、卡片、事实、领域、边、修订或发布。

精确持久 payload 由[规范数据](../interfaces/canonical-data_cn.md)与[检索数据](../interfaces/retrieval-data_cn.md)接口负责。

## 验证

测试语言与大小边界、历史丢弃、歧义保留、关系方向与证据规则、语义尺度排序、发布兼容性、同发布补全、显式降级及拒绝私有或持久化字段。
