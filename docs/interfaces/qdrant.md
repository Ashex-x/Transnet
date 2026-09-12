# Qdrant adapter interface

中文：[Qdrant 适配器接口](../../docs_cn/interfaces/qdrant_cn.md)

This contract defines the shared translation-wiki knowledge graph stored as versioned Qdrant node and edge collections. Qdrant is rebuildable and contains no learner-owned content.

Status: target contract; the current executable does not compose this adapter.

## Release and collection contract

Each logical knowledge release contains one immutable `knowledge_nodes` collection and one immutable `knowledge_edges` collection. Both collections pin the release ID, embedding models, vector dimensions, sparse configuration, payload schema, and content hashes. They activate and roll back as one unit.

Point IDs are deterministic. Nodes are built before edges. Publication rejects missing endpoints, cross-release references, invalid direction, duplicate typed edges, missing evidence, incompatible senses, unsupported language or domain claims, and mismatched embedding metadata.

Private learner identities, bookmarks, history, queries, passages, writing, answers, conversations, explanations, recordings, mastery, and preferences never enter Qdrant.

## Knowledge nodes

A node represents one independently explainable lexical sense, phrase, concept, entity, phenomenon, idiom, metaphor, speech act, cultural practice, grammar pattern, collocation, or misconception.

```json
{
  "node_id": "node_01J...",
  "node_type": "lexical_sense",
  "canonical_label": "sweltering",
  "aliases": ["oppressively hot"],
  "translations": ["酷热的"],
  "description": "uncomfortably hot, especially because of weather",
  "language": "en",
  "domains": ["weather"],
  "evidence_ids": ["evidence_01J..."],
  "confidence": 0.98,
  "verification_state": "verified",
  "release_id": "knowledge-2026-09"
}
```

Each node has a named dense cross-lingual semantic vector and sparse lexical vector. Payload indexes cover release, state, type, language, dialect, region, period, and domain.

## Knowledge edges

An edge is both a typed connection and a searchable explanation of why two nodes relate.

```json
{
  "edge_id": "edge_01J...",
  "source_node_id": "node_01J...",
  "target_node_id": "node_01K...",
  "relation_type": "intensity_neighbor",
  "direction": "outgoing",
  "explanation": "scorching expresses a stronger degree of heat",
  "restrictions": {"dimension": "temperature", "register": "general"},
  "evidence_ids": ["evidence_01K..."],
  "confidence": 0.96,
  "verification_state": "verified",
  "release_id": "knowledge-2026-09"
}
```

Supported families are naming, lexical, conceptual, contrast, cultural, and exploratory. Exact relation types follow the [system design](../transnet.md). Intensity or gradient relations name their dimension and are not encoded as hypernym or hyponym edges.

Each edge has a dense vector for the complete source–relation–target explanation and a sparse lexical representation. Payload indexes cover both endpoints, relation type, verification state, release, language, region, period, and domain.

## Retrieval

`search_nodes` and `search_edges` combine named dense and sparse retrieval with exact aliases, translations, transliterations, abbreviations, formulas, and domain terms. Eligibility filters apply before limiting. Scores are comparable only inside the same model and release.

`neighbors` retrieves verified incoming and outgoing edges through endpoint filters, then fetches the opposite nodes by ID. The adapter checks release compatibility and endpoint presence but does not infer ontology semantics or factual multi-hop paths.

The caller deduplicates and reranks a bounded candidate set by exactness, evidence, domain relevance, relationship diversity, and ephemeral learner strategy. It must display verified and exploratory results separately. Vector similarity alone never establishes translation, synonymy, hierarchy, causation, shared mechanism, or cultural meaning.

Expansion follows one learner-selected node at a time. Arbitrary-depth traversal, shortest paths, centrality, and mutable graph transactions are outside this contract.

## Reconciliation and lifecycle

Builds compare node and edge counts, endpoint coverage, content hashes, embedding versions, and release metadata with a signed or otherwise authenticated manifest. A partial or mismatched pair never activates.

Quarantine blocks ineligible material before deletion. Correction creates a new immutable release. Rollback selects an unchanged retained node-and-edge pair. Exploratory neighbors remain derived and cannot become verified edges without an evidence-backed publishing decision.

Closed outcomes are `ok`, `missing`, `invalid_payload`, `version_mismatch`, `unavailable`, and `timeout`. Logs omit credentials, vectors, source text, learner content, and raw Qdrant bodies.

## Related documents

- [System design](../transnet.md)
- [MySQL interface](mysql.md)
- [Content publishing](../guides/content-publishing.md)
- [Quality assurance](../guides/quality-assurance.md)
