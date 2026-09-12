# MySQL adapter interface

中文：[MySQL 适配器接口](../../docs_cn/interfaces/mysql_cn.md)

This contract defines logical MySQL 8 operations for shared canonical words, phrases, senses, domains, evidence metadata, and immutable content releases. JSON examples describe typed adapter values, not a network protocol or stored JSON columns.

Status: target contract; the current executable does not compose this adapter.

## Storage boundary

MySQL is the authoritative store for compact, structured lexical content and publication state. It contains no user, learner, account, profile, preference, history, saved item, bookmark, practice, answer, mastery, schedule, graph layout, feedback, privacy request, or ownership record. It also does not retain raw translation text, lookup queries, or disambiguating context.

Allowed Transnet service data includes:

- immutable knowledge releases and compatibility manifests;
- canonical words and phrases, language-tagged forms, aliases, and senses;
- concise definitions, translations, pronunciations, morphology, examples, and usage notes;
- canonical domains and their scope definitions;
- stable references to Qdrant knowledge roots and evidence records;
- publication jobs, validation results, idempotency records, and a Qdrant projection outbox, provided none contains request text or user data.

Use `utf8mb4`, UTC timestamps with microsecond precision, opaque stable public IDs, explicit foreign keys, and immutable published revisions. Credentials and encryption keys remain outside MySQL.

## Common operation envelope

Every adapter operation carries a request ID, deadline, expected schema version, and optionally an immutable content release. Publication mutations also require an idempotency key. Reads return `ok`, `not_found`, `version_mismatch`, `unavailable`, or `timeout`; mutations may additionally return `conflict` or `invalid`.

Errors never expose SQL, credentials, request text, provider bodies, or internal connection details.

Request context:

```json
{
  "request_id": "req_01K4Z8P8Y7D3N5Q2F6M1J9T0VX",
  "deadline_at": "2026-09-12T10:30:05.000000Z",
  "schema_version": "mysql-adapter-v1",
  "content_release": "knowledge-2026-09"
}
```

Closed error response:

```json
{
  "outcome": "version_mismatch",
  "error": {
    "code": "content_release_mismatch",
    "message": "The requested content release is not available.",
    "retryable": false
  }
}
```

## resolve_basic_cards

Normalization belongs to the Transnet runtime. The adapter receives a bounded, ordered set of derived forms; it never receives the raw query, intermediate transformations, or context. Exact canonical and alias forms precede inflection, spelling-correction, and relaxed aliases. Significant symbols remain distinct, so `C`, `C++`, and `C#` cannot collapse into one identity.

Request:

```json
{
  "lookup_forms": [
    {
      "form": "sweltering",
      "match_class": "exact_canonical",
      "rank": 0
    }
  ],
  "normalizer_version": "unicode-nfkc-v2",
  "source_language": "en",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "content_release": "knowledge-2026-09",
  "limit": 5
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "matches": [
      {
        "matched_form": "sweltering",
        "match_class": "exact_canonical",
        "card": {
          "card_id": "card_sweltering_en_adj_01",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_form": "sweltering",
          "language": "en",
          "part_of_speech": "adjective",
          "translations": [
            {
              "language": "zh-CN",
              "text": "酷热的"
            }
          ],
          "definitions": ["uncomfortably hot, especially because of the weather"],
          "knowledge_root_ids": ["node_sweltering_hot_01"],
          "cefr": "B2",
          "domain_ids": ["domain_weather"],
          "revision": 3
        }
      }
    ],
    "alternatives": []
  },
  "content_release": "knowledge-2026-09"
}
```

Uniqueness is enforced by stable form, card, and sense IDs plus published canonical-form and alias rows, never by an ad hoc normalized lookup string. All eligible collisions at the best applicable rank are returned for resolution by the service.

## get_sense

Returns one compact canonical sense revision. Detailed relationship data remains in Qdrant.

Request:

```json
{
  "sense_id": "sense_sweltering_hot_01",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "content_release": "knowledge-2026-09"
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [
      {
        "language": "zh-CN",
        "text": "酷热的"
      }
    ],
    "pronunciations": [
      {
        "dialect": "en-US",
        "ipa": "/ˈswɛltərɪŋ/"
      }
    ],
    "forms": [
      {
        "form": "swelteringly",
        "label": "adverb"
      }
    ],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"],
    "revision": 3
  },
  "content_release": "knowledge-2026-09"
}
```

