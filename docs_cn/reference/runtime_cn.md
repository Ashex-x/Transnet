# 运行时模块

English: [Runtime module](../../docs/reference/runtime.md)

运行时模块负责进程配置、启动、依赖组合、就绪、listener 生命周期与优雅停机。它包含生命周期接线，不包含翻译或检索策略。

状态：当前可执行文件绑定过渡期回环 TCP。目标组合使用 [Transnet 服务接口](../interfaces/transnet_cn.md)定义的 UDS 边界。

## 配置

进程相对 Cargo manifest 读取 config/transnet.toml。RUST_LOG 可覆盖配置 filter。解析拒绝未知字段；创建 client 或 listener 前，校验检查单项边界与不兼容组合。

Provider 凭据是 secret，绝不能出现在已提交文件、Debug 输出、日志、指标、trace 或错误中。配置与 bootstrap 之外的代码接收有类型设置，不重新读取文件或环境变量。精确字段、默认值与目标基础设施设置由[配置指南](../guides/configuration_cn.md)负责。

## 启动与组合

当前入口加载配置、初始化脱敏日志与 provider client、构造过渡 router、绑定 listener 并等待停机。默认可执行文件组合健康、翻译与旧模型查询；其他已纳入仓库的基础不一定完成生产组合。

目标 bootstrap 在副作用前校验设置，由外向内构造 adapter，绑定所属 Unix socket，只注册依赖存在的路由，并仅在必需依赖可用后报告就绪。Handler 与 adapter 接收显式依赖，不创建全局 client。

## 就绪与停机

Liveness 反映进程健康。Readiness 反映服务能否安全接收工作，可报告必需依赖失败，但不把可选增强当作致命问题。

停机停止准入，在现有 deadline 内排空已接受工作，关闭 client，并且只移除本进程拥有的 socket inode。绝不删除未解析路径或替换活动 listener。

## 验证

测试默认值、未知字段、覆盖优先级、脱敏、校验边界、致命启动错误、依赖敏感路由注册、就绪转换、信号、有界排空与安全 socket 清理。
