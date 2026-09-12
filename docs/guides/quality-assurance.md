# Quality assurance

中文：[质量保证](../../docs_cn/guides/quality-assurance_cn.md)

This guide covers the Island-port product around Transnet, including MySQL, Qdrant, privacy, persistence, feedback, practice, and worker behavior. The current Transnet executable implements only the applicable HTTP, provider-resilience, schema-validation, and redaction subset.

## Status

This guide defines the proposed evaluation and release process for Transnet learning features. The evaluation harness and datasets are not implemented at the branch point.

Quality is measured per enabled source language, English dialect, learner level, content category, and retrieval path. A global average cannot hide a weak language or safety-critical slice.

## Evaluation artifacts

Version and retain:

- Multilingual lookup benchmark.
- Lexical relation benchmark.
- Learning-card human-review rubric.
- Practice-item benchmark and grading fixtures.
- Prompt-injection and sensitive-content suite.
- API contract and failure-mode fixtures.
- Load and dependency-failure profiles.
- Published lexicon, Qdrant collection, prompt, model, ranker, rubric, evaluator, and scheduler versions.

Test data must be licensed for evaluation and must not contain private production queries unless a separate consent and de-identification process permits them.

## Multilingual lookup set

Each enabled language includes balanced cases for:

- High-frequency and long-tail words.
- Regular and irregular inflections.
- Multiple parts of speech.
- Polysemy and homographs.
- Idioms, phrasal verbs, and short expressions.
- False friends and learner transfer errors.
- Supported romanization and transliteration.
- Misspellings that should and should not be corrected.
- Ambiguous language detection and code switching.
- Context-dependent sense choice.
- Dialect, register, slang, dated, technical, and regional usage.
- Sensitive, taboo, hateful, sexual, and violent vocabulary used in legitimate educational contexts.
- Low-source-coverage and intentionally unsupported cases.

Bilingual reviewers judge source analysis and English equivalence. English teachers judge explanation clarity, learner level, examples, contrasts, and practice usefulness.

## Lookup metrics

- Language detection accuracy and explicit-abstention quality.
- Lemma, phrase, morphology, and part-of-speech coverage.
- Top-1 and top-k source-sense retrieval.
- Top-1 and top-k English-equivalent retrieval.
- Translation adequacy, naturalness, and context fit.
- Unsupported-claim and hallucination rate.
- Assertion-level citation correctness and source permission.
- Definition simplification meaning preservation.
- Usage, grammar, collocation, register, dialect, CEFR, frequency, pronunciation, etymology, and history correctness.
- Spelling-suggestion precision and harmful silent-correction count.
- Latency, cache hit, fallback, token, and cost distribution.

## Relationship metrics

Review endpoints, sense pairing, type, direction, scope, and evidence independently.

Include difficult cases:

- Near-synonyms that are not interchangeable.
- Antonyms that embed near each other.
- Hypernym/hyponym direction.
- Meronym/holonym direction.
- Modern derivation versus historical etymology.
- Collocations with asymmetric grammatical roles.
- Context-qualified degree scales.
- Homographs whose unrelated senses must remain disconnected.
- Associations that must not be promoted to stronger relation types.

Measure default-graph type-and-sense accuracy, candidate-retrieval recall, edge-scope completeness, ranking relevance, truncation quality, and feedback calibration.

## Learning-card rubric

Reviewers score each sense independently for:

1. Correct lemma, part of speech, and sense.
2. Plain-English definition appropriate to the requested level.
3. Accurate localized gloss in the explanation language.
4. Natural and sense-matched examples.
5. Correct grammar patterns and collocations.
6. Useful contrast with near-synonyms and confusables.
7. Source-qualified usage and dialect guidance.
8. Correct coverage state for missing, disputed, filtered, or degraded sections.
9. Complete assertion-level provenance.
10. Neutral and educational treatment of sensitive content.

A generated field fails if it introduces a factual detail absent from evidence, cites an invalid fragment, imitates restricted source text, or hides material uncertainty.

## Practice evaluation

Every objective item must have:

- One defensible intended answer or explicit accepted-answer set.
- Plausible but incorrect distractors.
- No answer leakage in prompt, ordering, formatting, or metadata.
- A frozen focus sense and every secondary target.
- Correct prompt language, English dialect, and level.
- A clear target skill.
- A concise evidence-backed correction.
- Safe and non-demeaning content.

Measure answerability, answer-key completeness, distractor quality, sense alignment, skill alignment, level fit, evaluator calibration, hint behavior, and duplicate-submission safety.

Scheduling simulations cover time zones, daylight-saving changes, clock skew, concurrent submissions, retries, scheduler migration, pauses, sense splits, and account deletion.

