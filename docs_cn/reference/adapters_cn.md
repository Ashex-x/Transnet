# Adapter 模块

English: [Adapters module](../../docs/reference/adapters.md)

Adapter 模块实现模型与数据 port。它负责外部协议机制，同时保留 domain deadline、发布固定值、闭合结果与隐私规则。

## 模型 provider

共享 OpenAI-compatible client 负责 HTTP 构造、认证、响应大小限制、严格结构化输出解码、安全错误映射与容错集成。Gemma 4 负责短文本及有界结构化组织请求策略；TranslateGemma 负责较长连续文本策略。当前运行时在 translation.long_text_chars 处选择两者。

Provider adapter 绝不记录 prompt、源文本、历史、provider body、凭据或生成内容。错误只暴露闭合依赖与操作分类。

## Island-port 数据访问

Island-port client 把数据 port 操作映射到 island-port 在其所属 Unix socket 上提供的版本化 HTTP/1.1 JSON 调用。它负责连接生命周期、content-type 与 body 限制、schema 版本处理、deadline 和安全传输错误。

结构化与向量映射保留发布标识符与闭合结果。Island-port 负责 MySQL 和 Qdrant driver、查询、连接池、事务、collection 选择与凭据。Transnet 不暴露 SQL 或 Qdrant-native 请求。文件系统权限认证进程；JSON 绝不转发终端用户身份或凭据。

在线 adapter 只读。单独授权的 publisher 组合使用可变更操作。精确 payload 保留在 [SQL](../interfaces/mysql_cn.md) 与[向量](../interfaces/qdrant_cn.md)接口。

## 验证

测试精确 provider 与 island-port envelope、严格解码、响应边界、脱敏、deadline 传播、发布保留、状态分类、重试资格、断路器行为与本地 socket 失败。
