# Island-port interface

This document is the authoritative human-readable contract between trusted Island-port and Transnet. Transnet does not authenticate callers, manage sessions, or accept cookies or bearer tokens. Island-port performs authentication and authorization before forwarding an internal request.

## Transport

Requests and responses use JSON over loopback HTTP. Every response contains `X-Request-Id`; an inbound value must contain 1–128 ASCII letters, digits, hyphens, underscores, or periods. All `/v1` responses use `Cache-Control: no-store` unless an endpoint explicitly returns an immutable public representation with an `ETag`.

Stateful routes require `X-Learner-Id`, an opaque Island-port-issued identifier of 1–128 URL-safe characters. Transnet trusts this header only at the internal boundary and never accepts a learner ID in a request body. Mutating routes require an `Idempotency-Key` of 16–128 URL-safe characters. Reuse with the same normalized request replays the original result; reuse with different content returns `409 idempotency_conflict`.

`Lookup-Capability` and `Privacy-Capability` are 32–256-character URL-safe bearer capabilities used only to resume anonymous asynchronous work. They are returned once, never placed in URLs, and are redacted from logs. `If-Match` carries an opaque quoted ETag for optimistic replacement. Cursors are opaque, version-bound strings of at most 4,096 UTF-8 bytes.

Request bodies are limited by `[http].max_request_body_bytes`. Unknown JSON fields and query parameters are rejected. Timestamps are RFC 3339 UTC strings, IDs are opaque strings, language values are BCP-47 tags, and all bounded counts are inclusive.

## Endpoint inventory

| Method and route | Required context | Purpose |
| --- | --- | --- |
| `GET /health` | None | Process health |
| `GET /livez` | None | Process liveness |
| `GET /readyz` | None | Required-dependency readiness |
| `POST /translate` | None | Direct translation |
| `POST /v1/lookups` | Optional learner | Build an evidence-backed learning card |
| `GET /v1/lookup-jobs/{job_id}` | Learner or `Lookup-Capability` | Poll lookup work |
| `GET /v1/senses/{sense_id}` | None | Read canonical sense details |
| `GET /v1/graph` | None | Read a bounded canonical graph |
| `GET /v1/graph/nodes/{kind}/{id}/neighbors` | None | Page direct graph neighbors |
| `POST /v1/graph-edges/{edge_id}/feedback` | Learner, idempotency | Record personal feedback |
| `GET /v1/history` | Learner | Page lookup history |
| `GET /v1/history/{lookup_id}` | Learner | Reopen a lookup |
| `DELETE /v1/history/{lookup_id}` | Learner, idempotency | Delete one history item |
| `DELETE /v1/history` | Learner, idempotency | Start clear-history work |
| `GET /v1/saved-senses` | Learner | Page saved senses |
| `GET /v1/saved-senses/{sense_id}` | Learner | Read one saved sense |
| `PUT /v1/saved-senses/{sense_id}` | Learner, idempotency | Save learning state |
| `DELETE /v1/saved-senses/{sense_id}` | Learner, idempotency | Remove learning state |
| `POST /v1/practice/sessions` | Learner, idempotency | Create an adaptive session |
| `POST /v1/practice/sessions/{session_id}/next` | Learner, idempotency | Claim the outstanding item |
| `GET /v1/practice/sessions/{session_id}/current` | Learner | Read the outstanding item |
| `POST /v1/practice/sessions/{session_id}/attempts` | Learner, idempotency | Submit one answer exactly once |
| `GET /v1/progress` | Learner | Read due counts and mastery |
| `POST /v1/graph-views` | Learner, idempotency | Create a graph view |
| `GET /v1/graph-views` | Learner | Page graph views |
| `GET /v1/graph-views/{view_id}` | Learner | Read a graph view |
| `PUT /v1/graph-views/{view_id}` | Learner, idempotency, `If-Match` | Replace layout state |
| `DELETE /v1/graph-views/{view_id}` | Learner, idempotency | Delete a graph view |
| `GET /v1/me` | Learner | Read learner preferences |
| `PATCH /v1/me/preferences` | Learner, idempotency | Update preferences |
| `POST /v1/me/export` | Learner, idempotency | Start an export |
| `DELETE /v1/me` | Learner, idempotency | Start learner-data deletion |
| `GET /v1/privacy-requests/{request_id}` | Learner or `Privacy-Capability` | Poll privacy work |
| `POST /v1/privacy-requests/{request_id}/result` | Learner or capability, idempotency | Mint a one-use export URL |

