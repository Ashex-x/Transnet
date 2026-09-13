# 时钟端口

English: [Clock port](../../../../docs/reference/modules/ports/clock.md)

本文定义 `src/ports/clock.rs` 中用于确定性截止时间与测试的 UTC 时间访问，供实现或评审目标服务边界的贡献者阅读。

状态：已有部分当前基础；目标边界与生产组合尚未完成。

## 契约

仓库当前的 `Clock` trait 返回基于 `SystemTime` 的 `UtcTimestamp`，并提供系统时钟适配器。目标编排使用该抽象进行截止时间核算，而不是在业务逻辑各处直接读取墙上时钟。它不携带请求数据。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

