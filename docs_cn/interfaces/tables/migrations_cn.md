# MySQL 迁移策略

English: [MySQL migration policy](../../../docs/interfaces/tables/migrations.md)

本策略约束目标 MySQL 实现的未来变更。[`mysql.sql`](../../../docs/interfaces/tables/mysql.sql) 是已审核目标快照，不是生产升级脚本。开始实现时，其精确逻辑结构冻结为 `000001_baseline.sql`；全新生产安装与升级都通过 runner 执行 island-port 拥有的不可变、单调编号 migration 链。

## 迁移身份与归属

规范 schema 与产品 schema 拥有独立迁移序列及各自的 `schema_migration` ledger。Migration 文件名使用 `NNNNNN_description.sql`；数字版本、名称、SHA-256 checksum 与 runner 版本必须匹配 ledger。已应用 migration 绝不编辑、重排或复用；修正使用新的 migration。

每次部署为对应 schema 持有一个数据库 advisory lock，拒绝未知或已变化 checksum，并在任一 ledger 行仍为 `applying` 时拒绝继续。由于 MySQL DDL 可能隐式提交，runner 在执行前记录 `applying`，且仅在 postcondition 通过后改为 `applied`。恢复过程检查声明的 precondition 与 postcondition，绝不盲目重跑部分执行的文件。

## 兼容窗口

每次 rollout 遵循扩展、迁移、切换、收缩：

1. **扩展：**增加 nullable 列、新表、索引、registry 行或可双读 payload 版本；旧 reader 与 writer 必须继续工作。
2. **迁移：**按有界、可重启的主键范围 backfill，在请求内容之外记录进度，限制负载，并校验数量与 hash。
3. **切换：**先部署 writer，再让 reader 依赖新数据；双写只允许存在于有界且有文档的窗口，然后激活 manifest 声明新 schema capability 的内容发布。
4. **收缩：**只有所有受支持 binary 与保留发布都不再需要旧读取、字段、索引或 payload 版本，且 rollback 窗口已结束后才移除它们。

单个 migration 不得同时重命名并删除在线列、缩窄列类型、重新解释现有 code、重写大表，或在没有 backfill/default 兼容阶段时增加必填字段。破坏性收缩 migration 需要备份/恢复演练及显式运维批准。

## 可扩展 Registry 与不可变数据

实体族与关系语义通过 `entity_type_revision` 和 `relation_type_revision` 数据驱动；增加类型无需修改 `canonical_entity`。发布固定精确的 source、evidence、类型 registry、entity 与 relationship 修订。已发布修订绝不原地更新；修正插入新修订并由后续发布引用。

数据库 `ENUM` 仅用于小型基础设施状态机；新增状态本来就需要协调代码与 migration。可扩展产品/知识分类使用 registry 行或版本化 payload schema，而不使用 `ENUM` 或每种类型一列。

JSON payload 在同一 schema 版本内只做 additive 演进。破坏性 payload 变更使用新的 `payload_schema_version`；reader 在兼容窗口明确支持有限版本集合。Publisher 在 hash 前校验并 canonicalize JSON，因此 key 顺序或 serializer 变化不能静默改变身份。

评估算法与隐私 threshold 是版本化行数据，而不是 DDL 常量。Publisher 在激活前根据 aggregate release 校验 projection，包括 `algorithm_version`、`policy_version` 与 `minimum_group_size`。新算法构建新的不可变 aggregate release，而不是改写活动版本。

## 在线 DDL 与 Backfill

执行前记录表大小、replica lag、剩余空间、预期锁行为及精确 MySQL 版本。生产 `ALTER TABLE` 语句声明可接受的 `ALGORITHM` 与 `LOCK`，使其在无法满足时失败，而不是静默退化为复制表或更强锁。只有验证该服务器版本确实支持后，才优先使用 instant 或 in-place additive DDL。如果 MySQL 会复制大表或长时间持有 metadata lock，应采用已审核 online-schema-change 流程，而不是强制执行语句。MySQL atomic DDL 保护单条受支持语句；它不是 transactional DDL，也不会使多语句 migration 原子化。

Backfill 必须幂等、可恢复、限速且可通过仅聚合计数观测。它使用稳定主键分页，绝不使用 `OFFSET`，也不把规范正文或私有产品值写入日志。只有 orphan 与重复检查通过后才增加外键与索引。

## 激活、回滚与移除

Schema 部署不会激活内容。发布先验证所有 serving binary 都理解 release manifest 与固定 payload 版本，再对账规范 SQL 与 Qdrant 投影，最后原子切换内容发布。

Application rollback 选择前一个不可变发布。只有数据库仍位于该 binary 声明的兼容范围时才允许 binary rollback。DDL rollback 通常采用向前修复；只有确认无损时才允许 down migration。删除列、表、registry 修订或保留发布前，必须证明没有受支持 binary、发布、外键、outbox command 或审计义务仍引用它。

## 必需验证

每个 migration 都测试全新安装、从最旧受支持 schema 升级、中断执行与恢复、新旧 binary 混合、有界 backfill、激活、application rollback 及 checksum drift。CI 还会比较通过所有 migration 创建的数据库与已审核快照，只忽略生成 constraint 名等有文档的物理差异。

相关：[存储目录](README_cn.md)、[持久化边界](../../reference/persistence_cn.md)、[内容发布](../../guides/content-publishing_cn.md)、[MySQL atomic DDL](https://dev.mysql.com/doc/refman/8.4/en/atomic-ddl.html)和 [InnoDB online DDL](https://dev.mysql.com/doc/refman/8.4/en/innodb-online-ddl.html)。
