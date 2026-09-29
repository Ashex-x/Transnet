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

已实现的 M3 foundation 通过 `KnowledgeReleaseTrio` 表示该激活候选：现有 canonical-only `CanonicalReleasePin`、一个强类型不可变节点 collection manifest、一个强类型不可变边 collection manifest，以及共享的 dense/sparse embedding 修订。边 manifest 绑定已验证节点内容哈希并携带完整端点数量。`ActiveContentVersion` 只保留给较早的进程内单索引检索基础，不是发布权威；其中单个 `vector_collection_id` 绝不能代替两个 M3 collection。

已实现的关系 registry 保留现有图身份，并校验端点 family、逆关系和对称性、证据、已验证生命周期及同发布所有权。精确 Qdrant 映射目前只覆盖合同已冻结的分类与具名强度方向。其他 M3 关系 wire 名及逐类型传递性/因果性仍是目标合同缺口，因此未来投影必须闭合失败。目标节点目录比已实现的 `Sense`、`Lexeme`、`Construction` 和 `Scale` 读取模型 family 更广；phrase、term、concept、entity 及专业节点必须先具备 publisher 所有的 canonical entity 映射，才能在不使用合成 ID 的情况下加入。

`PublishedRelationship` 是已实现的 Stage 2 admission aggregate。它将 stored relation 与两个端点的发布所有权、精确声明的 wire 方向和 inverse、经审核的 M2 evidence lineage 以及 verified lifecycle state 绑定。稳定 `PublishedEdgeIdentity` 包含 canonical relationship ID 与 revision，但在合同决策前排除 evidence revision。批量校验先按该 identity 排序，再验证候选，并使用独立的 scope-aware semantic key 拒绝重复 typed assertion，不受输入顺序或所提供 edge ID 影响。

Stage 3 preparation foundation 将 active 的权威 lexeme 与 sense 转换为确定性 node 工件，然后仅在两个端点都能从精确 node set 解析后，把已 admission 的 relationship 转换为 edge 工件。规范 lemma 现在携带有界、有序的 `lemma_evidence_ids`；这些引用支持 lemma assertion，不参与 lexeme identity，并且必须解析为同发布且允许 embedding 的 lineage。Projection admission 从 lexeme、可选且归属一致的 sense、active word form、localized gloss、经审核的词汇 translation 与精确 evidence lineage 构造一个有界权威 aggregate。缺失、重复、冲突、dangling、跨发布、未经审核或无权限的 material 均闭合失败；passage translation 以及 query/model 派生 material 被排除。

稳定 point ID 与内容 hash 使用带版本的 `knowledge-projection-hash-v1` 长度前缀规范序列化，而不依赖 JSON map 顺序或 Debug 输出。独立的 `node-dense-input-v1`、`node-lexical-input-v1`、`edge-dense-input-v1` 和 `edge-lexical-input-v1` 合同使用 UTF-8 NFC、保留大小写与技术符号、编码冻结字段顺序/tag、字节长度、presence marker 及确定性 list 顺序，并为 dense 与 lexical 使用不同 hash domain。Edge input 绑定冻结的 source/target node input hash、typed wire relation、已接纳 scope 与 verified evidence metadata，不包含生成式 relationship prose。Execution contract 将 dense family 固定为 1,024 维的 `Qwen/Qwen3-Embedding-0.6B`，其精确不可变 artifact revision 仍由部署提供；sparse 侧固定为非神经的 `transnet-lexical-bm25-v1` encoder。闭合 compatibility registry 必须精确匹配 artifact/encoder revision、dimensions、vector name 以及 node/edge input-spec version。这些仍是不含 collection ID、也不声明 production `Verified` 的预发布值；当前不存在 Qdrant client、embedding execution、mutation、reconciliation 或 activation 路径。

向量失败时可返回带显式降级的 MySQL 基础卡，但不得虚构关系或隐藏缺失知识族。权威内容缺失或发布不兼容必须安全失败。实时请求不能创建别名、卡片、事实、领域、边、修订或发布。

精确持久 payload 由 [SQL](../interfaces/mysql_cn.md) 与[向量](../interfaces/qdrant_cn.md)接口负责。

## 验证

测试语言与大小边界、历史丢弃、歧义保留、关系方向与证据规则、语义尺度排序、发布兼容性、同发布补全、显式降级及拒绝私有或持久化字段。
