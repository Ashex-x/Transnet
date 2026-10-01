# Transnet service interface

中文：[Transnet 服务接口](../../docs_cn/interfaces/transnet_cn.md)

This contract defines the target island-port-to-Transnet interface and the shared internal HTTP/1.1-over-UDS rules. Island-port owns internet transport, authentication, user state, file ingestion, document reconstruction, and final presentation. Transnet receives no end-user identity and persists no live request content.

Status: revised target v1 contract. The checked-in executable serves HTTP/1.1 only through the target inbound UDS and implements target capability discovery, probes, translation, BasicCard and pinned-sense reads, and conditionally composed knowledge views and paths. The translation boundary strictly validates tagged text, structured segments, image regions, history, and professional guidance; guided text, structured segments, and bounded image regions reach the current neutral Gemma VLM orchestrator. Live retrieval remains unavailable without complete search/fetch runtime composition.

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

Every operation uses HTTP/1.1, an origin-form `/api/v1/...` path, `Host: localhost`, UTF-8 JSON, and `POST`. Empty input is `{}`. Clients send `Content-Type: application/json`, `Accept: application/json`, a bounded `Content-Length`, and optionally `X-Request-Id` and `X-Deadline-At`. Query strings, chunked request bodies, multipart bodies, upgrades, and response streaming are rejected. Servers reject unknown JSON fields and return `X-Request-Id` plus `Cache-Control: no-store`.

The default body limit is 1 MiB. A translation request containing inline images may use the route-specific 12 MiB encoded-body limit. At most four decoded images are accepted, each no larger than 2 MiB or 4096 by 4096 pixels, with at most sixteen regions across the request. Supported image media types are `image/png`, `image/jpeg`, and `image/webp`.

IDs are opaque URL-safe strings. Timestamps are UTC RFC 3339 with microsecond precision. Language values are canonical BCP 47 tags advertised by capabilities; `auto` is accepted only for the source language.

## Privacy and request lifetime

Transnet accepts no user, learner, account, owner, session, cookie, bearer token, profile, preference, save state, mastery state, or durable history. Island-port may send current text, bounded inline images, professional guidance, and a chronological list of minimal prior translations as request-scoped linguistic context.

Text, segments, images, protected ranges, terminology, history, normalized forms, chunk plans, provider input and output, hidden reasoning, live-search queries and results, inferred explanations, and online embeddings exist only for the request. They never enter MySQL, Qdrant, logs, traces, metrics, caches, durable queues, backups, publication candidates, or later training data.

Canonical content enters storage only through the authenticated offline publication workflow. A live result never publishes itself. Product-owned saves, edits, feedback, and document state remain in island-port.

## Deadlines and call budget

One caller deadline covers the complete operation. Canonical reads, embeddings, generation, and permitted live retrieval receive sub-deadlines capped by the remaining time and cannot extend the request.

The HTTP boundary implements the request-context foundation on transitional and target paths. `X-Deadline-At`, when present, must contain a future UTC RFC 3339 timestamp at microsecond precision no more than 120 seconds from admission. An omitted header receives a 30-second deadline. Invalid or overlong deadlines return `400 invalid_deadline`; already exhausted deadlines return `504 deadline_exceeded`. Middleware stores one immutable `RequestContext` with the safe request ID, absolute deadline, `transnet-service-v1` schema, remaining-budget calculation, and an optional release pin for later application composition. Translation generation consumes that remaining deadline and a cooperative request-local cancellation signal on every fast or reasoning call.

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

Vision input contains sanitized inline images and normalized rectangles. Caller-owned segment, image, and region IDs are nonblank opaque values of at most 128 Unicode scalars; image and region IDs exclude `:` because `reading_order` uses the unambiguous `image_id:region_id` form. Coordinates are finite decimal values from 0 through 1, measured from the top-left. Each region ID is unique within its image, rectangles must have positive area and remain in bounds, and `reading_order` references every region exactly once. If a file or PDF is involved, island-port renders and selects pages before this request.

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

