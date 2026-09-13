# Island-port SQL 适配器

English: [Island-port SQL adapter](../../../../docs/reference/adapters/island_port/sql.md)

本文定义 `src/adapters/island_port/sql.rs` 中将结构化数据端口映射到 island-port 版本化 SQL 数据端点，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标运行时适配器在 `data/sql/v1` 下执行面向操作的读取调用，并保留闭合结果、发布固定信息和权威补全语义。MySQL 查询、事务、连接池和凭据由 island-port 负责。发布使用单独授权且具备变更能力的组合，绝不使用线上运行时端口。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

