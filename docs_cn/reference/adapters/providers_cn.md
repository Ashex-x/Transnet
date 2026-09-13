# Provider adapter

English: [Provider adapters](../../../docs/reference/adapters/providers.md)

本模块在 OpenAI-compatible endpoint 上实现模型 port，同时将协议机制与角色专属策略分离。

共享 client 负责 HTTP 构造、认证、响应大小限制、严格结构化输出解码、安全错误映射与容错集成。Gemma 4 负责短文本和有界结构化组织请求策略；TranslateGemma 负责较长连续文本请求策略。当前运行时在 `translation.long_text_chars` 处选择两者；目标 application 编排可增加请求级规划，但不得把该策略移入 HTTP handler。

Adapter 绝不记录 prompt、源文本、历史、provider body、凭据或生成内容。错误只暴露闭合依赖与操作分类。

使用本地 stub 测试精确请求 envelope、严格 schema、状态分类、响应边界、脱敏、deadline 传播、重试资格与断路器行为。
