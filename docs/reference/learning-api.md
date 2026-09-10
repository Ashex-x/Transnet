# Learning HTTP API

## Status and compatibility

`POST /v1/lookups` has an implemented model-only basic-core slice. All other endpoints in this document remain proposed. The implemented `GET /health` and `POST /translate` contracts remain in [Transnet HTTP API](transnet-api.md).

The implemented lookup does not yet have canonical lexical content, retrieval, persistence, or authentication. It returns synchronous anonymous results with `Cache-Control: no-store`; identifies every generated assertion; uses null canonical sense and relation IDs; exposes no evidence IDs; and reports `evidence_backed: false`. The richer evidence-backed shape below is the target contract that will replace these provisional gaps without inventing canonical data.

The learning API uses JSON, opaque public IDs, UTC RFC 3339 timestamps, OpenAPI, and versioned JSON Schemas. Removing fields or changing enum meaning requires a new API version.

## Common behavior

### Authentication

Anonymous clients can perform rate-limited lookups and graph reads without persistence. Authentication is required for history, saved senses, feedback, layouts, practice, progress, exports, and profile changes.

The Rust API acts as the browser-facing OIDC relying party. It uses authorization code with PKCE, state, and nonce, then issues short-lived service sessions with rotating refresh tokens in secure HTTP-only cookies. [System design](../transnet.md) defines the trust boundary.

### Idempotency

Authenticated creation, feedback, attempt, and privacy mutations require `Idempotency-Key` with 16 through 128 printable ASCII characters.

The server retains a scoped HMAC and normalized request hash for at least 24 hours:

- The same key and request returns the stored response.
- The same key with a different request returns `409 idempotency_conflict`.
- A concurrent incomplete request returns `409 idempotency_in_progress` with `Retry-After`.

### Pagination and caching

Mutable lists use opaque cursors. Graph and content reads expose `ETag`. Saved-layout replacements require `If-Match`.

Context-bearing and personalized responses use `Cache-Control: private`; context-bearing lookup envelopes use `no-store`. Shared caches contain only context-free canonical content and never lookup IDs or private fields.

### Version fields

Content responses identify the applicable schema, lexicon release, vector collection, ranking version, and generation versions. Clients must not compare numeric ranks across ranking versions.

## Endpoint inventory

| Method and path | Authentication | Purpose |
| --- | --- | --- |
| `GET /livez` | None | Process liveness |
| `GET /readyz` | None or internal | Required dependency and migration readiness |
| `GET /v1/auth/authorize` | None | Start OIDC authorization |
| `GET /v1/auth/callback` | OIDC state | Complete OIDC and issue a service session |
| `POST /v1/auth/refresh` | Session cookie | Rotate the refresh token |
| `POST /v1/auth/logout` | Required | Revoke current or all sessions |
| `POST /v1/lookups` | Optional | Implemented model-only learning card; evidence-backed retrieval remains proposed |
| `GET /v1/lookup-jobs/{job_id}` | Owner or capability | Poll asynchronous generation |
| `GET /v1/senses/{sense_id}` | Optional | Read a current sense card |
| `GET /v1/graph` | Optional | Read a bounded graph for a typed root |
| `GET /v1/graph/nodes/{node_kind}/{node_id}/neighbors` | Optional | Expand one typed graph node |
| `POST /v1/graph-edges/{edge_id}/feedback` | Required | Append usefulness or accuracy feedback |
| `GET /v1/history` | Required | Cursor-page private history |
| `GET /v1/history/{lookup_id}` | Required | Reopen current content or a retained snapshot |
| `DELETE /v1/history/{lookup_id}` | Required | Delete one owned history event |
| `DELETE /v1/history` | Required | Start a clear-history job |
| `PUT /v1/saved-senses/{sense_id}` | Required | Create or update a learning state |
| `DELETE /v1/saved-senses/{sense_id}` | Required | Remove a saved sense |
| `POST /v1/practice/sessions` | Required | Create an adaptive session |
| `POST /v1/practice/sessions/{session_id}/next` | Required | Claim or return the outstanding item |
| `GET /v1/practice/sessions/{session_id}/current` | Required | Read the outstanding item without mutation |
| `POST /v1/practice/sessions/{session_id}/attempts` | Required | Evaluate one answer and advance mastery once |
| `GET /v1/progress` | Required | Read due counts and mastery summaries |
| `POST /v1/graph-views` | Required | Create a saved graph view |
| `GET /v1/graph-views` | Required | List or find saved graph views |
| `GET /v1/graph-views/{view_id}` | Required | Read one saved graph view |
| `PUT /v1/graph-views/{view_id}` | Required | Replace a view with optimistic concurrency |
| `DELETE /v1/graph-views/{view_id}` | Required | Delete a saved graph view |
| `GET /v1/me` | Required | Read profile and preferences |
| `PATCH /v1/me/preferences` | Required | Update preferences |
| `POST /v1/me/export` | Required | Start a portable data export |
| `DELETE /v1/me` | Required | Begin account deletion |
| `GET /v1/privacy-requests/{request_id}` | Owner or capability | Poll a privacy request |
| `POST /v1/privacy-requests/{request_id}/result` | Owner or capability | Mint a one-use export URL |

