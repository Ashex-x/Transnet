# 运行时启动

English: [Runtime startup](../../../docs/reference/runtime/startup.md)

本模块负责进程启动、依赖组合、就绪、listener 生命周期与优雅停机。

## 当前运行时

[`src/main.rs`](../../../src/main.rs) 加载配置、初始化日志和 provider、构造过渡期 router、绑定回环 TCP 并等待停机。开始服务前的失败为致命错误；Ctrl-C 与 Unix 终止信号启动有界停机。

默认可执行文件当前组合健康、翻译和旧模型查询。仓库中的规范数据、图、发布与就绪基础并不表示所有目标路由都已生产组合。

## 目标边界

Bootstrap 在产生副作用前校验全部设置，由外向内构造 adapter，绑定所属 Unix socket，并仅在必需依赖可用后报告就绪。停机停止准入、有界排空进行中工作、关闭 client，并且只移除本进程拥有的 socket inode。

启动模块负责组合，不负责业务策略。路由 handler 和 adapter 接收显式依赖；不得独立创建全局 client 或重新读取配置。

## 验证

覆盖致命配置与绑定失败、依赖敏感的路由注册、就绪转换、信号处理、有界排空及安全 socket 清理。配置细节见[运行时配置](configuration_cn.md)，socket 规则见 [Transnet 服务接口](../../interfaces/transnet_cn.md)。
