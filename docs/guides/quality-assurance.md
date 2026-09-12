# Quality assurance

中文：[质量保证](../../docs_cn/guides/quality-assurance_cn.md)

This guide defines evaluation, release gates, and monitoring for the [Transnet design](../transnet.md). It is intended for content, model, application, privacy, and release owners.

Status: proposed; the complete harness and datasets are not implemented.

## Versioned evaluation artifacts

Every result identifies application, schema, prompt, model role, evaluator, rubric, normalization, MySQL card release, Qdrant node and edge releases, ranker, pronunciation analyzer, and scheduler versions as applicable. Datasets use licensed or consented content and contain no production credentials or unintended learner data.

Splits cover CEFR A1–C2, enabled source languages, English dialects, domains, registers, regional and historical scope, ambiguous forms, idioms, technical terminology, cultural situations, noisy audio, and accessibility settings. Regression sets remain isolated from prompt and content development.

## Intent and translation evaluation

Measure lexical-unit versus sentence-or-passage routing, including ambiguous short fragments. A routing error must choose the least intrusive useful experience and must never create learning state.

Translation evaluation covers meaning, completeness, tone, register, structure, names, terminology, numbers, negation, idioms, and dialect. Normal text must return only translation; tips are penalized unless they address a material ambiguity, idiom, register choice, or cultural context, and every response is checked for the two-tip limit.

## Basic-card and retrieval evaluation

Measure exact sense resolution, language detection, morphology, spelling suggestions, alias, transliteration, sparse technical-term recall, dense cross-lingual recall, reranking, evidence eligibility, and degraded MySQL-only behavior.

Relationship tests report precision by type and scope. They verify endpoint existence, release compatibility, direction, sense, evidence, region, period, domain, and confidence. Adversarial cases ensure intensity is not taxonomy and embedding proximity is not promoted to translation, synonymy, hierarchy, causation, shared mechanism, or cultural fact.

Bounded exploration tests expand one selected node at a time, separate verified and exploratory results, and reject plausible but unsupported multi-hop narratives.

## Learning-card and practice evaluation

Only explicit bookmark tests may create a durable card. Lookup, translation, graph expansion, writing, conversation, and speech tests assert that no target appears implicitly. Refresh tests require immutable traceable revisions, compatible state migration, and regeneration after source correction or withdrawal.

Practice datasets cover recognition, recall, spelling, morphology, collocation, grammar, composition, writing, register, cultural pragmatics, listening, and pronunciation independently. A correct result in one dimension must not update another. Exercises are checked for answerability, leakage, accepted variants, hint progression, focused retry, and later transfer in unseen contexts.

Free production evaluation measures task fulfillment, meaning preservation, grammar, lexical precision, naturalness, organization, tone, and cultural suitability. Valid dialect and style variation must survive. Feedback is penalized for erasing voice, inventing a uniquely correct rewrite, or giving more than two unrelated high-priority corrections.

## Scheduling and strategy evaluation

Replay tests pin FSRS-style scheduler parameters and verify difficulty, stability, retrievability, due ordering, hint effects, accessibility handling, and rollback. Uncertain evaluation cannot reduce mastery. New bookmarks cannot crowd out fragile due cards, and an unbookmarked neighbor cannot enter the queue automatically.

Strategy tests use only current bookmarks and eligible compact history. They verify the 200-event and 30-day bounds, bookmark weighting, time decay, low-confidence neutral fallback, and immediate effect of history clearing. Presentation preferences must not change inferred level, domain, weakness, or priority.

## Speech evaluation

Reference-speech tests cover intelligibility, dialect match, generated-voice labeling, streaming behavior, syllables, stress, rhythm, reductions, linking, and natural slowed output.

Pronunciation sets vary microphones, noise, clipping, silence, accents, speech rates, target phrases, and non-target speech. Feedback must trace to acoustic and alignment evidence, select at most two intelligibility targets, and return uncertainty rather than a score when evidence is inadequate.

## Cultural safety, grounding, and injection

Cultural scenarios vary relationship, hierarchy, distance, setting, medium, dialect, and region. Evaluation rejects stereotypes, universal group claims, unsupported etiquette, and unsafe confidence while rewarding scoped alternatives and acknowledged variation.

Treat learner text, retrieved documents, evidence, and model output as untrusted data. Injection suites attempt to replace system instructions, exfiltrate credentials, bypass release filters, fabricate evidence, store incidental content, and alter rubrics or mastery without authority.

## Privacy and accessibility

Privacy tests prove that raw queries, passages, writing, answers, conversations, explanations, and recordings do not enter strategy history or Qdrant. They cover bookmark removal, history clearing, account export and deletion, recording defaults, logs, traces, caches, backups, idempotency, and cross-learner isolation.

Accessibility tests cover keyboard-only graph exploration, list and tree parity, screen readers, reduced motion, non-color-only meaning, captions and text alternatives, audio controls, input time independence, and disabled response-time scheduling signals.

## Reliability and release gates

Inject MySQL, Qdrant node, Qdrant edge, model, TTS, speech-recognition, alignment, and scheduler failures; invalid structured output; rate limits; stale releases; and partial publication. Expected behavior includes one bounded schema repair, deterministic fallback, explicit uncertainty, no partial release activation, no unsupported relationship generation, and no duplicate attempt or bookmark mutation.

A release passes only when:

- translation and intent thresholds pass for every enabled language and dialect;
- card, retrieval, relationship, evidence, degraded-mode, and release-reconciliation thresholds pass;
- bookmark-only durability, independent mastery, scheduling, feedback, transfer, writing, cultural, speech, privacy, accessibility, and injection suites pass;
- regression deltas are explained and approved by named owners; and
- the application, content trio, prompts, rubrics, models, analyzers, and scheduler can be rolled back independently where their contracts allow.

Production monitoring records aggregate outcomes and stage latency, routing mix, retrieval path, coverage, schema validity, uncertainty, graph truncation, review due health, pronunciation assessability, release drift, and bounded dependency failures. Logs and metrics contain no learner content or raw provider bodies. Production data never becomes training data automatically.

## Related documents

- [System design](../transnet.md)
- [Learning experience](../product/learning-experience.md)
- [Content publishing](content-publishing.md)
- [Island-port interface](../interfaces/port.md)
