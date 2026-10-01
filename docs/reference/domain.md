# Domain module

中文：[Domain 模块](../../docs_cn/reference/domain_cn.md)

The domain module owns transport-independent translation vocabulary, lexical and canonical knowledge identity, relationship semantics, release compatibility, and degraded-result invariants. It contains no HTTP, provider, database, logging, or process-lifecycle code.

## Translation values

A translation request contains source text, source and target language selectors, response level, and optional chronological minimal history. Language tags are canonicalized and bounded. History is linguistic context, not identity or durable state, and domain values never carry user IDs, persistence policy, provider selection, or storage instructions.

A translation result separates primary translated text from optional ambiguity, register, terminology, or cultural notes. Brief, standard, and full control deterministic breadth after the complete result is assembled. Exact public shapes and limits belong to the [Transnet service interface](../interfaces/transnet.md).

## Lexical identity

A sense or established phrase has a stable canonical ID independent of spelling normalization. Forms, aliases, pronunciation, definitions, grammar, register, morphology, examples, collocations, and restrictions remain attached to the applicable meaning. Different parts of speech or materially different meanings are not collapsed.

Canonical public IDs follow the versioned `canonical-id-v1` policy. Entity-family prefixes distinguish translations, cards, concept roots, and domains, while the remaining opaque value is assigned by the publication workflow rather than derived from a query, normalized form, definition, translation, database row number, or content hash. Corrections retain the stable entity ID and create the next positive immutable revision; published revision content is never changed in place.

A reviewed translation has a stable translation ID, release membership, source and target languages, one positive revision, evidence references, and either an explicit lexical meaning scope or passage scope. Lexical scope retains lexeme ID, independently selectable sense ID, part of speech, compositional-versus-phrase-level status, and an ordered bounded set of canonical domain IDs. These values keep homographs, parts of speech, established phrase meanings, and field-specific senses separate even when their normalized surface forms are equal.

The versioned `translation-source-v1` fingerprint is a private candidate-selection key over NFC source text, source language, and the fingerprint-contract version. Significant symbols and case are preserved, so `C`, `C++`, and `C#` remain different candidates. A fingerprint hit is never identity or proof of equality: application code must compare the returned stored source with the requested source under the same normalization contract before accepting the candidate.

## Knowledge and relationships

Canonical knowledge consists of immutable, release-scoped nodes and atomic facts. Relationships have explicit type, direction, endpoints, applicability, conditions, provenance, evidence, verification state, and revision. Symmetry, inverse projection, and transitivity are declared properties of a relationship type, never guesses from wording.

Semantic scales are named ordered dimensions separate from taxonomy and synonymy. Member positions express order, not equal numeric distance. Derived adjacent-degree edges remain distinguishable from stored canonical edges.

Verified content is published canonical knowledge. Inferred explanations and exploratory candidates exist only for a request and are never persisted as facts. Every displayed node is the selected root or has a useful explicit path to it; similarity cannot prove translation, synonymy, hierarchy, causality, mechanism, or cultural meaning.

## Releases and degradation

One content view consists of a MySQL card release plus paired immutable Qdrant node and edge collections. A request pins the trio once, and every structured and vector read uses it. MySQL or a signed publication artifact is authoritative; Qdrant is a rebuildable projection whose candidates require same-release hydration.

The implemented M3 foundation represents that activation candidate as a `KnowledgeReleaseTrio`: the existing canonical-only `CanonicalReleasePin`, one typed immutable node-collection manifest, one typed immutable edge-collection manifest, and shared dense/sparse embedding revisions. The edge manifest binds to the verified node content hash and carries complete endpoint counts. `ActiveContentVersion` remains only for the older process-local single-index retrieval foundation and is not publication authority; its one `vector_collection_id` must never stand in for the two M3 collections.

The implemented relationship registry freezes every v1 wire name and inverse pair in the retrieval-data contract. Only the taxonomy pair `is_a` / `has_subtype` is transitive; every current v1 relation declares causality not applicable, and every non-taxonomy relation declares transitivity not applicable. Publication fails closed when a declaration contradicts those registered semantics. The target node catalog remains broader than the implemented `Sense`, `Lexeme`, `Construction`, and `Scale` read-model families; phrase, term, concept, entity, and specialist node identities require a publisher-owned canonical entity mapping before they can be added without synthetic IDs.

