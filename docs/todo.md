# Transnet delivery plan

中文：[Transnet 交付计划](../docs_cn/todo_cn.md)

This plan turns the authoritative [Transnet design](transnet.md) into ordered, verifiable delivery slices. It tracks repository state rather than aspiration: an item is checked only when its code, tests, and applicable contracts agree.

Status: active implementation plan. The checked-in runtime provides loopback translation and model-backed structured lookup. Canonical MySQL/Qdrant grounding, relationship-first page composition, and domain-aware expansion remain target capabilities.

## Delivery rules

- Keep the product focused on two experiences: translation for connected text and a relationship-centered translation-wiki page for a resolved lexical sense or domain concept.
- Preserve the stateless boundary. Request text, context, derived vectors, and intermediate analysis exist only for one bounded request and never become canonical or user data.
- Deliver vertical slices through types, ports, adapters, runtime composition, HTTP behavior, observability, tests, and bilingual documentation.
- Keep current and target behavior explicit. A target OpenAPI route or schema is not evidence that the checked-in executable enables it.
- Keep verified, inferred, and exploratory relationships distinct. Only the publication workflow may create or change canonical knowledge.
- Update English and Chinese documents together when product semantics, contracts, configuration, or release behavior changes.

## Verified baseline

- [x] Private loopback HTTP process, request bounds, exact-origin CORS, request IDs, health probes, graceful shutdown, and redacted structured tracing.
- [x] Length-based translation-provider routing with bounded resilience and aggregate operational metrics.
- [x] Model-backed structured lexical lookup in the default executable.
- [x] Library foundations and process-local test adapters for canonical lookup, sense detail, content releases, and bounded graph reads; these are not production storage composition.
- [x] Target human and machine contracts for translation, lookup, sense detail, and bounded graph reads.
- [ ] Production MySQL and Qdrant adapters, publication and paired activation, relationship-page composition, and complete target wire semantics.

## Milestone 0: enforce the focused stateless boundary

- [ ] Remove learner profiles, history, saved vocabulary, mastery, scheduling, exercises, practice sessions, coaching, graph layouts, private feedback, writing evaluation, and speech or pronunciation modules from reachable service APIs and reusable public domain types.
- [ ] Remove asynchronous lookup jobs and durable work paths that can retain queries, context, derived vectors, model output, or provider results; lookup stays within the request lifetime.
- [ ] Remove transitional user-oriented fields and reject cookies, end-user credentials, user or account identifiers, private-state fields, and unknown fields without echoing their values.
- [ ] Audit logs, traces, metrics, caches, queues, errors, debug formatting, and provider telemetry for request content, context, intermediate analysis, credentials, and caller identity.
- [ ] Synchronize code comments, tests, human contracts, and OpenAPI around translation plus relationship-centered lookup only.

Exit criteria: no reachable route or reusable public service API accepts product-owned state or exposes an out-of-scope learning module; no request-derived content can reach a durable port; boundary rejection and non-persistence tests pass.

## Milestone 1: freeze translation and intent routing

- [ ] Implement shared success and error envelopes, strict unknown-field rejection, language-tag validation, request metadata, and safe status mapping.
- [ ] Implement a versioned request-local normalizer with Unicode normalization, language-aware case folding, whitespace and punctuation handling, meaningful-symbol preservation, and bounded derived forms.
- [ ] Route a confident word, term, idiom, phrasal verb, or established phrase to lookup; route clauses, sentences, passages, and ambiguous short fragments to translation.
- [ ] Align `POST /translate` with the target response: preserve meaning, tone, terminology, register, paragraph structure, protected spans, and formatting; return at most two material one-sentence tips and at most one labeled alternative.
- [ ] Add request-local chunk planning and a disposable terminology ledger for long or difficult text without creating translation memory.
- [ ] Pin response metadata to the applicable schema, normalizer, model, prompt, and retrieval versions.

Exit criteria: contract and routing tests cover lexical units, technical symbols, phrases, ambiguous fragments, sentences, and passages; translation evaluation covers fidelity, naturalness, terminology, structure, register, and tip limits.

## Milestone 2: build canonical identity and MySQL basic cards

- [ ] Define migrations and production adapters for cards, senses, forms, aliases, definitions, translations, pronunciation, morphology, examples, usage notes, domains, evidence, immutable revisions, and release manifests.
- [ ] Make card, sense, concept-root, domain, evidence, and revision IDs stable under a documented versioned policy; never use a normalized query string as identity.
- [ ] Resolve exact canonical forms and aliases before bounded inflection, spelling correction, transliteration, and semantic candidates.
- [ ] Keep homographs, parts of speech, phrase-level meanings, and field-specific senses separate; preserve compositional versus phrase-level meaning.
- [ ] Return a concise, release-pinned basic card that remains useful without Qdrant or an LLM.
- [ ] Implement draft staging, collision and license checks, quarantine, correction by new revision, immutable retention, readiness, fixtures, and safe failure mapping.

Exit criteria: exact lookup and sense reads work from MySQL alone; ambiguous forms return ranked candidates or clarification; publication, collision, evidence, quarantine, and immutability tests pass.

## Milestone 3: publish the typed relationship model