## POST /v1/lookups

Creates or retrieves an evidence-backed learning card.

```json
{
  "query": "caliente",
  "source_language": "es",
  "target_language": "en",
  "context": "La sopa está muy caliente.",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "learner_level": "B1",
  "detail": "full",
  "include": ["relations", "word_history", "practice_preview"],
  "history_mode": "save"
}
```

Rules:

- `query` contains one word or short expression of at most 100 Unicode scalar values.
- `context` is optional and contains at most 1,000 Unicode scalar values.
- `source_language` is a BCP-47 tag or `auto`.
- Basic core supports only `en` as `target_language`.
- An unsupported explanation language or dialect returns a typed validation error.
- `history_mode` is `save` or `incognito`.
- Anonymous requests and accounts with disabled history operate incognito.

A synchronous result returns `200`. A request predicted to exceed the soft synchronous budget commits a durable job and returns `202` with `Location` and `Retry-After`.

An asynchronous anonymous request also returns a high-entropy capability once in the `Lookup-Capability` header. Polling supplies that header; capabilities never appear in URLs and only their hashes are stored.

### Lookup response shape

```json
{
  "schema_version": "1.0",
  "lookup_id": "01JEXAMPLELOOKUP0000000000",
  "query": {
    "original": "caliente",
    "normalized": "caliente",
    "language": "es",
    "language_confidence": "high"
  },
  "matches": [
    {
      "source_sense_id": "01JEXAMPLESOURCESENSE000",
      "english_senses": [
        {
          "sense_id": "01JEXAMPLEENGLISHSENSE00",
          "lemma": "hot",
          "part_of_speech": "adjective",
          "definition": {
            "text": "having a high temperature",
            "evidence_ids": ["ev_definition_01"]
          },
          "localized_gloss": {
            "text": "温度高的",
            "language": "zh-CN",
            "evidence_ids": ["ev_gloss_01"]
          },
          "confidence": "high"
        }
      ],
      "context_relevance": 0.96
    }
  ],
  "coverage": {
    "parts_of_speech": "available",
    "examples": "partial",
    "word_history": "unavailable"
  },
  "warnings": [],
  "provenance": {
    "lexicon_release": "01JLEXICONRELEASE000000000",
    "index_version": "multilingual-v3",
    "ranking_version": "lookup-rank-v1",
    "generated_at": "2026-09-10T10:00:00Z"
  }
}
```

`lookup_id` is absent for incognito requests. Every factual nested assertion carries its own evidence IDs. Generated content carries `generated: true` plus the evidence on which it is based.

Coverage values are `available`, `partial`, `unavailable`, `disputed`, `not_requested`, `blocked_by_policy`, or `temporarily_unavailable`.

## GET /v1/lookup-jobs/{job_id}

Returns `202` while queued or running, `200` with the completed lookup envelope, or the stored typed failure. An authenticated job requires ownership. An anonymous job requires `Lookup-Capability`.

Expired jobs return `410 lookup_job_expired`. Raw query and context payloads are encrypted during the job and erased at completion or expiry.

## Graph reads

`GET /v1/graph` accepts a typed `root_kind`, `root_id`, requested depth, relation filters, level filters, node cap, edge cap, and cursor. Basic core defaults to depth 1, at most 75 nodes and 200 edges, with a hard maximum depth of 2.

```json
{
  "schema_version": "1.0",
  "root": {"kind": "sense", "id": "01JEXAMPLEENGLISHSENSE00"},
  "nodes": [
    {
      "id": "01JEXAMPLEENGLISHSENSE00",
      "kind": "sense",
      "label": "hot",
      "language": "en",
      "part_of_speech": "adjective",
      "definition_short": "having a high temperature",
      "expandable": true
    },
    {
      "id": "01JEXAMPLESCORCHINGSENSE0",
      "kind": "sense",
      "label": "scorching",
      "language": "en",
      "part_of_speech": "adjective",
      "definition_short": "extremely hot",
      "expandable": true
    }
  ],
  "edges": [
    {
      "id": "derived:temperature-scale:3:4",
      "source": "01JEXAMPLEENGLISHSENSE00",
      "target": "01JEXAMPLESCORCHINGSENSE0",
      "type": "higher_degree",
      "directed": true,
      "relation_version": null,
      "feedback_capabilities": [],
      "display_rank": 0.84,
      "ranking_version": "graph-rank-v1"
    }
  ],
  "lexicon_release": "01JLEXICONRELEASE000000000",
  "ranking_version": "graph-rank-v1",
  "community_aggregate_version": "community-v42",
  "personal_overlay_version": null,
  "truncated": false,
  "next_cursor": null
}
```

