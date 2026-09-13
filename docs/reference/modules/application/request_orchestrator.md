# Request orchestrator

中文：[请求编排器](../../../../docs_cn/reference/modules/application/request_orchestrator_cn.md)

The request orchestrator owns one target translation turn after the API has decoded and validated its wire shape. It coordinates application services without embedding transport, provider, retrieval, or presentation policy in the handler.

Status: target design. The current executable does not compose this module; its loopback handlers directly wire the existing translation and legacy structured-lookup paths.

## Responsibilities and flow

The orchestrator receives the validated request, request ID, deadline, and a compatible active release pin. It invokes unit and intent routing, then the connected-text or lexical/domain path, and finally projects and validates one superset result. Every canonical and vector read in the turn uses the same release trio.

It owns cancellation and degradation decisions across the turn. It may use a bounded alternate provider, may return MySQL-only lexical content when Qdrant is unavailable, and stops downstream work when the shared deadline is exhausted. It does not reinterpret storage or provider failures as canonical facts.

## Dependencies and invariants

Dependencies are narrow application services and read-only ports for time, canonical structured data, vector candidates, translation/composition models, and safe aggregate metrics. The online composition never receives a mutation-capable publication port.

Current text, history, model bodies, and request-local proposals remain in request-owned memory and are discarded before return. No cache key, metric, trace, queue, or durable adapter may retain them. Response level changes projection breadth only; it does not select a different release, rerun fact selection, or change truth state.

## Verification

Tests should cover both routing branches, one release pin across all reads, deadline cancellation, bounded provider fallback, Qdrant degradation, dependency incompatibility, and absence of durable writes. Contract tests should prove that all successful paths end in response projection and validation and that safe errors reveal no request or dependency content.

## Related documents

- [Service module reference](../../modules.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [System design](../../../transnet.md)
- [Quality assurance](../../../guides/quality-assurance.md)
