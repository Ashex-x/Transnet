# Transnet service delivery plan

中文：[Transnet 服务交付计划](../docs_cn/todo_cn.md)

This plan turns the authoritative [Transnet service design](transnet.md) into ordered, verifiable delivery slices. It tracks repository state rather than aspiration: an item is checked only when its code, tests, and applicable contracts are synchronized.

Status: active implementation plan. Translation and model-backed lookup run today; canonical storage, release-pinned retrieval, and the extended stateless operations are not composed by the executable.

## Delivery rules

- Preserve the service boundary in every slice: Transnet owns canonical language knowledge and no user or product state.
- Complete milestones in order. A later milestone may be prototyped, but it is not releasable until the earlier exit criteria pass.
- Deliver vertical slices through domain types, ports, adapters, runtime composition, HTTP behavior, observability, tests, and documentation rather than declaring an isolated layer complete.
- Keep runtime availability explicit. Target OpenAPI coverage does not mean that a route is enabled.
- Update English and Chinese documentation together when a contract, configuration field, or process boundary changes.

## Verified baseline

- [x] Private loopback HTTP process, request bounds, exact-origin CORS, request IDs, health probes, graceful shutdown, and redacted structured tracing.
- [x] Length-based translation-provider routing with bounded resilience and closed operational metrics.
- [x] Model-backed structured lexical lookup in the default executable.
- [x] Library foundations and process-local test adapters for canonical lookup, sense detail, content releases, and bounded graph reads; these are dependency-injected and are not production storage composition.
- [x] Target human contracts and OpenAPI for translation, canonical lookup, sense detail, and bounded graph reads.
- [ ] Production MySQL and Qdrant adapters, an ingestion pipeline, paired release activation, and full target wire semantics.

## Milestone 0: enforce the stateless boundary

This is the prerequisite for all new storage and API work. Existing product-owned foundations must not become accidental service commitments.

- [ ] Remove learner profile, history, saved vocabulary, mastery, scheduling, practice-session, graph-layout, and private-feedback domain, application, port, and adapter code from Transnet.
- [ ] Remove asynchronous lookup-job and durable-work paths that can retain request queries or provider results; keep lookup execution within the bounded request lifetime.
- [ ] Remove transitional user-oriented lookup fields and sections, including learner level, word history, and practice preview, from request types, provider schemas, responses, and tests.
- [ ] Remove graph feedback capabilities and other product-owned metadata from public canonical graph shapes where they are not canonical evidence.
- [ ] Reject cookies, end-user bearer credentials, user or account identifiers, private-state fields, and unknown fields without echoing their values.
- [ ] Audit logs, traces, metrics, caches, queues, debug formatting, and errors so request text, context, provider bodies, credentials, caller identity, and audio cannot cross a persistence boundary. Cache only release-pinned canonical data addressed by canonical identifiers.
- [ ] Delete or rewrite tests and source comments that describe Transnet as the owner of personal state, then synchronize the human and machine contracts.

Exit criteria: no reachable route or reusable public service API accepts product-owned state; no request payload or derived request vector can reach a durable port; boundary rejection and non-persistence tests pass.

## Milestone 1: freeze request semantics and routing

- [ ] Implement the shared success and error envelopes, request metadata, strict unknown-field rejection, language-tag validation, and safe status mapping defined by the service interface.
- [ ] Implement a versioned request-local normalizer with Unicode normalization, language-aware case folding, whitespace and punctuation handling, meaningful-symbol preservation, and bounded derived forms.
- [ ] Implement typed intent routing: confident lexical units use canonical lookup, while clauses, sentences, passages, and ambiguous short fragments default to translation.
- [ ] Align POST `/translate` with the target response, including detected language, formatting and register handling, and no more than two material one-sentence tips.
- [ ] Align lookup, sense, graph, and neighbor request and response schemas with the target contract; remove transitional wire aliases after a documented compatibility decision.
- [ ] Report route availability and required versus optional dependencies accurately through readiness without exposing infrastructure details.
- [ ] Pin response metadata to schema, normalizer, model, retrieval configuration, and content-release versions as applicable.

Exit criteria: contract tests cover every enabled route and error envelope; routing evaluation covers words, lexical phrases, symbols, sentences, passages, and ambiguous fragments; runtime docs identify the exact enabled subset.

## Milestone 2: build the canonical MySQL core

- [ ] Define migrations and production repository adapters for cards, senses, forms, aliases, definitions, translations, pronunciations, morphology, examples, usage notes, canonical domains, evidence, immutable revisions, and release manifests.
- [ ] Make stable card, sense, domain, evidence, and revision IDs deterministic under a documented versioned policy; never use normalized query strings as identity.
- [ ] Implement exact canonical and alias resolution before bounded inflection, spelling-correction, transliteration, or relaxed-alias candidates.
- [ ] Implement draft staging, collision detection, evidence and license validation, quarantine, correction as a new revision, and immutable retention.
- [ ] Return a concise release-pinned basic card from MySQL without requiring a model or Qdrant.
- [ ] Add configuration, readiness probes, migrations, test fixtures, and failure mapping without exposing credentials or storage internals.

Exit criteria: a staged canonical release can be built reproducibly; exact lookup and sense reads work from MySQL alone; migration, collision, evidence, quarantine, and immutability tests pass.

## Milestone 3: build the Qdrant projection and safe retrieval