The current runtime accepts both the tagged text shape and the legacy top-level `text` field during migration; callers must send exactly one. After validation, the request domain retains the complete tagged input, guidance, and history only for that request. Segment orchestration sends one strict structure-aware prompt per segment through the neutral generation port with bounded parallel fast-profile calls and at most one request-wide reasoning repair. It restores caller order and IDs deterministically, then rejects output unless every protected scalar range remains verbatim and ordered, newline count is unchanged, and Markdown delimiters or HTML tags match exactly. Image-region requests receive bounded structural, media-header, decoded-byte, dimension, rectangle, and exact reading-order validation. Transnet decodes each image locally, crops every declared region into a bounded attachment, discards pixels outside those rectangles before provider submission, and makes one deadline- and cancellation-bound VLM call with a strict attachment-to-region binding. Output must match every caller image/region ID in reading order and uses the requested target language; invalid or reordered output fails closed. Image bytes, crops, prompt material, OCR-like text, and output are discarded with the request.

`history` is optional and chronological. Each item contains only previous source text, translated text, and language tags. It has no turn ID, time, user ID, feedback, model metadata, or save state. There is no separate item-count limit, but the JSON encoding of history plus guidance is limited to 8,192 bytes so every accepted text request fits the 65,536-byte generation-input contract. An oversized aggregate returns `422 invalid_translation_request` with the content-free field `generation_context` before any model call.

## Professional guidance

`guidance` is optional and request-scoped. Omitted values use `general`, `general`, `preserve`, zero alternatives, the response-level default annotations, and `offline` freshness. `max_alternatives` is accepted from zero through two. A nonzero value requires the optional relationship-page runtime and an exactly resolved lexical result; otherwise the request fails before generation rather than fabricating alternatives.

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

Until guidance-aware orchestration is composed for an input family, valid execution-dependent guidance returns `501 translation_capability_unavailable`; the runtime never silently ignores an accepted constraint. Text and segment workflows execute their documented guidance. The current image-region workflow rejects execution-dependent guidance before decoding or model invocation. Explicit `offline` freshness by itself is accepted because it preserves the default no-network behavior.

## Translation output

The result is discriminated by `unit`: `word`, `phrase`, `passage`, `segment`, or `image_region`. Word, phrase, and passage keep the existing ordered `translations`; every choice has a position-derived `translation_id` and zero-based `order` that remain stable across projections. Segment and image-region results preserve caller IDs and an explicit zero-based `order`; each nested unit carries its detected language, primary translations, typed annotations, and invariant review outcome. Structured results also carry the ordered request-scoped `terminology_decisions` that were actually enforced. Every unit may later carry up to the requested number of materially useful labeled alternatives; that Milestone 5 capability remains unavailable until its deterministic evaluator is composed.

```json
{
  "translation": {
    "unit": "segment",
    "segments": [
      {
        "segment_id": "seg_title",
        "order": 0,
        "detected_source_language": "en",
        "translations": [{"translation_id": "translation_0", "order": 0, "text": "发布 {product_name}", "language": "zh-CN"}],
        "annotations": [
          {"type": "format", "code": "protected_content_preserved", "message": "Protected content was copied unchanged."}
        ],
        "review": {"state": "clean", "issues": []}
      }
    ],
    "terminology_decisions": []
  }
}
```

Image output uses `unit: "image_region"` and `regions` with `image_id`, `region_id`, zero-based reading `order`, detected language, translations, annotations, and review. It does not return the image or an unrestricted OCR transcript. Review state is `clean` or `review_recommended`; issue codes are closed and include `low_confidence`, `source_ambiguous`, `terminology_conflict`, `format_risk`, `protected_content_mismatch`, `visual_order_uncertain`, and `live_source_incomplete`.

`brief`, `standard`, and `full` are deterministic projections of one validated superset. A lower level removes supporting detail but never changes the selected meaning, translation, protected content, evidence state, or review outcome. Empty sections are omitted.

