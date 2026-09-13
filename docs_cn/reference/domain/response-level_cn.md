# 响应级别

English: [Response level](../../../docs/reference/domain/response-level.md)

本文定义 `src/domain/response_level.rs` 中闭合的输出广度选择与确定性投影策略身份，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

公开值为 `brief`、`standard` 和 `full`。投影通过确定性允许列表从同一个已验证超集中移除字段和较低价值条目；它绝不改变选定事实、真实性状态或发布，也不能在会造成误导时隐藏实质上合理的词义。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

