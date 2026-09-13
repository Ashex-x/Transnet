# JSON 传输

English: [JSON transport](../../../docs/reference/transport/json.md)

`src/transport/json.rs` 是所有 Transnet HTTP Body 的目标 Codec 边界。

状态：目标模块设计。当前 Axum API 具有 Body 限制与 JSON 处理，但共享严格目标 Codec 尚未拆分到此文件。

## 合同

请求与响应使用 UTF-8 JSON 及 `Content-Type: application/json`；客户端发送 `Accept: application/json`。每项操作都携带一个 JSON Object，探针也使用 `{}`。Body 限制默认为 1,048,576 字节，并在缓冲或解码前执行。

解码依据所属 Wire 类型拒绝畸形 JSON、重复或未知字段、非 Object 顶层值、不支持的媒体类型、Chunked 请求 Body 及超限 Body。编码使用路由的成功或问题 Envelope，且绝不记录或追踪 Body。

Codec 区分传输拒绝与有效应用结果，并把语义验证留给 API 请求类型与 Handler。它不添加用户身份、持久化元数据或隐藏兼容字段。

## 验证

合同测试应覆盖 UTF-8、媒体类型、准确大小边界、未知字段、`{}` 探针及安全编码。规范规则与示例见[Transnet 接口](../../interfaces/transnet_cn.md)。

