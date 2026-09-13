# 发布模块

English: [Publication module](../../../docs/reference/publication/overview.md)

本文定义 `src/publication/mod.rs` 中可复用的离线暂存、验证、投影、对账、激活、隔离与回滚逻辑，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

该目标库是唯一允许组合具备变更能力的结构化与向量端口的组件。它先构建权威 MySQL 内容，再构建 Qdrant 投影，随后对账身份与哈希、评估精确发布三元组并原子激活。生成候选最初处于隔离状态；模型来源不是证据，激活前必须完成权利审核、确定性验证、证据或获批编辑来源策略以及审核人批准。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

