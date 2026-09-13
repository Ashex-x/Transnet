# 证据

English: [Evidence](../../../docs/reference/domain/evidence.md)

本文定义 `src/domain/evidence.rs` 中具备权利信息的支持状态与可安全展示的规范内容来源，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

证据记录来源身份、权利与展示策略、支持状态、范围以及可安全展示的来源信息。它区分来源是否支持某项陈述与存储项是否已被规范验证。移除和取代通过新的不可变修订保留谱系，而不是改写历史。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../modules_cn.md)
- [系统设计](../../transnet_cn.md)
- [Transnet 服务接口](../../interfaces/transnet_cn.md)
- [内容发布](../../guides/content-publishing_cn.md)

