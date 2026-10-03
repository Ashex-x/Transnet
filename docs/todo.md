# Transnet delivery plan

中文：[Transnet 交付计划](../docs_cn/todo_cn.md)

This plan turns the authoritative [Transnet design](transnet.md) into ordered, verifiable delivery slices. It tracks repository state rather than aspiration: an item is checked only when its code, tests, and applicable contracts agree.

Status: repository implementation complete with named external acceptance blockers. The checked-in runtime serves the target API on one owned Unix socket, uses one provider-neutral Gemma4-27B generation boundary, and optionally composes canonical and knowledge reads. Transnet-owned publication projection/client and relationship-page contracts are executable; production relationship-page authority composition remains disabled. Island-port server behavior, production MySQL/Qdrant execution, activation/rollback, retention, provider attestation, and production acceptance remain external or unverified.

## Delivery rules

- Keep the product focused on two experiences: translation for connected text and a relationship-centered translation-wiki page for a resolved lexical sense or domain concept.
- Preserve the stateless boundary. Live request text, translation history, derived vectors, and intermediate analysis exist only for one bounded request and never become canonical or user data; separate reviewed publisher input may become canonical content.
- Deliver vertical slices through types, ports, adapters, runtime composition, HTTP behavior, observability, tests, and bilingual documentation.
- Keep current and target behavior explicit. A documented target route or schema is not evidence that the checked-in executable enables it.
- Keep verified, inferred, and exploratory relationships distinct. Only the publication workflow may create or change canonical knowledge.
- Update English and Chinese documents together when product semantics, contracts, configuration, or release behavior changes.

## Verified baseline

- [x] UDS-only HTTP process, request bounds, request IDs, health probes, graceful shutdown, and redacted structured tracing.
- [x] One neutral fast/reasoning generation provider with bounded resilience and operational metrics.
- [x] Target translation plus opt-in canonical and knowledge route composition.
- [x] Strict canonical-data and retrieval-data clients, the BasicCard mapper, release-pinned root/view/path services, offline publication fakes, and contract tests; production authorities remain external.
- [x] Target human contracts for translation, lookup, sense detail, and bounded graph reads.
- [x] Transnet-owned publication projection/client, relationship-page composition seam, and complete target wire semantics.
- [ ] Production Island-port/MySQL/Qdrant execution, paired activation, production relationship-page authority, and real acceptance.

## Milestone 0: enforce the focused stateless boundary

- [x] Remove learner profiles, history, saved vocabulary, mastery, scheduling, exercises, practice sessions, coaching, graph layouts, private feedback, writing evaluation, and speech or pronunciation modules from reachable service APIs and reusable public domain types.
- [x] Remove asynchronous lookup jobs and durable work paths that can retain queries, context, derived vectors, model output, or provider results; lookup stays within the request lifetime.
- [x] Remove transitional user-oriented fields and reject cookies, end-user credentials, user or account identifiers, private-state fields, and unknown fields without echoing their values.
- [ ] Audit logs, traces, metrics, caches, queues, errors, debug formatting, and provider telemetry for request content, context, intermediate analysis, credentials, and caller identity.
- [x] Synchronize code comments, tests, and human contracts around translation plus relationship-centered lookup only.

Exit criteria: no reachable route or reusable public service API accepts product-owned state or exposes an out-of-scope learning module; no request-derived content can reach a durable port; boundary rejection and non-persistence tests pass.

Repository-side admission, request/result Debug redaction, closed telemetry, and removal of query-derived caches and queues are implemented. External model-server telemetry and retention still require deployment verification; this audit item remains open. UDS and target translation/history contracts remain milestone 1 and transport work.

## Milestone 1: freeze translation and intent routing

- [x] Replace the transitional length router with one Gemma4-27B VLM exposing internal fast and reasoning profiles.
- [ ] Compose the request-local nomination embedding port in production where semantic nomination is enabled; publication embedding remains island-port-owned.
- [x] Keep ordinary and chunked translation on the fast profile, permit at most one closed-policy reasoning escalation per request, and prove hidden reasoning is never returned or logged.
- [x] Implement shared success and error envelopes, strict unknown-field rejection, language-tag validation, request metadata, and safe status mapping.
- [x] Implement the simple text, source-language, target-language, response-level, and optional minimal-history request plus the discriminated word, phrase, and passage `TranslationResult` response.
- [x] Extend the same entry point with bounded professional segment and image-region inputs; keep file parsing, OCR policy, and durable document ownership in island-port.
- [x] Add request-local purpose, audience, register, protected-range, terminology, annotation, alternative-count, and freshness guidance without creating a profile or translation memory.
- [x] Publish a no-store capabilities response so island-port can discover currently implemented input kinds, limits, retrieval policy, and generation-profile availability without trial requests.
- [x] Implement a versioned request-local normalizer with Unicode normalization, language-aware case folding, whitespace and punctuation handling, meaningful-symbol preservation, and bounded derived forms.
- [x] Behind one translation entry point, route a confident word, term, idiom, phrasal verb, or established phrase to lexical composition and route clauses, sentences, passages, and ambiguous short fragments to connected-text translation.
- [x] Align `POST /api/v1/translations` with the target response while preserving meaning, tone, terminology, register, paragraph structure, protected spans, and formatting.
- [x] Add request-local chunk planning and a disposable terminology ledger for long or difficult text without creating translation memory.
- [x] Accept island-port-controlled translation-turn history with no independent item-count cap, use it only for current linguistic context, and prove it is never logged, cached, embedded, queued, or persisted.
- [x] Build one release-pinned superset aggregate and deterministic `brief`, `standard`, and `full` projectors; preserve materially different meanings even at lower levels.
- [x] Pin response metadata to the applicable schema, normalizer, model, prompt, and retrieval versions.