All edge endpoints are present in `nodes`. Stored feedback-enabled edges use opaque public IDs and a non-null relation version. Derived edges use namespaced IDs, have a null relation version, and advertise no feedback capability.

The graph response contains topology and optional deterministic layout hints, never authoritative coordinates. Clients incrementally expand nodes instead of requesting an unrestricted component.

## POST /v1/graph-edges/{edge_id}/feedback

Appends one immutable feedback event and updates the personal projection transactionally.

```json
{
  "relation_version": 3,
  "dimension": "accuracy",
  "judgment": "missing_restriction",
  "context": {
    "root_kind": "sense",
    "root_id": "01JEXAMPLEENGLISHSENSE00",
    "english_dialect": "en-US"
  },
  "comment": "These are synonyms only in informal American English."
}
```

Usefulness judgments are `more`, `less`, or `reset`. Accuracy judgments are `accurate`, `wrong_sense`, `wrong_type`, `too_broad`, `missing_restriction`, `unsupported`, `unsure`, or `reset`.

A stale relation version returns `409 relation_version_conflict` with the current edge summary. A derived edge without feedback capability returns `422 feedback_not_supported`.

## History and saved senses

`GET /v1/history` returns encrypted-at-rest private history ordered by occurrence time. `GET /v1/history/{lookup_id}?view=current` resolves the current card; `view=snapshot` returns the retained context-free canonical snapshot and its original release.

`DELETE /v1/history/{lookup_id}` removes one event. `DELETE /v1/history` creates a retryable privacy request. The configured retention period determines each new event's non-null expiration time.

`PUT /v1/saved-senses/{sense_id}` accepts `learning`, `known`, `paused`, or `archived`, plus an optional encrypted note. Sense split or merge migrations may pause ambiguous saved state for learner confirmation.

## Practice

`POST /v1/practice/sessions` accepts a bounded item count and optional skill filters. It returns the scheduler version and session public ID.

`POST /v1/practice/sessions/{session_id}/next` locks the owned session and returns its existing served-but-unanswered item. Only when none exists does it mark the next queued item served. Retries and concurrent tabs cannot consume additional exercises.

`GET /v1/practice/sessions/{session_id}/current` returns the outstanding item without mutation. Exercise delivery never includes the accepted-answer specification.

`POST /v1/practice/sessions/{session_id}/attempts` accepts the session-item ID, answer, hint count, and client-observed response duration. One transaction inserts the immutable attempt, marks the item answered, advances session counters, and updates focus-sense mastery exactly once.

An uncertain free-form evaluator returns `needs_review` and `scheduler_rating: unchanged`. A skipped attempt has no stored answer. Other raw answers are encrypted and redacted after the configured correction window.

## Saved graph views

`POST /v1/graph-views` creates a view and server-generated public ID for one root kind, root ID, filter set, content release, and layout algorithm. `PUT` replaces camera and pinned finite coordinates only when `If-Match` matches the current layout version.

Saved layout data is private presentation state. It never changes shared topology, edge rank, or semantic confidence.

## Privacy requests

Clear-history, export, and account deletion create durable privacy requests. The server returns a one-time high-entropy `Privacy-Capability`; only its hash is retained.

The authenticated owner can poll export and clear-history requests. Account deletion revokes sessions, so subsequent polling requires the capability. A completed export result endpoint mints a short-lived one-use download URL; export objects and capabilities expire.

## Error envelope

Errors follow RFC 9457 problem-details semantics with a stable application code:

```json
{
  "type": "https://transnet.example/problems/ambiguous-source-language",
  "title": "Source language is ambiguous",
  "status": 422,
  "code": "ambiguous_source_language",
  "detail": "Choose a source language to continue.",
  "request_id": "req_01JEXAMPLE",
  "retryable": false,
  "details": {
    "candidates": [
      {"language": "es", "confidence": 0.55},
      {"language": "pt", "confidence": 0.41}
    ]
  }
}
```

| Status | Meaning |
| --- | --- |
| `400` | Malformed JSON or transport-level request error |
| `401` | Authentication is required or invalid |
| `403` | A visible resource is blocked by role or policy |
| `404` | Resource is absent or not owned; private ownership is not disclosed |
| `409` | Idempotency, version, or optimistic-concurrency conflict |
| `410` | A known job, capability, snapshot, or export has expired |
| `422` | Well-formed request fails semantic validation |
| `429` | Account, network, endpoint, or model budget is rate-limited |
| `503` | A required dependency is unavailable after safe retries |
| `504` | No asynchronous continuation was persisted before the hard deadline |

Provider response bodies, credentials, raw learner content, and internal stack details never appear in errors.

## Related documents

- [System design](../transnet.md)
- [Learning experience](../product/learning-experience.md)
- [MySQL schema](mysql-schema.md)
- [Overall plan](../todo.md)
