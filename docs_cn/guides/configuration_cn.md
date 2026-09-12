# 配置

English: [Configuration](../../docs/guides/configuration.md)

本文档说明当前可执行文件的已实现配置。仓库中的 `config/transnet.toml` 是默认值与字段的权威来源。

`[server]` 配置回环 `host`、`port`、`log_level` 和 `log_format`。`RUST_LOG` 覆盖日志等级；debug/release 分别写入 `logs/debug/transnet.log` 和 `logs/release/transnet.log`。`[http]` 配置请求体限制、精确 CORS origin 和 credential 行为；生产不允许浏览器直接调用 Transnet。

翻译与结构化查询 provider 表配置 OpenAI-compatible endpoint、model 和可选 API key 环境变量；不得提交真实凭据。容错表限制 timeout、retry、concurrency 和 circuit breaker。

当前可执行文件没有 MySQL、Qdrant、语音、排程和持久学习状态配置。目标能力将需要版本化的发布、保留、模型角色、rubric 和 scheduler 设置。不得在配置中放入学习者内容。

相关：[设计](../transnet_cn.md)、[MySQL](../interfaces/mysql_cn.md)、[Qdrant](../interfaces/qdrant_cn.md)、[Island-port](../interfaces/port_cn.md)和[开发](development_cn.md)。