## Process and translation

`GET /health` and `GET /livez` return `200`; `GET /readyz` returns `503` when a mandatory dependency cannot safely serve traffic.

```json
{
  "status": "ok"
}
```

`POST /translate` accepts nonblank text and source and target language tags. Text longer than `translation.long_text_chars` uses the long-text provider.

```json
{
  "text": "Hello",
  "source_lang": "en",
  "target_lang": "zh-CN"
}
```

```json
{
  "translation": "你好"
}
```

## Lookups and jobs

`POST /v1/lookups` accepts a word or short expression of at most 100 Unicode characters and optional context of at most 1,000 characters. `source_language` is a language tag or `auto`; `target_language` is `en`; dialect is `en-US` or `en-GB`; learner levels are `A1`–`C2`; detail is `brief` or `full`; include values are `relations`, `word_history`, and `practice_preview`; history mode is `save` or `incognito`. Without `X-Learner-Id`, history mode is incognito.

```json
{
  "query": "caliente",
  "source_language": "es",
  "target_language": "en",
  "context": "La sopa está caliente.",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "learner_level": "B1",
  "detail": "full",
  "include": [
    "relations",
    "word_history"
  ],
  "history_mode": "save"
}
```

A completed lookup returns `200`. Durable work returns `202`, `Location`, `Retry-After`, and an anonymous `Lookup-Capability` when no learner header was supplied.

```json
{
  "schema_version": "1.0",
  "lookup_id": "01JLOOKUP",
  "query": {
    "original": "caliente",
    "normalized": "caliente",
    "language": "es",
    "language_confidence": "high"
  },
  "matches": [
    {
      "source_sense_id": "01JSOURCE",
      "english_senses": [
        {
          "sense_id": "01JHOT",
          "lemma": "hot",
          "part_of_speech": "adjective",
          "definition": {
            "text": "having a high temperature",
            "evidence_ids": [
              "ev-1"
            ]
          },
          "confidence": "high"
        }
      ],
      "context_relevance": 0.96
    }
  ],
  "coverage": {
    "definitions": "available",
    "relations": "partial",
    "word_history": "unavailable"
  },
  "warnings": [],
  "provenance": {
    "release_id": "01JRELEASE",
    "collection_version": "sense-v3",
    "ranking_version": "lookup-v1",
    "generated_at": "2026-09-12T10:00:00Z"
  }
}
```

```json
{
  "job_id": "01JJOB",
  "status": "queued",
  "expires_at": "2026-09-12T10:15:00Z"
}
```

`GET /v1/lookup-jobs/{job_id}` returns `202` for `queued` or `running`, `200` with the lookup envelope for `completed`, `200` with a redacted terminal failure for `failed`, `404` for absent or foreign work, and `410` after expiry.

## Canonical senses and graph

`GET /v1/senses/{sense_id}` returns localized glosses, pronunciations, usage labels, grammar patterns, collocations, examples, pitfalls, etymology, history, and assertion-level permitted evidence.

```json
{
  "schema_version": "1.0",
  "sense": {
    "id": "01JHOT",
    "lemma": "hot",
    "part_of_speech": "adjective",
    "definition": "having a high temperature"
  },
  "localized_glosses": [
    {
      "language": "zh-CN",
      "text": "温度高的",
      "evidence_ids": [
        "ev-1"
      ]
    }
  ],
  "pronunciations": [
    {
      "dialect": "en-US",
      "notation": "ipa",
      "value": "hɑt",
      "evidence_ids": [
        "ev-2"
      ]
    }
  ],
  "usage_labels": [],
  "grammar_patterns": [],
  "collocations": [],
  "examples": [],
  "pitfalls": [],
  "etymologies": [],
  "sense_history": [],
  "provenance": {
    "release_id": "01JRELEASE"
  }
}
```

`GET /v1/graph` requires `root_kind` (`sense`, `lexeme`, `construction`, or `scale`) and `root_id`. `depth` is 0–2; `node_limit` is 1–75; `edge_limit` is 1–200. `relation_types` is comma-separated. The neighbor route accepts the same filters plus an opaque cursor and requires `node_limit` of 2–75.

```json
{
  "schema_version": "1.0",
  "root": {
    "kind": "sense",
    "id": "01JHOT"
  },
  "content_version": {
    "release_id": "01JRELEASE",
    "ranking_version": "graph-v1",
    "community_aggregate_version": "community-v42"
  },
  "nodes": [
    {
      "id": "01JHOT",
      "kind": "sense",
      "label": "hot",
      "language": "en",
      "expandable": true
    }
  ],
  "edges": [],
  "relation_list": [],
  "truncated": false,
  "next_cursor": null
}
```

