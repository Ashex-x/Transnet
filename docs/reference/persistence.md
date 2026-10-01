# Persistence boundaries

中文：[持久化边界](../../docs_cn/reference/persistence_cn.md)

This page separates application data operations, service transport, database repositories, and physical storage implementation. It is authoritative for dependency direction; exact wire fields belong to the [canonical-data](../interfaces/canonical-data.md) and [retrieval-data](../interfaces/retrieval-data.md) contracts, while physical schemas belong to the [storage catalog](../interfaces/tables/README.md).

## Layering

Application services depend only on use-case ports such as active-release selection, canonical candidate resolution, fact hydration, and candidate retrieval. Domain types and closed outcomes cross those ports; SQL strings, table names, collection names, vector-vendor filters, credentials, pools, and transactions do not.

Transnet implements those ports with outbound island-port UDS adapters. The adapters translate domain operations into versioned JSON and preserve request IDs, deadlines, release pins, response bounds, and closed errors. They do not connect to a database.

Island-port owns repository implementations. Its canonical-data repository may use MySQL and its retrieval-data repository may use Qdrant, but either implementation can change without changing Transnet application ports or public service routes. Repository code owns native queries, migrations, transactions, pooling, credentials, collection aliases, and vendor failure mapping.

Physical MySQL DDL and Qdrant payload/index definitions are implementation artifacts. They implement the logical release, identity, evidence, assertion, and candidate contracts but are not themselves service interfaces. Publication tooling is the only composition allowed to receive mutation-capable repositories; the online Transnet composition receives read-only ports.

## Consistency

One request selects a compatible immutable canonical and retrieval release view once. Every downstream read carries that pin. Candidate retrieval may nominate identifiers, but canonical-data hydration remains authoritative. A repository outage, incompatible schema, missing pinned release, or partial projection is a closed condition and never becomes an empty factual result.

The publisher stages canonical revisions first, builds projections from those revisions, reconciles counts, hashes, evidence coverage, endpoint coverage, schema versions, and embedding metadata, then activates the compatible view. Rollback selects retained immutable artifacts rather than rewriting them.

## Verification

Test that application ports contain no vendor types, Transnet adapters contain no native database requests, online composition has no mutation capability, release pins survive every mapping, repository errors map to closed outcomes, and physical implementations reconcile exactly before activation.
