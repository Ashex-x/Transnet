# 运行时配置

English: [Runtime configuration](../../../docs/reference/modules/config.md)

`src/config.rs` 负责强类型运行时设置、默认值、验证与秘密引用，绝不读取请求数据。

状态：部分实现。当前类型覆盖回环 Listener、HTTP Body 与 CORS 策略、翻译路由、两个 Provider 和 Provider 容错。目标 UDS、island-port、发布、检索、模型角色与评估设置尚未实现。

## 合同

进程相对于 Cargo Manifest 读取 `config/transnet.toml`。跨字段验证必须在 Listener 接纳请求前使启动失败。秘密值在 Git 外提供，仅用于其出站边界，并从诊断中脱敏。

目标 Listener 使用可配置套接字路径与模式，而 API namespace 固定。当前 `server.host`、`server.port` 与浏览器 CORS 设置仅属于过渡性回环运行时，并在 UDS 服务实现后移除。

仅在文档规定处，Provider 专属容错覆盖才继承旧版翻译默认值。零限制、不安全 Listener 暴露、无效 Origin、不可用 Deadline 与不兼容发布设置均为配置错误，而非请求错误。

## 验证

测试应覆盖默认值、反序列化、脱敏、所有验证边界与跨字段不兼容。准确字段和默认值由[配置指南](../../guides/configuration_cn.md)负责；目标进程边界见[系统设计](../../transnet_cn.md)。