Typed annotations use the closed families `ambiguity`, `terminology`, `register`, `culture`, `format`, and `review`; closed codes are `ambiguity_detected`, `term_selected`, `protected_content_preserved`, `register_applied`, `cultural_context`, `format_preserved`, `review_required`, and `live_source_used`. Each annotation has a bounded display message and optional response-local citation references. `data.external_sources`, when present, contains only sources referenced by those citations; source, claim, and fragment IDs are response-local and never canonical evidence. The current offline text path returns no external sources. Result validation rejects unknown citation targets, duplicate source/claim pairs, duplicate source IDs, identity/order gaps, duplicate review issues, contradictory clean review state, and empty primary translations before serialization.

Validation is request-bound before HTTP serialization: structured result IDs and order must exactly equal the originating segment or image reading order, every translation must use the requested target language, and a declared source language must be preserved. Translation IDs are exactly `translation_<zero-based order>`. Passage, segment, and image-region units have one primary connected-text translation and no lexical meaning/detail object; word and phrase units retain bounded meaning labels and may carry only their matching generated exploratory detail shape. Annotation codes have one closed family, review issues are strictly ordered, and `review_required` appears exactly for `review_recommended` units. Projection prunes source descriptors when their last citation is removed. Schema, normalizer, projector, model, prompt, profile, retrieval, and release metadata are bounded and validated rather than passed through unchecked.

## Live retrieval

`guidance.freshness` accepts `offline`, `allowed`, or `required`. `offline` is the default and prohibits network retrieval. The current claim-bound implementation supports non-offline freshness only for text; segment and image-region requests using `allowed` or `required` fail with `501 translation_capability_unavailable` before network, decoding, or generation. For text, `allowed` is explicit permission to retrieve only when deterministic classification finds a freshness-sensitive claim. `required` always attempts retrieval and returns `503 live_retrieval_unavailable` if the live subdeadline expires or the bounded dependency operation cannot complete safely; exhaustion of the caller's overall deadline remains `504`.

Live retrieval is an orchestrated search/fetch port, not unrestricted model browsing. It performs at most one search round, selects at most five results, fetches at most three pages concurrently, and obeys a configured sub-deadline. The fetcher allows only public HTTP(S), resolves and validates every redirect, rejects loopback, link-local, private, reserved, and Unix-socket destinations, bounds response bytes, and accepts only configured textual media types.

`required` always spends the single round and succeeds only when at least one safely fetched page yields usable text. `allowed` spends it only when deterministic classification finds a freshness-sensitive claim and canonical material is insufficient. Partial fetch success is usable when at least one page is safe. The redirect cap is three, each page is limited to 512 KiB, all pages together to 1 MiB, and each retained fragment to 8,192 Unicode scalars. Accepted media types are `text/plain`, `text/html`, and `application/xhtml+xml`. The live sub-deadline defaults to five seconds, cannot exceed fifteen seconds, and is always capped by the request's remaining budget.

Fetched content is untrusted data. It cannot modify system instructions, request another URL, expose credentials, bypass release filters, or become canonical evidence. The embedding model may rank fetched fragments in memory; both fragments and vectors are discarded with the request.

Claims based on live retrieval use strict response-local `{source_id, claim_id}` citations. Lexical claims use `translation_N`; connected-text claims use the deterministic `chunk_N` identity from ordered reassembly. Retrieved fragments are structured untrusted prompt data and never instructions. Every live-assisted claim must cite at least one admitted `live_N` source; fabricated, duplicate, missing, cross-claim, and uncited exposed sources fail closed. Only cited source titles and the final validated fetched URLs leave the operation, every descriptor is labeled `live_external` rather than `verified`, metadata records `translation-live-v1`, and fetched material is discarded with the request. An `allowed` attempt that cannot obtain material or reaches only its live subdeadline is an explicit review-recommended degradation; `required` maps the same condition to `503 live_retrieval_unavailable`.

