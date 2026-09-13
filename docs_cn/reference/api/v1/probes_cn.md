# 版本 1 探针

English: [Version 1 probes](../../../../docs/reference/api/v1/probes.md)

`src/api/v1/probes.rs` 是健康、存活与就绪探针的目标薄 Handler 所有者。

状态：目标路由拆分。当前运行时暴露过渡性探针行为，但 UDS 上三个目标 `POST /transnet/v1/...` 路由尚未实现。

## 路由

`POST /transnet/v1/health` 报告进程可提供 HTTP 并解析空 `{}` 请求。`POST /transnet/v1/livez` 报告事件循环与强制内部控制状态是否工作。两者均不声称外部依赖或规范数据已就绪。

`POST /transnet/v1/readyz` 仅在已启用路由所需依赖可提供兼容 Schema，且规范路由具有兼容活跃发布时报告就绪。可选依赖降级按接口合同准确表示，不得把缺少强制能力视为就绪。

Handler 验证 `{}`、查询进程或就绪状态并序列化结果，不包含依赖探测策略、重试、翻译逻辑或持久化。

## 验证

测试应在启动、依赖降级、不兼容发布、停止接纳与排空阶段区分健康、存活和就绪。准确 Schema 与状态见[Transnet 接口](../../../interfaces/transnet_cn.md)。

