# 持久化边界

English: [Persistence boundaries](../../docs/reference/persistence.md)

本页分离 application 数据操作、服务 transport、数据库 repository 与物理存储实现。它是依赖方向的权威来源；精确线上字段由[规范数据](../interfaces/canonical-data_cn.md)与[检索数据](../interfaces/retrieval-data_cn.md)合同负责，物理 schema 由[存储目录](../interfaces/tables/README_cn.md)负责。

## 分层

Application service 只依赖面向用例的 port，例如活动发布选择、规范候选解析、事实 hydration 与候选检索。跨越这些 port 的是 domain 类型与闭合结果；SQL 字符串、表名、collection 名、向量厂商 filter、凭据、pool 和事务都不会跨越。

Transnet 使用出站 island-port UDS adapter 实现这些 port。Adapter 把 domain 操作转换为版本化 JSON，并保留请求 ID、deadline、发布 pin、响应边界和闭合错误。它们不连接数据库。

Island-port 拥有 repository 实现。其规范数据 repository 可以使用 MySQL，检索数据 repository 可以使用 Qdrant；任一实现都能在不改变 Transnet application port 或公开服务路由的情况下替换。Repository 代码拥有原生查询、migration、事务、连接池、凭据、collection alias 与厂商失败映射。

物理 MySQL DDL 与 Qdrant payload/index 定义是实现 artifact。它们实现逻辑发布、身份、证据、assertion 与候选合同，但本身不是服务接口。只有发布工具组合可接收具备 mutation 能力的 repository；在线 Transnet 组合只接收只读 port。

MySQL 演进遵循不可变编号 migration 与扩展/迁移/切换/收缩兼容窗口。安装快照绝不作为升级脚本重复执行。规范与产品 schema 保持独立迁移 ledger，内容激活也与物理 schema 部署分离。精确运维规则见 [MySQL 迁移策略](../interfaces/tables/migrations_cn.md)。

## 一致性

一个请求只选择一次兼容且不可变的规范与检索发布视图。每个下游读取都携带该 pin。候选检索可以提名标识符，但规范数据 hydration 仍是权威。Repository 故障、不兼容 schema、固定发布缺失或局部投影都是闭合条件，绝不能变成空的事实结果。

Publisher 先暂存规范修订，再由这些修订构建投影，核对数量、hash、证据覆盖、endpoint 覆盖、schema 版本与 embedding metadata，然后激活兼容视图。回滚选择已保留不可变 artifact，而不是重写它们。

## 验证

测试 application port 不含厂商类型、Transnet adapter 不含原生数据库请求、在线组合不具备 mutation 能力、发布 pin 经每次映射仍保持、repository 错误映射到闭合结果、物理实现在激活前完成精确核对，且全新安装与升级路径收敛到等价 schema。
