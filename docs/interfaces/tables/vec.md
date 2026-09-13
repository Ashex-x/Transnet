# Target vector collections

中文：[目标向量集合](../../../docs_cn/interfaces/tables/vec_cn.md)

This catalog defines the target Qdrant collections behind the [vector data endpoint](../qdrant.md). Qdrant is a rebuildable release-pinned projection; MySQL canonical revisions and signed release artifacts remain authoritative.

Status: target schema; the current Transnet executable does not create or activate these collections.

## Collection set

Each content release creates exactly two immutable physical collections:

- `knowledge_nodes__<release>`: canonical nodes and semantic-scale projections.
- `knowledge_edges__<release>`: canonical typed relationship projections.

Stable aliases `knowledge_nodes__active` and `knowledge_edges__active` switch together only after reconciliation. Collection metadata pins release ID, payload schema, dense and sparse model versions, vector dimensions, point count, content hash, and build timestamp. Point IDs are deterministic within their family and release.

There is no user-feedback, judgment-event, judgment-aggregate, or distance collection. Those mutable values belong to island-port SQL storage and are joined after canonical retrieval. This keeps votes out of embeddings and avoids mutating an active knowledge collection.

## knowledge_nodes payload

Required indexed fields are `release_id`, `publication_state`, `verification_state`, `node_type`, `node_id`, `sense_id`, `language`, `dialect`, `region`, `period`, `domain_ids`, and `evidence_ids`. Optional display fields include canonical label, aliases, translations, concise description, and the release-pinned domain knowledge profile.

Semantic-scale points use `node_type: semantic_scale` and additionally contain scale ID, dimension, direction, conditions, ordered sense-qualified members, evidence IDs, and fact IDs. Member position expresses order only.

Every point has a named dense `semantic` vector and named sparse `lexical` vector derived exclusively from published canonical content. Runtime query vectors are ephemeral and never stored.

## knowledge_edges payload

Required indexed fields are `release_id`, `publication_state`, `verification_state`, `edge_id`, `relation_version`, `fact_id`, `fact_revision`, `source_node_id`, `target_node_id`, `relation_type`, `language`, `dialect`, `region`, `period`, `domain_ids`, `applicable_sense_ids`, and `evidence_ids`.

The payload also carries the complete canonical relationship explanation, direction, conditions, restrictions, evidence state, provenance references, confidence, and `assessment_enabled`. `relation_version` matches the relationship entry in MySQL `release_member`. `assessment_enabled` declares target eligibility only; no judgment, count, community score, distance adjustment, user identifier, or aggregate version is stored in Qdrant.

The dense vector embeds the complete source–relation–target explanation. The sparse vector indexes canonical endpoint labels, relation terminology, and reviewed aliases. Similarity proposes candidates and cannot establish or invalidate a relationship.

## Retrieval and distance join

Island-port first resolves the active content release and immutable node/edge aliases. After Qdrant returns eligible canonical candidates, it joins the active `relationship_assessment_projection` by `(content_release, edge_id, relation_version)`. That optimized table keeps anonymous counts and their derived distance in the same immutable aggregate row because both share one key and lifecycle. A missing or below-threshold projection gives zero adjustment. Transnet then orders or lays out edges using effective distance and pins both the content release and aggregate version in response metadata and cursors.

The join cannot add an edge Qdrant did not return, bypass verification filters, change endpoints or relation type, or replace evidence hydration from MySQL. If the aggregate dependency is unavailable, graph reads use base distance, mark the response degraded when the public contract requires it, and never reuse a projection from another relation version or release.

## Publication and rebuild

Publish nodes before edges, validate every edge endpoint against the staged node manifest, and reject cross-release references. Reconciliation checks point counts, endpoint coverage, schema and model versions, vector dimensions, hashes, evidence coverage, and relation-version agreement with MySQL. Both aliases activate atomically with the compatible SQL release.

Corrections build new physical collections. Rollback selects a retained immutable pair. Re-embedding canonical content never copies private judgment or distance data into Qdrant.

## Related documents

- [Vector endpoint contract](../qdrant.md)
- [MySQL schema](sql.sql)
- [Relationship assessment contract](../transnet.md#relationship-assessment-metadata)
- [Content publishing](../../guides/content-publishing.md)
