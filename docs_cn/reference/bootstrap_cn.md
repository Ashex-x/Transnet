# Bootstrap 组合

English: [Bootstrap composition](../../docs/reference/bootstrap.md)

`src/bootstrap.rs` 是在线服务的目标组合根，面向把基础设施连接到应用 Port 的实现者。

状态：目标设计；当前运行时不存在此模块。组合仍嵌入 `src/main.rs`，仅暴露过渡性回环翻译与模型支持的 lookup。

## 职责

Bootstrap 加载并验证配置、初始化可观测性、构造 Provider 与 island-port 客户端、组合应用服务、注册就绪检查、绑定自有 Unix 套接字并协调关闭。在线组合仅接收只读结构化与向量 Port，绝不能接收发布写入能力。

启动顺序为配置 -> 可观测性 -> Provider 客户端 -> island-port 客户端 -> 应用服务 -> 就绪注册表 -> UDS 绑定 -> accept loop。仅当已启用路由所需依赖报告兼容 Schema 及兼容活跃发布时，才进入就绪状态。

关闭时停止接纳，在 Deadline 内排空已接收工作，关闭客户端，仅解除本进程拥有的套接字，刷新安全遥测后退出。部分启动只能清理由 bootstrap 已确立所有权的资源。

## 验证

组合测试应证明路由注册随依赖可用性变化，失败不会声称就绪，且关闭遵守所有权与 Deadline。另见[配置模块](config_cn.md)、[UDS 服务器](transport/uds_server_cn.md)与[Transnet 接口](../interfaces/transnet_cn.md)。

