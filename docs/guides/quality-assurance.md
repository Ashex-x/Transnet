# Quality assurance

中文：[质量保证](../../docs_cn/guides/quality-assurance_cn.md)

This guide defines evaluation, release gates, and monitoring for the [Transnet service design](../transnet.md).

Status: partially implemented. The checked-in synthetic semantic challenge set and executable-evidence matrix cover the runtime slices named below; production datasets, external-provider acceptance, migrations, and the complete release harness remain external or incomplete.

## Versioned evaluation artifacts

Every result identifies schema, prompt, model role, normalizer, MySQL card release, Qdrant node and edge releases, ranker, retrieval configuration, and content manifest as applicable. Datasets use licensed or consented canonical material and contain no production request content, credentials, or user data.

## Request and translation evaluation

Measure lexical-unit versus sentence-or-passage routing behind the single translation endpoint, including ambiguous short fragments. A routing error must choose the least intrusive useful response and create no durable state. Translation evaluation covers meaning, completeness, tone, register, structure, names, terminology, numbers, negation, idioms, and dialect. Multi-turn suites vary history length and verify reference resolution and terminology continuity without silent truncation of an accepted request. Ambiguity suites require materially different meanings and reject padded synonym lists.

Professional-input suites cover segment ID and order preservation, protected ranges, terminology constraints, bounded alternatives, image-region coordinates and reading order, mixed visual/text content, and unsupported-media rejection. They measure zero-model canonical hits, ordinary fast calls, bounded chunk calls, and at most one reasoning escalation against latency budgets. Live-retrieval suites enforce explicit opt-in, one search round, result/fetch limits, public-network-only fetches, redirect and DNS-rebinding checks, prompt-injection resistance, citation coverage, and disposal of queries and pages.

Projection tests build one canonical superset and compare `brief`, `standard`, and `full` responses. Lower levels must be strict field-and-item subsets except for envelope metadata, while preserving the same ranked meanings, canonical IDs, translation text, evidence states, and degradation signals. Tips are penalized unless material and responses are checked for the two-tip limit.

## Canonical retrieval evaluation

Measure exact sense and concept resolution, language detection, morphology, spelling suggestions, aliases, transliteration, sparse technical-term recall, dense cross-lingual recall, reranking, evidence eligibility, and degraded MySQL-only behavior. Domain-assessment tests cover existing-domain allowlist selection, genuine `proposed_new` detection, `general`, and `uncertain`, including familiar words used technically and term-like strings with weak evidence. Dependency failure must never masquerade as a new domain. Knowledge-profile tests verify fact-family and coverage counts against the release manifest.

Relationship tests verify endpoint existence, release compatibility, direction, applicable sense and domain, conditions, evidence, language, region, period, confidence, provenance, and verification state. Taxonomy suites validate `is_a` direction, `has_subtype` inverses, cycle rejection, and sense qualification. Semantic-scale suites validate named dimensions, ordering, conditions, evidence, unequal position spacing, and complete `warm → hot → sweltering → scorching` rendering. Adversarial cases ensure intensity is not taxonomy and vector proximity is not promoted to translation, synonymy, hierarchy, causation, shared mechanism, or cultural fact. Page-composition tests start from one selected root, omit weak sections, rank groups by purpose, validate every step of a short connection path, and keep verified, inferred, and exploratory results visibly separate.

Assertion tests validate relation-registry versioning, arity, participant roles, literal/entity exclusivity, binary projection eligibility, and lossless linkage from every projected edge to its authoritative assertion. Guided-view tests exercise learning, terminology, mechanism, comparison, and application lenses; enforce root, depth, item, evidence, and cycle bounds; and prove that a tree presentation never changes assertion truth or duplicates a canonical fact.

## Content, privacy, and injection safety

