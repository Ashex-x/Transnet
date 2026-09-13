# Interface catalog

中文：[接口目录](../../docs_cn/interfaces/README_cn.md)

This directory defines the target contracts between Transnet and island-port. They are normative within their stated target status; they do not claim that the current executable already exposes every route.

| Document | Boundary | Read it for |
| --- | --- | --- |
| [Transnet service interface](transnet.md) | island-port → Transnet | The public translation action, request-scoped history, response levels, result envelope, and follow-up canonical reads. |
| [SQL data endpoint interface](mysql.md) | Transnet / publisher → island-port → MySQL | Release-pinned canonical translations, cards, domains, facts, semantic scales, staging, and activation. |
| [Vector data endpoint interface](qdrant.md) | Transnet / publisher → island-port → Qdrant | Candidate node, relationship, and scale retrieval plus immutable projection publication. |

## Reading order

1. Start with [Transnet](transnet.md) for the user-facing and service-facing request/response contract.
2. Read [SQL](mysql.md) for what is authoritative and may be shown as a fact.
3. Read [vector](qdrant.md) for how candidates are retrieved; vector results are pointers, not canonical facts.

All three use the shared UDS JSON transport described in [Transnet](transnet.md). MySQL is authoritative; Qdrant is a rebuildable, release-pinned retrieval projection. Live request text and request-scoped history are never sent to these data contracts.
