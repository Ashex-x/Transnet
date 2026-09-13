# Island-port adapter

English: [Island-port adapters](../../../docs/reference/adapters/island-port.md)

本模块把数据 port 映射到 island-port 在其所属 Unix socket 上提供的版本化 HTTP/1.1 JSON 操作。

共享 client 负责连接生命周期、content-type 与 body 限制、schema 版本处理、deadline 及安全传输错误。结构化与向量 adapter 将面向操作的 domain 输入映射为 `data/sql/v1` 与 `data/vec/v1` 调用，同时保留发布固定值与闭合结果。

Island-port 负责 MySQL 和 Qdrant driver、查询、连接池、事务、collection 选择与凭据。Transnet adapter 不暴露 SQL 或 Qdrant-native 请求结构。文件系统权限认证进程；JSON 不携带转发的终端用户凭据或身份。

在线 adapter 只读。单独授权的 publisher 组合使用可变更操作。精确 payload 保留在 [SQL](../../interfaces/mysql_cn.md) 与[向量](../../interfaces/qdrant_cn.md)接口中。
