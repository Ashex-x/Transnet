# 向量数据 endpoint 接口

English: [Vector data endpoint interface](../../docs/interfaces/qdrant.md)

本合同定义 island-port 提供的向量与图 HTTP endpoint，用于版本化规范节点与边。每个操作均为 UDS 上的 JSON。各 endpoint 的请求示例表示置于通用请求 envelope 内的 `input` object；响应示例是完整 body。Point 示例描述 island-port 的内部投影。

状态：目标 island-port 合同。Transnet 已包含强类型发布三件套、关系 admission 基础以及确定性的预发布节点/边构建工件，但当前可执行文件尚未组合向量客户端或 publisher。准备步骤当前只投影权威且 active 的 `Lexeme` 与 `Sense` 记录，解析其具备 embedding 权限的词汇 evidence，冻结独立 dense/lexical 规范输入但不生成向量，并且仅在所有端点都能从同一精确节点工件解析后构建边。Construction、scale 与更广泛的目标目录在 publisher-owned 规范来源冻结前保持闭合。Island-port/Qdrant collection 构建、对账、激活、回滚及生产验收仍是外部工作。

## 目录

- [向量数据 endpoint 接口](#向量数据-endpoint-接口)
  - [目录](#目录)
  - [endpoint 参考](#endpoint-参考)
  - [存储边界](#存储边界)
  - [发布与集合合同](#发布与集合合同)
  - [知识节点 point](#知识节点-point)
  - [知识边 point](#知识边-point)
  - [语义尺度 point](#语义尺度-point)
  - [POST /api/v1/nodes/search](#post-apiv1nodessearch)
  - [POST /api/v1/scales/search](#post-apiv1scalessearch)
  - [POST /api/v1/edges/search](#post-apiv1edgessearch)
  - [POST /api/v1/neighbors/search](#post-apiv1neighborssearch)
  - [Internal publication operations](#internal-publication-operations)
  - [Deprecated POST /api/v1/releases/publish](#deprecated-post-apiv1releasespublish)
  - [相关文档](#相关文档)

## endpoint 参考

- [向量数据 endpoint 接口](#向量数据-endpoint-接口)
  - [目录](#目录)
  - [endpoint 参考](#endpoint-参考)
  - [存储边界](#存储边界)
  - [发布与集合合同](#发布与集合合同)
  - [知识节点 point](#知识节点-point)
  - [知识边 point](#知识边-point)
  - [语义尺度 point](#语义尺度-point)
  - [POST /api/v1/nodes/search](#post-apiv1nodessearch)
  - [POST /api/v1/scales/search](#post-apiv1scalessearch)
  - [POST /api/v1/edges/search](#post-apiv1edgessearch)
  - [POST /api/v1/neighbors/search](#post-apiv1neighborssearch)
  - [Internal publication operations](#internal-publication-operations)
  - [Deprecated POST /api/v1/releases/publish](#deprecated-post-apiv1releasespublish)
  - [相关文档](#相关文档)

Island-port 默认监听 `/run/island-port/island-port.sock`，并遵循[共享 UDS JSON 传输](transnet_cn.md)。调用方绝不直接连接 Qdrant 或提交原生 Qdrant 请求；collection 选择、查询构造、凭据和连接池均由 island-port 负责。只有 Transnet 运行时和经过认证的发布工具可以访问套接字。运行时调用方具有搜索权限；发布要求 publisher 服务账户。

所有路由统一使用 `/api/v1` 前缀。Island-port 套接字与资源路径共同标识本向量数据 API；调用方无需在路径中添加 `data`、`vec` 或存储厂商名称。

每个精确请求 body 的结构为 `{"context": RequestContext, "input": EndpointInput}`。`RequestContext` 包含 `request_id`、`deadline_at`、值为 `vector-data-v1` 的 `schema_version`，并在适用时包含固定的 `content_release`。下方 endpoint 示例仅展示 `EndpointInput`。闭合 outcome 为 `ok`、`missing`、`invalid_payload`、`version_mismatch`、`unavailable` 和 `timeout`；发布还可返回 `conflict`。

## 存储边界

Qdrant 存储规范 Transnet 概念之间的关系。它是可重建的只读投影，MySQL 与经认证的发布工件仍是权威来源。向量只从已发布规范内容与已接纳的结构化关系生成，绝不嵌入或保存运行时请求文本。

## 规范 embedding 输入

冻结的输入合同为 `node-dense-input-v1`、`node-lexical-input-v1`、`edge-dense-input-v1` 与 `edge-lexical-input-v1`。每种合同使用结构化 UTF-8 规范字节、NFC（不使用 NFKC）、保留大小写与技术符号、固定字段 tag/顺序、无符号大端字节长度前缀、显式单字节 optional presence marker，以及确定性排序的有界列表。Release、typed identity、input family 与 input-spec version 都进入序列化。Dense 与 lexical 使用独立的带版本 hash domain；二者都不是 Stage 3 projection content hash，也不是未来 persisted collection hash。

Node material 仅允许 active 且归属同一发布的 lexeme、lemma 专属 evidence、可选 active 且归属一致的 sense 与 definition evidence、active word form、具备来源的 localized gloss、meaning scope 与节点匹配的已审核非 passage translation，以及允许 `embedding` 的精确 source/evidence lineage。空 optional list 明确编码 absence。禁止 query-derived alias、heuristic form、模型输出及未经审核的 translation。

Edge input 不包含 publisher 编写或生成式 explanation prose。它绑定冻结的 source/target node input hash、规范 relationship identity 与 revision、精确 typed/wire relation、已接纳 structured scope，以及 verified evidence identity、source、content hash 与 confidence。Island-port 在编码向量前必须通过 endpoint hash 解析已冻结 node input。

Island-port 是 embedding authority。`semantic` 向量使用 `Qwen/Qwen3-Embedding-0.6B`、1,024 维以及适用的 dense input specification。生产执行还必须具有精确且不可变的 artifact revision；model name、`latest`、branch name、可变 provider alias 或 deployment label 均不算 revision。只有部署提供并验证该不可变 revision 后，production dense registry entry 才成立。`lexical` 向量使用 revision 为 `v1` 的确定性 `transnet-lexical-bm25` encoder。闭合 compatibility registry 将 dense model family 与精确 artifact revision、lexical encoder identity 与 revision 映射至 dimensions、vector name 以及适用的 node/edge input specification。Registry entry 缺失、dimension 漂移、revision 漂移或 input-spec 不匹配均闭合失败。

### Lexical encoder 合同

`transnet-lexical-bm25-v1` 消费从 `node-lexical-input-v1` 或 `edge-lexical-input-v1` 解码出的文本字段；它绝不 tokenize 二进制 framing、不透明 ID、evidence ID、source ID 或 content hash。Node 文本包括 lemma、normalized lemma、可选 definition、form value 与 normalized form value、可选 morphology、localized gloss text，以及已审核 translation 的 source/target text。对于 edge，island-port 先将冻结的 source/target node input hash 解析为精确 node lexical input，再加入闭合 wire relationship 与已接纳的文本 scope value。缺失 endpoint input、hash 不匹配或不支持的 input version 均闭合失败。

文本仅执行一次 NFC normalization，并保持大小写敏感。禁止 NFKC、stemming、stop-word removal、依赖 locale 的 case folding、transliteration 与 heuristic alias generation。Token 是 Unicode letter、mark 或 decimal digit 的最大连续序列；ASCII `+` 与 `#` 仅在直接连接到该序列时保留。其他标点与空白均作为分隔符，空 token 丢弃。因此 `C`、`C++`、`C#` 是三个不同 term，其拼写和符号不会互相规范化。单个 token 最多 256 UTF-8 bytes；单 point 最多 16,384 个 token occurrence 与 4,096 个 unique term；超限闭合失败。

Release-local lexical dictionary 是完整冻结 collection 内所有不同 token UTF-8 byte string 的排序集合，按 unsigned bytewise order 排序。Index 0 保留，第一个 term 的 index 为 1，后续 term 使用连续 `u32` index。这是无碰撞 dictionary assignment，不是截断 token hash。Dictionary overflow、重复 index assignment 或 dictionary/input 不一致均闭合失败。Dictionary hash 与 encoder revision 属于 persisted collection manifest；两者都不会改变 Stage 4 embedding input hash。

Document-side sparse value 使用公式 `tf * (k1 + 1) / (tf + k1 * (1 - b + b * dl / avgdl))`，其中 `k1 = 1.2`、`b = 0.75`，`dl` 为精确 token occurrence count，`avgdl` 在完整 collection 冻结后计算。冻结文本字段中的重复 occurrence 分别计数，不应用未声明的 field boost。计算使用 IEEE-754 binary64 中间值，并以 round-to-nearest、ties-to-even 转换为持久化 binary32 value。名为 `lexical` 的 Qdrant sparse vector 必须启用 `idf` modifier。IDF 是由 persisted collection statistics 得到的 collection/query-time state，因此明确不进入 per-point canonical input bytes 或 input hash。M4 必须先定义独立的 `query-lexical-input` 合同才能实现 query encoding；publication 不会把 document input contract 当作未声明的 query contract 复用。

每个 canonical embedding input 最多 65,536 serialized bytes。Publication batch 最多 256 points 且 serialized request body 最多 1,048,576 bytes，两项限制独立执行。这些是执行边界，不是 canonical identity。Stage 4 的四类 input format 与 hash domain 均保持不变。

Compatibility registry schema 记录 `dense_model_family`、`dense_artifact_revision`、`dense_dimensions`、`dense_vector_name`、`node_dense_input_spec`、`edge_dense_input_spec`、`lexical_encoder_identity`、`lexical_encoder_revision`、`lexical_vector_name`、`node_lexical_input_spec` 与 `edge_lexical_input_spec`。Island-port 必须在未来 build receipt 中返回所匹配的 registry-entry identity 与观测到的不可变 dense artifact revision。该观测必须来自已加载的 deployment artifact 或 provider attestation，不能简单回显请求。观测结果与 registry entry 不一致时闭合失败。精确 Qwen artifact revision 及其 attestation mechanism 仍是 deployment blocker，不在本合同中以占位值冒充。

已实现的 Transnet publication foundation 现在表达该 registry schema、精确 execution receipt、无碰撞 lexical-dictionary manifest、稳定 build/batch identity，以及 domain-separated persisted-collection/publication-manifest hash。出站 `KnowledgePublicationPort` 与严格 island-port client 已通过共享 UDS transport 实现 publisher 侧 begin、有界 batch、freeze、reconcile、status 与 abort contract。空 registry 是合法的部署前状态；任何非空 entry 都必须携带精确不可变 dense artifact revision，因此仓库不会把浮动 model reference 冒充为可部署 entry。该 client 只验证 fake-transport contract：它不调用 embedding provider、不运行 lexical encoder、不创建或修改 Qdrant collection、不实现 island-port publication server，也不激活 release。

闭合 build lifecycle 为 `accepting_nodes` -> `nodes_frozen` -> `accepting_edges` -> `edges_frozen` -> `reconciling` -> `activation_candidate`。非终态 build 也可进入 `failed` 或 `aborting`；`aborting` 只能进入 `abandoned`，`failed` 或 `abandoned` 只能进入 `gc_eligible`。终态 failure/abandonment 不能原地恢复或成为 activation candidate。重复已完成的 finalize/reconciliation operation 属于 transport-level idempotent replay，不是第二次 lifecycle transition。

Build identity 由 immutable release、projection schema、node projection hash 与 compatibility-registry entry identity 派生。Batch identity 还绑定 collection family、连续 ordinal、canonical request fingerprint 与有序 batch content hash。完全一致的 retry 重放已存结果；相同 build/ordinal 携带不同 fingerprint 或 content hash 时闭合失败。Request ID、clock time、insertion order、randomness、Qdrant-generated value 与 raw vector bytes 均不定义 canonical publication identity。

Hash hierarchy 保持独立：canonical embedding input hash -> projection content hash -> persisted collection hash -> publication manifest hash。Persisted collection hash 绑定 collection family、release、projection schema/hash、排序后的 point identity 与 point projection hash、dense/lexical input hash、精确 compatibility entry、vector name、dimensions、lexical dictionary hash/cardinality 及 point count，并排除 raw dense/sparse vector bytes。Publication manifest hash 将 canonical release/schema 与不同的 node/edge persisted collection hash 绑定。上述强类型值在 wire 中的最终位置仍属于 publication transport contract。

Qdrant 不包含用户、学习者、账户、画像、偏好、查询、上下文、源段落、历史、保存项目、书签、练习、答案、掌握度、日程、布局、反馈、录音或隐私流程数据。向量只能由已发布规范内容及已接纳的结构化关系生成；运行时请求文本绝不嵌入或存储。

## 发布与集合合同

每个逻辑知识发布包含一个不可变 `knowledge_nodes` 集合和一个不可变 `knowledge_edges` 集合。两者共同钉住发布 ID、嵌入模型、向量维度、稀疏配置、payload schema 与内容哈希，并作为一个单元激活和回滚。

Point ID 必须确定。先构建节点再构建边。发布拒绝缺失端点、跨发布引用、无效方向、重复有类型边、缺失证据、不兼容词义、无支持的语言或领域主张，以及不匹配的嵌入元数据。

发布 manifest 示例：

下方 dense artifact revision 占位值仅用于展示必填字段。在替换为已部署的不可变 revision 并由闭合 registry 精确匹配之前，它不能用于 production publication。

```json
{
  "release_id": "knowledge-2026-09",
  "canonical_schema_version": "canonical-v1",
  "collections": {
    "nodes": {
      "collection_id": "knowledge_nodes__knowledge_2026_09",
      "payload_schema_version": "knowledge-graph-v1",
      "content_hash": "sha256:63af5c1e...",
      "point_count": 184220,
      "state": "verified"
    },
    "edges": {
      "collection_id": "knowledge_edges__knowledge_2026_09",
      "payload_schema_version": "knowledge-graph-v1",
      "content_hash": "sha256:b19d28a7...",
      "point_count": 612840,
      "verified_node_content_hash": "sha256:63af5c1e...",
      "state": "verified"
    }
  },
  "embeddings": {
    "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
    "dense_artifact_revision": "<deployment-supplied-immutable-revision>",
    "dense_dimensions": 1024,
    "sparse_encoder_identity": "transnet-lexical-bm25",
    "sparse_encoder_revision": "v1"
  },
  "endpoint_coverage": {
    "expected": 1225680,
    "resolved": 1225680
  }
}
```

物理节点与边 collection ID 是不同的强类型成员；活动 alias 不能作为任一不可变 ID。规范发布、两个 collection manifest、两种嵌入修订与端点覆盖共同形成一个激活候选。两个 collection 均须已验证，payload schema 必须匹配，边 manifest 必须指向精确的已验证节点哈希，且所有边端点都必须在该节点 collection 中解析。Transnet 本地 ranking version 属于请求时策略，不进入此权威 manifest。

节点投影必须先完成并验证，之后才开始边构建。Island-port 拒绝成员缺失、跨发布成员、schema 或嵌入不匹配、数量或哈希不匹配、端点覆盖不完整以及未验证构建；不得使用占位 collection ID。

Transnet 的预发布工件明确不是 collection manifest。它没有物理 collection ID，也绝不声明 `verified` collection 生命周期状态。稳定 point ID 是对发布、payload schema、强类型规范身份和 point family 的带长度前缀、版本化规范序列化取 SHA-256 得到的值。Point 与 build 内容 hash 使用相同的显式序列化、排序后的强类型身份与 evidence 引用以及固定字段顺序；输入顺序、request ID、时间戳、ranking score、Debug 输出和 Qdrant 生成值均不参与。完全相同的重复节点会折叠，冲突重复节点闭合失败，Stage 2 拒绝重复 typed relationship；edge 工件绑定精确 node build hash 以及 expected/resolved endpoint 数量。

## 知识节点 point

节点表示一个可独立解释的词义、短语、多语言术语、概念、实体、现象、机理、过程、方程、物理量、材料、仪器、方法、技术、应用、标准、组织、人物、地点、习语、隐喻、语法模式、搭配、误解或规范领域。

```json
{
  "id": "node_sweltering_hot_01",
  "vectors": {
    "semantic": "<1024-dimensional canonical-content vector>",
    "lexical": {
      "indices": [1842, 99104],
      "values": [1.0, 0.62]
    }
  },
  "payload": {
    "node_id": "node_sweltering_hot_01",
    "node_type": "lexical_sense",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_label": "sweltering",
    "aliases": ["oppressively hot"],
    "translations": [
      {
        "language": "zh-CN",
        "text": "酷热的"
      }
    ],
    "description": "uncomfortably hot, especially because of the weather",
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "confidence": 0.98,
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

Payload 索引覆盖发布、发布状态、验证状态、节点类型、词义 ID、语言、方言、地区、时期、领域 ID 和证据 ID。

规范领域节点还携带精简知识 profile，使 LLM 能区分可用 RAG 覆盖与空结果。Profile 列出可用事实族、支持语言、已验证事实数及 `seed`、`partial` 或 `curated` 覆盖。它是发布固定的清单元数据，不是证据，也不声称完整。

```json
{
  "node_id": "domain_weather",
  "node_type": "domain",
  "canonical_label": "weather",
  "aliases": ["meteorology context"],
  "description": "Conditions of the atmosphere at a place and time.",
  "inclusion_scope": ["temperature", "precipitation", "wind", "humidity"],
  "exclusion_scope": ["long-term climate classification"],
  "knowledge_profile": {
    "available_fact_families": ["definition", "taxonomy", "terminology", "measurement"],
    "languages": ["en", "zh-CN"],
    "verified_fact_count": 184,
    "coverage_state": "partial"
  },
  "verification_state": "verified",
  "release_id": "knowledge-2026-09"
}
```

## 知识边 point

边既是有类型连接，也是可检索的关系原因说明。

```json
{
  "id": "edge_sweltering_scorching_01",
  "vectors": {
    "semantic": "<1024-dimensional canonical-relationship vector>",
    "lexical": {
      "indices": [1842, 77103, 99104],
      "values": [0.71, 1.0, 0.48]
    }
  },
  "payload": {
    "edge_id": "edge_sweltering_scorching_01",
    "relation_version": 3,
    "fact_id": "fact_sweltering_degree_scorching_01",
    "fact_revision": 1,
    "source_node_id": "node_sweltering_hot_01",
    "target_node_id": "node_scorching_heat_01",
    "relation_type": "higher_degree",
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "conditions": ["temperature describes weather or an environment"],
    "restrictions": {
      "dimension": "temperature_intensity",
      "register": "general"
    },
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "evidence_state": "supported",
    "provenance": ["source_dictionary_2026_01"],
    "confidence": 0.96,
    "verification_state": "verified",
    "assessment_enabled": true,
    "release_id": "knowledge-2026-09"
  }
}
```

关系族覆盖词汇命名与翻译等价、分类与整体—部分、同义/反义/对比/明确命名的强度、配价/语法/搭配/固定表达、形态、语域/方言/地区/时期/场景/领域适用性、文化延伸，以及领域机理、因果、依赖、实现、应用、测量、标准化和术语。探索关系保持独立。版本化关系类型注册表定义方向、逆关系、对称性、传递性和因果性；UI 与 LLM 不从措辞猜测。Payload 索引覆盖两端、关系类型与版本、评估资格、发布与验证状态、发布版本、适用词义、语言、方言、地区、时期、领域和证据 ID。Qdrant 不存储任何判断或聚合值。

当前 Transnet registry 只冻结此处已有规范方向的映射：内部 `Hypernym` 以宽义指向窄义并发布为 `has_subtype`；内部 `Hyponym` 以窄义指向宽义并发布为 `is_a`；`LowerDegree` 与 `HigherDegree` 分别发布为 `lower_degree_than` 与 `higher_degree_than`。同义、反义、翻译等价、形态、构式、词源及弱关联保留已有内部方向与逆关系规则，但其 Qdrant wire 名、传递性与因果性仍是未解决的 contract gap。发布必须闭合失败，不能从 Rust variant 或英文标签推导名称。

投影前，已实现的 Transnet admission boundary 要求声明的 wire relation 与 inverse 精确匹配 registry，验证允许的端点 kind，并为 identity 确定性规范化对称端点，但不生成第二条 inverse edge。一个稳定 edge identity 包含不可变发布、publisher 分配的 relationship ID 与 revision、规范端点、内部关系类型和已接纳 scope；runtime rank、插入顺序、request ID、时间戳及 Qdrant 生成 ID 均不参与。重复 typed assertion 另行按发布、规范化端点、关系类型和 scope 拒绝，即使 publisher 提供了不同 relationship ID。Evidence revision 是否进入 edge identity 仍未冻结，因此不会猜测。

Admission 通过现有 canonical evidence lineage 解析每个 evidence ID。精确 ID 集必须匹配，每个 fragment 必须属于关系发布，source 与 fragment permission 都必须允许 storage 和 embedding，fragment 必须 active，generated evidence 必须已完成审核提升。Evidence confidence 使用现有闭合 `High`、`Medium`、`Low` domain 值，因此缺失或越界的数值无法进入该 domain boundary。当前 `GraphScope` 可安全携带强类型 dialect 与有界非空 register。自由文本 condition 与字符串 domain scope 在 canonical condition 和 domain-ID 语义冻结前一律拒绝进入 M3 投影。

`is_a` 从较窄词义指向较宽类别，`has_subtype` 是其逆关系。`lower_degree_than` 与 `higher_degree_than` 只在命名且兼容的维度内比较成员。程度边不暗示分类、同义或可互换。

## 语义尺度 point

第一类语义尺度存储为节点投影，使一次检索可返回完整合格梯度，而不是从无关 pairwise 边重建。成员为词义限定节点；位置表示顺序而非相等距离，可从该记录派生相邻程度边。

```json
{
  "id": "scale_environmental_heat_intensity_01",
  "vectors": {
    "semantic": "<1024-dimensional scale-description vector>",
    "lexical": {
      "indices": [1842, 77103, 99104],
      "values": [0.7, 1.0, 0.8]
    }
  },
  "payload": {
    "node_id": "scale_environmental_heat_intensity_01",
    "node_type": "semantic_scale",
    "dimension": "environmental_heat_intensity",
    "direction": "increasing",
    "domain_ids": ["domain_weather"],
    "conditions": ["describes weather or an environment"],
    "members": [
      {"node_id": "node_warm_temperature_01", "position": 10},
      {"node_id": "node_hot_temperature_01", "position": 20},
      {"node_id": "node_sweltering_hot_01", "position": 30},
      {"node_id": "node_scorching_heat_01", "position": 40}
    ],
    "evidence_ids": ["evidence_dictionary_1042"],
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

## POST /api/v1/nodes/search

结合命名稠密与稀疏检索、精确规范标签、别名、翻译、转写、缩写、公式和领域术语。服务临时创建查询向量，Qdrant 不接收租户或所有者标识。

请求：

```json
{
  "dense_vector": "<1024-dimensional ephemeral query vector>",
  "sparse_vector": {
    "indices": [1842, 99104],
    "values": [1.0, 0.55]
  },
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "verification_states": ["verified"],
    "node_types": ["lexical_sense", "phrase"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "node_id": "node_sweltering_hot_01",
        "score": 0.93,
        "matched_by": ["dense", "sparse", "canonical_label"],
        "payload": {
          "node_type": "lexical_sense",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_label": "sweltering",
          "verification_state": "verified"
        }
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

分数只可在相同模型和发布内比较。向量相似度仅是候选信号，不能证明翻译、同义、层级、因果、共同机制或文化意义。

## POST /api/v1/scales/search

查找包含一个选定规范节点的完整尺度候选。这是带可选向量排序的索引读取，只返回 ID、位置和资格元数据。Transnet 在把它呈现为事实前，必须通过 SQL endpoint 补全完整尺度与证据。

请求：

```json
{
  "member_node_id": "node_sweltering_hot_01",
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "verification_states": ["verified"],
    "domain_ids": ["domain_weather"]
  },
  "limit": 5
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "scale_id": "scale_environmental_heat_intensity_01",
        "score": 1.0,
        "member_position": 30,
        "verification_state": "verified",
        "fact_ids": ["fact_sweltering_degree_scorching_01"]
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

该 endpoint 不推断新尺度，也不返回不完整梯度。缺少结果仅表示未找到合格的已发布尺度，并不表示选定节点不可能具有强度关系。

## POST /api/v1/edges/search

检索规范结构化关系。先应用资格过滤条件再限制数量，已验证和探索性结果必须分开。

请求：

```json
{
  "dense_vector": "<1024-dimensional ephemeral relationship vector>",
  "sparse_vector": {
    "indices": [77103, 99104],
    "values": [1.0, 0.6]
  },
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "relation_types": ["higher_degree", "lower_degree"],
    "verification_states": ["verified"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "score": 0.91,
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree",
        "fact_id": "fact_sweltering_degree_scorching_01",
        "fact_revision": 2,
        "verification_state": "verified"
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

每项结果均为候选指针。在响应使用事实性解释、证据或来源前，Transnet 必须针对同一发布通过 `POST /api/v1/knowledge-facts/get` 补全引用的事实修订。

## POST /api/v1/neighbors/search

通过端点索引读取直接入边和出边，再按 ID 获取另一端节点。它不推断本体语义、不合成边，也不执行事实性多跳遍历。

请求：

```json
{
  "node_id": "node_sweltering_hot_01",
  "direction": "both",
  "relation_types": ["higher_degree", "lower_degree", "collocation"],
  "verification_states": ["verified"],
  "languages": ["en"],
  "domain_ids": ["domain_weather"],
  "release_id": "knowledge-2026-09",
  "limit": 20,
  "cursor": null
}
```

响应：

```json
{
  "outcome": "ok",
  "value": {
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_sweltering_scorching_01",
          "source_node_id": "node_sweltering_hot_01",
          "target_node_id": "node_scorching_heat_01",
          "relation_type": "higher_degree",
          "verification_state": "verified"
        },
        "node": {
          "node_id": "node_scorching_heat_01",
          "node_type": "lexical_sense",
          "canonical_label": "scorching"
        }
      }
    ],
    "next_cursor": null
  },
  "release_id": "knowledge-2026-09"
}
```

扩展始终限制为一次跟随一个选定根，并只返回对该根与请求范围合格的关系。只有每一步都是具名且独立证据合格的边时，服务才可组织短路径。任意深度遍历、基于相似链的路径断言、中心性和可变图事务不属于本合同。

## Internal publication operations

原先单 body 的 `/api/v1/releases/publish` 提案由仅供 publisher 调用的 island-port operation `POST /api/v1/knowledge-publications/begin`、`nodes/batch`、`nodes/freeze`、`edges/batch`、`edges/freeze`、`reconcile`、`status` 与 `abort` 取代。它们使用 transport schema `knowledge-publication-v1`、共享 `{ "context": ..., "input": ... }` envelope、RFC 3339 deadline、request ID 与不可变 `content_release`。Unknown field、malformed body、回显 context 不匹配，以及未知或矛盾的 outcome/code 组合都会闭合失败。

Begin 绑定稳定 build ID、canonical/projection schema、完整 node/edge projection hash、精确 compatibility entry、预期 count、调用方 idempotency key 与 canonical request fingerprint。每个 family-local batch 最多包含 256 个有序 point，序列化 JSON 最多 1 MiB；其零基 ordinal 必须连续，identity 绑定 build、ordinal、request fingerprint 与 canonical batch content hash。Control request 与所有 response 上限为 256 KiB。完全相同的 retry 重放已存结果；以不同 canonical content 复用 identity 时返回 `conflict` 与 `idempotency_conflict`。

Node freeze 先于 edge admission。Freeze response 提供由 island-port 分配的不可变 collection ID、persisted collection hash、projection hash/count，以及 dense/lexical execution receipt。Requested compatibility 不等于 execution proof：Transnet 会核对 server assertion 中的精确 dense artifact revision、dimensions、vector/input-spec name、lexical encoder revision、dictionary hash 与 processed count。该 assertion 背后的 production provenance 仍由 island-port deployment 负责。

Reconcile 绑定两个不可变 collection ID、projection/persisted hash、edge-to-node projection binding、point/endpoint count、已验证 receipt 与 publication manifest hash。只有完整 endpoint coverage 和精确 cross-artifact agreement 才产生闭合 `activation_candidate` 状态；该 operation 不修改 active release。Status 只读；abort 遵守 domain state machine，不能离开或中止 activation candidate。

```json
{
  "context": {
    "request_id": "req_publish_01",
    "deadline_at": "2026-10-01T12:00:00Z",
    "schema_version": "knowledge-publication-v1",
    "content_release": "knowledge-2026-10"
  },
  "input": {
    "build_id": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "canonical_schema_version": "canonical-v1",
    "projection_schema_version": "knowledge-graph-v1",
    "node_projection_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "edge_projection_hash": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "compatibility": {
      "entry_id": "deployment-qwen-r1",
      "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
      "dense_artifact_revision": "deployment-supplied-immutable-revision",
      "dense_dimensions": 1024,
      "dense_vector_name": "semantic",
      "node_dense_input_specification": "node-dense-input-v1",
      "edge_dense_input_specification": "edge-dense-input-v1",
      "lexical_encoder_identity": "transnet-lexical-bm25",
      "lexical_encoder_revision": "v1",
      "lexical_contract_identity": "transnet-lexical-bm25-v1",
      "lexical_vector_name": "lexical",
      "node_lexical_input_specification": "node-lexical-input-v1",
      "edge_lexical_input_specification": "edge-lexical-input-v1"
    },
    "expected_node_count": 184220,
    "expected_edge_count": 612840,
    "expected_endpoint_count": 1225680,
    "idempotency_key": "publish-knowledge-2026-10",
    "request_fingerprint": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  }
}
```

Status-shaped success response 回显 build ID、一个闭合 lifecycle state，以及每个 family 的下一预期 ordinal。闭合 lifecycle state 为 `accepting_nodes`、`nodes_frozen`、`accepting_edges`、`edges_frozen`、`reconciling`、`activation_candidate`、`failed`、`aborting`、`abandoned` 与 `gc_eligible`。闭合 outcome 为 `ok`、`missing`、`invalid_payload`、`version_mismatch`、`conflict`、`unavailable` 与 `timeout`；每个非成功 outcome 必须携带兼容的结构化 publication failure code。绝不根据 human message 分类。

## Deprecated POST /api/v1/releases/publish

这个较早的单 body target 示例只保留为历史上下文。它没有实现，新 publisher client 不得使用；上面的有界 operation 取代它。发布向新的不可变集合写入确定性 point，并在激活前校验；不得改写活动集合。

请求：

```json
{
  "manifest": {
    "release_id": "knowledge-2026-10",
    "canonical_schema_version": "canonical-v1",
    "collections": {
      "nodes": {
        "collection_id": "knowledge_nodes__knowledge_2026_10",
        "payload_schema_version": "knowledge-graph-v1",
        "content_hash": "sha256:dd401f2a...",
        "point_count": 184220,
        "state": "verified"
      },
      "edges": {
        "collection_id": "knowledge_edges__knowledge_2026_10",
        "payload_schema_version": "knowledge-graph-v1",
        "content_hash": "sha256:98d3a647...",
        "point_count": 612840,
        "verified_node_content_hash": "sha256:dd401f2a...",
        "state": "verified"
      }
    },
    "embeddings": {
      "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
      "dense_artifact_revision": "<deployment-supplied-immutable-revision>",
      "dense_dimensions": 1024,
      "sparse_encoder_identity": "transnet-lexical-bm25",
      "sparse_encoder_revision": "v1"
    },
    "endpoint_coverage": {
      "expected": 1225680,
      "resolved": 1225680
    }
  },
  "idempotency_key": "publish-qdrant-knowledge-2026-10"
}
```

响应：

```json
{
  "request_id": "req_publish_01",
  "schema_version": "vector-data-v1",
  "outcome": "ok",
  "value": {
    "release_id": "knowledge-2026-10",
    "node_collection": "knowledge_nodes__knowledge_2026_10",
    "edge_collection": "knowledge_edges__knowledge_2026_10",
    "node_count": 184220,
    "edge_count": 612840,
    "manifest_hash": "sha256:manifest-771e...",
    "endpoint_coverage": 1.0,
    "validation_state": "ready_for_activation"
  },
  "release_id": "knowledge-2026-10"
}
```

构建对账将数量、端点覆盖、内容哈希、嵌入版本和发布元数据与经认证 manifest 比较。不完整或不匹配的集合对绝不激活。修正创建新不可变发布；回滚选择未变更的保留集合对。

发布失败使用闭合结构化 code：`canonical_release_unavailable`、`node_build_unavailable`、`edge_build_unavailable`、`schema_incompatible`、`embedding_metadata_incompatible`、`invalid_lifecycle_transition`、`idempotency_conflict`、`artifact_revision_mismatch`、`lexical_encoder_mismatch`、`dictionary_mismatch`、`endpoint_reconciliation_failed`、`hash_or_count_reconciliation_failed`、`incomplete_trio`、`activation_conflict`、`immutable_release_unavailable`、`timeout` 和 `dependency_unavailable`。Island-port 从构建与对账状态映射这些 code；调用方不得解析 message 字符串。成功发布只产生不可变激活候选，不切换活动发布。

```json
{
  "request_id": "req_publish_01",
  "schema_version": "vector-data-v1",
  "outcome": "conflict",
  "error": {
    "code": "endpoint_reconciliation_failed",
    "message": "release projection did not pass reconciliation"
  },
  "release_id": "knowledge-2026-10"
}
```

错误响应不含 `value`；成功响应不含 `error`。`request_id`、传输层 `schema_version` 与所选 `release_id` 回显请求 context，不能替代 manifest 中的 canonical schema 或 collection payload schema。

通用闭合 outcome 仍为 `ok`、`missing`、`invalid_payload`、`version_mismatch`、`unavailable` 和 `timeout`；发布还可返回带上述发布失败 code 的 `conflict`。日志不得包含凭据、向量、规范源文本、请求内容或原始 Qdrant body。

## 相关文档

- [共享 UDS JSON 传输与 Transnet 接口](transnet_cn.md)
- [目标向量 collection 目录](tables/vec_cn.md)
- [Transnet 设计与外部接口](../transnet_cn.md)
- [MySQL 接口](mysql_cn.md)
- [内容发布](../guides/content-publishing_cn.md)
- [质量保证](../guides/quality-assurance_cn.md)
