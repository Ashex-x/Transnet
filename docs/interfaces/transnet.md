# Transnet service interface

中文：[Transnet 服务接口](../../docs_cn/interfaces/transnet_cn.md)

This contract defines the target island-port-to-Transnet interface and the shared internal HTTP/1.1-over-UDS rules. Island-port owns internet transport, authentication, user state, file ingestion, document reconstruction, and final presentation. Transnet receives no end-user identity and persists no live request content.

Status: revised target v1 contract. The checked-in executable still uses loopback HTTP and implements only the documented transitional translation, BasicCard, pinned-sense, and legacy graph slices. Structured segments, image regions, capabilities, live retrieval, target inbound UDS, guided knowledge views, and knowledge paths are not implemented until their handlers, composition, tests, and documentation land together.

## Contents

- [Connection and wire rules](#connection-and-wire-rules)
- [Privacy and request lifetime](#privacy-and-request-lifetime)
- [Deadlines and call budget](#deadlines-and-call-budget)
- [Success and error envelopes](#success-and-error-envelopes)
- [Translation input](#translation-input)
- [Professional guidance](#professional-guidance)
- [Translation output](#translation-output)
- [Live retrieval](#live-retrieval)
- [Knowledge views](#knowledge-views)
- [POST /api/v1/capabilities](#post-apiv1capabilities)
- [POST /api/v1/health](#post-apiv1health)
- [POST /api/v1/livez](#post-apiv1livez)
- [POST /api/v1/readyz](#post-apiv1readyz)
- [POST /api/v1/translations](#post-apiv1translations)
- [POST /api/v1/basic-cards/lookup](#post-apiv1basic-cardslookup)
- [POST /api/v1/senses/get](#post-apiv1sensesget)
- [POST /api/v1/knowledge/views](#post-apiv1knowledgeviews)
- [POST /api/v1/knowledge/paths](#post-apiv1knowledgepaths)
- [Related documents](#related-documents)

## Connection and wire rules

Transnet listens on `/run/transnet/transnet.sock`; Transnet data adapters call `/run/island-port/island-port.sock`. Deployments may relocate sockets through configuration, but endpoint paths and payloads do not change. Socket owners create the parent directory, prove a stale socket is inactive before removing it, bind with mode `0660`, and rely on filesystem workload identity rather than forwarded user headers.

Every operation uses HTTP/1.1, an origin-form `/api/v1/...` path, `Host: localhost`, UTF-8 JSON, and `POST`. Empty input is `{}`. Clients send `Content-Type: application/json`, `Accept: application/json`, a bounded `Content-Length`, and optionally `X-Request-Id`. Query strings, chunked request bodies, multipart bodies, upgrades, and response streaming are rejected. Servers reject unknown JSON fields and return `X-Request-Id` plus `Cache-Control: no-store`.

The default body limit is 1 MiB. A translation request containing inline images may use the route-specific 12 MiB encoded-body limit. At most four decoded images are accepted, each no larger than 2 MiB or 4096 by 4096 pixels, with at most sixteen regions across the request. Supported image media types are `image/png`, `image/jpeg`, and `image/webp`.

IDs are opaque URL-safe strings. Timestamps are UTC RFC 3339 with microsecond precision. Language values are canonical BCP 47 tags advertised by capabilities; `auto` is accepted only for the source language.

## Privacy and request lifetime

Transnet accepts no user, learner, account, owner, session, cookie, bearer token, profile, preference, save state, mastery state, or durable history. Island-port may send current text, bounded inline images, professional guidance, and a chronological list of minimal prior translations as request-scoped linguistic context.

Text, segments, images, protected ranges, terminology, history, normalized forms, chunk plans, provider input and output, hidden reasoning, live-search queries and results, inferred explanations, and online embeddings exist only for the request. They never enter MySQL, Qdrant, logs, traces, metrics, caches, durable queues, backups, publication candidates, or later training data.

Canonical content enters storage only through the authenticated offline publication workflow. A live result never publishes itself. Product-owned saves, edits, feedback, and document state remain in island-port.

## Deadlines and call budget

One caller deadline covers the complete operation. Canonical reads, embeddings, generation, and permitted live retrieval receive sub-deadlines capped by the remaining time and cannot extend the request.

A sufficient canonical match uses zero generation calls. Ordinary translation, visual reading, classification, and grounded composition use the Gemma4-27B `fast` profile. Long input uses bounded semantic chunks, bounded parallel fast calls, one request-local terminology ledger, and deterministic reassembly on the same model.

One request may use the `reasoning` profile at most once, only for unresolved material ambiguity, conflicting terminology or formatting constraints, a verified multi-hop explanation that fast composition cannot safely express, or one invalid structured fast result. Input length alone never triggers reasoning. Hidden reasoning is never returned. Exact policy belongs to the [model-runtime reference](../reference/model-runtime.md).

## Success and error envelopes

Success uses `data` and `meta`. `meta.request_id` matches the response header. Optional metadata appears only when the component participated.

```json
{
  "data": {},
  "meta": {
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "schema_version": "translation-result-v1",
    "inference_profiles": ["fast"],
    "reasoning_escalated": false
  }
}
```

Errors use `application/problem+json`, never echo private content, and reject unknown fields.

```json
{
  "type": "about:blank",
  "title": "Invalid translation request",
  "status": 422,
  "code": "invalid_translation_request",
  "detail": "One or more translation fields are invalid.",
  "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
  "retryable": false,
  "errors": [{"field": "target_language", "message": "is not supported by this deployment."}]
}
```

Common statuses are `400` malformed JSON or unknown fields, `401` failed workload authentication, `404` unknown canonical resource, `409` unavailable pinned release, `413` body too large, `415` unsupported image type, `422` invalid semantic input, `429` bounded capacity, `502` invalid dependency result, `503` required dependency unavailable, and `504` deadline exceeded.

## Translation input

`input` is a tagged union. `text` is the simplest default.

```json
{
  "input": {"type": "text", "text": "The launch date is still up in the air."},
  "source_language": "auto",
  "target_language": "zh-CN",
  "response_level": "standard"
}
```

Structured document and localization input uses ordered segments. Segment IDs are request-local and returned unchanged. `role` accepts `title`, `paragraph`, `list_item`, `caption`, `ui`, or `subtitle`; `format` accepts `plain`, `markdown`, or `html`. Protected ranges use zero-based Unicode scalar offsets, are start-inclusive and end-exclusive, must not overlap, and must be reproduced exactly.

```json
{
  "input": {
    "type": "segments",
    "segments": [
      {
        "segment_id": "seg_title",
        "text": "Launch {product_name}",
        "role": "title",
        "format": "plain",
        "protected_ranges": [{"start": 7, "end": 21}]
      }
    ]
  },
  "source_language": "en",
  "target_language": "zh-CN",
  "response_level": "standard"
}
```

Vision input contains sanitized inline images and normalized rectangles. Coordinates are finite decimal values from 0 through 1, measured from the top-left. Each region ID is unique within its image, rectangles must have positive area and remain in bounds, and `reading_order` references every region exactly once. If a file or PDF is involved, island-port renders and selects pages before this request.

```json
{
  "input": {
    "type": "image_regions",
    "images": [
      {
        "image_id": "page_1",
        "media_type": "image/png",
        "data": "<base64-encoded PNG>",
        "regions": [
          {"region_id": "heading", "x": 0.05, "y": 0.06, "width": 0.9, "height": 0.12}
        ]
      }
    ],
    "reading_order": ["page_1:heading"]
  },
  "source_language": "auto",
  "target_language": "en",
  "response_level": "standard"
}
```

Text is limited to 131,072 Unicode scalars. Segmented input accepts at most 256 segments, 8,192 scalars per segment, and 131,072 aggregate scalars. Each segment accepts at most 128 protected ranges. These limits are subordinate to the encoded body limit.

`history` is optional and chronological. Each item contains only previous source text, translated text, and language tags. It has no turn ID, time, user ID, feedback, model metadata, or save state. The common body limit bounds history; there is no separate item-count limit.

## Professional guidance

`guidance` is optional and request-scoped. Omitted values use `general`, `general`, `preserve`, zero alternatives, the response-level default annotations, and `offline` freshness.

```json
{
  "purpose": "technical",
  "audience": "specialist",
  "register": "preserve",
  "terminology": [
    {"source": "torque", "target": "扭矩", "policy": "required"}
  ],
  "max_alternatives": 1,
  "annotations": ["ambiguity", "terminology", "register", "culture"],
  "freshness": "offline"
}
```

`purpose` accepts `general`, `publication`, `technical`, `localization`, or `subtitles`. `audience` accepts `general`, `professional`, `specialist`, or `young_reader`. `register` accepts `preserve`, `neutral`, `formal`, or `informal`. Terminology policy accepts `required`, `preferred`, or `forbidden`; at most 128 entries are accepted, and source and target values are each limited to 256 scalars. `max_alternatives` is 0 through 2.

Guidance constrains the current result but never creates a profile, translation memory, or canonical term. Contradictory required terms, protected ranges, or format rules return `422 constraint_conflict` rather than silently dropping a constraint.

## Translation output

Text output preserves the existing meaning-specific ordered translation list. Segment and image-region outputs preserve request order and IDs. Every unit has one primary translation and up to the requested number of materially useful alternatives.

```json
{
  "translation": {
    "input_type": "segments",
    "detected_source_languages": ["en"],
    "segments": [
      {
        "segment_id": "seg_title",
        "translations": [{"text": "发布 {product_name}", "language": "zh-CN"}],
        "annotations": [
          {"type": "terminology", "code": "protected_content_preserved", "message": "Protected content was copied unchanged."}
        ],
        "review": {"state": "clean", "issues": []}
      }
    ],
    "terminology_decisions": []
  }
}
```

Image output uses `regions` with `image_id`, `region_id`, detected language, translations, annotations, and review. It does not return the image or an unrestricted OCR transcript. Review state is `clean` or `review_recommended`; issue codes are closed and include `low_confidence`, `source_ambiguous`, `terminology_conflict`, `format_risk`, `protected_content_mismatch`, `visual_order_uncertain`, and `live_source_incomplete`.

`brief`, `standard`, and `full` are deterministic projections of one validated superset. A lower level removes supporting detail but never changes the selected meaning, translation, protected content, evidence state, or review outcome. Empty sections are omitted.

## Live retrieval

`guidance.freshness` accepts `offline`, `allowed`, or `required`. `offline` is the default and prohibits network retrieval. `allowed` is explicit permission to retrieve only when deterministic classification finds a freshness-sensitive claim. `required` always attempts retrieval and returns `503 live_retrieval_unavailable` if the bounded operation cannot complete safely.

Live retrieval is an orchestrated search/fetch port, not unrestricted model browsing. It performs at most one search round, selects at most five results, fetches at most three pages concurrently, and obeys a configured sub-deadline. The fetcher allows only public HTTP(S), resolves and validates every redirect, rejects loopback, link-local, private, reserved, and Unix-socket destinations, bounds response bytes, and accepts only configured textual media types.

Fetched content is untrusted data. It cannot modify system instructions, request another URL, expose credentials, bypass release filters, or become canonical evidence. The embedding model may rank fetched fragments in memory; both fragments and vectors are discarded with the request.

Claims based on live retrieval reference response-local sources. Live sources are labeled `live_external`, not `verified`.

```json
{
  "external_sources": [
    {
      "source_id": "live_1",
      "title": "Example current terminology notice",
      "publisher": "Example standards body",
      "url": "https://example.org/notices/current-term",
      "published_at": "2026-09-20T00:00:00.000000Z",
      "retrieved_at": "2026-10-01T08:00:00.000000Z",
      "evidence_state": "live_external"
    }
  ]
}
```

## Knowledge views

Canonical knowledge is an evidence-backed assertion graph. A knowledge tree is a deterministic, root-specific projection through one lens; it is never stored as canonical parentage. The same stable node may appear in several lens branches without acquiring a second identity.

Closed lenses are `meaning`, `contrast`, `usage`, `form`, `origin`, `domain`, `mechanism`, and `application`. Initial lexical translations may return `available_lenses`; follow-up requests choose one returned lens. Transnet derives relationship families, bounds, and ranking from the lens and response level. Callers do not submit raw relation filters, graph depth, node limits, vector selectors, or arbitrary traversal queries.

Each displayed item is the root or includes an explicit path to the root, a concise relevance reason, assertion and evidence references, and one of `verified`, `inferred`, or `exploratory`. Only hydrated `verified` assertions may form a factual connection path. Inferred and exploratory material is request-local and visually separate.

## POST /api/v1/capabilities

Returns configured BCP 47 language pairs, input kinds, image types, purposes, annotation families, knowledge lenses, body and semantic limits, live-retrieval availability, and schema versions. It exposes no credentials, provider URLs, socket paths, concurrency state, or private feature flags.

Request: `{}`

```json
{
  "data": {
    "source_languages": ["auto", "en", "zh-CN"],
    "target_languages": ["en", "zh-CN"],
    "input_types": ["text", "segments", "image_regions"],
    "image_media_types": ["image/png", "image/jpeg", "image/webp"],
    "knowledge_lenses": ["meaning", "contrast", "usage", "form", "origin", "domain", "mechanism", "application"],
    "live_retrieval": {"available": false, "default": "offline"},
    "schema_versions": ["translation-result-v1", "knowledge-view-v1"]
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/health

Returns `200` when the process can answer. It does not probe dependencies.

Request: `{}`

Response data: `{"status":"ok"}`.

## POST /api/v1/livez

Returns `200` while the event loop and listener are alive. It does not report readiness.

Request: `{}`

Response data: `{"status":"alive"}`.

## POST /api/v1/readyz

Returns `200` only when every configured required dependency can safely serve new work. Optional canonical, retrieval, embedding, vision, reasoning, or live-retrieval capabilities are reported as closed component states and do not become required unless configuration says so.

Request: `{}`

```json
{
  "data": {
    "status": "ready",
    "components": {
      "generation_fast": "available",
      "generation_reasoning": "available",
      "embedding": "available",
      "canonical_data": "disabled",
      "retrieval_data": "disabled",
      "live_retrieval": "disabled"
    }
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/translations

Accepts the translation input, language tags, response level, optional guidance, and optional history defined above. It automatically chooses lexical knowledge, connected-text, structured-segment, or visual-region processing. The caller never selects a model, inference profile, chunk policy, canonical release, retrieval strategy, or repair policy.

Successful metadata includes result, normalizer, projection, model, and prompt versions that actually participated; optional `content_release`, `retrieval_version`, `embedding_version`, and `live_retrieval` appear only when used. `inference_profiles` is ordered and de-duplicated. `reasoning_escalated` reports the safe policy outcome without exposing reasoning content.

Closed route errors additionally include `invalid_translation_request`, `constraint_conflict`, `unsupported_input_type`, `unsupported_language_pair`, `invalid_image`, `invalid_model_output`, `translation_model_unavailable`, and `live_retrieval_unavailable`.

## POST /api/v1/basic-cards/lookup

Performs a canonical-data-only, release-pinned lexical lookup. The request accepts `query`, `source_language`, and `target_language`; it accepts no release, derived form, ranker, index, or vector selector. Normal outcomes are `resolved`, `clarification_required`, and `not_found`. A resolved result returns the canonical root, eligible translations, evidence-backed definition, coverage, available knowledge lenses, and immutable content pin.

This route remains a direct diagnostic and compatibility read. New translation turns use `/api/v1/translations`.

## POST /api/v1/senses/get

Reads one canonical sense under the exact `content_release` and `canonical_schema_version` returned by a prior result. It never reselects active content or silently upgrades the pin. The request accepts `sense_id`, `target_language`, `content_release`, and `canonical_schema_version`.

## POST /api/v1/knowledge/views

Returns one guided tree-lens projection for a canonical root.

```json
{
  "root": {"kind": "sense", "id": "sense_sweltering_hot_01"},
  "lens": "contrast",
  "target_language": "zh-CN",
  "response_level": "full",
  "content_release": "knowledge-2026-09",
  "cursor": null
}
```

The response contains the root, lens, ordered branches, stable nodes, explicit paths, relevance reasons, assertion and evidence references, evidence states, truncation, and an opaque next cursor. Cursors bind the root, lens, language, response level, release, projection version, and ordering version and contain no request text.

## POST /api/v1/knowledge/paths

Returns up to three independently verified paths of at most three hops between two canonical roots. The server chooses and enforces relation eligibility and does not perform arbitrary-depth or shortest-path inference.

```json
{
  "from": {"kind": "concept", "id": "concept_coriolis_force"},
  "to": {"kind": "concept", "id": "concept_weather_system"},
  "target_language": "en",
  "content_release": "knowledge-2026-09"
}
```

Normal outcomes are `connected` and `no_verified_path`. Every path step names one hydrated assertion, direction, conditions, relevance, evidence references, and release. Similarity-only candidates may be returned in a separate exploratory section but never as a path step.

The former target drafts `POST /api/v1/graph/get` and `POST /api/v1/graph/neighbors` are removed from the revised target contract. Transitional `GET /v1/graph...` handlers in the current executable remain implementation compatibility behavior until migrated or removed; their existence does not make them target v1 routes.

## Related documents

- [System design](../transnet.md)
- [Canonical-data interface](canonical-data.md)
- [Retrieval-data interface](retrieval-data.md)
- [Model runtime](../reference/model-runtime.md)
- [Persistence boundaries](../reference/persistence.md)
- [Quality assurance](../guides/quality-assurance.md)