```json
{
  "external_sources": [
    {
      "source_id": "live_1",
      "title": "Example current terminology notice",
      "url": "https://example.org/notices/current-term",
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

Returns currently implemented BCP 47 language selectors, input kinds, image types, purposes, annotation families, knowledge lenses, body and semantic limits, live-retrieval availability, generation profiles, and schema versions. Empty closed sets explicitly mean that the current runtime does not implement that capability. It exposes no credentials, provider URLs, socket paths, concurrency state, or private feature flags.

Installing the translation orchestrator atomically advertises `segments`, `image_regions`, the three accepted image media types, and the `format` annotation family; a state without that dependency advertises none of them. The flat v1 capability shape cannot express input-specific guidance support, so `purposes` remains empty until every advertised input executes the same purpose set. Replacing a caller-supplied capability declaration cannot partially remove or fabricate that atomic set.

Live-retrieval capability additionally declares the exact `input_types` with complete claim-bound attribution. The current composed implementation reports only `text` and adds the `review` annotation family used for `live_external` attribution; an unavailable deployment reports an empty input list and no live review family together with `available: false`.

Knowledge-lens activation is atomic. `AppState` accepts one validated, indivisible knowledge-route dependency bundle whose view and path services share the same complete immutable projection expectation; installing it also installs matching active-release readiness. Both routes are absent without the bundle. A runtime advertises exactly `meaning`, `contrast`, `usage`, `form`, `origin`, and `domain` only while that bundle is installed, even if a caller supplied a stale capability declaration. `mechanism` and `application` remain absent because their explicit technical relation policies are not executable. The executable constructs the bundle only when optional knowledge configuration and one complete active release trio validate successfully.

Capabilities follows the interface-wide response policy: every response carries `Cache-Control: no-store`. Callers may refresh it when they need current deployment information, but the contract promises no HTTP caching or validator semantics.

Request: `{}`

```json
{
  "data": {
    "source_languages": ["auto", "en", "zh-CN"],
    "target_languages": ["en", "zh-CN"],
    "input_types": ["text", "segments", "image_regions"],
    "image_media_types": ["image/png", "image/jpeg", "image/webp"],
    "purposes": [],
    "annotation_families": ["format"],
    "knowledge_lenses": [],
    "limits": {"max_request_body_bytes": 1048576, "max_translation_bytes": 1048576, "max_generation_context_bytes": 8192, "max_lexical_chars": 128, "max_connected_chunk_chars": 8192, "max_connected_chunks": 128},
    "live_retrieval": {"available": false, "default": "offline", "input_types": []},
    "generation_profiles": ["fast", "reasoning"],
    "schema_versions": ["translation-result-v1"]
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

Returns `200` only when every configured required dependency can safely serve new work. The knowledge bundle reports only `canonical_data`, `retrieval_data`, and `knowledge_projection` with the closed values `available`, `unavailable`, or `disabled`; it exposes no release identifiers, collection identifiers, hashes, or endpoints. A configured bundle is available only when the active-trio authority returns the exact full canonical pin and immutable node/edge projection tuple expected by the view services. Any missing, invalid, or different tuple makes the whole atomic bundle unavailable. An unconfigured bundle reports all three components as disabled and does not affect readiness.

Request: `{}`

```json
{
  "data": {
    "status": "ready",
    "components": {
      "canonical_data": "disabled",
      "retrieval_data": "disabled",
      "knowledge_projection": "disabled"
    }
  },
  "meta": {"request_id": "req_example"}
}
```

## POST /api/v1/translations

Accepts the translation input, language tags, response level, optional guidance, and optional history defined above. It automatically chooses lexical knowledge, connected-text, structured-segment, or visual-region processing. The caller never selects a model, inference profile, chunk policy, canonical release, retrieval strategy, or repair policy.

Successful metadata includes result, normalizer, projection, model, and prompt versions that actually participated; optional `content_release`, `retrieval_version`, `embedding_version`, and `live_retrieval` appear only when used. `inference_profiles` is ordered and de-duplicated. `reasoning_escalated` reports the safe policy outcome without exposing reasoning content.

Only resolved word and established-phrase results may embed the relationship-page object. Passage, segment, and image-region results never do. The page begins with a full-release-pinned BasicCard or concept-summary authority receipt, then applies deterministic progressive disclosure to supported direct groups, complete semantic scales, optional verified short paths, labeled generated examples and inferred explanations, and a visibly separate exploratory section. Verified groups retain the exact first-step hydration proof and admit only relations declared by the closed grouping policy; unsupported mechanism and application claims fail closed. A complete semantic scale is included atomically with its admitted taxonomy group. Explicitly requested labeled alternatives are capped at two per lexical unit, bind a stable translation ID and order, and name the changed dimension, practical consequence, and usefulness reason; they are not aliases or normalized lookup forms. Request-local relationship-gap nominations carry no canonical endpoint, relation, or evidence authority, are excluded from the online response, and remain input only to a separate offline review workflow. No additional public relationship-page route exists. The embedded page is emitted only when one atomically configured material authority supplies a matching root, target language, response level, full release pin, and validated material. It reports `complete` whenever verified groups, paths, or evidence-grounded explanations are present, and `canonical_only` only when those relationship sections are absent. `relationship-page-v1` appears in capabilities only while both the translation orchestrator and that authority are installed; builder order cannot advertise a partial runtime. Authority deadline and cancellation outcomes retain the standard closed problem semantics. Structured inputs requesting labeled alternatives fail before generation. The default executable currently omits the material authority because production root-resolution and canonical/retrieval composition is incomplete.

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

The response contains the root, lens, ordered branches, stable nodes, explicit paths, relevance reasons, assertion and evidence references, evidence states, truncation, and an opaque next cursor. Ordered branches partition every non-root item exactly once, and truncation is true exactly when a next cursor is present. The response breadth is always the level bound into the request. Mechanism and application remain unavailable until the canonical registry exposes explicit technical traversal families; a weak topical association cannot establish either lens. Every factual path step is constructed from an exact canonical assertion hydration under the response's full content pin and follows the selected registry traversal from source to target. Reverse traversal requires a separately declared inverse. Cursors bind the root, closed lens, language, response level, release, projection version, and ordering version and contain no request text.

The implemented target cursor foundation uses the opaque `k1.<nonce>.<ciphertext>` format protected with XChaCha20-Poly1305 and distinct authenticated context from the transitional graph cursor. Its encrypted payload binds the typed root family and ID, one of the eight closed lenses, language, response level, complete canonical release/schema pin, immutable node and edge collection IDs and SHA-256 hashes, assertion/registry/projection/lens-policy/ordering versions, stable ordering key, and at most eight bounded dependency continuation tokens. Decoding requires an exact current-request binding match and rejects unknown lens or cursor versions, excessive length, malformed base64, tampering, another key, or changed release/projection data. The payload contains no query text, generated prose, evidence excerpts, credentials, user identity, or durable history. This foundation does not itself expose a route or add application state.

The checked-in handler bundle implements this route together with the path route without adding either to the default runtime composition. It strictly rejects unknown JSON fields, maps the shared request context into the standard success envelope, and applies `Cache-Control: no-store` to success, problem, and wrong-method responses. A well-formed pin that is not the bundle's configured immutable release returns `409 content_release_unavailable`; malformed fields return `422`. The handler decodes a public `k1` cursor against the exact current request and immutable execution binding, passes only its stable final-item ordering key to the application, and re-encrypts the next ordering key for the response. It never forwards a public cursor to retrieval-data or exposes a dependency continuation token. The application exhausts bounded dependency pagination to materialize and deterministically rank the eligible superset before selecting a response-level page, so a resumed page cannot skip a child frontier or change order because of dependency page boundaries. Every serialized verified step includes both the release ID and canonical schema version and fails closed when its relation has no declared projection wire name.

A successful response has the shared envelope and this route-specific shape:

```json
{
  "data": {
    "root": {"kind": "lexical_sense", "id": "sense_sweltering_hot_01"},
    "lens": "contrast",
    "content_release": "knowledge-2026-09",
    "canonical_schema_version": "canonical-v1",
    "branches": [
      {
        "order": 1,
        "reason": "contrast",
        "item_ids": [{"kind": "lexical_sense", "id": "sense_cool_01"}]
      }
    ],
    "items": [
      {
        "node": {"kind": "lexical_sense", "id": "sense_sweltering_hot_01"},
        "order": 1,
        "relevance_reason": "contrast",
        "evidence_state": "verified",
        "path_to_root": null
      }
    ],
    "truncated": false,
    "next_cursor": null
  },
  "meta": {
    "request_id": "01JKNOWLEDGEVIEW0000000000",
    "schema_version": "knowledge-view-result-v1",
    "content_release": "knowledge-2026-09"
  }
}
```

## POST /api/v1/knowledge/paths

Returns up to three independently verified paths of at most three hops between two canonical roots. The server chooses and enforces relation eligibility and does not perform arbitrary-depth or shortest-path inference.

```json
{
  "from": {"kind": "concept", "id": "concept_coriolis_force"},
  "to": {"kind": "concept", "id": "concept_weather_system"},
  "target_language": "en",
  "content_release": "knowledge-2026-09",
  "canonical_schema_version": "canonical-v1"
}
```

The request body and both nested node references reject unknown fields. Node kinds use the closed canonical family catalog; lexeme roots, invented family names, and node identifiers longer than 256 Unicode scalars are rejected. `target_language` is a strict BCP-47 tag, and the immutable release plus canonical schema form the full pin used by the shared request context. A well-formed unavailable pin returns `409 content_release_unavailable`, distinct from malformed input.

```json
{
  "data": {
    "outcome": "connected",
    "from": {"kind": "concept", "id": "concept_coriolis_force"},
    "to": {"kind": "concept", "id": "concept_weather_system"},
    "target_language": "en",
    "paths": [{
      "order": 1,
      "steps": [{
        "edge_id": "edge_weather_17",
        "relationship_revision": 2,
        "assertion_id": "assertion_weather_17",
        "assertion_revision": 3,
        "traversal_id": "traversal_cause_effect",
        "relation": "has_subtype",
        "relation_registry_revision": 1,
        "direction": "forward",
        "source": {"kind": "concept", "id": "concept_coriolis_force"},
        "target": {"kind": "concept", "id": "concept_weather_system"},
        "conditions": [],
        "evidence_ids": ["evidence_weather_4"],
        "content_release": "knowledge-2026-09",
        "canonical_schema_version": "canonical-v1"
      }]
    }]
  },
  "meta": {
    "request_id": "01JPATHREQUEST0000000000000",
    "schema_version": "knowledge-path-result-v1",
    "content_release": "knowledge-2026-09"
  }
}
```

Normal outcomes are `connected` and `no_verified_path`; the latter returns an empty `paths` array in the same success envelope. `no_verified_path` means the bounded search found no path whose every edge had an exact eligible fact revision successfully hydrated in the pinned release; it does not prove that no relationship exists outside the searched bounds, in another release, or in unpublished knowledge. Every path step names one hydrated assertion, declared forward direction, structured conditions, relation relevance, evidence references, and full release pin. Similarity-only candidates never become path steps.

Malformed JSON or content type returns `400 invalid_json`; invalid fields return `422 invalid_knowledge_path_request`; a retired pin returns `409 content_release_unavailable`; dependency and incomplete-search failures return retryable `503` problems; deadline exhaustion returns retryable `504 deadline_exceeded`; and contradictory immutable proof returns `502 invalid_knowledge_proof`. Every success and problem response uses `Cache-Control: no-store`; errors never echo node IDs or dependency payloads.

The former target drafts `POST /api/v1/graph/get` and `POST /api/v1/graph/neighbors` and the transitional `GET /v1/graph...` handlers are removed. Guided views and paths are the only public relationship traversal surface.

## Related documents

- [System design](../transnet.md)
- [Canonical-data interface](canonical-data.md)
- [Retrieval-data interface](retrieval-data.md)
- [Model runtime](../reference/model-runtime.md)
- [Persistence boundaries](../reference/persistence.md)
- [Quality assurance](../guides/quality-assurance.md)