Transnet publication tests validate deterministic IDs, immutable projection payloads, manifest reconciliation, activation-candidate submission, explicit rollback selection, and fail-closed control receipts. Quarantine transitions, candidate retention, and direct-predecessor rollback eligibility are island-port authority acceptance requirements and remain open until exercised against that production boundary. Bootstrap policy still requires generated candidates to remain quarantined until evidence or an approved editorial-source policy, rights checks, deterministic validation, and review succeed; model output alone can never become verified. Repository privacy tests seed synthetic sentinels across current text, translation history, professional guidance, structured inputs, provider output, hidden reasoning, and request-local live material, then prove that Transnet-owned trace/log output, closed metrics, errors, and Debug diagnostics remain content-free. They also retain the structural checks that online routes expose no private-state or durable-job surface. Provider-side telemetry, collector and infrastructure retention, backups, external MySQL/Qdrant behavior, and production vector retention remain external acceptance items, so repository evidence does not close the production-wide non-persistence requirement.

Migration tests build a fresh schema and upgrade from every supported starting version, compare their logical shape, exercise interrupted DDL recovery and checksum drift, run old and new binaries across the declared compatibility window, verify resumable bounded backfills, and prove that contract migrations cannot run before retained releases and rollback binaries stop depending on the old shape.

Treat request text, retrieved documents, evidence, and model output as untrusted data. Injection suites attempt to replace instructions, exfiltrate credentials, bypass release filters, fabricate evidence, or add hidden persistence.

## Telemetry verification

Capture log, trace, metric, and audit sinks for every request class and seed recognizable secrets into text, segments, images, history, prompts, provider responses, web queries, pages, citations, and errors. No seed or deterministic fingerprint may appear. Enforce the [observability contract](../reference/observability.md): closed low-cardinality labels, static span names and routes, trace continuity, exactly one request completion event, bounded queues, visible drop counters, append-only audit ordering, and unchanged business responses when exporters fail.

## Reliability and release gates

Inject MySQL, Qdrant node, Qdrant edge, and model failures; invalid structured output; rate limits; stale releases; and partial publication. Expected behavior includes bounded schema repair, deterministic fallback, explicit uncertainty, no partial activation, no unsupported relationship generation, and no request-content persistence.

A release passes only when translation, routing, canonical-card, sense and concept resolution, domain assessment, retrieval, relationship semantics, page composition, evidence, degraded-mode, non-persistence, activation, rollback, schema-compatibility, and injection suites pass. Production monitoring records only aggregate operational outcomes and contains no request content or raw provider bodies.

The checked-in `tests/fixtures/target_challenges_v1.json` foundation is manually reviewed synthetic material, licensed under the repository's Apache-2.0 license, schema-versioned, and covers routing, translation fidelity, terminology, register, formatting, sense, concept and domain resolution, domain assessment, relationship semantics and selection, path validity, omission, fabrication, culture, and prompt injection. Its contract test fixes the license, reviewed repository-authored synthetic provenance, and explicit absence of production data; it rejects unknown fields, missing categories, excessive fixture or case counts, duplicate or unsafe IDs, unsupported language pairs, unbounded content, and malformed or duplicate expected codes.

`tests/fixtures/target_execution_matrix_v1.json` records the currently executable success and failure evidence. Success rows cover segments, image regions, guidance, live citations, labeled alternatives, and relationship-page projection. Failure rows cover canonical, retrieval, live, and model dependencies; stale releases and partial publication; rate limits, timeouts, cancellation, rollback, observability drops, UDS lifecycle, non-persistence, privacy, and injection isolation. Its strict contract test uses closed suite values, bounded identifiers/seeds/codes, exact category coverage, reviewed Apache-2.0 synthetic provenance, and repository-relative evidence references. Every evidence reference must resolve to a checked-in Rust test function; a fixture row without executable evidence fails the suite. This matrix is status evidence for those named tests, not a claim that production authorities or the complete release harness exist.

## Related documents

- [System design](../transnet.md)
- [Service behavior](../product/service-behavior.md)
- [Content publishing](content-publishing.md)
- [Transnet service interface](../interfaces/transnet.md)
- [Observability contract](../reference/observability.md)
- [MySQL migration policy](../interfaces/tables/migrations.md)
