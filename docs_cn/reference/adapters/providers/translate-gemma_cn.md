# TranslateGemma 提供方适配器

English: [TranslateGemma provider adapter](../../../../docs/reference/adapters/providers/translate-gemma.md)

本文定义 `src/adapters/providers/translate_gemma.rs` 中TranslateGemma 的较长连贯文本翻译策略，供实现或评审目标服务边界的贡献者阅读。

状态：已有部分当前基础；目标边界与生产组合尚未完成。

## 契约

当前提供方模块仅在字符数超过 `translation.long_text_chars` 时选择 TranslateGemma，并发送其结构化消息形式。目标适配器把该角色特定请求构造封装在翻译模型端口之后。分块计划与术语台账属于请求内的应用编排，而不属于本适配器。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

