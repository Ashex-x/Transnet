# Quality assurance

中文：[质量保证](../../docs_cn/guides/quality-assurance_cn.md)

This guide defines evaluation, release gates, and monitoring for the [Transnet service design](../transnet.md).

Status: proposed; the complete harness and datasets are not implemented.

## Versioned evaluation artifacts

Every result identifies schema, prompt, model role, normalizer, MySQL card release, Qdrant node and edge releases, ranker, retrieval configuration, and content manifest as applicable. Datasets use licensed or consented canonical material and contain no production request content, credentials, or user data.

## Request and translation evaluation

Measure lexical-unit versus sentence-or-passage routing, including ambiguous short fragments. A routing error must choose the least intrusive useful response and create no durable state. Translation evaluation covers meaning, completeness, tone, register, structure, names, terminology, numbers, negation, idioms, and dialect. Tips are penalized unless material and responses are checked for the two-tip limit.

## Canonical retrieval evaluation

Measure exact sense resolution, language detection, morphology, spelling suggestions, aliases, transliteration, sparse technical-term recall, dense cross-lingual recall, reranking, evidence eligibility, and degraded MySQL-only behavior.

Relationship tests verify endpoint existence, release compatibility, direction, sense, evidence, language, region, period, domain, confidence, and verification state. Adversarial cases ensure intensity is not taxonomy and vector proximity is not promoted to translation, synonymy, hierarchy, causation, shared mechanism, or cultural fact. Bounded exploration tests one selected root at a time and separates verified from exploratory results.

## Content, privacy, and injection safety

Publication tests validate deterministic IDs, immutable collections, manifest reconciliation, quarantine, activation, and rollback. Privacy tests prove that request text, context, caller identity, generated provider bodies, and user-related state do not enter MySQL, Qdrant, caches, logs, traces, metrics, backups, queues, or vectors.

Treat request text, retrieved documents, evidence, and model output as untrusted data. Injection suites attempt to replace instructions, exfiltrate credentials, bypass release filters, fabricate evidence, or add hidden persistence.

## Reliability and release gates

Inject MySQL, Qdrant node, Qdrant edge, and model failures; invalid structured output; rate limits; stale releases; and partial publication. Expected behavior includes bounded schema repair, deterministic fallback, explicit uncertainty, no partial activation, no unsupported relationship generation, and no request-content persistence.

A release passes only when translation, routing, canonical-card, retrieval, relationship, evidence, degraded-mode, non-persistence, activation, rollback, schema-compatibility, and injection suites pass. Production monitoring records only aggregate operational outcomes and contains no request content or raw provider bodies.

## Related documents

- [System design](../transnet.md)
- [Service behavior](../product/learning-experience.md)
- [Content publishing](content-publishing.md)
- [Service interface](../interfaces/port.md)
