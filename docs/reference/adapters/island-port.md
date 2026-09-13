# Island-port adapters

中文：[Island-port adapter](../../../docs_cn/reference/adapters/island-port_cn.md)

This module maps data ports to island-port's versioned HTTP/1.1 JSON operations over its owned Unix socket.

The shared client owns connection lifecycle, content-type and body bounds, schema-version handling, deadlines, and safe transport errors. Structured and vector adapters map operation-specific domain inputs to `data/sql/v1` and `data/vec/v1` calls while preserving release pins and closed outcomes.

Island-port owns MySQL and Qdrant drivers, queries, pooling, transactions, collection selection, and credentials. Transnet adapters do not expose SQL or Qdrant-native request shapes. Filesystem permissions authenticate processes; JSON never carries forwarded end-user credentials or identity.

Online adapters are read-only. A separately authorized publisher composition uses mutation-capable operations. Exact payloads remain in the [SQL](../../interfaces/mysql.md) and [vector](../../interfaces/qdrant.md) interfaces.