Exit criteria: contract and routing tests cover lexical units, technical symbols, phrases, ambiguous fragments, sentences, and passages; translation evaluation covers fidelity, naturalness, terminology, structure, and register.

Passage tips and labeled alternatives are deferred beyond milestone 1. Their later owning milestone must first add explicit application/domain results and evaluation limits; the milestone 1 transport must not fabricate them.

## Milestone 2: build canonical identity and MySQL basic cards

- [ ] Define migrations and production adapters for cards, senses, forms, aliases, definitions, translations, pronunciation, morphology, examples, usage notes, domains, evidence, immutable revisions, and release manifests.
- [x] Implement canonical translation identities, immutable revisions, exact fingerprint resolution with source verification, publication staging, rights and evidence checks, and release membership for reviewed words, phrases, and bounded passages.
- [x] Make card, sense, concept-root, domain, evidence, and revision IDs stable under a documented versioned policy; never use a normalized query string as identity.
- [x] Resolve exact canonical forms and aliases before bounded inflection, spelling correction, transliteration, and semantic candidates.
- [x] Keep homographs, parts of speech, phrase-level meanings, and field-specific senses separate; preserve compositional versus phrase-level meaning.
- [x] Return a concise, release-pinned basic card that remains useful without Qdrant or an LLM.
- [ ] Implement draft staging, collision and license checks, quarantine, correction by new revision, immutable retention, readiness, fixtures, and safe failure mapping.

Exit criteria: exact lookup and sense reads work from MySQL alone; ambiguous forms return ranked candidates or clarification; publication, collision, evidence, quarantine, and immutability tests pass.

## Milestone 3: publish the typed relationship model

- [x] Define extensible node types for lexical senses, phrases, terms, concepts, phenomena, mechanisms, processes, equations, quantities, materials, instruments, methods, technologies, applications, standards, organizations, people, places, grammar patterns, collocations, idioms, metaphors, misconceptions, and domains.
- [x] Define relation types and their direction, inverse, symmetry, transitivity, causality, applicable sense and domain, conditions, evidence requirements, confidence, provenance, and verification rules.
- [x] Cover lexical naming, translation equivalence, taxonomy, part-whole, named intensity, contrast, syntax, collocation, morphology, suitability, cultural extension, terminology, domain membership, mechanism, causation, dependency, implementation, application, measurement, and standardization.
- [x] Build deterministic immutable node and edge projection artifacts using named cross-lingual dense inputs and sparse lexical inputs from published content only; island-port remains responsible for collection execution.
- [x] Validate endpoint existence, release compatibility, duplicate typed edges, direction, scope, conditions, evidence, confidence, verification state, and embedding metadata.
- [ ] Reconcile MySQL roots with Qdrant hashes and endpoint coverage, then atomically activate or roll back one compatible release trio.

Exit criteria: the projection rebuilds reproducibly from one canonical release; every verified relationship is named, scoped, evidence-backed, and release-pinned; partial or incompatible releases cannot activate.

## Milestone 4: implement bounded relationship retrieval

- [x] Implement exact, sparse, dense, hybrid, endpoint, and reranked retrieval with release, state, language, dialect, region, period, domain, evidence, and verification filters applied before limits.
- [x] Resolve one canonical root before expansion and return only relationships with an explicit useful path back to that root.
- [x] Support purpose-ranked direct groups and short evidence-backed paths; require every intermediate step to have a named relationship and independently eligible evidence.
- [x] Keep arbitrary-depth traversal, unrestricted neighbor dumps, shortest-path inference, and mutable graph transactions outside the service contract.
- [x] Separate verified canonical edges from request-local inferred synthesis and exploratory vector or model proposals in storage, response shapes, ranking, and presentation.
- [x] Return an explicit MySQL-only degraded card when Qdrant is unavailable, with no invented replacement relationships.
- [x] Replace raw public graph filters with bounded `knowledge/views` lenses and evidence-backed `knowledge/paths` so one assertion graph can support tree-like learning, terminology, mechanism, and comparison views.

Exit criteria: retrieval is bounded, release-pinned, filter-safe, and useful for one selected root; adversarial tests prove that similarity never establishes translation, synonymy, hierarchy, causation, shared mechanism, or cultural meaning.

## Milestone 5: compose relationship-centered translation-wiki pages

