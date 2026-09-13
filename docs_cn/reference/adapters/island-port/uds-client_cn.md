# Island-port UDS 客户端

English: [Island-port UDS client](../../../../docs/reference/adapters/island-port/uds-client.md)

本文定义 `src/adapters/island_port/uds_client.rs` 中通过 island-port 所拥有 Unix 套接字进行的有界 HTTP/1.1 JSON 传输，供实现或评审目标服务边界的贡献者阅读。

状态：目标模块设计；目标源文件尚未纳入仓库。

## 契约

目标客户端负责套接字连接生命周期、公共请求上下文、截止时间、正文上限、schema 版本结果和安全传输错误。文件系统凭据用于调用方授权；JSON 不得携带转发的用户凭据。它不暴露 SQL、Qdrant 原生请求或直接数据库驱动。

## 所有权与依赖

本模块包含领域词汇，或仅实现上述边界。它必须保持 Transnet 与用户无关、请求无状态的设计。线路结构以接口契约为准，发布策略以内容发布指南为准，跨模块语义以系统设计为准；本文不创建额外 API。

## 验证

实现代码后，在代码旁添加覆盖验证与不变量的单元测试，并在值跨进程边界时添加集成或契约测试。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 和 `cargo doc --no-deps`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [系统设计](../../../transnet_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [内容发布](../../../guides/content-publishing_cn.md)

