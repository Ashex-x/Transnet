# Transnet service HTTP interface

中文：[Transnet 服务接口](../../docs_cn/interfaces/port_cn.md)

This document is the primary interface contract for Transnet. Transnet is a shared, user-agnostic translation and relationship-knowledge service: it translates connected text and builds a bounded relationship-centered page around one resolved lexical sense or domain concept. Calling products own their users, private state, and presentation workflows.

Status: target service contract. The current runtime implements parts of this surface with transitional wire shapes, and canonical sense and graph reads are feature-gated. The checked-in OpenAPI file mirrors this target contract; runtime availability remains documented here rather than inferred from the machine contract.

## Service boundary

Transnet accepts no user ID, learner ID, account ID, cookie, end-user bearer token, profile, preference set, saved-item state, mastery state, or personal history. Learning profiles, lessons, exercises, mastery, review scheduling, coaching, progress tracking, writing evaluation, speech, and pronunciation are not Transnet modules.

Source text, lookup text, and optional disambiguating context are request payloads, not user records. They may exist in memory only for the bounded request lifetime and must not be written to MySQL, Qdrant, logs, metrics, traces, caches, or durable queues. A caller that needs personalization supplies only request-scoped linguistic options and owns any association between a response and an end user.

Transnet binds a private address and does not terminate public TLS. Deployment authentication identifies the calling service, never its end user. Outside a single-host loopback deployment, a gateway or service mesh must authenticate callers.

## Shared wire rules

Requests and responses use JSON. Every response returns `X-Request-Id`. Timestamps are UTC RFC 3339 with microsecond precision. IDs are opaque URL-safe strings; clients must not infer type or order from them. Unknown request fields are rejected.

Successful application responses use `data` and `meta`. `meta.request_id` matches the response header. Canonical reads also return the immutable content release used to answer the request.

```json
{
  "data": {},
  "meta": {
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "content_release": "knowledge-2026-09"
  }
}
```

Errors use one safe envelope and never echo request text, context, credentials, provider bodies, vectors, or storage internals.

```json
{
  "error": {
    "code": "invalid_request",
    "message": "The request is invalid.",
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
    "retryable": false,
    "fields": [
      {
        "field": "target_language",
        "message": "A valid BCP 47 language tag is required."
      }
    ]
  }
}
```

Common statuses are `400` malformed JSON or unknown fields, `401` failed deployment authentication, `404` unknown canonical resource, `409` release conflict, `413` body too large, `422` invalid semantic input, `429` bounded capacity, `502` invalid provider result, `503` required dependency unavailable, and `504` deadline exceeded.

For the GET examples below, the request JSON is documentation notation for path and query parameters; GET requests have no JSON body.

## GET /health

Returns process health without probing dependencies or revealing configuration.

Request parameters:

```json
{}
```

Response `200`:

```json
{
  "data": {
    "status": "ok"
  },
  "meta": {
    "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX"
  }
}
```

## GET /livez

Returns success while the process event loop is responsive.

Request parameters:

```json
{}
```

Response `200`:

```json
{
  "data": {
    "status": "ok"
  },
  "meta": {
    "request_id": "req_01K4Z8Q8X2A6B7C4D9E0F3G5HJ"
  }
}
```

## GET /readyz

Returns `200` only when dependencies required by enabled routes are ready. Optional capabilities may be degraded without making the process unready.

Request parameters:

```json
{}
```

Response `200`:

```json
{
  "data": {
    "status": "ok",
    "dependencies": {
      "mysql": "ready",
      "qdrant": "ready",
      "translation_provider": "ready"
    },
    "capabilities": {
      "translation": "available",
      "canonical_lookup": "available",
      "relationship_pages": "available"
    }
  },
  "meta": {
    "request_id": "req_01K4Z8R4CX7E2J6K1M9N3P5Q8S"
  }
}
```

Response `503` uses the standard error envelope with code `not_ready`. It may name a dependency class but must not expose a host, credential, collection name, or provider response.

## POST /translate

Translates bounded text. `source_language` may be `auto`; the remaining options are linguistic instructions for this request only. Dialect, domain, context, audience, purpose, and register affect this response but create no history, translation memory, canonical fact, or reusable profile.

Request:

```json
{
  "text": "That plan is still up in the air.",
  "source_language": "en",
  "target_language": "zh-CN",
  "dialect": "en-US",
  "domain": "general",
  "audience": "general",
  "purpose": "inform",
  "preserve_formatting": true,
  "register": "neutral"
}
```

Response `200`:

```json
{
  "data": {
    "translation": "那个计划仍然悬而未决。",
    "detected_source_language": "en",
    "tips": [
      {
        "kind": "idiom",
        "message": "“up in the air” means undecided rather than physically airborne."
      }
    ]
  },
  "meta": {
    "request_id": "req_01K4Z8S1AK6C8D2F0G4H7J9M3N",
    "model_version": "translate-2026-09"
  }
}
```

`tips` contains at most two one-sentence items and is omitted when it adds no material value. When context is insufficient, `alternative` may contain one clearly labeled translation and reason; otherwise it is omitted. Protected spans, paragraph structure, and formatting are preserved as requested. Any chunk plan or terminology ledger used for long text is discarded with the request.

## POST /v1/lookups

Resolves a word, term, idiom, phrasal verb, or established phrase to one selected canonical lexical sense or domain concept, then composes a concise translation-wiki page around that root. Ambiguity returns ranked candidates or a clarification result. `context`, `domain`, `audience`, `purpose`, `register`, and `dialect` affect sense selection, ranking, and explanation for this request only; they never change canonical identity or relationship facts. `detail` controls response size, not personalization.

Request:

```json
{
  "query": "sweltering",
  "source_language": "en",
  "explanation_language": "zh-CN",
  "dialect": "en-US",
  "domain": "weather",
  "context": "a sweltering afternoon",
  "audience": "general",
  "purpose": "translation",
  "register": "neutral",
  "detail": "full",
  "include": ["meaning", "degree", "contrasts", "collocations", "usage"]
}
```

Response `200`:

```json
{
  "data": {
    "query_analysis": {
      "normalized_form": "sweltering",
      "detected_language": "en",
      "match_class": "exact_canonical"
    },
    "result_mode": "lookup",
    "selected_root": {
      "kind": "sense",
      "sense_id": "sense_sweltering_hot_01",
      "node_id": "node_sweltering_hot_01"
    },
    "domain_assessment": {
      "classification": "general",
      "candidate_domain_ids": ["domain_weather"],
      "reason": "The selected sense describes uncomfortable atmospheric heat."
    },
    "page": {
      "basic_card": {
          "card_id": "card_sweltering_en_adj_01",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_form": "sweltering",
          "part_of_speech": "adjective",
          "definitions": ["uncomfortably hot, especially because of the weather"],
          "translations": ["酷热的", "闷热难耐的"],
          "domain_ids": ["domain_weather"]
      },
      "pronunciations": [
          {
            "dialect": "en-US",
            "ipa": "/ˈswɛltərɪŋ/"
          }
        ],
      "examples": [
          {
            "text": "We waited until evening to leave the sweltering house.",
            "translation": "我们一直等到傍晚才离开闷热难耐的房子。"
          }
        ],
      "relationship_sections": [
        {
          "kind": "degree",
          "title": "Intensity",
          "items": [
          {
            "edge_id": "edge_sweltering_scorching_01",
            "relation_type": "higher_degree",
            "target_node_id": "node_scorching_heat_01",
            "target_label": "scorching",
            "explanation": "Scorching usually expresses a stronger degree of heat.",
            "restrictions": {"dimension": "temperature_intensity"},
            "evidence_state": "verified",
            "confidence": 0.96,
            "provenance": ["evidence_dictionary_1042"]
          }
          ]
        }
      ],
      "connection_paths": [],
      "exploratory_sections": []
    },
    "alternatives": []
  },
  "meta": {
    "request_id": "req_01K4Z8T5BN2P6Q9R1S3V7W0XYZ",
    "content_release": "knowledge-2026-09",
    "degraded": false
  }
}
```

`relationship_sections` are purpose-ranked and omit empty or weakly supported groups. Items use `verified`, `inferred`, or `exploratory` evidence states; inferred and exploratory content is request-local and never silently phrased as canonical fact. Every connection path is short and gives each step a named relationship plus independently eligible evidence. If Qdrant is unavailable but MySQL resolves a basic card, Transnet returns the card with empty relationship and path sections plus `meta.degraded: true`; it never invents replacements.

