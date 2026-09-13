# Module reference

中文：[模块参考](../../docs_cn/reference/modules_cn.md)

This directory documents the important service modules and their ownership boundaries. Read this index first, then open the module page that owns the behavior. Exact wire and storage schemas stay in [interfaces](../interfaces/README.md); product semantics stay in the [system design](../transnet.md); procedures stay in [guides](../documentation-index.md#guides); public Rust item details stay in source comments and rustdoc.

Status: the current executable uses transitional loopback HTTP. Target pages describe intended boundaries unless their status section explicitly says the behavior is composed in the default runtime.

## Module map

- [Runtime](runtime/startup.md): process launch, composition, readiness, shutdown, and [configuration](runtime/configuration.md).
- [Transport](transport/server.md): UDS server, JSON admission, middleware, and thin API mapping.
- [Application](application/orchestration.md): use-case ownership; see the end-to-end [request dataflow](application/request-dataflow.md), [translation](application/translation.md), and [knowledge](application/knowledge.md).
- [Domain](domain/translation.md): translation vocabulary; see [lexical knowledge](domain/lexical-knowledge.md) and [releases](domain/releases.md).
- [Ports](ports/models.md): model operations; see [data access](ports/data.md).
- [Adapters](adapters/providers.md): provider clients; see [island-port](adapters/island-port.md).
- [Operations](operations/observability.md): safe telemetry and resilience; see [publication](operations/publication.md).

## Dependency rule

Dependencies point inward as `transport -> application -> domain <- ports <- adapters`. API code validates and maps wire data. Application code coordinates use cases. Domain code owns transport-independent rules. Ports describe operations needed by the application. Adapters implement external I/O.

The online service receives read-only data ports. Only the offline publisher may receive mutation-capable ports. Request text, history, model output, and request-local proposals never cross a durable write boundary.

## Finding the owner

Search this index by concept first. If the concept is a request or response field, route, error, SQL operation, or vector operation, use the owning interface document instead. If it is a current Rust symbol, use `rg` and rustdoc. Add a module page only when a stable boundary needs design, lifecycle, or cross-file guidance that source comments cannot express without duplication.

## Change rule

A module page explains ownership, dependencies, invariants, current/target status, and verification. It links to authoritative contracts instead of copying fields, examples, or procedures. Update the English page and its Chinese mirror together.
