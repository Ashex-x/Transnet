# 模型 port

English: [Model ports](../../../docs/reference/ports/models.md)

本模块为普通翻译与有界结构化生成定义窄 application 操作。

Port 接收已校验 domain 输入、显式 deadline 与版本化操作标识符，返回已校验候选输出或闭合失败分类。Provider URL、凭据、HTTP envelope、prompt、role message、JSON Schema 机制、重试 header 与模型专属 payload 保留为 adapter 职责。

Application 代码选择操作，而非 provider 品牌。模型输出绝不是规范证据。调用方必须在使用前校验结构与引用，并在请求结束时丢弃请求内容与生成材料。

测试在此边界使用 fake，以覆盖路由、timeout、畸形输出、不支持的 schema 行为与安全错误分类，无需调用外部 provider。
