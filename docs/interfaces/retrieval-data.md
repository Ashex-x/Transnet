# Retrieval-data endpoint interface

中文：[检索数据 endpoint 接口](../../docs_cn/interfaces/retrieval-data_cn.md)

This contract defines island-port's storage-neutral candidate-retrieval and projection HTTP endpoints for versioned canonical nodes and relationships. Every operation is JSON over UDS. Endpoint request examples show the `input` object placed inside the common request envelope; response examples are complete bodies. Point examples describe the target Qdrant implementation without making it part of the wire contract.

Status: target island-port contract with checked-in strict Transnet read and publication clients; neither is composed into the online executable. The read client implements bounded `nodes`, `scales`, `edges`, and direct `neighbors` operations through an injected UDS transport, validates request-context and duplicate release equality, and fails closed on envelope, echo, relation-registry, eligibility, topology, and result-bound violations. A separate strict read-only canonical-data client selects the atomically active canonical/node/edge tuple and reconstructs the exact execution expectation required by those retrieval operations; see `POST /api/v1/knowledge-releases/active` in the canonical-data contract. Its fake transport tests do not implement the island-port server or prove production Qdrant behavior. Transnet also contains the typed release-trio and relationship admission foundation plus deterministic pre-publication node/edge build artifacts. Preparation currently projects only authoritative active `Lexeme` and `Sense` records, resolves their embedding-authorized lexical evidence, freezes separate dense and lexical canonical inputs without generating vectors, and builds edges only after every endpoint resolves in the exact node artifact. Construction, scale, and the broader target catalog remain closed until publisher-owned canonical sources are frozen. The island-port server, production build/status and reconciliation persistence, embedding and lexical-encoder execution, Qdrant collection mutation and verification, activation, rollback, and production acceptance remain external work.

## Contents

