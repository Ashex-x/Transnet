# Learning HTTP API

## Status and compatibility

`POST /v1/lookups` has an implemented model-only basic-core slice. `GET /v1/lookup-jobs/{job_id}` and the graph-read routes are implemented only when their explicit application dependencies are injected; the default process does not inject them, so those routes are absent. All other endpoints in this document remain proposed. The implemented `GET /health`, `GET /livez`, `GET /readyz`, and `POST /translate` contracts remain in [Transnet HTTP API](transnet-api.md).

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

Mutable lists use opaque cursors. Content reads expose `ETag`; the implemented graph reads are `no-store` and use an opaque neighbor cursor. Saved-layout replacements require `If-Match`.

Context-bearing and personalized responses use `Cache-Control: private`; context-bearing lookup envelopes use `no-store`. Shared caches contain only context-free canonical content and never lookup IDs or private fields.

### Version fields

Content responses identify the applicable schema, lexicon release, vector collection, ranking version, and generation versions. Clients must not compare numeric ranks across ranking versions.

## Endpoint inventory

| Method and path | Authentication | Purpose |
| --- | --- | --- |
| `GET /livez` | None | Process liveness |
| `GET /readyz` | None | Implemented injected dependency readiness probe |
| `GET /v1/auth/authorize` | None | Start OIDC authorization |
| `GET /v1/auth/callback` | OIDC state | Complete OIDC and issue a service session |
| `POST /v1/auth/refresh` | Session cookie | Rotate the refresh token |
| `POST /v1/auth/logout` | Required | Revoke current or all sessions |
| `POST /v1/lookups` | Optional | Implemented model-only learning card; evidence-backed retrieval remains proposed |
| `GET /v1/lookup-jobs/{job_id}` | Owner or capability | Implemented when a lookup-job store is injected; poll asynchronous generation |
| `GET /v1/senses/{sense_id}` | Optional | Read a current sense card |
| `GET /v1/graph` | None | Implemented when a graph service is injected; read a bounded graph for a typed root |
| `GET /v1/graph/nodes/{node_kind}/{node_id}/neighbors` | None | Implemented when a graph service is injected; expand one typed graph node |
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

The graph routes are registered only when the process injects a `GraphService`; this keeps the default model-only executable from implying that canonical graph content exists. They are public, canonical reads with `Cache-Control: no-store`, have no private feedback overlay, and never carry saved layout coordinates.

`GET /v1/graph` requires `root_kind` (`sense`, `lexeme`, `construction`, or `scale`) and `root_id`. `root_id` is a nonblank opaque canonical identifier of at most 256 Unicode scalar values. Optional `depth`, `node_limit`, and `edge_limit` default to 1, 75, and 200; their inclusive ranges are 0 through 2, 1 through 75, and 1 through 200. `relation_types` is an optional comma-separated set of at most 21 supported snake-case relation types. Malformed query syntax and unknown query fields return `400 invalid_graph_request`; malformed identifiers and out-of-range limits return `422 invalid_graph_request`, without echoing request values.

Supported `relation_types` values are `synonym`, `near_synonym`, `translation_equivalent`, `antonym`, `hypernym`, `hyponym`, `holonym`, `meronym`, `confusable_with`, `associated_with`, `inflection_of`, `has_inflection`, `derivationally_related_to`, `etymologically_derived_from`, `etymological_source_of`, `construction_member`, `has_construction_member`, `scale_contains`, `member_of_scale`, `lower_degree`, and `higher_degree`.

`GET /v1/graph/nodes/{node_kind}/{node_id}/neighbors` expands only one typed node; its path parameters use the same kind and identifier constraints as `root_kind` and `root_id`. Its `node_limit` is 2 through 75 so every page can contain the root plus at least one adjacent endpoint; `edge_limit` and `relation_types` use the same limits as the full graph read. It accepts an opaque `cursor` of at most 4,096 UTF-8 bytes from the prior neighbor response. The cursor is confidentiality- and integrity-protected and bound to the typed root, active graph content version, exact normalized relation filter, and prior ordering key; it must be treated as an opaque string. A cursor for another root, a stale content version, a changed relation filter, or a modified cursor returns `422 invalid_graph_request`.

