# UDS HTTP server

中文：[UDS HTTP 服务器](../../../../docs_cn/reference/modules/transport/uds_server_cn.md)

`src/transport/uds_server.rs` is the target listener boundary for HTTP/1.1 over a Unix domain stream socket.

Status: target design; the file does not exist. The current executable binds transitional loopback TCP, so target socket paths and routes are not currently runnable.

## Lifecycle and ownership

The server creates the configured parent directory, proves no listener is active before removing its own stale socket, binds the socket, applies mode `0660` and configured ownership, and then admits bounded connections. It accepts origin-form paths with `Host: localhost`; the host value is ignored for routing.

The server never binds TCP, trusts forwarded identity headers, or exposes peer details. During shutdown it stops admission, drains accepted requests within their deadlines, closes the listener, and unlinks only the socket whose ownership it established.

Connection lifetime, requests per connection, request parsing, concurrency, and response size are bounded. HTTP upgrades, streaming, query strings, and chunked request bodies are rejected by the shared transport contract.

## Verification

Tests require a temporary directory and must cover stale-socket safety, active-listener refusal, mode checks, bounded shutdown, and absence of a TCP listener. The normative wire and ownership rules are in the [Transnet interface](../../../interfaces/transnet.md).

