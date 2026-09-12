# Transnet service delivery plan

中文：[Transnet 服务交付计划](../docs_cn/todo_cn.md)

This plan sequences implementation of the [Transnet service design](transnet.md). It records target work, not implemented behavior.

## Current baseline

- [x] Private HTTP process, health probes, request bounds, request IDs, graceful shutdown, and redacted tracing.
- [x] Translation provider routing, resilience, and structured lexical-lookup foundations.
- [ ] Canonical MySQL content, Qdrant relationship releases, and the full stateless service contract.

## Phase 1: stateless HTTP semantics

- [ ] Implement the target translation envelope and translation-first routing.
- [ ] Implement request-local lexical-unit versus sentence-or-passage routing.
- [ ] Implement the target canonical lookup, sense, graph, and neighbor response schemas.
- [ ] Reject user identifiers, cookies, end-user bearer tokens, and unsupported mutable product routes.
- [ ] Ensure request text and context are absent from logs, telemetry, caches, and durable work.

## Phase 2: canonical content and releases

- [ ] Implement MySQL cards, senses, aliases, domains, evidence records, immutable revisions, and release activation.
- [ ] Implement deterministic IDs, publication validation, quarantine, correction, and rollback.
- [ ] Build immutable Qdrant node and edge collections with named dense and sparse vectors.
- [ ] Reconcile endpoint coverage, content hashes, embedding metadata, and authenticated manifests before activation.

## Phase 3: retrieval and graph safety

- [ ] Implement exact, hybrid, endpoint, and bounded one-root graph retrieval.
- [ ] Keep verified typed edges separate from exploratory vector associations.
- [ ] Implement explicit degraded responses when MySQL is available and Qdrant is not.
- [ ] Enforce release, language, dialect, region, period, domain, evidence, and verification filters.

## Phase 4: reliability and contract quality

- [ ] Design and implement stateless exercise generation, writing evaluation, reference speech, and pronunciation-analysis operations without adding user persistence.
- [ ] Version prompts, schemas, models, normalizers, retrieval configuration, and content releases.
- [ ] Add deterministic validation and bounded repair for invalid structured provider output.
- [ ] Test input non-persistence across storage, caches, logs, traces, metrics, queues, and vectors.
- [ ] Gate releases on lexical resolution, retrieval, evidence, safety, degraded dependencies, activation, rollback, and schema compatibility.
- [ ] Keep English and Chinese docs, OpenAPI, source comments, and tests synchronized with the service contract.

## Definition of done

- A sentence receives a translation-first response and a lexical unit receives a canonical sense-specific response.
- MySQL basic cards remain useful without Qdrant, and graph results never promote similarity into unsupported fact.
- Content releases activate only as reconciled MySQL/Qdrant pairs and can roll back without mutation.
- No request text, context, caller identity, or user-related state is stored by Transnet.
- English and Chinese docs, human contracts, machine contract, source comments, and tests agree.

## Related documents

- [System design](transnet.md)
- [Service behavior](product/learning-experience.md)
- [Service interface](interfaces/port.md)
- [Quality assurance](guides/quality-assurance.md)
