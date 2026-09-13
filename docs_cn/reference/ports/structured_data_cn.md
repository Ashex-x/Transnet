# 结构化数据端口

English: [Structured data port](../../../docs/reference/ports/structured_data.md)

本文定义 `src/ports/structured_data.rs` 中针对权威规范内容的只读应用操作，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标运行时端口按操作提供规范翻译、卡片、词义、领域清单与配置、事实与证据补全、语义尺度和活动发布读取。调用携带派生查询形式或规范 ID 以及发布固定信息，绝不携带用户身份或原始历史。具备变更能力的发布接口相互独立，线上组合无法使用。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

