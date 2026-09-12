# 开发与运维

English: [Development and operations](../../docs/guides/development.md)

在仓库根目录按 [README](../../README.md) 构建、运行和验证。公开 Rust item 和文档变更遵循[仓库约定](../conventions-v1.0.1_cn.md)。

## 运维

当前可执行文件不终止 TLS，必须在网关或服务网格后保持回环访问。它尚未组合目标 MySQL 卡片、Qdrant 知识图谱或规范内容发布能力。请求体有界且请求 ID 会传播。实现目标能力时必须同步更新状态、合同和测试。

相关：[配置](configuration_cn.md)、[设计](../transnet_cn.md)和 [Transnet 服务接口](../interfaces/port_cn.md)。