`PublishedRelationship` is the implemented Stage 2 admission aggregate. It binds the stored relation to the release ownership of both endpoints, the exact declared wire direction and inverse, reviewed M2 evidence lineage, and verified lifecycle state. Its stable `PublishedEdgeIdentity` includes canonical relationship ID and revision and excludes evidence revisions. Evidence changes are bound through a new immutable relationship revision, exact release membership, evidence content hashes, and projection/content hashes. Batch validation sorts by stable identity before checking candidates and uses a separate scope-aware semantic key to reject duplicate typed assertions independently of input order or supplied edge ID.

The Stage 3 preparation foundation converts active authoritative lexemes and senses into deterministic node artifacts, then converts only admitted relationships into edge artifacts after resolving both endpoints from the exact node set. A canonical lemma now carries bounded, sorted `lemma_evidence_ids`; those references support the lemma assertion, do not enter lexeme identity, and must resolve to same-release lineage that permits embedding. Projection admission builds one bounded authoritative aggregate from the lexeme, optional owned sense, active word forms, localized glosses, reviewed lexical translations, and exact evidence lineage. Missing, duplicate, conflicting, dangling, cross-release, unreviewed, or unpermitted material fails closed; passage translations and query/model-derived material are excluded.

Stable point IDs and content hashes use the versioned `knowledge-projection-hash-v1` length-prefixed canonical serialization rather than JSON map order or Debug output. The separate `node-dense-input-v1`, `node-lexical-input-v1`, `edge-dense-input-v1`, and `edge-lexical-input-v1` contracts use UTF-8 NFC, preserve case and technical symbols, encode frozen field order/tags, byte lengths, presence markers, and deterministic list order, and hash dense and lexical families in separate domains. Edge inputs bind the frozen source and target node-input hashes, typed wire relation, admitted scope, and verified evidence metadata; they contain no generated relationship prose. The execution contract fixes the dense family to 1,024-dimensional `Qwen/Qwen3-Embedding-0.6B` while leaving its exact immutable artifact revision deployment-supplied, and fixes the sparse side to the non-neural `transnet-lexical-bm25-v1` encoder. A closed compatibility registry must exactly match artifact/encoder revision, dimensions, vector names, and both node and edge input-spec versions. These remain pre-publication values with no collection ID and no production `Verified` claim. Transnet now has a strict outbound publication client and application reconciliation orchestration, but it still has no embedding execution, Qdrant mutation, island-port publication server, persisted production reconciliation, or activation path.

`knowledge_publication` defines the transport-independent node-first lifecycle, deterministic build/request/batch identities, exact retry classification, collision-free lexical dictionary proofs, persisted collection and publication manifest hashes, and dense/lexical execution receipts. Raw vectors, request IDs, clocks, random values, and storage-generated identities cannot enter canonical hashes. Receipt validation compares independently observed execution metadata against one exact registry entry and fails closed on build, revision, dimensions, encoder, dictionary, input-spec, or count mismatch. `KnowledgePublicationService` drives begin, authoritative status-based resume, bounded node batches, node freeze, bounded edge batches, edge freeze, and reconciliation through `KnowledgePublicationPort` without retaining local progress. Successful reconciliation returns only a typed activation candidate; the service does not execute embeddings, mutate collections, persist authority state, activate, or roll back a release.

Vector failure may yield an explicitly degraded MySQL-backed basic card, but never invented relationships or hidden missing knowledge families. Missing authoritative content or incompatible releases fail safely. Live requests cannot create aliases, cards, facts, domains, edges, revisions, or releases.

Exact persisted payloads belong to the [canonical-data](../interfaces/canonical-data.md) and [retrieval-data](../interfaces/retrieval-data.md) interfaces.

## Verification

Test language and size bounds, history disposal, ambiguity preservation, relationship direction and evidence rules, semantic-scale ordering, release compatibility, same-release hydration, explicit degradation, and rejection of private or persistence fields.
