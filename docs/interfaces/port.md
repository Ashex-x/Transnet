# Island-port interface

中文：[Island-port 接口](../../docs_cn/interfaces/port_cn.md)

This document defines the trusted internal HTTP boundary through which Island-port exposes the Transnet learning agent. The [system design](../transnet.md) owns product semantics.

Status: health, translation, structured lookup, and feature-gated canonical graph reads exist in the current runtime. Bookmark-driven learning, writing, speech, and review routes below are proposed and must not be advertised as implemented until added to the OpenAPI document and runtime together.

## Shared wire rules

Transnet binds a loopback address and does not terminate public TLS. Island-port authenticates the end user and authorizes learner-owned operations before calling it. Internal deployment authentication is required outside a single-host loopback deployment; Transnet never accepts end-user cookies or bearer tokens.

Requests and responses use JSON unless a speech route explicitly negotiates audio. `X-Request-Id` is returned on every response. Timestamps are UTC RFC 3339 with microsecond precision. IDs are opaque URL-safe strings and clients must not infer type or order from them.

Learner-owned routes require `X-Learner-Id`, an opaque authorized identifier of 1–128 URL-safe characters. Mutations require `Idempotency-Key` with 16–128 URL-safe characters. Reuse with different normalized content returns `409 idempotency_conflict`.

Standard success responses contain `data` and `meta`. Standard errors contain a stable `code`, safe `message`, `request_id`, `retryable`, and optional field details. Errors never echo learner content, credentials, provider bodies, or storage internals.

```json
{
  "error": {
    "code": "invalid_request",
    "message": "The request is invalid.",
    "request_id": "req_01J...",
    "retryable": false
  }
}
```

Common statuses are `400` invalid input, `401` failed internal authentication, `403` learner mismatch, `404` unknown eligible resource, `409` idempotency or revision conflict, `413` body too large, `422` unassessable input, `429` bounded capacity, `502` invalid provider result, `503` dependency unavailable, and `504` deadline exceeded.

## GET /health

Authentication: internal deployment policy. Returns process health and no dependency or learner details.

## GET /livez

Authentication: internal deployment policy. Returns success while the process event loop is responsive.

## GET /readyz

Authentication: internal deployment policy. Returns `200` only when dependencies required by enabled routes are ready; optional degraded capabilities appear in metadata.

## POST /translate

Authentication: internal deployment policy; no learner identity is required. The request contains text, optional source language, target language, dialect, and register preferences. Text is bounded by configured body and provider limits.

```json
{
  "text": "That plan is still up in the air.",
  "source_lang": "en",
  "target_lang": "zh-CN"
}
```

The response returns the translation as the primary result and zero to two one-sentence tips only for meaningful ambiguity, idiom, consequential register, or cultural context. It does not create history, a bookmark, or learning state.

## POST /v1/lookups

Authentication: internal deployment policy; `X-Learner-Id` is optional and may affect only ephemeral ranking. The request contains a word or lexical phrase, source language, explanation language, English dialect, and optional bounded context.

The response contains query analysis, a concise basic card, selected sense, relevant translation-wiki sections, verified relationships, separate exploratory associations, evidence and release metadata, and bounded expansion links. It is not a learning card and creates no durable target.

When Qdrant is unavailable, a resolved MySQL basic card may return with explicit degraded metadata. The route never invents related knowledge. Ambiguous non-lexical fragments may return a translation-mode result.

## GET /v1/knowledge/nodes/{node_id}/neighbors

Authentication: internal deployment policy. Query parameters select relation families, direction, language, domain, release, and a bounded limit. The response pins node and edge releases and separates verified edges from exploratory associations. A cursor expands one selected node only.

## POST /v1/bookmarks

Status: proposed. Authentication: required `X-Learner-Id`. The idempotent request names a selected basic-card and sense revision. The response returns the bookmark and first frozen learning-card revision.

```json
{
  "basic_card_id": "card_01J...",
  "sense_id": "sense_01J...",
  "content_release": "knowledge-2026-09"
}
```

Errors include `404` for an ineligible card or sense and `409` for a stale release or incompatible existing bookmark.

## PATCH /v1/bookmarks/{bookmark_id}

Status: proposed. Authentication: required `X-Learner-Id`. The request may pause, resume, reprioritize, or explicitly refresh a card and includes the expected revision. Refresh creates a new traceable card revision; it never silently rewrites the current revision.

## DELETE /v1/bookmarks/{bookmark_id}

Status: proposed. Authentication: required `X-Learner-Id`. The idempotent operation stops future scheduling and applies the documented retention policy. `404` does not reveal another learner's resource.

## GET /v1/learning-cards

Status: proposed. Authentication: required `X-Learner-Id`. Returns paginated active, paused, due, or regeneration-required cards. Each item exposes independent mastery dimensions, due time, source and generation versions, and the reason for its current priority.

## POST /v1/practice/sessions

Status: proposed. Authentication: required `X-Learner-Id`. Creates a session from due bookmarked cards, compact misconceptions, and directly relevant transfer tasks. It does not insert unbookmarked graph neighbors into the study queue.

## POST /v1/practice/sessions/{session_id}/attempts

Status: proposed. Authentication: required `X-Learner-Id`. The idempotent request submits one frozen exercise answer and its exercise revision. The result contains `correct`, `needs_revision`, or `needs_review`, observations, bounded feedback, confidence, mastery effects, and the next action.

Only sufficiently confident evidence changes the demonstrated skill. `needs_review` has no negative mastery effect. Pronunciation results require acoustic and alignment evidence; unusable audio returns `422 unassessable_audio` without a score.

## POST /v1/writing/evaluations

Status: proposed. Authentication: required `X-Learner-Id` only when the task is tied to a bookmarked card. The request states audience, purpose, medium, desired register, constraints, and learner text. The response preserves intended meaning and voice and returns a minimal correction, optional natural alternative, at most two prioritized explanations, and a retry task.

Raw writing is not added to strategy history or durable state. A standalone evaluation creates no bookmark.

## POST /v1/speech/reference

Status: proposed. Authentication: internal deployment policy. Generates clearly labeled reference speech for bounded text, dialect, voice, rate, and exercise purpose. Streaming may be used when supported. Slower output preserves natural stress and phrasing.

## POST /v1/pronunciation/evaluations

Status: proposed. Authentication: required `X-Learner-Id` only for bookmarked practice. Accepts bounded audio plus expected language and optional target phrase. The response reports recording quality, alignment confidence, at most two intelligibility targets, evidence-based cues, and a retry prompt.

Audio is not retained by default and never enters strategy history. Only a separately authorized compact derived outcome may update the applicable bookmarked skill.

## GET /v1/history

Status: proposed. Authentication: required `X-Learner-Id`. Returns at most 200 compact canonical events from the previous 30 days. It never returns raw queries, passages, writing, answers, conversations, explanations, or recordings.

## DELETE /v1/history

Status: proposed. Authentication: required `X-Learner-Id`. Immediately removes history from future strategy reconstruction. It does not delete bookmarks; learner-wide deletion is a separate Island-port account workflow.

## Related documents

- [System design](../transnet.md)
- [Learning experience](../product/learning-experience.md)
- [MySQL interface](mysql.md)
- [Qdrant interface](qdrant.md)
- [Current OpenAPI subset](../reference/transnet-openapi.json)
