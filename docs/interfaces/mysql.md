# MySQL adapter interface

This is the typed MySQL 8 adapter contract. JSON examples represent logical adapter values, not a network protocol or stored JSON schema. SQL rows remain private.

## Common contract

Every operation is bounded, uses UTC timestamps, and returns a closed result. Learner operations receive the opaque `X-Learner-Id` value; public canonical operations omit it. Hashes and encrypted values are produced before persistence and are redacted from diagnostics.

    {
      "operation": "saved_sense.upsert",
      "request_id": "01JREQUEST",
      "learner_id": "learner_01",
      "content_version": {
        "release_id": "01JRELEASE",
        "schema_version": "1",
        "ranking_version": "lookup-v1",
        "collection_version": "sense-v3"
      },
      "idempotency": {
        "key_digest": "sha256:BASE64",
        "request_fingerprint": "sha256:BASE64",
        "expires_at": "2026-09-13T10:00:00Z"
      },
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {}
    }

Results are `ok`, `missing`, `conflict`, `in_progress`, `replayed`, `expired`, `lease_lost`, or `unavailable`, with a safe operation-specific value.

    {
      "result": "ok",
      "value": {
        "revision": 4
      }
    }

## Canonical content and graph

| Family | Operations |
| --- | --- |
| Active content | `content.active` |
| Lexical lookup | `lexical.search`, `candidate.load`, `sense_details.load` |
| Graph | `graph.nodes`, `graph.adjacency`, `graph.neighbor_page` |
| Releases | `release.stage`, `release.read`, `release.record_gate`, `release.begin_vector_build`, `release.reconcile_vector_build`, `release.publish`, `release.rollback`, `source.quarantine` |

    {
      "operation": "lexical.search",
      "request_id": "01JREQUEST",
      "content_version": {
        "release_id": "01JRELEASE",
        "schema_version": "1",
        "ranking_version": "lookup-v1",
        "collection_version": "sense-v3"
      },
      "input": {
        "query": "caliente",
        "normalized_query": "caliente",
        "source_language": "es",
        "signals": [
          "exact_form",
          "phrase",
          "lemma",
          "morphology",
          "full_text"
        ],
        "evidence_use": "api_redistribution",
        "limit": 100
      }
    }

    {
      "operation": "release.stage",
      "request_id": "01JREQUEST",
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "release_id": "01JRELEASE",
        "schema_version": "1",
        "ranking_version": "lookup-v1",
        "collection_version": "sense-v3",
        "source_manifest_hash": "sha256:BASE64",
        "sources": [
          {
            "source_id": "dictionary-1",
            "version": "2026-09",
            "permissions": [
              "display",
              "embed",
              "api_redistribution"
            ]
          }
        ],
        "required_gates": [
          "schema",
          "license",
          "quality",
          "vector_reconciliation"
        ]
      }
    }

Pinned reads never mix releases. Publication locks the singleton pointer, verifies gates and sources, changes lifecycle state, retains the predecessor, and writes an outbox event in one transaction. Quarantine blocks affected publication and rollback immediately.

## Learner state and privacy

| Family | Operations |
| --- | --- |
| Profile | `profile.read`, `preferences.replace` |
| History | `history.append`, `history.read`, `history.page`, `history.delete`, `history.clear` |
| Saved senses | `saved_sense.upsert`, `saved_sense.read`, `saved_sense.page`, `saved_sense.mutate`, `saved_sense.delete` |
| Privacy | `privacy.inventory`, `privacy.create`, `privacy.poll`, `privacy.complete`, `privacy.fail`, `privacy.mint_result` |

    {
      "operation": "history.page",
      "request_id": "01JREQUEST",
      "learner_id": "learner_01",
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "before": {
          "occurred_at": "2026-09-11T10:00:00Z",
          "id": "01JLOOKUP"
        },
        "limit": 50
      }
    }

    {
      "operation": "saved_sense.upsert",
      "request_id": "01JREQUEST",
      "learner_id": "learner_01",
      "idempotency": {
        "key_digest": "sha256:BASE64",
        "request_fingerprint": "sha256:BASE64",
        "expires_at": "2026-09-13T10:00:00Z"
      },
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "entry_id": "01JSAVED",
        "sense_id": "01JHOT",
        "state": "learning",
        "note_ciphertext": "BASE64",
        "note_nonce": "BASE64",
        "key_version": 4,
        "expected_revision": 3
      }
    }

