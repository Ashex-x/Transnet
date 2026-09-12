# MySQL adapter interface

中文：[MySQL 适配器接口](../../docs_cn/interfaces/mysql_cn.md)

This contract defines logical MySQL 8 operations for canonical basic cards, private bookmarked learning cards, bounded history, and scheduling state. JSON examples describe typed adapter values, not a network protocol or stored JSON schema.

Status: target contract; the current executable does not compose this adapter.

## Common contract

Every operation carries a request ID, deadline, and expected schema version. Reads pin a content or learning-card revision when consistency matters. Mutations require an idempotency key and optimistic revision where a concurrent learner action is possible.

Success returns `ok` with the requested value. Closed outcomes are `not_found`, `conflict`, `invalid`, `version_mismatch`, `unavailable`, and `timeout`. Adapter errors never expose SQL, credentials, learner content, ciphertext, or provider bodies.

## Basic cards

`resolve_basic_card` accepts a normalized form, source language, optional context hints, English dialect, and active release. It returns ranked sense-specific cards and explicit spelling or language alternatives when resolution is uncertain.

```json
{
  "card_id": "card_01J...",
  "sense_id": "sense_01J...",
  "canonical_form": "sweltering",
  "language": "en",
  "part_of_speech": "adjective",
  "translations": ["酷热的"],
  "definitions": ["uncomfortably hot"],
  "knowledge_root_ids": ["node_01J..."],
  "cefr": "B2",
  "domains": ["weather"],
  "content_release": "knowledge-2026-09"
}
```

Cards are concise and independently useful when Qdrant is unavailable. Detailed graph relationships, exploratory associations, long cultural notes, and private state do not belong in a basic card.

Publication operations stage, validate, activate, quarantine, and withdraw immutable card revisions. Activation references the compatible Qdrant node and edge releases. A release cannot activate until every referenced knowledge root exists and passes eligibility checks.

## Bookmarks and learning cards

`create_bookmark` accepts a learner ID, selected basic-card and sense IDs, a frozen generated card body, target skills, inferred generation context, knowledge release, generator, prompt, rubric, evaluator, and scheduler versions. The operation atomically creates the bookmark and first `LearningCard` revision.

Only an explicit bookmark may create durable learning state. A lookup, graph expansion, translation, writing sample, conversation, or pronunciation recording must not create a learning card implicitly.

```json
{
  "bookmark_id": "bookmark_01J...",
  "learning_card_id": "learning_card_01J...",
  "revision": 1,
  "source_card_id": "card_01J...",
  "selected_sense_id": "sense_01J...",
  "targets": ["recall", "collocation", "listening"],
  "state": "active",
  "due_at": "2026-09-13T02:00:00.000000Z"
}
```

`refresh_learning_card` creates a new frozen revision and carries forward only compatible review state. Source correction, quarantine, or withdrawal marks dependent cards for regeneration. It never silently rewrites a learner-owned revision.

`pause_bookmark`, `reprioritize_bookmark`, and `remove_bookmark` use optimistic revisions. Removal excludes the card from future scheduling while retaining only data required by the declared deletion and audit policy.

## History and strategy snapshot

`append_history_event` accepts only a canonical card or knowledge-node ID, selected sense, action, timestamp, and optional compact outcome, hint count, or misconception category. The adapter retains at most the newest 200 events from the previous 30 days per learner.

Raw queries, passages, writing, answers, conversations, explanations, and recordings are rejected from strategy history. `clear_history` removes all history influence immediately.

`load_strategy_snapshot` returns current bookmarks plus eligible compact history. It does not return a stored hidden profile. Level, domain, weak-skill, and priority estimates are reconstructed by the agent and are not written back as independent strategy evidence.

## Attempts, mastery, and scheduling

`record_attempt` atomically consumes one frozen exercise attempt, stores its typed result and evaluator confidence, updates only the demonstrated skill dimensions, and advances the versioned FSRS-style schedule when permitted. Replaying the same idempotency key returns the original result.

An uncertain or `needs_review` evaluation has no negative mastery effect. Recognition never updates recall, spelling, writing, listening, pronunciation, or cultural pragmatics without direct evidence. Objective correctness and hint use are primary scheduling inputs; response time is optional, bounded, and learner-relative.

Stored outcomes are compact. Raw free-form answers and recordings are excluded from long-term learning state unless a separate, explicit retention contract is introduced.

## Storage and privacy rules

Use `utf8mb4`, UTC timestamps with microsecond precision, opaque public IDs, indexed ownership foreign keys, and transactional bookmark and attempt mutations. Encrypt private learner fields with unique authenticated-encryption nonces and versioned keys stored outside MySQL.

Learner deletion covers bookmarks, learning-card revisions, mastery, schedules, and history. Canonical cards and release manifests remain shared public content and are not learner-owned.

## Related documents

- [System design](../transnet.md)
- [Qdrant interface](qdrant.md)
- [Island-port interface](port.md)
- [Content publishing](../guides/content-publishing.md)