- [ ] Build deterministic immutable node collections before edge collections from published canonical content only.
- [ ] Store named cross-lingual dense vectors and sparse lexical vectors with release, language, dialect, region, period, domain, evidence, and verification payloads.
- [ ] Validate every typed edge for endpoint existence, direction, restrictions, evidence, confidence, verification state, and release compatibility.
- [ ] Implement exact, sparse, dense, hybrid, endpoint, and reranked retrieval with filters applied before limits.
- [ ] Implement shallow one-root graph expansion and opaque bounded pagination; keep taxonomy distinct from intensity dimensions and other typed relations.
- [ ] Keep verified relationships separate from exploratory associations in storage, ranking, response shapes, and evaluation. Similarity must never establish a canonical fact.
- [ ] Add a production Qdrant adapter, configuration, readiness probes, deterministic rebuild tests, and dependency-failure mapping.

Exit criteria: Qdrant can be rebuilt from one MySQL release with stable hashes; retrieval is release-pinned and filter-safe; adversarial tests prove that vector proximity is never promoted into a verified relationship.

## Milestone 4: activate paired releases and compose the runtime

- [ ] Reconcile MySQL revisions, Qdrant node and edge hashes, endpoint coverage, embedding metadata, schema compatibility, and an authenticated manifest before activation.
- [ ] Atomically select one compatible MySQL/Qdrant release pair for new requests while in-flight requests remain pinned to their starting release.
- [ ] Implement quarantine, failed-publication cleanup, correction, retention, and rollback by selecting an unchanged retained pair.
- [ ] Compose the production MySQL and Qdrant adapters into the executable and enable canonical lookup, sense, graph, and neighbor routes only when their required dependencies are ready.
- [ ] Return explicit MySQL-only degraded lookup responses when Qdrant is unavailable; never fill missing relationships with generated claims.
- [ ] Exercise stale, partial, mismatched, unavailable, and rollback states without partial activation or cross-release reads.

Exit criteria: a reconciled pair can be published, served, degraded safely, and rolled back without mutation; readiness and response metadata report the active capability and release accurately.

## Milestone 5: ground model-assisted responses and gate the core release

- [ ] Split translation, routing, lexical analysis, relationship explanation, and curation into bounded model roles with versioned schemas, prompts, limits, and evaluation criteria.
- [ ] Build request-local context only from eligible release-pinned canonical records and treat request text, retrieved material, and model output as untrusted data.
- [ ] Add deterministic structured-output validation, bounded repair, explicit uncertainty, and safe failure when repair is exhausted.
- [ ] Prevent generated text and similarity scores from becoming canonical content outside the publication workflow.
- [ ] Build versioned evaluation datasets for routing, translation, lexical resolution, retrieval, evidence, cultural scope, prompt injection, degraded dependencies, and schema compatibility.
- [ ] Prove request non-persistence across MySQL, Qdrant, caches, logs, traces, metrics, queues, backups, and provider telemetry.
- [ ] Run formatting, linting, tests, rustdoc, contract checks, publication reconciliation, rollback drills, and bilingual documentation checks as release gates.

Exit criteria: all core quality scenarios in the design and [quality-assurance guide](guides/quality-assurance.md) pass against the composed executable, and the enabled contract is ready for a versioned release.

## Milestone 6: add extended stateless operations

Start only after the core release gates pass. Each operation is a separate contract and delivery slice and remains disabled until its own privacy, quality, and failure criteria pass.

- [ ] Specify and implement stateless exercise generation for a caller-supplied canonical target, objective, difficulty, and constraints; return material and rubrics without creating a queue or attempt record.
- [ ] Specify and implement writing evaluation with meaning-preserving correction, an optional natural alternative, at most two prioritized explanations, and a retry prompt.
- [ ] Specify and implement scoped communication and cultural guidance with explicit relationship, setting, medium, dialect, and region boundaries.
- [ ] Specify and implement labeled reference TTS with bounded dialect, voice, rate, and purpose options.
- [ ] Specify and implement bounded pronunciation analysis with audio-quality and alignment checks, at most two intelligibility targets, uncertainty for inadequate evidence, and no retained audio.
- [ ] Add independent model and analyzer versioning, operational limits, evaluation datasets, readiness behavior, and release gates for every enabled operation.

Exit criteria: each enabled operation is reproducible, contract-tested, explicitly advertised, and proven stateless; the product remains the sole owner of exercises, attempts, feedback records, audio retention, mastery, and review scheduling.

## Service definition of done

- A sentence receives a translation-first response, while a confidently resolved lexical unit receives a canonical sense-specific response.
- MySQL basic cards remain useful without Qdrant, and graph results never promote similarity into unsupported fact.
- Content releases activate only as reconciled MySQL/Qdrant pairs and roll back by immutable selection.
- No request text, context, audio, caller identity, or user-related state is stored by Transnet or disclosed through observability and errors.
- Every enabled response identifies the versions and release needed to reproduce it, subject to provider determinism.
- English and Chinese design, human contracts, machine contract, configuration, source comments, and tests agree with runtime behavior.

## Related documents

- [System design](transnet.md)
- [Service behavior](product/learning-experience.md)
- [Service interface](interfaces/port.md)
- [MySQL interface](interfaces/mysql.md)
- [Qdrant interface](interfaces/qdrant.md)
- [Content publishing](guides/content-publishing.md)
- [Quality assurance](guides/quality-assurance.md)