A graph-serving host that needs pagination across process restarts or replicas must inject one shared, high-entropy secret of at least 32 bytes with `AppState::with_graph_cursor_protection_key(GraphCursorProtectionKey::new(...))`. Without that explicit injection, `AppState::new` uses a redacted process-local ephemeral key, so cursors intentionally remain valid only within that process lifetime. Every `truncated: true` neighbor response has a `next_cursor` that advances after the last safely processed ordering key; when an endpoint is no longer eligible, following that cursor can yield an empty terminal page rather than exposing an incomplete edge or the withheld ordering key.

```json
{
  "schema_version": "1.0",
  "root": {"kind": "sense", "id": "01JEXAMPLEENGLISHSENSE00"},
  "content_version": {
    "release_id": "01JLEXICONRELEASE000000000",
    "ranking_version": "graph-rank-v1",
    "community_aggregate_version": "community-v42"
  },
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
      "id": "derived:scale:01JLEXICONRELEASE000000000:01JEXAMPLETEMPERATURESCALE:01JEXAMPLEENGLISHSENSE00:01JEXAMPLESCORCHINGSENSE0",
      "source": {"kind": "sense", "id": "01JEXAMPLEENGLISHSENSE00"},
      "target": {"kind": "sense", "id": "01JEXAMPLESCORCHINGSENSE0"},
      "relation_type": "higher_degree",
      "direction": {
        "directed": true,
        "canonical_projection": {
          "kind": "scale_adjacency",
          "scale_id": "01JEXAMPLETEMPERATURESCALE"
        }
      },
      "relation_version": null,
      "feedback_capabilities": [],
      "evidence": {
        "evidence_ids": ["ev_scale_01"],
        "confidence": "high"
      },
      "scope": {
        "dialect": null,
        "domain": "weather",
        "register": null,
        "note": null
      },
      "ranking": {
        "display_rank_basis_points": 8400,
        "components": {
          "evidence_basis_points": 9000,
          "community_basis_points": null,
          "pedagogical_basis_points": 7000
        },
        "version": "graph-rank-v1"
      }
    }
  ],
  "relation_list": [
    {
      "edge_id": "derived:scale:01JLEXICONRELEASE000000000:01JEXAMPLETEMPERATURESCALE:01JEXAMPLEENGLISHSENSE00:01JEXAMPLESCORCHINGSENSE0",
      "source": {"kind": "sense", "id": "01JEXAMPLEENGLISHSENSE00", "label": "hot"},
      "relation_type": "higher_degree",
      "direction": {
        "directed": true,
        "canonical_projection": {
          "kind": "scale_adjacency",
          "scale_id": "01JEXAMPLETEMPERATURESCALE"
        }
      },
      "target": {"kind": "sense", "id": "01JEXAMPLESCORCHINGSENSE0", "label": "scorching"}
    }
  ],
  "truncated": false,
  "next_cursor": null
}
```

Every edge endpoint is present in `nodes`. `direction.canonical_projection` is `stored`, `inverse_projection`, `scale_adjacency`, or `scale_membership`; clients retain it rather than attempting to infer canonical direction from a label. Stored feedback-enabled edges use opaque public IDs and a non-null relation version. Derived edges use namespaced IDs, have a null relation version, and advertise no feedback capability. Scores are basis points, not probabilities or semantic truth, and are comparable only within the returned ranking version.

`relation_list` duplicates each ranked edge in the same deterministic order with typed source and target labels, so an accessible client can present relationships without relying on graph coordinates or a 3D renderer. The response contains canonical topology only; it has no layout hints or authoritative coordinates. Clients incrementally expand nodes instead of requesting an unrestricted component.

An absent typed root returns `404 graph_root_not_found`. A graph dependency failure or internally inconsistent graph read returns `503 graph_unavailable` without internal storage details.

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