Relation types are `synonym`, `near_synonym`, `translation_equivalent`, `antonym`, `hypernym`, `hyponym`, `holonym`, `meronym`, `confusable_with`, `associated_with`, `inflection_of`, `has_inflection`, `derivationally_related_to`, `etymologically_derived_from`, `etymological_source_of`, `construction_member`, `has_construction_member`, `scale_contains`, `member_of_scale`, `lower_degree`, and `higher_degree`. Every edge endpoint appears in `nodes`; derived edges have no feedback capability.

## Feedback

Feedback accepts usefulness judgments `more`, `less`, or `reset`, and accuracy judgments `accurate`, `wrong_sense`, `wrong_type`, `too_broad`, `missing_restriction`, `unsupported`, `unsure`, or `reset`. A stale relation version returns `409`; a derived edge returns `422`.

```json
{
  "relation_version": 3,
  "dimension": "accuracy",
  "judgment": "missing_restriction",
  "context": {
    "root_kind": "sense",
    "root_id": "01JHOT",
    "english_dialect": "en-US"
  },
  "comment": "Only synonymous in informal American English."
}
```

```json
{
  "event_id": "01JFEEDBACK",
  "recorded_at": "2026-09-12T10:00:00Z",
  "current": {
    "usefulness": null,
    "accuracy": "missing_restriction",
    "revision": 4
  }
}
```

## History and saved senses

History uses `limit` 1–100 and an opaque `before` cursor. A history item may be reopened with `view=current` or `snapshot`. Delete-one returns `204`; clear-all returns `202` with a privacy request.

```json
{
  "items": [
    {
      "lookup_id": "01JLOOKUP",
      "sense_id": "01JHOT",
      "occurred_at": "2026-09-12T10:00:00Z",
      "expires_at": "2026-12-11T10:00:00Z"
    }
  ],
  "next_cursor": null
}
```

Saved senses use `limit` 1–100, an opaque `after` cursor, and optional state filter. States are `learning`, `known`, `paused`, and `archived`.

```json
{
  "state": "learning",
  "note": "temperature sense",
  "expected_revision": 3
}
```

```json
{
  "id": "01JSAVED",
  "sense_id": "01JHOT",
  "state": "learning",
  "note": "temperature sense",
  "revision": 4,
  "updated_at": "2026-09-12T10:00:00Z"
}
```

List and single-item reads use that shape; lists contain `items` and `next_cursor`. Successful deletion returns `204`.

## Practice and progress

Session creation accepts `item_count` 1–100 and optional skills: `meaning_recognition`, `sense_discrimination`, `english_recall`, `spelling_form`, `collocation`, `grammar_pattern`, `register_choice`, `contrast`, `free_production`, and `listening_pronunciation`.

```json
{
  "item_count": 10,
  "skills": [
    "meaning_recognition",
    "english_recall"
  ],
  "english_dialect": "en-US",
  "explanation_language": "zh-CN"
}
```

```json
{
  "session_id": "01JSESSION",
  "scheduler_version": "scheduler-v1",
  "item_count": 10,
  "created_at": "2026-09-12T10:00:00Z"
}
```

Claim-next has an empty JSON body and replays the outstanding item. Current returns the same item without mutation; both return `204` when no item remains.

```json
{}
```

```json
{
  "exercise_id": "01JEXERCISE",
  "session_item_id": "01JITEM",
  "skill": "meaning_recognition",
  "prompt": {
    "kind": "multiple_choice",
    "text": "Which meaning fits?",
    "choices": [
      "hot",
      "cold"
    ]
  },
  "served_at": "2026-09-12T10:01:00Z"
}
```

Attempts require the outstanding item, answer, hint count 0–20, and optional client duration. Results are `correct`, `incorrect`, `skipped`, or `needs_review`; only definite results advance mastery.

```json
{
  "session_item_id": "01JITEM",
  "answer": {
    "kind": "choice",
    "value": "hot"
  },
  "hint_count": 0,
  "response_duration_ms": 4200
}
```

```json
{
  "attempt_id": "01JATTEMPT",
  "resolution": "correct",
  "scheduler_rating": "good",
  "explanation": "The context refers to temperature.",
  "mastery": {
    "sense_id": "01JHOT",
    "skill": "meaning_recognition",
    "revision": 8,
    "due_at": "2026-09-15T10:00:00Z"
  }
}
```

