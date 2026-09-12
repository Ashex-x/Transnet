# Transnet delivery plan

中文：[Transnet 交付计划](../docs_cn/todo_cn.md)

This plan sequences implementation of the [English-learning agent design](transnet.md). It records target work, not implemented behavior.

## Current baseline

- [x] Loopback HTTP process, health probes, request bounds, request IDs, graceful shutdown, and redacted tracing.
- [x] Translation provider routing, resilience, and structured lexical-lookup foundations.
- [ ] Product routes, storage adapters, knowledge releases, bookmark-driven learning, evaluation, scheduling, writing, and speech aligned with the target design.

## Phase 1: intent and response semantics

- [ ] Replace character-count product behavior with typed lexical-unit versus sentence-or-passage routing while retaining provider size limits as operational constraints.
- [ ] Return translation-first responses with no more than two justified tips.
- [ ] Return translation-wiki results separately from learner-owned learning cards.
- [ ] Define typed role schemas for translation, lexical analysis, planning, generation, evaluation, writing, pragmatics, speech recognition, pronunciation analysis, and TTS.
- [ ] Add bounded repair and deterministic fallback for invalid structured model output.

## Phase 2: canonical cards and knowledge graph

- [ ] Implement sense-specific MySQL basic cards that remain useful without vector retrieval.
- [ ] Build immutable Qdrant node and edge collections with dense and sparse named vectors.
- [ ] Add deterministic IDs, endpoint validation, evidence scope, release pinning, reconciliation, atomic logical activation, quarantine, and rollback.
- [ ] Implement exact, hybrid, endpoint, and bounded one-node expansion retrieval.
- [ ] Keep verified typed edges separate from exploratory vector associations in storage and presentation.

## Phase 3: bookmarks and continuous learning

- [ ] Create a complete frozen learning-card revision only after an explicit bookmark.
- [ ] Support inspect, pause, reprioritize, refresh, remove, and regeneration-required states.
- [ ] Store at most 200 compact canonical events from the previous 30 days and implement immediate history clearing.
- [ ] Reconstruct approximate level, domain, weak skills, and priority only from current bookmarks and eligible history, with confidence and decay.
- [ ] Track mastery independently for recognition, recall, spelling, morphology, collocation, grammar, composition, writing, register, culture, listening, and pronunciation when applicable.
- [ ] Implement a versioned FSRS-style scheduler using objective correctness and hints as primary signals.

## Phase 4: practice, writing, culture, and speech

- [ ] Add controlled recognition, recall, spelling, dictation, morphology, collocation, grammar, composition, and transfer exercises.
- [ ] Add writing evaluation that preserves meaning and voice and prioritizes no more than two actionable corrections.
- [ ] Add context-scoped communication coaching with dialect, region, relationship, medium, and uncertainty.
- [ ] Add provider-independent reference TTS with generated-voice labeling and natural slowed speech.
- [ ] Add recording-quality checks, speech recognition, phoneme alignment, acoustic evidence, uncertainty, and focused pronunciation retry.
- [ ] Ensure standalone writing, translation, lookup, and speech never create durable learning targets.

## Phase 5: privacy, accessibility, and release quality

- [ ] Keep raw queries, passages, writing, answers, conversations, explanations, and recordings out of strategy history and shared Qdrant collections.
- [ ] Add learner controls for bookmarks, learning-card revisions, schedules, history clearing, recording retention, export, and deletion.
- [ ] Provide keyboard-accessible list or tree alternatives to graph views, reduced motion, text labels, and non-color-only meaning.
- [ ] Version prompts, schemas, rubrics, releases, models, normalization, analyzers, and scheduler parameters.
- [ ] Gate releases on retrieval, pedagogy, transfer, dialect, cultural safety, pronunciation assessability, privacy, injection, degraded dependency, and rollback suites.

## Definition of done

- A normal sentence receives a translation-first response; a lexical unit receives a sense-specific translation-wiki page.
- A bookmark is the only event that creates a durable learning card or scheduled target.
- MySQL basic cards degrade safely without Qdrant, and graph results never promote similarity into unsupported fact.
- Only directly demonstrated skills advance, and uncertain evaluation does not reduce mastery.
- Personal strategy can be explained entirely from current bookmarks and bounded history and is reset by the documented controls.
- Writing and cultural coaching preserve learner intent, and pronunciation claims are grounded in acoustic and alignment evidence.
- English and Chinese docs, human contracts, machine contracts, source comments, and tests change with the behavior they describe.

## Related documents

- [System design](transnet.md)
- [Learning experience](product/learning-experience.md)
- [Island-port interface](interfaces/port.md)
- [Quality assurance](guides/quality-assurance.md)
