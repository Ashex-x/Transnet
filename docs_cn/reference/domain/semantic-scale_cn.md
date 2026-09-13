# 语义尺度

English: [Semantic scale](../../../docs/reference/domain/semantic-scale.md)

本文定义 `src/domain/semantic_scale.rs` 中一等的有序强度或程度维度，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

尺度具有稳定身份、命名维度与方向、条件、领域、证据、发布以及按顺序排列的词义限定成员。位置表示顺序，不表示相等的数值距离。检索可派生相邻的 `lower_degree_than` 或 `higher_degree_than` 边，但尺度绝不是分类法或同义关系的别名。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

