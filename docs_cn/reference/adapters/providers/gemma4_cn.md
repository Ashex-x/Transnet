# Gemma 4 提供方适配器

English: [Gemma 4 provider adapter](../../../../docs/reference/adapters/providers/gemma4.md)

本文定义 `src/adapters/providers/gemma4.rs` 中Gemma 4 的短文本翻译与有界结构化组合策略，供实现或评审目标服务边界的贡献者阅读。

状态：已有部分当前基础；目标边界与生产组合尚未完成。

## 契约

当前提供方模块将不超过配置阈值的文本路由到 Gemma 4，当前旧版结构化查询也使用 Gemma 4 的严格 JSON Schema 输出。目标适配器通过版本化提示词与 schema 实现翻译模型端口，验证每个响应，并把线路机制委托给共享 OpenAI 兼容客户端。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

