# Qdrant adapter interface

中文：[Qdrant 适配器接口](../../docs_cn/interfaces/qdrant_cn.md)

This contract defines the shared canonical relationship graph stored as versioned Qdrant node and edge collections. JSON examples describe typed adapter values and point payloads, not a public network API.

Status: target contract; the current executable does not compose this adapter.

## Storage boundary

Qdrant stores relationships between canonical Transnet concepts. It is a rebuildable read projection, while MySQL and authenticated release artifacts remain authoritative.

Qdrant contains no user, learner, account, profile, preference, query, context, source passage, history, saved item, bookmark, practice, answer, mastery, schedule, layout, feedback, recording, or privacy-workflow data. Vectors are produced only from published canonical content and relationship explanations. Runtime request text is never embedded or stored.

## Release and collection contract

Each logical knowledge release contains one immutable `knowledge_nodes` collection and one immutable `knowledge_edges` collection. Both pin the release ID, embedding models, vector dimensions, sparse configuration, payload schema, and content hashes. They activate and roll back as one unit.

Point IDs are deterministic. Nodes are built before edges. Publication rejects missing endpoints, cross-release references, invalid directions, duplicate typed edges, missing evidence, incompatible senses, unsupported language or domain claims, and mismatched embedding metadata.

Release manifest example:

```json
{
  "release_id": "knowledge-2026-09",
  "collections": {
    "nodes": "knowledge_nodes__knowledge_2026_09",
    "edges": "knowledge_edges__knowledge_2026_09"
  },
  "dense_model": "multilingual-embedding-v4",
  "dense_dimensions": 1536,
  "sparse_model": "lexical-sparse-v2",
  "payload_schema_version": "knowledge-graph-v1",
  "node_content_hash": "sha256:63af5c1e...",
  "edge_content_hash": "sha256:b19d28a7..."
}
```

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

## Knowledge edge point

An edge is both a typed connection and a searchable explanation of why two nodes relate.

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
    "source_node_id": "node_sweltering_hot_01",
    "target_node_id": "node_scorching_heat_01",
    "relation_type": "higher_degree",
    "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
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
    "release_id": "knowledge-2026-09"
  }
}
```

Supported families cover lexical naming and translation equivalence; taxonomy and part-whole structure; synonymy, antonymy, contrast, and named intensity dimensions; valency, grammar, collocation, and fixed expressions; morphology; suitability by register, dialect, region, period, scene, and domain; cultural extension; and domain mechanism, causation, dependency, implementation, application, measurement, standardization, and terminology. Exploratory associations remain a separate family. A versioned relation-type registry defines direction, inverse, symmetry, transitivity, and causality; neither the UI nor the LLM infers those properties from wording. Payload indexes cover both endpoints, relation type, publication and verification state, release, applicable sense, language, dialect, region, period, domain, and evidence ID.

## search_nodes

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

## search_edges

Searches canonical relationship explanations. Eligibility filters apply before limiting, and verified and exploratory results remain separate.

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
        "explanation": "Scorching usually expresses a stronger degree of heat than sweltering.",
        "verification_state": "verified"
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

## neighbors

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

## publish_release_projection

Publication writes deterministic points to new immutable collections and verifies them before activation. It does not mutate an active collection.

Request:

```json
{
  "manifest": {
    "release_id": "knowledge-2026-10",
    "payload_schema_version": "knowledge-graph-v1",
    "dense_model": "multilingual-embedding-v4",
    "dense_dimensions": 1536,
    "sparse_model": "lexical-sparse-v2",
    "expected_node_count": 184220,
    "expected_edge_count": 612840,
    "node_content_hash": "sha256:dd401f2a...",
    "edge_content_hash": "sha256:98d3a647..."
  },
  "idempotency_key": "publish-qdrant-knowledge-2026-10"
}
```

Response:

```json
{
  "outcome": "ok",
  "value": {
    "release_id": "knowledge-2026-10",
    "node_collection": "knowledge_nodes__knowledge_2026_10",
    "edge_collection": "knowledge_edges__knowledge_2026_10",
    "node_count": 184220,
    "edge_count": 612840,
    "endpoint_coverage": 1.0,
    "validation_state": "ready_for_activation"
  }
}
```

Build reconciliation compares counts, endpoint coverage, content hashes, embedding versions, and release metadata with an authenticated manifest. A partial or mismatched pair never activates. Correction creates a new immutable release; rollback selects an unchanged retained pair.

Closed outcomes are `ok`, `missing`, `invalid_payload`, `version_mismatch`, `unavailable`, and `timeout`. Logs omit credentials, vectors, canonical source text, request content, and raw Qdrant bodies.

## Related documents

- [Transnet service interface](port.md)
- [MySQL interface](mysql.md)
- [Content publishing](../guides/content-publishing.md)
- [Quality assurance](../guides/quality-assurance.md)
