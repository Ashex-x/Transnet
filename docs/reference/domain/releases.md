# Releases and degradation

中文：[发布与降级 domain](../../../docs_cn/reference/domain/releases_cn.md)

This module owns immutable compatible release identity and explicit degraded-read state.

## Release trio

One active content view consists of a MySQL card release plus paired immutable Qdrant node and edge collections. A request pins the complete trio once; every structured and vector read must use it. Mixing revisions or silently switching releases during a request is invalid.

MySQL or a signed publication artifact is authoritative for structured canonical content. Qdrant is a rebuildable projection. A vector candidate becomes usable only after same-release authoritative hydration.

## Degradation

If vector retrieval is unavailable, the service may return a useful MySQL-backed basic card with explicit degraded metadata. It must not invent relationships or hide missing knowledge families. If authoritative structured data or release compatibility fails, the operation fails safely.

Publication state changes occur only through the offline workflow. Live requests cannot create aliases, cards, facts, domains, edges, revisions, or releases.