- [ ] Define extensible node types for lexical senses, phrases, terms, concepts, phenomena, mechanisms, processes, equations, quantities, materials, instruments, methods, technologies, applications, standards, organizations, people, places, grammar patterns, collocations, idioms, metaphors, misconceptions, and domains.
- [ ] Define relation types and their direction, inverse, symmetry, transitivity, causality, applicable sense and domain, conditions, evidence requirements, confidence, provenance, and verification rules.
- [ ] Cover lexical naming, translation equivalence, taxonomy, part-whole, named intensity, contrast, syntax, collocation, morphology, suitability, cultural extension, terminology, domain membership, mechanism, causation, dependency, implementation, application, measurement, and standardization.
- [ ] Build deterministic immutable Qdrant node collections before edge collections, using named cross-lingual dense vectors and sparse lexical vectors from published content only.
- [ ] Validate endpoint existence, release compatibility, duplicate typed edges, direction, scope, conditions, evidence, confidence, verification state, and embedding metadata.
- [ ] Reconcile MySQL roots with Qdrant hashes and endpoint coverage, then atomically activate or roll back one compatible release trio.

Exit criteria: the projection rebuilds reproducibly from one canonical release; every verified relationship is named, scoped, evidence-backed, and release-pinned; partial or incompatible releases cannot activate.

## Milestone 4: implement bounded relationship retrieval

- [ ] Implement exact, sparse, dense, hybrid, endpoint, and reranked retrieval with release, state, language, dialect, region, period, domain, evidence, and verification filters applied before limits.
- [ ] Resolve one canonical root before expansion and return only relationships with an explicit useful path back to that root.
- [ ] Support purpose-ranked direct groups and short evidence-backed paths; require every intermediate step to have a named relationship and independently eligible evidence.
- [ ] Keep arbitrary-depth traversal, unrestricted neighbor dumps, shortest-path inference, and mutable graph transactions outside the service contract.
- [ ] Separate verified canonical edges from request-local inferred synthesis and exploratory vector or model proposals in storage, response shapes, ranking, and presentation.
- [ ] Return an explicit MySQL-only degraded card when Qdrant is unavailable, with no invented replacement relationships.

Exit criteria: retrieval is bounded, release-pinned, filter-safe, and useful for one selected root; adversarial tests prove that similarity never establishes translation, synonymy, hierarchy, causation, shared mechanism, or cultural meaning.

## Milestone 5: compose relationship-centered translation-wiki pages

- [ ] Add a bounded domain assessment after sense resolution with `general`, `domain_specific`, `mixed`, or `uncertain`, validated candidate domain IDs, and a concise reason.
- [ ] Resolve multilingual terms and aliases to shared concepts while preserving preferred term, translated term, alias, region, discipline, and usage status.
- [ ] Rank and group only useful supported content: meaning, terminology, taxonomy or degree, contrasts, valency, collocations, suitability, morphology, cultural extensions, mechanisms, neighboring phenomena, applications, measurements, standards, and usage conventions.
- [ ] Use progressive disclosure: begin with the basic card or concept summary, then high-value direct groups, optional named short paths, and a visibly separate exploratory section.
- [ ] Version the router, resolver, domain assessor, ranker, composer, prompts, schemas, and repair policy; validate structure, scope, evidence labels, concision, and uncertainty deterministically.
- [ ] Allow request-local generated examples and inferred explanations only when labeled; never persist them or present them as verified facts.
- [ ] Send structured missing-relationship proposals only to an offline review workflow; live lookup must not publish or display them as canonical edges.

Exit criteria: general vocabulary, compounds, ambiguous technical senses, and multilingual domain concepts produce concise pages whose groups and paths are relevant, correctly scoped, evidence-aware, and reproducible.

## Milestone 6: gate the focused product release

- [ ] Build versioned challenge sets for routing, translation, sense and concept resolution, domain assessment, relationship selection, path validity, omission, fabrication, terminology, register, culture, and prompt injection.
- [ ] Test relationship-family semantics, including taxonomy versus intensity, phrase versus component meaning, sense applicability, inverse direction, conditional validity, and verified/inferred/exploratory separation.
- [ ] Exercise model, MySQL, Qdrant, stale-release, partial-publication, invalid-output, rate-limit, timeout, and rollback failures with safe degradation and bounded repair.
- [ ] Prove non-persistence across MySQL, Qdrant, caches, logs, traces, metrics, queues, backups, provider telemetry, and derived vectors.
- [ ] Gate releases on formatting, linting, tests, rustdoc, contract checks, OpenAPI validation, publication reconciliation, rollback drills, and bilingual documentation checks.

Exit criteria: a user can translate connected text or deeply understand one selected lexical sense or domain concept through concise, accurate relationships without irrelevant graph expansion or unsupported model claims.

## Definition of done

- A sentence or passage receives translation first; a confidently resolved lexical unit receives a relationship-centered page rooted in one applicable sense or concept.
- MySQL basic cards remain useful without Qdrant, and unavailable graph retrieval degrades explicitly.
- Each displayed relationship is useful to the selected root and exposes its type, direction, applicable scope, evidence state, confidence, and provenance as appropriate.
- Verified, inferred, and exploratory content is never conflated, and similarity is never promoted into canonical fact.
- Content releases activate only as reconciled MySQL/Qdrant trios and roll back by immutable selection.
- No request content, context, intermediate analysis, caller identity, or user state is persisted or disclosed through observability and errors.
- English and Chinese design, behavior, interfaces, machine contract, guides, comments, and tests agree with the enabled runtime.

## Related documents

- [System design](transnet.md)
- [Service behavior](product/service-behavior.md)
- [Service interface](interfaces/port.md)
- [MySQL interface](interfaces/mysql.md)
- [Qdrant interface](interfaces/qdrant.md)
- [Content publishing](guides/content-publishing.md)
- [Quality assurance](guides/quality-assurance.md)
