# 发布

English: [Release](../../../../docs/reference/modules/domain/release.md)

本文定义 `src/domain/release.rs` 中一个兼容的不可变规范数据快照及其降级状态，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标发布身份固定一个 MySQL 卡片发布及其匹配的不可变 Qdrant 节点和边集合。一次请求在整个编排过程中使用同一个兼容三元组。Qdrant 不可用时可降级到仅 MySQL 内容；权威结构化数据缺失或不兼容时，规范读取会安全失败。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

