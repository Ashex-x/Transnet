# UDS HTTP 服务器

English: [UDS HTTP server](../../../docs/reference/transport/uds-server.md)

`src/transport/uds_server.rs` 是 Unix 域流套接字上 HTTP/1.1 的目标 Listener 边界。

状态：目标设计；文件尚不存在。当前可执行文件绑定过渡性回环 TCP，因此目标套接字路径与路由当前不可运行。

## 生命周期与所有权

服务器创建配置的父目录，在删除自己的陈旧套接字前证明没有活跃 Listener，绑定套接字，应用 `0660` 模式与配置所有权，然后接纳有界连接。它接受带 `Host: localhost` 的 origin-form 路径，Host 值不参与路由。

服务器绝不绑定 TCP、信任转发身份 Header 或暴露 Peer 详情。关闭期间停止接纳，在 Deadline 内排空已接收请求，关闭 Listener，并仅解除其已确立所有权的套接字。

连接寿命、每连接请求数、请求解析、并发与响应大小均有界。共享传输合同拒绝 HTTP Upgrade、Streaming、Query String 与 Chunked 请求 Body。

## 验证

测试需使用临时目录，并覆盖陈旧套接字安全、活跃 Listener 拒绝、模式检查、有界关闭及不存在 TCP Listener。规范 Wire 与所有权规则见[Transnet 接口](../../interfaces/transnet_cn.md)。

