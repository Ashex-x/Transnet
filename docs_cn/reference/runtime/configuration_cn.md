# 运行时配置

English: [Runtime configuration](../../../docs/reference/runtime/configuration.md)

本模块负责进程配置的有类型加载、默认值、校验、secret 引用与脱敏诊断。

## 所有权

进程相对 Cargo manifest 读取 `config/transnet.toml`。`RUST_LOG` 可覆盖配置的 filter。Provider 凭据是 secret，绝不能出现在已提交文件、Debug 输出、日志、指标或错误中。

配置解析拒绝未知字段。校验在创建 client 或 listener 前检查单项边界与不兼容的跨字段组合。配置和 bootstrap 之外的代码接收有类型设置，不重新读取文件或环境变量。

## 当前与目标设置

当前 listener、provider 路由、CORS、日志、容错与默认值列在[配置指南](../../guides/configuration_cn.md)中。该指南也负责目标 UDS 与 island-port 设置。本页不复制字段目录。

## 验证

测试默认值、未知字段、覆盖优先级、脱敏、精确校验边界和跨字段不兼容。任何公开设置变更必须在同一变更中更新指南及英文镜像。