Ownership is frozen: the composed page is an optional field of lexical word or established-phrase `/api/v1/translations` results, never a new public route. The BasicCard, knowledge-view, and knowledge-path endpoints remain diagnostic reads.

- [x] Add a bounded domain assessment after sense resolution with the closed outcomes `existing`, `proposed_new`, `general`, or `uncertain`, validated canonical domain IDs, and a concise reason.
- [x] Retrieve the existing-domain inventory and RAG coverage before assessment; return `proposed_new` only when the available catalog succeeds and no supplied scope fits, with no live creation endpoint.
- [x] Define, project through the outbound publication contract, and hydrate atomic basic facts, domain knowledge profiles, and first-class semantic scales; allow LLM bootstrap only through quarantined offline candidates.
- [x] Add explicitly requested labeled alternatives, capped at two per unit, through application/domain result types and deterministic usefulness evaluation rather than transport-only fabrication.
- [x] Resolve multilingual terms and aliases to shared concepts while preserving preferred term, translated term, alias, region, discipline, and usage status.
- [x] Rank and group only useful supported content: meaning, terminology, taxonomy or degree, contrasts, valency, collocations, suitability, morphology, cultural extensions, mechanisms, neighboring phenomena, applications, measurements, standards, and usage conventions.
- [x] Use progressive disclosure: begin with the basic card or concept summary, then high-value direct groups, optional named short paths, and a visibly separate exploratory section.
- [x] Version the router, resolver, domain assessor, ranker, composer, prompts, schemas, and repair policy; validate structure, scope, evidence labels, concision, and uncertainty deterministically.
- [x] Allow request-local generated examples and inferred explanations only when labeled; never persist them or present them as verified facts.
- [x] Send structured missing-relationship proposals only to an offline review workflow; live lookup must not publish or display them as canonical edges.
- [x] Publish a versioned relation registry and n-ary assertion participants in canonical data, while deriving only explicitly declared binary traversal projections for retrieval.
- [x] Permit at most one explicitly requested live-search round with bounded result and fetch counts, SSRF-safe fetches, citations, and no automatic publication or persistence.

Exit criteria: general vocabulary, compounds, ambiguous technical senses, and multilingual domain concepts produce concise pages whose groups and paths are relevant, correctly scoped, evidence-aware, and reproducible.

## Milestone 6: gate the focused product release

- [x] Build versioned challenge sets for routing, translation, sense and concept resolution, domain assessment, relationship selection, path validity, omission, fabrication, terminology, register, culture, and prompt injection.
- [x] Test relationship-family semantics, including taxonomy versus intensity, phrase versus component meaning, sense applicability, inverse direction, conditional validity, and verified/inferred/exploratory separation.
- [x] Exercise model, canonical, retrieval, stale-release, partial-publication, invalid-output, rate-limit, timeout, cancellation, and rollback failures with safe degradation and bounded repair through repository fakes.
- [ ] Prove non-persistence across MySQL, Qdrant, caches, logs, traces, metrics, queues, backups, provider telemetry, and derived vectors.
- [x] Implement the versioned whole-system observability contract with content-free NDJSON events, distributed trace continuity, closed low-cardinality metrics, bounded non-blocking export, drop counters, and separate append-only publication audits.
- [x] Gate repository changes on formatting, linting, tests, rustdoc, contract checks, publication reconciliation and rollback fakes, and bilingual documentation checks.

Exit criteria: a user can translate connected text or deeply understand one selected lexical sense or domain concept through concise, accurate relationships without irrelevant graph expansion or unsupported model claims.

Repository-side synthetic-sentinel tests cover Transnet-owned HTTP tracing, diagnostics, closed metrics, provider-reasoning Debug output, private-state admission, and the absence of request-local material from the enumerated repository-owned diagnostic and telemetry surfaces. The item remains open because provider telemetry, collectors, infrastructure retention, backups, external MySQL/Qdrant behavior, and production derived-vector retention require deployment acceptance.

## Definition of done

- A sentence or passage receives translation first; a confidently resolved lexical unit receives a relationship-centered page rooted in one applicable sense or concept.
- MySQL basic cards remain useful without Qdrant, and unavailable graph retrieval degrades explicitly.
- Each displayed relationship is useful to the selected root and exposes its type, direction, applicable scope, evidence state, confidence, and provenance as appropriate.
- Verified, inferred, and exploratory content is never conflated, and similarity is never promoted into canonical fact.
- Content releases activate only as reconciled MySQL/Qdrant trios and roll back by immutable selection.
- No request content, context, intermediate analysis, caller identity, or user state is persisted or disclosed through observability and errors.
- English and Chinese design, behavior, interfaces, guides, comments, and tests agree with the enabled runtime.

## Related documents

- [System design](transnet.md)
- [Service behavior](product/service-behavior.md)
- [Canonical-data endpoints](interfaces/canonical-data.md)
- [Retrieval-data endpoints](interfaces/retrieval-data.md)
- [Content publishing](guides/content-publishing.md)
- [Quality assurance](guides/quality-assurance.md)
