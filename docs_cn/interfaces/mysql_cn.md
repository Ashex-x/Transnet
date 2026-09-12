# MySQL 适配器接口

English: [MySQL adapter interface](../../docs/interfaces/mysql.md)

本合同定义 MySQL 8 中规范基础卡、私有书签学习卡、有界历史和排程状态的逻辑操作。JSON 示例表示有类型适配器值，不是网络协议或存储 schema。

状态：目标合同；当前可执行文件尚未组合此适配器。

## 通用合同

每个操作携带请求 ID、deadline 和预期 schema 版本。需要一致性的读操作钉住发布或卡片修订；变更操作使用幂等键和乐观修订。关闭结果为 `not_found`、`conflict`、`invalid`、`version_mismatch`、`unavailable` 和 `timeout`。

## 输入规范化与检索键

适配器保留 `original_input` 并接受版本化 `NormalizationResult`，不把原始输入当唯一卡片键。结果包含 Unicode NFKC、语言感知 case folding、首尾去空白、内部空白折叠、排版标点等价形式、可选宽松 alias、已应用变换和版本。

宽松 alias 可使 `Make`、`make`、`ma-ke`、`make*` 和 `make ` 都召回 `make`，但宽松相等不是身份相等。精确规范、精确 alias、屈折和拼写匹配排在前面；冲突返回备选。有意义的大小写、撗号、连字符、加号和井号保留，因此 `C`、`C++` 和 `C#` 不合并。

唯一性由稳定 card/sense ID 和已发布规范形式/alias 记录维护，不使用单一破坏性规范化字符串。

## 基础卡

`resolve_basic_card` 按规范形式、语言、可选上下文、英语方言和活动发布返回分词义卡。卡片包含规范形式、词性、精简翻译与定义、发音与形态摘要、CEFR、领域、Qdrant 根 ID 和发布。Qdrant 不可用时它仍须独立有用。

发布操作对不可变卡片修订做暂存、校验、激活、隔离和撤回。所有知识根通过合格检查后才可激活。

## 领域注册表与解析

领域是规范版本化记录，包含稳定 ID、规范名称、规范化名称、alias、精简定义、包含/排除范围、状态、发布和可选上位领域 ID。Alias 和规范化索引可映射多个候选，不静默合并冲突。

`resolve_domain` 先搜活动名称与 alias，再接收 Qdrant 召回的有界 RAG 候选，返回 `use_existing`、`needs_review` 或 `propose_new`。新建议必须说明名称、定义、范围边界、alias、上位/相关候选、证据以及为何旧领域不足。

模型不得写入活动注册表。`stage_domain_proposal` 在确定性规范化和冲突检查后只保存待审核工件。发布可拒绝重复、把 alias 并入旧领域，或分配新稳定 ID 和兼容 Qdrant 节点/边。基础卡只引用活动 domain ID。

## 书签与学习卡

`create_bookmark` 原子创建书签和第一个冻结 `LearningCard` 修订，并保存选定词义、正反面、例句、提示、目标、生成语境、知识发布、generator、prompt、rubric、evaluator 和 scheduler 版本。只有明确书签可创建持久状态。

`refresh_learning_card` 创建新修订并只迁移兼容的复习状态。来源修正、隔离或撤回会标记依赖卡待再生成。暂停、重排和删除都检查修订；删除后不再排程。

## 历史与策略快照

`append_history_event` 只接受规范卡/节点 ID、词义、操作、时间和可选紧凑结果、提示数或误解。每人最多保留过去 30 天内最新 200 条。原始查询、篇章、写作、答案、对话、解释和录音会被拒绝。

`load_strategy_snapshot` 只返回当前书签和合格历史，不返回持久隐藏画像。`clear_history` 立即移除所有历史影响。

## 尝试、掌握度与排程

`record_attempt` 幂等消费一次冻结练习，只更新已证明技能，并在允许时推进版本化 FSRS 风格日程。`needs_review` 或不确定评估不会负向更新。长期状态只保留紧凑结果。

## 存储与隐私规则

使用 `utf8mb4`、UTC 微秒时间、不透明公开 ID、所有权外键和事务性书签/尝试变更。私有字段使用唯一认证加密 nonce 和外部版本化密钥。删除覆盖书签、卡片、掌握度、日程和历史，不删除共享规范内容。

## 相关文档

- [系统设计](../transnet_cn.md)
- [Qdrant](qdrant_cn.md)
- [Island-port](port_cn.md)
- [内容发布](../guides/content-publishing_cn.md)
