# Interface catalog

中文：[接口目录](../../docs_cn/interfaces/README_cn.md)

This directory defines the target contracts between Transnet and island-port. They are normative within their stated target status; they do not claim that the current executable already exposes every route.

| Document | Boundary | Read it for |
| --- | --- | --- |
| [Transnet service interface](transnet.md) | island-port → Transnet | Text, segment, and image translation; professional guidance; explicit live retrieval; capabilities; and guided knowledge views and paths. |
| [Canonical-data endpoint interface](canonical-data.md) | Transnet / publisher → island-port | Release-pinned canonical translations, cards, domains, facts, semantic scales, staging, and activation without exposing SQL. |
| [Retrieval-data endpoint interface](retrieval-data.md) | Transnet / publisher → island-port | Candidate node, relationship, and scale retrieval plus immutable projection publication without exposing a vector vendor. |
| [Target storage catalog](tables/README.md) | island-port persistence | Optimized hybrid MySQL schema, immutable Qdrant collections, private judgments, and anonymous distance projections. |

## Reading order

1. Start with [Transnet](transnet.md) for the user-facing and service-facing request/response contract.
2. Read [canonical data](canonical-data.md) for what is authoritative and may be shown as a fact.
3. Read [retrieval data](retrieval-data.md) for how candidates are retrieved; retrieval results are pointers, not canonical facts.

All three use the shared UDS JSON transport described in [Transnet](transnet.md). MySQL is authoritative; Qdrant is a rebuildable, release-pinned retrieval projection. Live request text and request-scoped history are never sent to these data contracts.
