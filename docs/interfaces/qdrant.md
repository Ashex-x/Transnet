# Qdrant adapter interface

Qdrant is a rebuildable derived index. MySQL owns canonical truth and the active compatible `(release, collection, schema, ranker)` tuple. JSON examples show typed adapter requests.

## Collections and points

    {
      "operation": "collection.create",
      "request_id": "01JREQUEST",
      "input": {
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "release_id": "01JRELEASE",
        "purpose": "sense",
        "vector": {
          "size": 1024,
          "distance": "Cosine",
          "model": "embed-v5",
          "normalized": true
        },
        "payload_indexes": [
          "release_id",
          "entity_kind",
          "content_language",
          "purpose",
          "status",
          "source_ids"
        ]
      }
    }

    {
      "operation": "points.upsert",
      "request_id": "01JREQUEST",
      "input": {
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "wait": true,
        "points": [
          {
            "id": "7d9d7530-9258-5bc1-a826-37f3ec7d08c1",
            "vector": [
              0.125,
              -0.25,
              0.5
            ],
            "payload": {
              "record_id": "sense:01JHOT:definition:en",
              "entity_id": "01JHOT",
              "entity_kind": "sense",
              "purpose": "canonical_definition",
              "content_language": "en",
              "release_id": "01JRELEASE",
              "content_hash": "sha256:BASE64",
              "embedding_model": "embed-v5",
              "status": "active",
              "source_ids": [
                "dictionary-1"
              ]
            }
          }
        ]
      }
    }

Physical collections are immutable per release and embedding configuration. Point IDs are deterministic. Private learner text, identities, feedback, answers, notes, and unfiltered source documents never enter Qdrant.

## Search

    {
      "operation": "vector.search",
      "request_id": "01JREQUEST",
      "content_version": {
        "release_id": "01JRELEASE",
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "schema_version": "1",
        "ranking_version": "lookup-v1"
      },
      "input": {
        "purpose": "canonical_definition",
        "content_language": "es",
        "query_vector": [
          0.125,
          -0.25,
          0.5
        ],
        "filter": {
          "must": [
            {
              "key": "release_id",
              "match": {
                "value": "01JRELEASE"
              }
            },
            {
              "key": "status",
              "match": {
                "value": "active"
              }
            }
          ],
          "must_not": [
            {
              "key": "source_ids",
              "match": {
                "any": [
                  "quarantined-source"
                ]
              }
            }
          ]
        },
        "limit": 30,
        "with_payload": [
          "record_id",
          "entity_id",
          "entity_kind",
          "content_hash",
          "source_ids"
        ]
      }
    }

    {
      "result": "ok",
      "value": {
        "matches": [
          {
            "record_id": "sense:01JHOT:definition:en",
            "entity_id": "01JHOT",
            "score": 0.8731,
            "content_hash": "sha256:BASE64",
            "source_ids": [
              "dictionary-1"
            ]
          }
        ],
        "collection": "transnet_sense_01JRELEASE_e5_v3"
      }
    }

Limits are 1–100 and eligibility filters apply before limiting. Scores are ranking features, comparable only inside one model and collection. The adapter rejects mismatched releases, dimensions, filters, payloads, and hashes. MySQL hydrates and permission-checks all candidate IDs.

## Reconciliation and lifecycle

    {
      "operation": "collection.reconcile",
      "request_id": "01JREQUEST",
      "input": {
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "release_id": "01JRELEASE",
        "expected": {
          "record_count": 245120,
          "identity_manifest_hash": "sha256:BASE64",
          "content_manifest_hash": "sha256:BASE64"
        },
        "checks": [
          "missing_identity",
          "unexpected_identity",
          "content_hash",
          "release_id",
          "status",
          "vector_size"
        ]
      }
    }

    {
      "result": "ok",
      "value": {
        "record_count": 245120,
        "missing": 0,
        "unexpected": 0,
        "hash_mismatches": 0,
        "ready": true
      }
    }

Builds create a new collection, upsert deterministic points, reconcile every identity and hash, then record readiness in MySQL. Publication changes only the MySQL active tuple. Aliases are operational conveniences, never read authority. Rollback selects an unchanged retained collection.

    {
      "operation": "points.delete",
      "request_id": "01JREQUEST",
      "input": {
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "wait": true,
        "filter": {
          "must": [
            {
              "key": "release_id",
              "match": {
                "value": "01JRELEASE"
              }
            },
            {
              "key": "source_ids",
              "match": {
                "any": [
                  "dictionary-1"
                ]
              }
            }
          ]
        }
      }
    }

    {
      "operation": "collection.delete",
      "request_id": "01JREQUEST",
      "input": {
        "collection": "transnet_sense_01JRELEASE_e5_v3",
        "expected_release_id": "01JRELEASE",
        "reason": "retention_expired"
      }
    }

Quarantine first blocks the source in MySQL, then deletes matching points and reconciles. Collection deletion requires no active or retained rollback reference. Closed results are `ok`, `missing`, `version_mismatch`, `invalid_payload`, `unavailable`, or `timeout`; logs omit credentials, vectors, source text, and Qdrant bodies.
