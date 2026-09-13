# 指标

English: [Metrics](../../../../docs/reference/modules/observability/metrics.md)

本文定义 `src/observability/metrics.rs` 中用于服务健康状况的闭合聚合计数器与时延结果，供实现或评审目标服务边界的贡献者阅读。

状态：已有部分当前基础；目标边界与生产组合尚未完成。

## 契约

仓库基础已定义脱敏领域事件、指标端口和内存记录器，但当前可执行程序尚未组合目标导出器与路由级目录。目标标签保持低基数且闭合。导出、保留、采样、告警和仪表盘属于领域及端口类型之外的运维策略。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