## GET /v1/senses/{sense_id}

Reads one canonical sense and its concise MySQL card. The service keeps no access or saved-item records.

Request parameters:

```json
{
  "path": {
    "sense_id": "sense_sweltering_hot_01"
  },
  "query": {
    "explanation_language": "zh-CN",
    "release": "knowledge-2026-09"
  }
}
```

Response `200`:

```json
{
  "data": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "aliases": ["oppressively hot"],
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [
      {
        "language": "zh-CN",
        "text": "酷热的"
      }
    ],
    "forms": [
      {
        "form": "swelteringly",
        "label": "adverb"
      }
    ],
    "examples": [
      {
        "text": "We waited until evening to leave the sweltering house.",
        "translation": "我们一直等到傍晚才离开闷热难耐的房子。"
      }
    ],
    "usage_notes": ["Usually describes weather or an uncomfortably hot place."],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"]
  },
  "meta": {
    "request_id": "req_01K4Z8V2DE5F7G9H1J3K6M8NPQ",
    "content_release": "knowledge-2026-09"
  }
}
```

## GET /v1/graph

Reads a bounded canonical subgraph rooted at one sense, concept node, or domain. `depth` is limited to the configured shallow maximum. Results remain rooted, typed, and scope-filtered; this endpoint is not a general graph-query language or an unrestricted neighbor dump.

Request parameters:

```json
{
  "query": {
    "root_kind": "sense",
    "root_id": "sense_sweltering_hot_01",
    "depth": 1,
    "relation_types": ["lower_degree", "higher_degree", "collocation"],
    "verification_state": "verified",
    "node_limit": 20,
    "edge_limit": 30,
    "release": "knowledge-2026-09"
  }
}
```

Response `200`:

```json
{
  "data": {
    "root": {
      "kind": "sense",
      "id": "sense_sweltering_hot_01",
      "node_id": "node_sweltering_hot_01"
    },
    "nodes": [
      {
        "node_id": "node_sweltering_hot_01",
        "node_type": "lexical_sense",
        "label": "sweltering"
      },
      {
        "node_id": "node_scorching_heat_01",
        "node_type": "lexical_sense",
        "label": "scorching"
      }
    ],
    "edges": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree",
        "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
        "restrictions": {"dimension": "temperature_intensity"},
        "evidence_state": "verified",
        "confidence": 0.96,
        "provenance": ["evidence_dictionary_1042"],
        "verification_state": "verified"
      }
    ],
    "truncated": false
  },
  "meta": {
    "request_id": "req_01K4Z8W9RS2T4V6X0Y1Z3A5BCD",
    "content_release": "knowledge-2026-09"
  }
}
```

## GET /v1/graph/nodes/{kind}/{id}/neighbors

Pages direct incoming and outgoing relationships for one canonical node. The cursor is scoped to the root, filters, and release and must not contain request text.

Request parameters:

```json
{
  "path": {
    "kind": "knowledge_node",
    "id": "node_sweltering_hot_01"
  },
  "query": {
    "direction": "both",
    "relation_types": ["lower_degree", "higher_degree"],
    "verification_state": "verified",
    "limit": 10,
    "cursor": null,
    "release": "knowledge-2026-09"
  }
}
```

Response `200`:

```json
{
  "data": {
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_hot_sweltering_01",
          "source_node_id": "node_hot_temperature_01",
          "target_node_id": "node_sweltering_hot_01",
          "relation_type": "higher_degree",
          "explanation": "Sweltering expresses a more uncomfortable degree of heat than hot.",
          "restrictions": {"dimension": "temperature_intensity"},
          "evidence_state": "verified",
          "confidence": 0.96,
          "provenance": ["evidence_dictionary_1042"],
          "verification_state": "verified"
        },
        "node": {
          "node_id": "node_hot_temperature_01",
          "node_type": "lexical_sense",
          "label": "hot"
        }
      }
    ],
    "next_cursor": null
  },
  "meta": {
    "request_id": "req_01K4Z8X6FG1H3J5K7M9N2P4QRS",
    "content_release": "knowledge-2026-09"
  }
}
```

## Related documents

- [System design](../transnet.md)
- [MySQL interface](mysql.md)
- [Qdrant interface](qdrant.md)
- [Current OpenAPI snapshot](../reference/transnet-openapi.json)
