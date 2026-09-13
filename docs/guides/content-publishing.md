# Publish canonical knowledge content

中文：[发布规范知识内容](../../docs_cn/guides/content-publishing_cn.md)

This guide defines the proposed release workflow for MySQL basic cards and canonical translations plus paired Qdrant knowledge-node and knowledge-edge collections. It is intended for content engineers and release operators.

Status: proposed; the current runtime has no ingestion or publication pipeline.

## Preconditions

Every source has an owner, license and display policy, supported languages and domains, evidence granularity, update cadence, and removal procedure. A release pins normalization, alignment, embedding, schema, and evidence-policy versions.

Input must exclude credentials, user data, and raw production requests. Unreviewed model output may enter only the isolated bootstrap candidate stage described below; it cannot enter a release or become a verified fact without evidence, rights checks, validation, and review.

## Release flow

```mermaid
flowchart LR
  source["Licensed sources"] --> normalize["Normalize senses and concepts"]
  seed["Optional LLM seed generation"] --> candidates["Generated candidate quarantine"]
  candidates --> normalize
  normalize --> cards["Build MySQL basic cards"]
  normalize --> translations["Build canonical translations"]
  normalize --> nodes["Build knowledge nodes"]
  nodes --> edges["Build and validate typed edges"]
  cards --> reconcile["Reconcile roots and manifest"]
  translations --> reconcile
  edges --> reconcile
  reconcile --> evaluate["Evaluate staged release"]
  evaluate --> activate["Activate canonical SQL and vector versions"]
```

## Normalize cards and nodes

Normalize Unicode, language and script tags, language-aware canonical and queried forms, parts of speech, senses, translations, transliterations, pronunciation, morphology, domains, dialect, region, period, and evidence scope. Allocate deterministic IDs before embedding.

Create one concise MySQL `BasicCard` per independently selectable lexical sense. It must remain useful without Qdrant and contains canonical and alias forms, concise translations and definitions, pronunciation and morphology summaries, examples, usage notes, domains, evidence metadata, release state, and knowledge-root IDs.

Create a canonical translation revision only for reviewed, reusable shared content. Record its word, phrase, or passage unit; exact source and target language-tagged text; applicable sense, dialect, register, and domain scope; provenance and evidence; publication-rights assertion; selection reason; review decision; normalizer version; and content hash. Never source candidates from request logs, and never ingest private user saves. Basic cards reference the same published translation identities used by exact translation resolution.

Create domain knowledge profiles and atomic facts before building their vector projections. Each fact has one subject, typed predicate, object or literal value, statement, scope, conditions, evidence, provenance, verification state, and immutable revision. Each profile lists available fact families and honest `seed`, `partial`, or `curated` coverage. Missing families remain explicitly missing.

Create semantic scales independently from taxonomy. A scale names its dimension, direction, conditions, domains, evidence, and ordered sense-qualified members. Positions establish order but not equal distance. Validators reject cycles in taxonomy, inconsistent inverse edges, duplicated scale positions, incompatible member senses, missing evidence, and any conversion between `is_a` and degree relations.

Create Qdrant nodes for independently explainable lexical senses, phrases, terms, concepts, entities, phenomena, mechanisms, processes, equations, quantities, materials, instruments, methods, technologies, applications, standards, organizations, people, places, idioms, metaphors, grammar patterns, collocations, misconceptions, and domains. Aliases, translations, transliterations, romanizations, abbreviations, formulas, and exact technical forms feed the sparse representation; the scoped retrieval description feeds the dense vector.

## Bootstrap with generated candidates

An offline bootstrap job may ask an LLM to propose initial domains, translations, atomic facts, taxonomy links, and semantic scales. Every candidate records model and model version, prompt and schema version, generation time, run ID, confidence, and the exact proposed structure. The candidate begins quarantined and is unavailable to runtime retrieval.

Model output is provenance, not evidence. A candidate advances only after domain deduplication against the active inventory, independent source evidence or an explicitly approved editorial-source policy, publication-rights review, deterministic schema and relationship checks, and reviewer approval. The publisher may reject or edit it into a new revision. Live requests, histories, and provider responses never feed this job automatically.

## Build and validate edges

Build nodes before edges. Each edge names its source and target, relation type and direction, complete relationship explanation, applicable sense and domain, conditions, language, dialect, region, period, evidence state, confidence, provenance, verification state, and release. A versioned relation registry defines inverse, symmetric, transitive, and causal properties; publication does not infer them from labels.

Validation rejects orphan endpoints, cross-release references, invalid direction, duplicate typed edges, missing evidence, incompatible senses, and unsupported language or domain claims. Intensity gradients name their dimension and never masquerade as taxonomy. Embedding neighbors remain exploratory and separate from verified edges.

## Build immutable Qdrant collections

Create one immutable node collection and one immutable edge collection for the release. Both use named dense and sparse vectors and payload indexes required by the [Qdrant contract](../interfaces/qdrant.md). Record dimensions, normalization, embedding models, hashes, schema, counts, and endpoint coverage in the release manifest.

The edge dense vector embeds the complete source–relation–target explanation rather than either endpoint alone. Rebuilding derived exploratory neighbors does not modify verified content.

## Reconcile and evaluate

Reconcile every MySQL knowledge root with the staged node collection and every edge endpoint with the staged node manifest. Compare card, canonical-translation, domain-profile, fact, scale, node, and edge counts, identities, hashes, evidence coverage, embedding versions, and release metadata. Any missing, extra, stale, or incompatible record fails the stage.

Run the [quality-assurance guide](quality-assurance.md) against the exact staged trio. Evaluation covers exact and hybrid sense and concept resolution, cross-language terminology, domain assessment, sense separation, relationship precision, page usefulness, unsupported-path rejection, degraded MySQL-only cards, cultural scope, and latency bounds.

## Activate and roll back

Activate the MySQL card and canonical-translation release plus paired Qdrant node and edge versions as one logical release. Every request pins the same release identity across both stores. A partial build is never visible, and an alias is never the source of version authority.

Rollback selects one unchanged retained trio. Published canonical records are never silently rewritten.

## Correct, quarantine, and remove

Urgent quarantine first makes affected cards, nodes, and edges ineligible, then removes derived vectors and reconciles the release. Transnet has no dependent private records to migrate or regenerate.

A normal correction creates a new immutable release and preserves evidence lineage. A verified edge can be added, changed, or removed only through an evidence-backed publishing decision. Source removal follows the recorded license procedure and includes derived aliases, vectors, edges, examples, and cached artifacts.

## Related documents

- [System design](../transnet.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
- [Quality assurance](quality-assurance.md)
