# Vector data endpoint interface

中文：[向量数据 endpoint 接口](../../docs_cn/interfaces/qdrant_cn.md)

This contract defines island-port's vector and graph HTTP endpoints for versioned canonical nodes and edges. Every operation is JSON over UDS. Endpoint request examples show the `input` object placed inside the common request envelope; response examples are complete bodies. Point examples document island-port's internal projection.

Status: target island-port contract. Transnet contains the typed release-trio and relationship admission foundation plus deterministic pre-publication node/edge build artifacts, but the current executable does not compose a vector client or publisher. Preparation currently projects only authoritative active `Lexeme` and `Sense` records, resolves their embedding-authorized lexical evidence, freezes separate dense and lexical canonical inputs without generating vectors, and builds edges only after every endpoint resolves in the exact node artifact. Construction, scale, and the broader target catalog remain closed until publisher-owned canonical sources are frozen. Island-port/Qdrant collection build, reconciliation, activation, rollback, and production acceptance remain external work.

## Contents

- [Vector data endpoint interface](#vector-data-endpoint-interface)
  - [Contents](#contents)
  - [Endpoint reference](#endpoint-reference)
  - [Storage boundary](#storage-boundary)
  - [Release and collection contract](#release-and-collection-contract)
  - [Knowledge node point](#knowledge-node-point)
  - [Knowledge edge point](#knowledge-edge-point)
  - [Semantic scale point](#semantic-scale-point)
  - [POST /api/v1/nodes/search](#post-apiv1nodessearch)
  - [POST /api/v1/scales/search](#post-apiv1scalessearch)
  - [POST /api/v1/edges/search](#post-apiv1edgessearch)
  - [POST /api/v1/neighbors/search](#post-apiv1neighborssearch)
  - [POST /api/v1/releases/publish](#post-apiv1releasespublish)
  - [Related documents](#related-documents)

## Endpoint reference

Island-port listens on `/run/island-port/island-port.sock` by default and follows the [shared UDS JSON transport](transnet.md). Callers never connect to Qdrant or submit native Qdrant requests; island-port owns collection selection, query construction, credentials, and connection pooling. Only the Transnet runtime and authenticated publication tooling may access the socket. Runtime callers receive search access; publication requires the publisher service account.

Every route uses the shared `/api/v1` prefix. The island-port socket and the resource path identify this vector-data API; callers do not add `data`, `vec`, or a storage-vendor name to the path.

Every exact request body has the shape `{"context": RequestContext, "input": EndpointInput}`. `RequestContext` contains `request_id`, `deadline_at`, `schema_version` set to `vector-data-v1`, and the pinned `content_release` when applicable. Endpoint examples below show only `EndpointInput`. Closed outcomes are `ok`, `missing`, `invalid_payload`, `version_mismatch`, `unavailable`, and `timeout`; publication may also return `conflict`.

## Storage boundary

Qdrant stores relationships between canonical Transnet concepts. It is a rebuildable read projection, while MySQL and authenticated release artifacts remain authoritative.

Qdrant contains no user, learner, account, profile, preference, query, context, source passage, history, saved item, bookmark, practice, answer, mastery, schedule, layout, feedback, recording, or privacy-workflow data. Vectors are produced only from published canonical content and admitted structured relationships. Runtime request text is never embedded or stored.

## Canonical embedding inputs

The frozen input contracts are `node-dense-input-v1`, `node-lexical-input-v1`, `edge-dense-input-v1`, and `edge-lexical-input-v1`. Each uses structured UTF-8 canonical bytes, NFC normalization without NFKC, preserved case and technical symbols, fixed field tags and order, unsigned big-endian byte-length prefixes, explicit one-byte optional presence markers, and deterministically sorted bounded lists. Release, typed identity, input family, and input-spec version are serialized. Dense and lexical hashes use separate versioned domains; neither is the Stage 3 projection content hash or a future persisted collection hash.

Node material is limited to an active release-owned lexeme, its dedicated lemma evidence, an optional active owned sense and definition evidence, active word forms, source-backed localized glosses, reviewed non-passage translations whose meaning scope matches the node, and the exact source/evidence lineage permitting `embedding`. Empty optional lists encode explicit absence. Query-derived aliases, heuristic forms, model output, and unreviewed translations are forbidden.

An edge input contains no publisher-authored or generated explanation prose. It binds the frozen source and target node input hashes, canonical relationship identity and revision, exact typed/wire relation, admitted structured scope, and verified evidence identities, sources, content hashes, and confidence. Island-port resolves the endpoint hashes to the already frozen node inputs before encoding vectors.

Island-port is the embedding authority. `semantic` uses a controlled dense model; `lexical` uses a versioned deterministic lexical encoder. A closed compatibility registry maps an exact model/encoder identity and revision to allowed dimensions and input-spec versions. Missing entries, dimension drift, model revision drift, or input-spec mismatch fail closed. The contract intentionally does not name a production model revision or dimensions until a registry entry is approved.

## Release and collection contract

Each logical knowledge release contains one immutable `knowledge_nodes` collection and one immutable `knowledge_edges` collection. Both pin the release ID, embedding models, vector dimensions, sparse configuration, payload schema, and content hashes. They activate and roll back as one unit.

Point IDs are deterministic. Nodes are built before edges. Publication rejects missing endpoints, cross-release references, invalid directions, duplicate typed edges, missing evidence, incompatible senses, unsupported language or domain claims, and mismatched embedding metadata.

Release manifest example:

```json
{
  "release_id": "knowledge-2026-09",
  "canonical_schema_version": "canonical-v1",
  "collections": {
    "nodes": {
      "collection_id": "knowledge_nodes__knowledge_2026_09",
      "payload_schema_version": "knowledge-graph-v1",
      "content_hash": "sha256:63af5c1e...",
      "point_count": 184220,
      "state": "verified"
    },
    "edges": {
      "collection_id": "knowledge_edges__knowledge_2026_09",
      "payload_schema_version": "knowledge-graph-v1",
      "content_hash": "sha256:b19d28a7...",
      "point_count": 612840,
      "verified_node_content_hash": "sha256:63af5c1e...",
      "state": "verified"
    }
  },
  "embeddings": {
    "dense_model_version": "multilingual-embedding-v4",
    "dense_dimensions": 1536,
    "sparse_model_version": "lexical-sparse-v2"
  },
  "endpoint_coverage": {
    "expected": 1225680,
    "resolved": 1225680
  }
}
```

The physical node and edge collection identifiers are different typed members; an active alias is never accepted as either immutable identifier. The canonical release, both collection manifests, both embedding revisions, and endpoint coverage form one activation candidate. Both collections must be verified, the payload schemas must match, the edge manifest must name the exact verified node hash, and every edge endpoint must resolve in that node collection. A Transnet ranking version is request-time policy and is not part of this authority-owned manifest.

Node projection completes and verifies before edge construction starts. Island-port rejects a missing member, cross-release member, schema or embedding mismatch, count or hash mismatch, incomplete endpoint coverage, or unverified build; no placeholder collection identifier is permitted.

Transnet's pre-publication artifact is intentionally not a collection manifest. It has no physical collection ID and never claims the `verified` collection lifecycle state. Stable point IDs are SHA-256 values over a length-prefixed, versioned canonical serialization of the release, payload schema, typed canonical identity, and point family. Point and build content hashes use the same explicit serialization, sorted typed identities, sorted evidence references, and fixed field order; insertion order, request IDs, timestamps, ranking scores, Debug output, and Qdrant-generated values do not participate. Exact duplicate nodes are collapsed, conflicting duplicates fail closed, Stage 2 rejects duplicate typed relationships, and the edge artifact binds the exact node build hash together with expected and resolved endpoint counts.

## Knowledge node point

A node represents one independently explainable lexical sense, phrase, multilingual term, concept, entity, phenomenon, mechanism, process, equation, quantity, material, instrument, method, technology, application, standard, organization, person, place, idiom, metaphor, grammar pattern, collocation, misconception, or canonical domain.

```json
{
  "id": "node_sweltering_hot_01",
  "vectors": {
    "semantic": "<1536-dimensional canonical-content vector>",
    "lexical": {
      "indices": [1842, 99104],
      "values": [1.0, 0.62]
    }
  },
  "payload": {
    "node_id": "node_sweltering_hot_01",
    "node_type": "lexical_sense",
    "sense_id": "sense_sweltering_hot_01",
    "canonical_label": "sweltering",
    "aliases": ["oppressively hot"],
    "translations": [
      {
        "language": "zh-CN",
        "text": "酷热的"
      }
    ],
    "description": "uncomfortably hot, especially because of the weather",
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "confidence": 0.98,
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

Payload indexes cover release, publication and verification state, node type, sense ID, language, dialect, region, period, domain ID, and evidence ID.

Canonical domain nodes additionally carry a compact knowledge profile so the LLM can distinguish available RAG coverage from an empty result. The profile lists available fact families, supported languages, verified fact count, and `seed`, `partial`, or `curated` coverage. It is release-pinned inventory metadata, not evidence and not a completeness claim.

```json
{
  "node_id": "domain_weather",
  "node_type": "domain",
  "canonical_label": "weather",
  "aliases": ["meteorology context"],
  "description": "Conditions of the atmosphere at a place and time.",
  "inclusion_scope": ["temperature", "precipitation", "wind", "humidity"],
  "exclusion_scope": ["long-term climate classification"],
  "knowledge_profile": {
    "available_fact_families": ["definition", "taxonomy", "terminology", "measurement"],
    "languages": ["en", "zh-CN"],
    "verified_fact_count": 184,
    "coverage_state": "partial"
  },
  "verification_state": "verified",
  "release_id": "knowledge-2026-09"
}
```

## Knowledge edge point

An edge is a typed, searchable connection whose embedding input is derived only from its authoritative endpoints, relation, scope, and verified evidence metadata.

```json
{
  "id": "edge_sweltering_scorching_01",
  "vectors": {
    "semantic": "<1536-dimensional canonical-relationship vector>",
    "lexical": {
      "indices": [1842, 77103, 99104],
      "values": [0.71, 1.0, 0.48]
    }
  },
  "payload": {
    "edge_id": "edge_sweltering_scorching_01",
    "relation_version": 3,
    "fact_id": "fact_sweltering_degree_scorching_01",
    "fact_revision": 1,
    "source_node_id": "node_sweltering_hot_01",
    "target_node_id": "node_scorching_heat_01",
    "relation_type": "higher_degree",
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "conditions": ["temperature describes weather or an environment"],
    "restrictions": {
      "dimension": "temperature_intensity",
      "register": "general"
    },
    "language": "en",
    "domain_ids": ["domain_weather"],
    "evidence_ids": ["evidence_dictionary_1042"],
    "evidence_state": "supported",
    "provenance": ["source_dictionary_2026_01"],
    "confidence": 0.96,
    "verification_state": "verified",
    "assessment_enabled": true,
    "release_id": "knowledge-2026-09"
  }
}
```

Supported families cover lexical naming and translation equivalence; taxonomy and part-whole structure; synonymy, antonymy, contrast, and named intensity dimensions; valency, grammar, collocation, and fixed expressions; morphology; suitability by register, dialect, region, period, scene, and domain; cultural extension; and domain mechanism, causation, dependency, implementation, application, measurement, standardization, and terminology. Exploratory associations remain a separate family. A versioned relation-type registry defines direction, inverse, symmetry, transitivity, and causality; neither the UI nor the LLM infers those properties from wording. Payload indexes cover both endpoints, relation type and version, assessment eligibility, publication and verification state, release, applicable sense, language, dialect, region, period, domain, and evidence ID. Qdrant stores no judgment or aggregate value.

The current Transnet registry freezes only mappings whose exact direction is already normative here: internal `Hypernym` stores broader to narrower and publishes as `has_subtype`; internal `Hyponym` stores narrower to broader and publishes as `is_a`; `LowerDegree` and `HigherDegree` publish as `lower_degree_than` and `higher_degree_than`. Existing graph identities for synonymy, antonymy, translation equivalence, morphology, construction, etymology, and weak association retain their internal direction and inverse rules, but their Qdrant wire names, transitivity, and causality remain unresolved contract gaps. Publication fails closed rather than deriving names from Rust variants or English labels.

Before projection, the implemented Transnet admission boundary requires the declared wire relation and inverse to equal the registry, validates the permitted endpoint kinds, and canonicalizes symmetric endpoints for identity without emitting a second inverse edge. One stable edge identity contains the immutable release, publisher-assigned relationship ID and revision, canonical endpoints, internal relation type, and admitted scope; runtime rank, insertion order, request IDs, timestamps, and Qdrant-generated IDs never participate. Duplicate typed assertions are rejected separately by release, canonicalized endpoints, relation type, and scope even if a publisher supplied different relationship IDs. Evidence revision membership in edge identity remains unresolved and is therefore not guessed.

Admission resolves every evidence ID through the existing canonical evidence lineage. The exact ID set must match, every fragment must belong to the relationship release, both source and fragment permissions must allow storage and embedding, the fragment must be active, and generated evidence must have completed reviewed promotion. Evidence confidence uses the existing closed `High`, `Medium`, and `Low` domain values, so an absent or out-of-range numeric value cannot enter this domain boundary. The current `GraphScope` can safely carry a typed dialect and a bounded nonblank register. Free-text conditions and string domain scope are rejected from M3 projection until canonical condition and domain-ID semantics are frozen.

`is_a` points from a narrower sense to a broader category and `has_subtype` is its inverse. `lower_degree_than` and `higher_degree_than` compare members only within a named compatible dimension. No degree edge implies taxonomy, synonymy, or interchangeability.

## Semantic scale point

A first-class semantic scale is stored as a node projection so one retrieval can return the complete eligible ladder rather than reconstructing it from unrelated pairwise edges. Members are sense-qualified nodes. Positions express order, not equal distance; adjacent degree edges may be derived from this record.

```json
{
  "id": "scale_environmental_heat_intensity_01",
  "vectors": {
    "semantic": "<1536-dimensional scale-description vector>",
    "lexical": {
      "indices": [1842, 77103, 99104],
      "values": [0.7, 1.0, 0.8]
    }
  },
  "payload": {
    "node_id": "scale_environmental_heat_intensity_01",
    "node_type": "semantic_scale",
    "dimension": "environmental_heat_intensity",
    "direction": "increasing",
    "domain_ids": ["domain_weather"],
    "conditions": ["describes weather or an environment"],
    "members": [
      {"node_id": "node_warm_temperature_01", "position": 10},
      {"node_id": "node_hot_temperature_01", "position": 20},
      {"node_id": "node_sweltering_hot_01", "position": 30},
      {"node_id": "node_scorching_heat_01", "position": 40}
    ],
    "evidence_ids": ["evidence_dictionary_1042"],
    "verification_state": "verified",
    "release_id": "knowledge-2026-09"
  }
}
```

## POST /api/v1/nodes/search

Combines named dense and sparse retrieval with exact canonical labels, aliases, translations, transliterations, abbreviations, formulas, and domain terms. The service creates query vectors ephemerally and Qdrant receives no tenant or owner identifier.

Request:

```json
{
  "dense_vector": "<1536-dimensional ephemeral query vector>",
  "sparse_vector": {
    "indices": [1842, 99104],
    "values": [1.0, 0.55]
  },
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "verification_states": ["verified"],
    "node_types": ["lexical_sense", "phrase"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "node_id": "node_sweltering_hot_01",
        "score": 0.93,
        "matched_by": ["dense", "sparse", "canonical_label"],
        "payload": {
          "node_type": "lexical_sense",
          "sense_id": "sense_sweltering_hot_01",
          "canonical_label": "sweltering",
          "verification_state": "verified"
        }
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

Scores are comparable only within the same model and release. Vector similarity is a candidate signal, never proof of translation, synonymy, hierarchy, causation, shared mechanism, or cultural meaning.

## POST /api/v1/scales/search

Finds complete-scale candidates that contain one selected canonical node. This is an index lookup with optional vector ranking; it returns only IDs, positions, and eligibility metadata. Transnet must hydrate the complete scale and its evidence from the SQL endpoint before presenting it as a fact.

Request:

```json
{
  "member_node_id": "node_sweltering_hot_01",
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "verification_states": ["verified"],
    "domain_ids": ["domain_weather"]
  },
  "limit": 5
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "scale_id": "scale_environmental_heat_intensity_01",
        "score": 1.0,
        "member_position": 30,
        "verification_state": "verified",
        "fact_ids": ["fact_sweltering_degree_scorching_01"]
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

The endpoint does not infer a new scale or return an incomplete ladder. A missing result means no eligible published scale was found, not that the selected node has no possible intensity relationship.

## POST /api/v1/edges/search

Searches canonical structured relationships. Eligibility filters apply before limiting, and verified and exploratory results remain separate.

Request:

```json
{
  "dense_vector": "<1536-dimensional ephemeral relationship vector>",
  "sparse_vector": {
    "indices": [77103, 99104],
    "values": [1.0, 0.6]
  },
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "relation_types": ["higher_degree", "lower_degree"],
    "verification_states": ["verified"],
    "languages": ["en"],
    "dialects": ["en-US"],
    "regions": [],
    "periods": ["current"],
    "domain_ids": ["domain_weather"],
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "eligible_evidence_ids": ["evidence_dictionary_1042"]
  },
  "limit": 20
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "score": 0.91,
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree",
        "fact_id": "fact_sweltering_degree_scorching_01",
        "fact_revision": 2,
        "verification_state": "verified"
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

Each result is a candidate pointer. Before a factual explanation, evidence, or provenance is used in a response, Transnet hydrates the referenced fact revision through `POST /api/v1/knowledge-facts/get` for the same release.

## POST /api/v1/neighbors/search

Retrieves direct incoming and outgoing edges through endpoint indexes, then fetches the opposite nodes by ID. It does not infer ontology semantics, synthesize edges, or execute factual multi-hop traversal.

Request:

```json
{
  "node_id": "node_sweltering_hot_01",
  "direction": "both",
  "relation_types": ["higher_degree", "lower_degree", "collocation"],
  "verification_states": ["verified"],
  "languages": ["en"],
  "domain_ids": ["domain_weather"],
  "release_id": "knowledge-2026-09",
  "limit": 20,
  "cursor": null
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_sweltering_scorching_01",
          "source_node_id": "node_sweltering_hot_01",
          "target_node_id": "node_scorching_heat_01",
          "relation_type": "higher_degree",
          "verification_state": "verified"
        },
        "node": {
          "node_id": "node_scorching_heat_01",
          "node_type": "lexical_sense",
          "canonical_label": "scorching"
        }
      }
    ],
    "next_cursor": null
  },
  "release_id": "knowledge-2026-09"
}
```

Expansion remains bounded to one selected root at a time and returns only relationships eligible for that root and request scope. The service may assemble a short path only when every step is a named, independently evidence-eligible edge. Arbitrary-depth traversal, similarity-chain path claims, centrality, and mutable graph transactions are outside this contract.

## POST /api/v1/releases/publish

Publication writes deterministic points to new immutable collections and verifies them before activation. It does not mutate an active collection.

Request:

```json
{
  "manifest": {
    "release_id": "knowledge-2026-10",
    "canonical_schema_version": "canonical-v1",
    "collections": {
      "nodes": {
        "collection_id": "knowledge_nodes__knowledge_2026_10",
        "payload_schema_version": "knowledge-graph-v1",
        "content_hash": "sha256:dd401f2a...",
        "point_count": 184220,
        "state": "verified"
      },
      "edges": {
        "collection_id": "knowledge_edges__knowledge_2026_10",
        "payload_schema_version": "knowledge-graph-v1",
        "content_hash": "sha256:98d3a647...",
        "point_count": 612840,
        "verified_node_content_hash": "sha256:dd401f2a...",
        "state": "verified"
      }
    },
    "embeddings": {
      "dense_model_version": "multilingual-embedding-v4",
      "dense_dimensions": 1536,
      "sparse_model_version": "lexical-sparse-v2"
    },
    "endpoint_coverage": {
      "expected": 1225680,
      "resolved": 1225680
    }
  },
  "idempotency_key": "publish-qdrant-knowledge-2026-10"
}
```

Response:

```json
{
  "request_id": "req_publish_01",
  "schema_version": "vector-data-v1",
  "outcome": "ok",
  "value": {
    "release_id": "knowledge-2026-10",
    "node_collection": "knowledge_nodes__knowledge_2026_10",
    "edge_collection": "knowledge_edges__knowledge_2026_10",
    "node_count": 184220,
    "edge_count": 612840,
    "manifest_hash": "sha256:manifest-771e...",
    "endpoint_coverage": 1.0,
    "validation_state": "ready_for_activation"
  },
  "release_id": "knowledge-2026-10"
}
```

Build reconciliation compares counts, endpoint coverage, content hashes, embedding versions, and release metadata with an authenticated manifest. A partial or mismatched pair never activates. Correction creates a new immutable release; rollback selects an unchanged retained pair.

Publication failures use the closed structured codes `canonical_release_unavailable`, `node_build_unavailable`, `edge_build_unavailable`, `schema_incompatible`, `embedding_metadata_incompatible`, `endpoint_reconciliation_failed`, `hash_or_count_reconciliation_failed`, `incomplete_trio`, `activation_conflict`, `immutable_release_unavailable`, `timeout`, and `dependency_unavailable`. Island-port maps these from build and reconciliation state; callers never classify a message string. A successful publish response is an immutable activation candidate and does not switch the active release.

```json
{
  "request_id": "req_publish_01",
  "schema_version": "vector-data-v1",
  "outcome": "conflict",
  "error": {
    "code": "endpoint_reconciliation_failed",
    "message": "release projection did not pass reconciliation"
  },
  "release_id": "knowledge-2026-10"
}
```

An error has no `value`; success has no `error`. `request_id`, transport `schema_version`, and the selected `release_id` echo the request context and never substitute for the canonical schema or collection payload schema inside the manifest.

General closed outcomes remain `ok`, `missing`, `invalid_payload`, `version_mismatch`, `unavailable`, and `timeout`; publication may also return `conflict` with one of the publication failure codes. Logs omit credentials, vectors, canonical source text, request content, and raw Qdrant bodies.

## Related documents

- [Shared UDS JSON transport and Transnet interface](transnet.md)
- [Target vector collection catalog](tables/vec.md)
- [Transnet design and external interface](../transnet.md)
- [MySQL interface](mysql.md)
- [Content publishing](../guides/content-publishing.md)
- [Quality assurance](../guides/quality-assurance.md)
