# 指标端口

English: [Metrics port](../../../../docs/reference/modules/ports/metrics.md)

本文定义 `src/ports/metrics.rs` 中以尽力而为方式记录闭合且脱敏的聚合结果，供实现或评审目标服务边界的贡献者阅读。

状态：已有部分当前基础；目标边界与生产组合尚未完成。

## 契约

仓库当前的 `MetricsRecorder` 只接受闭合的 `MetricEvent` 值，并有意不返回错误，因此遥测失败不会改变用户可见结果。实现可按有界策略聚合、缓冲或丢弃事件，但不能添加请求文本、身份、凭据、模型 token、任意标签或数值载荷。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