Ownership and revision checks occur atomically with mutation. Missing and foreign rows are indistinguishable. History has a non-null expiry and never stores context. Privacy deletion durably records progress and is resumable.

## Feedback, views, and practice

| Family | Operations |
| --- | --- |
| Feedback | `feedback.idempotency`, `feedback.write`, `feedback.current`, `feedback.aggregate` |
| Graph views | `graph_view.create`, `graph_view.read`, `graph_view.page`, `graph_view.replace`, `graph_view.delete` |
| Practice | `practice.session_create`, `session_read`, `exercise_freeze`, `exercise_read`, `claim`, `submission_idempotency`, `submit`, `mastery`, `progress` |

    {
      "operation": "feedback.write",
      "request_id": "01JREQUEST",
      "learner_id": "learner_01",
      "idempotency": {
        "key_digest": "sha256:BASE64",
        "request_fingerprint": "sha256:BASE64",
        "expires_at": "2026-09-13T10:00:00Z"
      },
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "event_id": "01JFEEDBACK",
        "edge_id": "01JEDGE",
        "relation_version": 3,
        "dimension": "accuracy",
        "judgment": "missing_restriction",
        "comment_ciphertext": "BASE64",
        "comment_nonce": "BASE64",
        "key_version": 4
      }
    }

    {
      "operation": "practice.submit",
      "request_id": "01JREQUEST",
      "learner_id": "learner_01",
      "idempotency": {
        "key_digest": "sha256:BASE64",
        "request_fingerprint": "sha256:BASE64",
        "expires_at": "2026-09-13T10:00:00Z"
      },
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "session_id": "01JSESSION",
        "exercise_id": "01JEXERCISE",
        "attempt_id": "01JATTEMPT",
        "resolution": "correct",
        "scheduler_rating": "good",
        "answer_ciphertext": "BASE64",
        "answer_nonce": "BASE64",
        "key_version": 4
      }
    }

Feedback ledger and projection update together. View replacement checks owner and ETag. Practice claim replays the outstanding item; submission appends one attempt and advances counters and mastery exactly once. `needs_review` never advances mastery.

## Jobs and idempotency

| Family | Operations |
| --- | --- |
| Lookup jobs | `lookup_job.create`, `start`, `complete`, `fail`, `poll` |
| Durable queue | `job.enqueue`, `get`, `claim`, `heartbeat`, `complete`, `fail`, `replay` |
| Idempotency | `idempotency.begin`, `complete`, `abandon` |

    {
      "operation": "job.claim",
      "request_id": "01JREQUEST",
      "occurred_at": "2026-09-12T10:00:00Z",
      "input": {
        "worker_id": "worker-3",
        "accepted_kinds": [
          "lookup.generate",
          "privacy.export"
        ],
        "lease_duration_ms": 30000
      }
    }

    {
      "result": "ok",
      "value": {
        "job_id": "01JJOB",
        "kind": "lookup.generate",
        "payload_version": 3,
        "payload_ciphertext": "BASE64",
        "attempt": 1,
        "lease_token": "BASE64",
        "lease_expires_at": "2026-09-12T10:00:30Z"
      }
    }

Jobs persist before external work. Claims use row locking, bounded leases, and worker-bound tokens. Failures store closed codes and retry or become dead. Sensitive payloads are encrypted and erased after completion or expiry.

## Storage rules

Canonical tables cover releases, sources, evidence, lexemes, forms, senses, assertions, relations, and scales. Learner tables cover profiles, preferences, history, saved senses, feedback, graph views, practice, and mastery. Operations tables cover snapshots, lookup jobs, durable jobs, idempotency, outbox, and privacy requests.

Use `utf8mb4`, UTC `DATETIME(6)`, opaque ULID public IDs, numeric internal keys, binary collations for machine values, and indexed ownership foreign keys. JSON is limited to versioned opaque snapshots. Ledgers are append-only and projections transactional. Learner text uses unique authenticated-encryption nonces and versioned keys; equality lookup uses versioned HMACs with secrets outside MySQL. Errors never expose SQL, learner content, credentials, capabilities, hashes, or ciphertext.