## resolve_domain

Domains are canonical versioned records, not free-form tags. Resolution first checks published labels and aliases. If more than one scope matches, the adapter returns candidates and the publishing workflow must disambiguate.

Request:

```json
{
  "normalized_labels": ["meteorology", "weather"],
  "scope_key": "earth-atmosphere-weather",
  "content_release": "knowledge-2026-09",
  "limit": 5
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "resolution": "matched",
    "domain": {
      "domain_id": "domain_weather",
      "canonical_label": "weather",
      "definition": "Conditions of the atmosphere at a place and time.",
      "inclusion_scope": ["temperature", "precipitation", "wind", "humidity"],
      "exclusion_scope": ["long-term climate classification"],
      "broader_domain_ids": ["domain_earth_science"],
      "revision": 4
    }
  },
  "content_release": "knowledge-2026-09"
}
```

## create_domain_draft

This publication-only operation creates a domain draft when exact MySQL resolution and bounded Qdrant retrieval find no adequate existing scope. Runtime lookup traffic cannot create domains.

Request:

```json
{
  "domain_id": "domain_urban_climatology",
  "canonical_label": "urban climatology",
  "normalized_label": "urban climatology",
  "scope_key": "urban-atmosphere-climate",
  "definition": "Study of atmospheric conditions and climate processes in urban areas.",
  "inclusion_scope": ["urban heat island", "street-canyon airflow"],
  "exclusion_scope": ["general urban planning"],
  "aliases": ["urban climate science"],
  "broader_domain_ids": ["domain_climatology"],
  "related_candidate_ids": ["domain_urban_planning"],
  "generation": {
    "model_version": "domain-curator-2026-09",
    "prompt_version": "domain-draft-v3",
    "confidence": 0.94
  },
  "idempotency_key": "publish-domain-urban-climatology-v1"
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "domain_id": "domain_urban_climatology",
    "revision": 1,
    "publication_state": "draft",
    "outbox_event_id": "outbox_domain_urban_climatology_01"
  }
}
```

A unique constraint over normalized label and scope key plus a transactional upsert prevents concurrent duplicates. Generated related-domain links are exploratory until an evidence-backed publication decision verifies them.

## stage_card_revision

Stages an immutable word-or-phrase revision and its Qdrant root references. Staging validates all structured fields but does not make content readable from an active release.

Request:

```json
{
  "card": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_form": "sweltering",
    "language": "en",
    "part_of_speech": "adjective",
    "definitions": ["uncomfortably hot, especially because of the weather"],
    "translations": [
      {
        "language": "zh-CN",
        "text": "酷热的"
      }
    ],
    "knowledge_root_ids": ["node_sweltering_hot_01"],
    "domain_ids": ["domain_weather"]
  },
  "target_release": "knowledge-2026-10",
  "source_revision": 3,
  "evidence_ids": ["evidence_dictionary_1042"],
  "idempotency_key": "stage-card-sweltering-r4"
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "card_id": "card_sweltering_en_adj_01",
    "sense_id": "sense_sweltering_hot_01",
    "revision": 4,
    "publication_state": "staged",
    "target_release": "knowledge-2026-10"
  }
}
```

## activate_release

Activation is atomic and references a compatible immutable Qdrant node/edge release. It fails if any card root, domain, evidence record, content hash, or Qdrant manifest is missing or incompatible.

Request:

```json
{
  "release_id": "knowledge-2026-10",
  "expected_active_release": "knowledge-2026-09",
  "mysql_content_hash": "sha256:9c49d7f6...",
  "qdrant_manifest_hash": "sha256:2e17a054...",
  "idempotency_key": "activate-knowledge-2026-10"
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "active_release": "knowledge-2026-10",
    "previous_release": "knowledge-2026-09",
    "activated_at": "2026-10-01T00:00:00.000000Z"
  }
}
```

Quarantine, withdrawal, and correction create new publication state or a new release; published rows are never silently rewritten.

## Related documents

- [Transnet service interface](port.md)
- [Qdrant interface](qdrant.md)
- [Content publishing](../guides/content-publishing.md)
