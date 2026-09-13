# Version 1 graph reads

中文：[版本 1 图读取](../../../../docs_cn/reference/api/v1/graph_cn.md)

`src/api/v1/graph.rs` is the target thin-handler owner for bounded graph and neighbor reads rooted at a canonical ID returned by translation.

Status: target routes. Existing graph services, pagination protection, and tests are foundations behind legacy routes; the default launcher does not compose `POST /transnet/v1/graph/get` or `POST /transnet/v1/graph/neighbors` over UDS.

## Handler boundary

Handlers strictly decode the root, release, relation filters, and bounds defined by the interface, preserve the request deadline and release pin, invoke graph application services, and serialize nodes, typed directed edges, evidence labels, and pagination state. They contain no ranking, topology, retrieval, or persistence decisions.

Graph reads preserve exact direction: `is_a` points child sense -> parent category and `has_subtype` is its inverse. Semantic-scale order is separate from taxonomy. Vector similarity may propose candidates but cannot create a returned canonical fact.

Routes accept no learner, user, saved-view, layout, or entity-tag ownership input. Cursors are opaque and integrity-protected; they must not reveal internal filters or identifiers beyond the public result.

## Verification

Contract tests should cover node and edge bounds, relation direction, release pinning, opaque pagination, unknown fields, not found, dependency degradation, and exclusion of private state. Exact schemas belong to the [Transnet interface](../../../interfaces/transnet.md).

