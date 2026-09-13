# 可观测性模块

English: [Observability module](../../../docs/reference/observability/telemetry.md)

本文定义 `src/observability/mod.rs` 中进程遥测的安全初始化与所有权，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标模块在配置验证之后、外部客户端之前组合日志与指标。遥测是聚合且经过允许列表约束的：可描述操作类别、依赖类别、结果和有界时长桶，但绝不能包含当前文本、历史、规范文本、提供方正文、向量、凭据、身份或任意高基数值。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