Product evaluation measures delayed recall and correct use in unseen contexts after 7 and 30 days. Multiple-choice accuracy alone is not a success measure.

## Security and privacy suite

Test:

- Cross-account reads and writes return the same `404` as absent private resources.
- Island-port credential lifecycle, expiry, and account deletion.
- CSRF, credentialed CORS, origin validation, and cookie configuration.
- SQL, JSON, Unicode, prompt, retrieved-content, and output-rendering injection.
- Raw query, context, answer, note, comment, token, and identity leakage into logs or traces.
- Shared cache separation for contexts, mature-content settings, and personal overlays.
- Capability entropy, hashing, expiry, replay, and absence from URLs.
- History opt-out, incognito lookup, expiration, clear all, export, and deletion.
- Source removal from cards, vectors, caches, exports, and generated manifests.

## Reliability suite

Inject failures for MySQL, vector retrieval, cache, model timeout, invalid model JSON, rate limits, stale content, worker delay, outbox replay, and partial graph expansion.

Expected fallbacks: an unavailable LLM returns a deterministic card or typed retryable error; unavailable vectors preserve exact, morphology, phrase, and full-text lookup; unavailable MySQL fails authoritative reads and writes closed and never substitutes vector payloads for canonical or private state; unavailable cache leaves canonical services running under bounded concurrency; invalid model output receives one repair attempt before a partial or deterministic fallback; delayed feedback workers preserve the personal projection and expose aggregate age; an uncertain grader returns `needs_review` with no mastery penalty; an oversized graph returns ranked truncation and an expansion cursor; MySQL/Qdrant drift triggers active-version filtering, alerting, reconciliation, and rebuild.

Verify deadline propagation, retry classification, circuit breaking, request coalescing, job leases, dead-job replay, and no duplicate billable or state-changing work.

## API contract suite

- Every example validates against the published JSON Schema.
- OpenAPI changes are checked for backward incompatibility.
- Unknown enum handling follows the documented client policy.
- Cursor, ETag, `If-Match`, idempotency, capability, and `Retry-After` behavior is deterministic.
- Context responses are private and not stored in shared snapshots.
- Async jobs return the same final learning-card schema as synchronous lookups.
- Graph edge endpoints are present in the node list.
- Feedback-enabled edges have a relation version; derived edges do not accept feedback.

## Initial release gates

- All public examples and contract fixtures pass schema validation.
- Every enabled language reaches at least 90% top-1 intended English-equivalent accuracy on the balanced reviewed set.
- Every enabled language reaches at least 97% top-3 intended-equivalent recall.
- Critical mistranslation remains below 1% in every reported language and sensitive-content slice.
- Confident language resolution or explicit ambiguity is correct in at least 98% of the per-language benchmark.
- False high-confidence language selection remains below 1%.
- Every displayed etymology, usage, pronunciation, CEFR, and frequency assertion has permitted provenance.
- A reviewed default-graph sample reaches at least 90% correct relation type and sense pairing before community influence is enabled.
- A reviewed practice sample is at least 95% clearly answerable and contains no answer leakage.
- No attempt advances mastery more than once under retries or concurrency.
- Feedback is idempotent, reversible, and pinned to an edge version.
- Island-port permission, privacy, source-removal, degraded-dependency, load, and injection suites pass.
- Application, prompt, content release, ranker, scheduler, and Qdrant collection rollback is tested.

These are initial product gates, not permanent ceilings. Each language can adopt stricter thresholds as the benchmark grows. Threshold changes are reviewed and versioned rather than adjusted after seeing a failing release.

## Production monitoring

Monitor:

- Request outcomes and stage latency by route and language pair.
- Retrieval path, candidate count, ambiguity, abstention, and coverage.
- Model schema validity, rejected unsupported assertions, repair, fallback, tokens, and cost.
- Graph size, truncation, relation distribution, feedback rate, and aggregate lag.
- Practice completion, correctness by skill, hints, lapses, and scheduler version.
- Source freshness, outbox age, dead jobs, Qdrant synchronization, and reconciliation mismatch.
- Island-port rate-limit and abuse outcomes without learner content.

Production data identifies regressions but does not automatically become training or evaluation data. Sampling private learner content requires separate consent, minimization, access controls, and retention.

## Release report

Every published content, prompt, model, ranker, evaluator, or scheduler release records:

- Changed inputs and versions.
- Benchmark and human-review results by slice.
- Known limitations and unsupported coverage.
- Privacy, license, and safety sign-off.
- Capacity and cost impact.
- Migration and rollback procedure.
- Monitoring thresholds and owners.

## Related documents

- [System design](../transnet.md)
- [Learning experience](../product/learning-experience.md)
- [Island-port interface](../interfaces/port.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
- [Content publishing](content-publishing.md)
- [Overall plan](../todo.md)