Progress accepts optional `skill`, `due_before`, `cursor`, and `limit` 1–100.

```json
{
  "due_count": 12,
  "mastery": [
    {
      "sense_id": "01JHOT",
      "skill": "meaning_recognition",
      "level": 0.72,
      "due_at": "2026-09-15T10:00:00Z"
    }
  ],
  "next_cursor": null
}
```

## Saved graph views

Views contain a typed root, normalized relation filters, content release, layout algorithm, camera, and at most 75 finite positions.

```json
{
  "name": "Temperature words",
  "root": {
    "kind": "sense",
    "id": "01JHOT"
  },
  "relation_types": [
    "lower_degree",
    "higher_degree"
  ],
  "content_release_id": "01JRELEASE",
  "layout_algorithm": "force-v2",
  "camera": {
    "x": 0.0,
    "y": 1.0,
    "z": 4.0
  },
  "positions": {
    "01JHOT": {
      "x": 0.0,
      "y": 0.0,
      "z": 0.0
    }
  }
}
```

```json
{
  "view_id": "01JVIEW",
  "name": "Temperature words",
  "root": {
    "kind": "sense",
    "id": "01JHOT"
  },
  "relation_types": [
    "lower_degree",
    "higher_degree"
  ],
  "content_release_id": "01JRELEASE",
  "layout_algorithm": "force-v2",
  "camera": {
    "x": 0.0,
    "y": 1.0,
    "z": 4.0
  },
  "positions": {
    "01JHOT": {
      "x": 0.0,
      "y": 0.0,
      "z": 0.0
    }
  },
  "etag": "\"view-4\"",
  "updated_at": "2026-09-12T10:00:00Z"
}
```

List uses `limit` 1–100 and a cursor. Replace accepts mutable view fields and requires `If-Match`; stale state returns `412`. Delete returns `204`.

## Learner and privacy

Preferences updates accept any nonempty subset and require `expected_revision`.

```json
{
  "expected_revision": 7,
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "english_level": "B1",
  "time_zone": "Asia/Shanghai",
  "daily_goal": 10,
  "history_enabled": true,
  "history_retention_days": 90,
  "personalization_enabled": true,
  "mature_content_mode": "warn",
  "accessibility": {
    "reduced_motion": true
  }
}
```

```json
{
  "learner_id": "learner_01",
  "revision": 8,
  "preferences": {
    "explanation_language": "zh-CN",
    "english_dialect": "en-US",
    "english_level": "B1",
    "time_zone": "Asia/Shanghai",
    "daily_goal": 10,
    "history_enabled": true,
    "history_retention_days": 90,
    "personalization_enabled": true,
    "mature_content_mode": "warn",
    "accessibility": {
      "reduced_motion": true
    }
  }
}
```

Export and learner deletion start durable privacy work.

```json
{
  "format": "json",
  "include": [
    "profile",
    "history",
    "saved_senses",
    "feedback",
    "practice",
    "graph_views"
  ]
}
```

```json
{
  "request_id": "01JPRIVACY",
  "kind": "export",
  "status": "queued",
  "expires_at": "2026-09-19T10:00:00Z"
}
```

Learner deletion accepts confirmation and returns the same request shape with kind `delete_learner`.

```json
{
  "confirm": true
}
```

Privacy polling returns `queued`, `running`, `completed`, or `failed`. Export-result creation has an empty body and returns a short-lived, one-use URL.

```json
{}
```

```json
{
  "download_url": "https://download.example/one-use-token",
  "expires_at": "2026-09-12T10:15:00Z"
}
```

## Errors

Malformed JSON returns `400`, oversized bodies `413`, unavailable dependencies `503`, and a hard deadline without durable continuation `504`. Private absent and foreign resources both return `404`. Rate limits return `429` with `Retry-After`. `/translate` retains `{"error":"description"}`; `/v1` uses `application/problem+json`.

```json
{
  "type": "about:blank",
  "title": "Invalid request",
  "status": 422,
  "code": "validation_error",
  "detail": "One or more fields are invalid.",
  "request_id": "01JREQUEST",
  "retryable": false,
  "errors": [
    {
      "field": "query",
      "message": "must not be blank"
    }
  ]
}
```

Errors never contain learner text, credentials, capabilities, database details, provider bodies, or stack traces.