- [Retrieval-data endpoint interface](#retrieval-data-endpoint-interface)
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
  - [Internal publication operations](#internal-publication-operations)
  - [Deprecated POST /api/v1/releases/publish](#deprecated-post-apiv1releasespublish)
  - [Related documents](#related-documents)

## Endpoint reference

Island-port listens on `/run/island-port/island-port.sock` by default and follows the [shared UDS JSON transport](transnet.md). Callers never connect to Qdrant or submit native Qdrant requests; island-port owns collection selection, query construction, credentials, and connection pooling. Only the Transnet runtime and authenticated publication tooling may access the socket. Runtime callers receive search access; publication requires the publisher service account.

Every route uses the shared `/api/v1` prefix. The island-port socket and the resource path identify this vector-data API; callers do not add `data`, `vec`, or a storage-vendor name to the path.

Every exact request body has the shape `{"context": RequestContext, "input": EndpointInput}`. `RequestContext` contains `request_id`, `deadline_at`, `schema_version` set to `retrieval-data-v1`, and the pinned `content_release` when applicable. Endpoint examples below show only `EndpointInput`. Closed outcomes are `ok`, `missing`, `invalid_payload`, `version_mismatch`, `unavailable`, and `timeout`; publication may also return `conflict`.

The checked-in client requires one pinned release for all four reads and requires every duplicate `filters.release_id`, top-level input `release_id`, and response `release_id` to equal that context pin. Limits are 1 through 50. Dense vectors contain exactly 1,024 finite values; sparse vectors contain 1 through 4,096 finite values with equal-length, strictly increasing indices. Filter lists contain at most 50 entries, cursors contain at most 512 ASCII-graphic bytes, and request and response bodies are each limited to 1 MiB. Relation filters use only exact frozen registry wire names; aliases and Rust enum spellings fail closed. Edge candidates must echo relationship-registry version `1` exactly.

The closed node-family catalog is `lexical_sense`, `phrase`, `multilingual_term`, `concept`, `entity`, `phenomenon`, `mechanism`, `process`, `equation`, `quantity`, `material`, `instrument`, `method`, `technology`, `application`, `standard`, `organization`, `person`, `place`, `idiom`, `metaphor`, `grammar_pattern`, `collocation`, `misconception`, `domain`, and `semantic_scale`. Unknown families fail before transport when used as filters and fail closed when returned by island-port.

Non-success envelopes use an exact closed outcome/code topology: `missing` -> `not_found`; `invalid_payload` -> `invalid_payload`; `version_mismatch` -> `schema_incompatible | content_release_unavailable`; `unavailable` -> `dependency_unavailable`; and `timeout` -> `timeout`. A success has a value and no error; a failure has an error and no value. Unknown codes, mismatched pairs, or contradictory value/error presence are inconsistent data; human messages never classify failures.

## Storage boundary

Qdrant stores relationships between canonical Transnet concepts. It is a rebuildable read projection, while MySQL and authenticated release artifacts remain authoritative.

Qdrant contains no user, learner, account, profile, preference, query, context, source passage, history, saved item, bookmark, practice, answer, mastery, schedule, layout, feedback, recording, or privacy-workflow data. Vectors are produced only from published canonical content and admitted structured relationships. Runtime request text is never embedded or stored.

## Canonical embedding inputs

The frozen input contracts are `node-dense-input-v1`, `node-lexical-input-v1`, `edge-dense-input-v1`, and `edge-lexical-input-v1`. Each uses structured UTF-8 canonical bytes, NFC normalization without NFKC, preserved case and technical symbols, fixed field tags and order, unsigned big-endian byte-length prefixes, explicit one-byte optional presence markers, and deterministically sorted bounded lists. Release, typed identity, input family, and input-spec version are serialized. Dense and lexical hashes use separate versioned domains; neither is the Stage 3 projection content hash or a future persisted collection hash.

Node material is limited to an active release-owned lexeme, its dedicated lemma evidence, an optional active owned sense and definition evidence, active word forms, source-backed localized glosses, reviewed non-passage translations whose meaning scope matches the node, and the exact source/evidence lineage permitting `embedding`. Empty optional lists encode explicit absence. Query-derived aliases, heuristic forms, model output, and unreviewed translations are forbidden.

An edge input contains no publisher-authored or generated explanation prose. It binds the frozen source and target node input hashes, canonical relationship identity and revision, exact typed/wire relation, admitted structured scope, and verified evidence identities, sources, content hashes, and confidence. Island-port resolves the endpoint hashes to the already frozen node inputs before encoding vectors.

Island-port is the embedding authority. The `semantic` vector uses `Qwen/Qwen3-Embedding-0.6B` with 1,024 dimensions and the applicable dense input specification. Production execution additionally requires an exact immutable artifact revision; a model name, `latest`, branch name, mutable provider alias, or deployment label is not a revision. No production dense registry entry exists until deployment supplies and verifies that immutable revision. The `lexical` vector uses the deterministic `transnet-lexical-bm25` encoder at revision `v1`. A closed compatibility registry maps the dense model family plus exact artifact revision and the lexical encoder identity plus revision to dimensions, vector names, and the applicable node and edge input specifications. Missing entries, dimension drift, revision drift, or input-spec mismatch fail closed.

### Lexical encoder contract

`transnet-lexical-bm25-v1` consumes the decoded textual fields of `node-lexical-input-v1` or `edge-lexical-input-v1`; it never tokenizes the binary framing, opaque IDs, evidence IDs, source IDs, or content hashes. Node text consists of lemma, normalized lemma, optional definition, form values and normalized form values, optional morphology, localized gloss text, and reviewed translation source and target text. For an edge, island-port resolves the frozen source and target node input hashes to those exact node lexical inputs, then adds the closed wire relationship and admitted textual scope values. Missing endpoint input, hash mismatch, or an unsupported input version fails closed.

Text is NFC-normalized exactly once and remains case-sensitive. NFKC, stemming, stop-word removal, locale-dependent case folding, transliteration, and heuristic alias generation are forbidden. A token is a maximal run of Unicode letters, marks, or decimal digits, with ASCII `+` and `#` retained only when directly attached to such a run. All other punctuation and whitespace delimit tokens. Empty tokens are discarded. Consequently `C`, `C++`, and `C#` are three distinct terms; their spelling and symbols are not normalized into one another. A token may contain at most 256 UTF-8 bytes, a point may contain at most 16,384 token occurrences and 4,096 unique terms, and exceeding any bound fails closed.

The release-local lexical dictionary is the sorted set of distinct token UTF-8 byte strings from the complete frozen collection. Sorting is unsigned bytewise order. Index zero is reserved; the first term receives index 1 and subsequent terms receive consecutive `u32` indices. This is a collision-free dictionary assignment, not a truncated token hash. Dictionary overflow, duplicate index assignment, or any dictionary/input disagreement fails closed. The dictionary hash and encoder revision belong to the persisted collection manifest; neither changes a Stage 4 embedding input hash.

Document-side sparse values use `tf * (k1 + 1) / (tf + k1 * (1 - b + b * dl / avgdl))`, with `k1 = 1.2`, `b = 0.75`, the exact token occurrence count as `dl`, and `avgdl` computed after the complete collection is frozen. Repeated occurrences across the frozen text fields count independently; no undocumented field boost is applied. Computation uses IEEE-754 binary64 intermediates and round-to-nearest, ties-to-even conversion to binary32 persisted values. The Qdrant sparse vector named `lexical` must enable its `idf` modifier. IDF is collection/query-time state derived from the persisted collection statistics and is deliberately absent from per-point canonical input bytes and input hashes.

The query-side contract is `query-lexical-input-v1` under encoder identity `transnet-lexical-bm25` revision `v1`. A trusted request-local encoder receives the exact canonical release pin and the already normalized private query, applies the same NFC, case-sensitive tokenization and bounds above, and resolves tokens only through that release's frozen collision-free dictionary. Query values are finite binary32 term frequencies; Qdrant's configured `idf` modifier supplies collection statistics. Unknown tokens are omitted, and a query with no dictionary terms fails encoding instead of inventing an index. The encoder output contains sorted unique indices and never includes raw query text, a query hash, request ID, tenant, user, or history. Transnet passes only the resulting dense and sparse vectors to retrieval-data and discards both with the request. Publication inputs remain separate and unchanged.

Every canonical embedding input is limited to 65,536 final canonical serialized bytes; 65,536 is accepted and 65,537 fails closed before hashing or publication. Publication batches are limited to 256 points and 1,048,576 serialized request bytes, with both limits enforced independently. These are execution bounds, not canonical identity. The four Stage 4 input formats and their hash domains remain unchanged.

The compatibility registry schema records `dense_model_family`, `dense_artifact_revision`, `dense_dimensions`, `dense_vector_name`, `node_dense_input_spec`, `edge_dense_input_spec`, `lexical_encoder_identity`, `lexical_encoder_revision`, `lexical_vector_name`, `node_lexical_input_spec`, and `edge_lexical_input_spec`. Island-port must return the matched registry-entry identity and the observed immutable dense artifact revision in its future build receipt. It must derive that observation from the loaded deployment artifact or provider attestation, not echo the request. A receipt whose observation differs from the registry entry fails closed. The exact Qwen artifact revision and its attestation mechanism remain deployment blockers rather than placeholders in this contract.

The implemented Transnet publication foundation now represents that registry schema, exact execution receipts, collision-free lexical-dictionary manifests, stable build and batch identities, and domain-separated persisted-collection and publication-manifest hashes. The outbound `KnowledgePublicationPort` and strict island-port client implement the publisher-side begin, bounded batch, freeze, reconcile, status, and abort contract over the shared UDS transport. `KnowledgePublicationService` drives begin, authoritative status-based resume, node batches and freeze, edge batches and freeze, and reconciliation without storing local progress or a status cache. Only successful authoritative reconciliation returns a typed `PublicationActivationCandidate`; activation remains a separate mutation performed by an external authenticated publisher or control plane through island-port. An empty registry is the valid predeployment state; every populated entry requires an exact immutable dense artifact revision, so no checked-in entry pretends that a floating model reference is deployable. The client and service are covered by strict fake-transport and orchestration tests only: they do not call an embedding provider, run the lexical encoder, create or mutate a Qdrant collection, implement island-port's publication server or persistence, or activate or roll back a release trio.

The closed build lifecycle is `accepting_nodes` -> `nodes_frozen` -> `accepting_edges` -> `edges_frozen` -> `reconciling` -> `activation_candidate`. A nonterminal build may instead enter `failed` or `aborting`; `aborting` proceeds only to `abandoned`, and `failed` or `abandoned` may proceed only to `gc_eligible`. Terminal failure and abandonment cannot recover in place or become activation candidates. Repeating a completed finalize or reconciliation operation is a transport-level idempotent replay, not a second lifecycle transition.

Build identity is derived from immutable release, projection schema, node projection hash, and compatibility-registry entry identity. Batch identity additionally binds collection family, consecutive ordinal, canonical request fingerprint, and ordered batch content hash. An exact retry replays its stored result; reuse of the same build and ordinal with another fingerprint or content hash fails closed. Request ID, clock time, insertion order, randomness, Qdrant-generated values, and raw vector bytes never define canonical publication identity.

The hash hierarchy is distinct: canonical embedding input hash -> projection content hash -> persisted collection hash -> publication manifest hash. A persisted collection hash binds collection family, release, projection schema and hash, sorted point identities and point projection hashes, dense and lexical input hashes, exact compatibility entry, vector names, dimensions, lexical dictionary hash and cardinality, and point count. It excludes raw dense and sparse vector bytes. A publication manifest hash binds canonical release and schema to different node and edge persisted collection hashes. The eventual wire locations for these typed values remain part of the publication transport contract.

## Release and collection contract

Each logical knowledge release contains one immutable `knowledge_nodes` collection and one immutable `knowledge_edges` collection. Both pin the release ID, embedding models, vector dimensions, sparse configuration, payload schema, and content hashes. They activate and roll back as one unit.

Online callers obtain collection identity only through the atomic [active knowledge release read](canonical-data.md#post-apiv1knowledge-releasesactive). A successful value supplies the full canonical pin, distinct immutable node and edge collection IDs, exact collection hashes, relation-registry revision, and edge input versions. Search callers then require the retrieval endpoint's independently observed execution proof to equal that expectation. Absence, partial publication, unverified state, stale hashes, aliases, or version drift is not readiness and never falls back to whichever collection happens to answer.

Point IDs are deterministic. Nodes are built before edges. Publication rejects missing endpoints, cross-release references, invalid directions, duplicate typed edges, missing evidence, incompatible senses, unsupported language or domain claims, and mismatched embedding metadata.

Release manifest example:

The placeholder dense artifact revision below demonstrates the required field only. It is invalid for production publication until replaced by the deployed immutable revision and matched by the closed registry.

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
    "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
    "dense_artifact_revision": "<deployment-supplied-immutable-revision>",
    "dense_dimensions": 1024,
    "sparse_encoder_identity": "transnet-lexical-bm25",
    "sparse_encoder_revision": "v1"
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
    "semantic": "<1024-dimensional canonical-content vector>",
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
    "semantic": "<1024-dimensional canonical-relationship vector>",
    "lexical": {
      "indices": [1842, 77103, 99104],
      "values": [0.71, 1.0, 0.48]
    }
  },
  "payload": {
    "edge_id": "edge_sweltering_scorching_01",
    "relation_version": 3,
    "relation_registry_version": 1,
    "fact_id": "fact_sweltering_degree_scorching_01",
    "fact_revision": 1,
    "source_node_id": "node_sweltering_hot_01",
    "target_node_id": "node_scorching_heat_01",
    "relation_type": "higher_degree_than",
    "applicable_sense_ids": ["sense_sweltering_hot_01"],
    "conditions": [{"condition_id": "condition_environmental_weather_01", "condition_type": "usage_context", "parameter_ids": ["context_environment", "context_weather"]}],
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

The implemented v1 wire registry freezes these names and inverses: `synonym`, `near_synonym`, `translation_equivalent`, `antonym`, `confusable_with`, `associated_with`, and `derivationally_related_to` are symmetric and self-inverse; `has_subtype` / `is_a`, `has_part` / `part_of`, `inflection_of` / `has_inflection`, `etymologically_derived_from` / `etymological_source_of`, `member_of_construction` / `has_construction_member`, `scale_contains` / `member_of_scale`, and `lower_degree_than` / `higher_degree_than` are directed inverse pairs. Only `is_a` and `has_subtype` declare taxonomy transitivity. All other listed relations declare transitivity not applicable. None of these v1 relations is causal; only a separately registered explicit causation relation may declare causality, and every non-causation relation declares causality not applicable. Publication fails closed for a name, inverse, endpoint family, or property not present in the pinned registry rather than deriving semantics from Rust variants or labels.

Before projection, the implemented Transnet admission boundary requires the declared wire relation and inverse to equal the registry, validates the permitted endpoint kinds, and canonicalizes symmetric endpoints for identity without emitting a second inverse edge. One stable edge identity contains the immutable release, publisher-assigned relationship ID and relationship revision, canonical endpoints, internal relation type, and admitted scope; runtime rank, insertion order, request IDs, timestamps, Qdrant-generated IDs, and evidence revisions never participate. Evidence revisions are instead bound through the immutable relationship revision, exact release membership, verified evidence content hashes, and projection/content hashes. Duplicate typed assertions are rejected separately by release, canonicalized endpoints, relation type, and scope even if a publisher supplied different relationship IDs.

Admission resolves every evidence ID through the existing canonical evidence lineage. The exact ID set must match, every fragment must belong to the relationship release, both source and fragment permissions must allow storage and embedding, the fragment must be active, and generated evidence must have completed reviewed promotion. Evidence confidence uses the existing closed `High`, `Medium`, and `Low` domain values, so an absent or out-of-range numeric value cannot enter this domain boundary. Domain scope uses sorted, unique canonical `DomainId` values from the same release; labels and free-form strings are never domain identity. Each condition is a structured registry-owned object with `condition_id`, `condition_type`, and sorted `parameter_ids`; all three are canonical identifiers resolved in the same release, and prose belongs only in hydrated display data. The transitional `GraphScope.domain` and `GraphScope.note` strings therefore remain inadmissible to publication.

The implemented assertion admission model preserves every documented node family as a typed publisher-owned canonical ID but does not claim a Qdrant node mapping for families the current node projector cannot reconstruct. N-ary participants retain registry-owned role and ordinal plus exactly one entity or typed literal. Domains, conditions, and parameters must resolve to explicit same-release registry records. A binary edge is eligible only when the pinned registry explicitly declares a traversal from two required singleton entity roles and the publisher's binary record independently echoes the exact assertion identity and revision. Transnet verifies role-to-endpoint families and IDs, exact release and full evidence lineage, and the existing binary relation rule. The edge projection retains and hashes the assertion/registry/traversal reference. Because the current edge point cannot yet encode structured domain and condition scope, any nonempty structured scope fails closed instead of being omitted. Unknown or cross-release registry records, unsupported conditions or parameters, role/order violations, endpoint mismatches, and altered evidence metadata are rejected rather than synthesizing an ID, endpoint, inverse, or edge.

`is_a` points from a narrower sense to a broader category and `has_subtype` is its inverse. `lower_degree_than` and `higher_degree_than` compare members only within a named compatible dimension. No degree edge implies taxonomy, synonymy, or interchangeability.

## Semantic scale point

A first-class semantic scale is stored as a node projection so one retrieval can return the complete eligible ladder rather than reconstructing it from unrelated pairwise edges. Members are sense-qualified nodes. Positions express order, not equal distance; adjacent degree edges may be derived from this record.

```json
{
  "id": "scale_environmental_heat_intensity_01",
  "vectors": {
    "semantic": "<1024-dimensional scale-description vector>",
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
    "conditions": [{"condition_id": "condition_environmental_weather_01", "condition_type": "usage_context", "parameter_ids": ["context_environment", "context_weather"]}],
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
  "dense_vector": "<1024-dimensional ephemeral query vector>",
  "sparse_vector": {
    "indices": [1842, 99104],
    "values": [1.0, 0.55]
  },
  "execution": {
    "dense_artifact_revision": "sha256:qwen-node-query-r1",
    "dense_input_specification": "node-dense-input-v1",
    "lexical_encoder_identity": "transnet-lexical-bm25",
    "lexical_encoder_revision": "v1",
    "lexical_input_specification": "node-lexical-input-v1"
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
  "request_id": "01K6G7R1S8Z3Q4P5T6V7W8X9Y0",
  "schema_version": "retrieval-data-v1",
  "outcome": "ok",
  "value": {
    "execution": {
      "collection_id": "knowledge_nodes__knowledge_2026_09",
      "collection_content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "dense_artifact_revision": "sha256:qwen-node-query-r1",
      "dense_input_specification": "node-dense-input-v1",
      "lexical_encoder_identity": "transnet-lexical-bm25",
      "lexical_encoder_revision": "v1",
      "lexical_input_specification": "node-lexical-input-v1"
    },
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

The request binds the exact query-side dense artifact and frozen dense/lexical input contracts. The response must identify the immutable physical node collection, provide its exact SHA-256 content hash, and echo every execution member exactly; missing proof, malformed hash, or version drift fails the whole response closed. `matched_by` accepts only `dense`, `sparse`, `canonical_label`, `alias`, `translation`, `transliteration`, `abbreviation`, `formula`, and `domain_term`; duplicate mechanisms fail closed and never inflate rank. Scores are comparable only within that proven model, collection, and release. Vector similarity is a candidate signal, never proof of translation, synonymy, hierarchy, causation, shared mechanism, or cultural meaning.

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
  "request_id": "01K6G7R1S8Z3Q4P5T6V7W8X9Y0",
  "schema_version": "retrieval-data-v1",
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
  "dense_vector": "<1024-dimensional ephemeral relationship vector>",
  "sparse_vector": {
    "indices": [77103, 99104],
    "values": [1.0, 0.6]
  },
  "filters": {
    "release_id": "knowledge-2026-09",
    "publication_states": ["published"],
    "relation_types": ["higher_degree_than", "lower_degree_than"],
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
  "request_id": "01K6G7R1S8Z3Q4P5T6V7W8X9Y0",
  "schema_version": "retrieval-data-v1",
  "outcome": "ok",
  "value": {
    "candidates": [
      {
        "edge_id": "edge_sweltering_scorching_01",
        "score": 0.91,
        "source_node_id": "node_sweltering_hot_01",
        "target_node_id": "node_scorching_heat_01",
        "relation_type": "higher_degree_than",
        "relation_registry_version": 1,
        "assertion_id": "assertion_sweltering_degree_scorching_01",
        "assertion_revision": 2,
        "relationship_revision": 3,
        "relation_type_id": "relation_degree",
        "traversal_id": "traversal_higher_degree",
        "verification_state": "verified"
      }
    ]
  },
  "release_id": "knowledge-2026-09"
}
```

Each result is a candidate pointer. Before factual use, Transnet hydrates the exact relationship and assertion revisions and validates its selected registry traversal through `POST /api/v1/assertions/get` for the same release. Similarity remains nomination only.

## POST /api/v1/neighbors/search

Retrieves direct incoming and outgoing edges through endpoint indexes, then fetches the opposite nodes by ID. It does not infer ontology semantics, synthesize edges, or execute factual multi-hop traversal.

Request:

```json
{
  "node_id": "node_sweltering_hot_01",
  "direction": "both",
  "relation_types": ["higher_degree_than", "lower_degree_than", "near_synonym"],
  "verification_states": ["verified"],
  "languages": ["en"],
  "domain_ids": ["domain_weather"],
  "release_id": "knowledge-2026-09",
  "execution": {
    "content_release": "knowledge-2026-09",
    "canonical_schema_version": "canonical-v1",
    "node_collection_id": "knowledge_nodes__knowledge_2026_09",
    "node_collection_content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "edge_collection_id": "knowledge_edges__knowledge_2026_09",
    "edge_collection_content_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "relationship_registry_version": 1,
    "edge_dense_input_version": "edge-dense-input-v1",
    "edge_lexical_input_version": "edge-lexical-input-v1"
  },
  "limit": 20,
  "cursor": null
}
```

Response:

```json
{
  "request_id": "01K6G7R1S8Z3Q4P5T6V7W8X9Y0",
  "schema_version": "retrieval-data-v1",
  "outcome": "ok",
  "value": {
    "execution": {
      "content_release": "knowledge-2026-09",
      "canonical_schema_version": "canonical-v1",
      "node_collection_id": "knowledge_nodes__knowledge_2026_09",
      "node_collection_content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "edge_collection_id": "knowledge_edges__knowledge_2026_09",
      "edge_collection_content_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "relationship_registry_version": 1,
      "edge_dense_input_version": "edge-dense-input-v1",
      "edge_lexical_input_version": "edge-lexical-input-v1"
    },
    "root_node_id": "node_sweltering_hot_01",
    "neighbors": [
      {
        "edge": {
          "edge_id": "edge_sweltering_scorching_01",
          "source_node_id": "node_sweltering_hot_01",
          "target_node_id": "node_scorching_heat_01",
          "relation_type": "higher_degree_than",
          "relation_registry_version": 1,
          "assertion_id": "assertion_sweltering_degree_scorching_01",
          "assertion_revision": 2,
          "relationship_revision": 3,
          "relation_type_id": "relation_degree",
          "traversal_id": "traversal_higher_degree",
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

Expansion remains bounded to one selected root at a time and returns only relationships eligible for that root and request scope. The request and response bind the full content release/schema pin, immutable node and edge collection IDs and SHA-256 content hashes, registry revision, and edge dense/lexical input versions; any missing or changed member fails closed. Every neighbor edge carries exact relationship and assertion revisions, relation type identity, traversal identity, endpoints, and registry version `1`, forming the exact assertion projection reference Transnet must hydrate before treating the edge as a verified path step. A missing assertion, substituted revision, unknown traversal/relation, endpoint disagreement, or registry mismatch invalidates the candidate. The service may assemble a short path only when every step is a named, independently evidence-eligible edge. Arbitrary-depth traversal, similarity-chain path claims, centrality, and mutable graph transactions are outside this contract.

## Internal publication operations

The former single-body `/api/v1/releases/publish` proposal is replaced by the publisher-only island-port operations `POST /api/v1/knowledge-publications/begin`, `nodes/batch`, `nodes/freeze`, `edges/batch`, `edges/freeze`, `reconcile`, `status`, and `abort`. They use transport schema `knowledge-publication-v1`, the shared `{ "context": ..., "input": ... }` envelope, an RFC 3339 deadline, request ID, and immutable `content_release`. Unknown fields, malformed bodies, mismatched echoed context, or unknown and contradictory outcome/code pairs fail closed.

Begin binds the stable build ID, canonical and projection schemas, complete node and edge projection hashes, exact compatibility entry, expected counts, caller idempotency key, and canonical request fingerprint. Each family-local batch contains at most 256 ordered points and at most 1 MiB of serialized JSON. Its zero-based ordinal is consecutive, and its identity binds the build, ordinal, request fingerprint, and canonical batch content hash. The transport accepts at most 128 ASCII-graphic request-ID bytes and 64 RFC 3339 deadline bytes. Before mutation, the application obtains the largest fitting prefix through the port's pure wire-admission inspection. Inspection and send share the exact DTO, JSON, and base64 serializer; inspection uses the real release and payload with a worst-case legal transport context, so request IDs and deadlines never change deterministic boundaries. A single oversized point fails before `begin` or batch submission. The adapter repeats the 1 MiB validation immediately before transport. Control requests and every response are limited to 256 KiB. Identical retries replay the stored result; reuse of an identity with different canonical content returns `conflict` with `idempotency_conflict`.

Node freeze precedes edge admission. A freeze response supplies an island-port-allocated immutable collection ID, persisted collection hash, projection hash and count, plus dense and lexical execution receipts. Requested compatibility is not execution proof: Transnet compares the server assertion for the exact dense artifact revision, dimensions, vector and input-spec names, lexical encoder revision, dictionary hash, and processed count. The production provenance behind that assertion remains an island-port deployment responsibility.

Reconcile binds both immutable collection IDs, both projection and persisted hashes, the edge-to-node projection binding, point and endpoint counts, validated receipts, and the publication manifest hash. Only complete endpoint coverage and exact cross-artifact agreement produce the closed `activation_candidate` state. This operation does not mutate the active release. Status is read-only; abort follows the domain state machine and cannot leave or abort an activation candidate.

The returned candidate also binds stable build and reconciliation identities plus the storage-neutral canonical-content hash. The build identity commits to that canonical hash; reconciliation and publication-manifest identities additionally commit to both typed physical collection IDs and their persisted hashes. Changing any frozen proof member changes the corresponding identity. The separate offline `release-control-v1` client may explicitly submit that complete proof; reconciliation itself never calls release control. Rollback selection names a previously verified retained manifest rather than rebuilding or mutating either collection.

```json
{
  "context": {
    "request_id": "req_publish_01",
    "deadline_at": "2026-10-01T12:00:00Z",
    "schema_version": "knowledge-publication-v1",
    "content_release": "knowledge-2026-10"
  },
  "input": {
    "build_id": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "canonical_schema_version": "canonical-v1",
    "projection_schema_version": "knowledge-graph-v1",
    "node_projection_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "edge_projection_hash": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "compatibility": {
      "entry_id": "deployment-qwen-r1",
      "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
      "dense_artifact_revision": "deployment-supplied-immutable-revision",
      "dense_dimensions": 1024,
      "dense_vector_name": "semantic",
      "node_dense_input_specification": "node-dense-input-v1",
      "edge_dense_input_specification": "edge-dense-input-v1",
      "lexical_encoder_identity": "transnet-lexical-bm25",
      "lexical_encoder_revision": "v1",
      "lexical_contract_identity": "transnet-lexical-bm25-v1",
      "lexical_vector_name": "lexical",
      "node_lexical_input_specification": "node-lexical-input-v1",
      "edge_lexical_input_specification": "edge-lexical-input-v1"
    },
    "expected_node_count": 184220,
    "expected_edge_count": 612840,
    "expected_endpoint_count": 1225680,
    "idempotency_key": "publish-knowledge-2026-10",
    "request_fingerprint": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  }
}
```

Status-shaped success responses return the echoed build ID, one closed lifecycle state, and the next expected ordinal for each family. Closed lifecycle states are `accepting_nodes`, `nodes_frozen`, `accepting_edges`, `edges_frozen`, `reconciling`, `activation_candidate`, `failed`, `aborting`, `abandoned`, and `gc_eligible`. Closed outcomes are `ok`, `missing`, `invalid_payload`, `version_mismatch`, `conflict`, `unavailable`, and `timeout`; each non-success outcome must carry its compatible structured publication failure code. Human messages are never classified.

## Deprecated POST /api/v1/releases/publish

This earlier one-body target example is retained only as historical context. It is not implemented and must not be used by new publisher clients; the bounded operations above replace it. Publication writes deterministic points to new immutable collections and verifies them before activation. It does not mutate an active collection.

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
      "dense_model_family": "Qwen/Qwen3-Embedding-0.6B",
      "dense_artifact_revision": "<deployment-supplied-immutable-revision>",
      "dense_dimensions": 1024,
      "sparse_encoder_identity": "transnet-lexical-bm25",
      "sparse_encoder_revision": "v1"
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

Publication failures use the closed structured codes `canonical_release_unavailable`, `node_build_unavailable`, `edge_build_unavailable`, `schema_incompatible`, `embedding_metadata_incompatible`, `invalid_lifecycle_transition`, `idempotency_conflict`, `artifact_revision_mismatch`, `lexical_encoder_mismatch`, `dictionary_mismatch`, `endpoint_reconciliation_failed`, `hash_or_count_reconciliation_failed`, `incomplete_trio`, `activation_conflict`, `immutable_release_unavailable`, `timeout`, and `dependency_unavailable`. Island-port maps these from build and reconciliation state; callers never classify a message string. A successful publish response is an immutable activation candidate and does not switch the active release.

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
- [Target Qdrant implementation](tables/qdrant.md)
- [Transnet design and external interface](../transnet.md)
- [Canonical-data interface](canonical-data.md)
- [Content publishing](../guides/content-publishing.md)
- [Quality assurance](../guides/quality-assurance.md)
