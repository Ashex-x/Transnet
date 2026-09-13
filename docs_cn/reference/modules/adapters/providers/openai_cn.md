# OpenAI 兼容提供方适配器

English: [OpenAI-compatible provider adapter](../../../../../docs/reference/modules/adapters/providers/openai.md)

本文定义 `src/adapters/providers/openai.rs` 中OpenAI 兼容模型端点的共享协议处理，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标适配器负责 HTTP 客户端构造、认证、端点信封、严格结构化输出解码、安全错误映射以及有界韧性集成。它绝不记录提示词、源文本、提供方正文、凭据或生成内容。角色特定的模型选择及提示词/schema 策略保留在 Gemma 适配器中。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../../modules_cn.md)
- [系统设计](../../../../transnet_cn.md)
- [Transnet 服务接口](../../../../interfaces/transnet_cn.md)
- [内容发布](../../../../guides/content-publishing_cn.md)

