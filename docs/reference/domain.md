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

Vector failure may yield an explicitly degraded MySQL-backed basic card, but never invented relationships or hidden missing knowledge families. Missing authoritative content or incompatible releases fail safely. Live requests cannot create aliases, cards, facts, domains, edges, revisions, or releases.

Exact persisted payloads belong to the [SQL](../interfaces/mysql.md) and [vector](../interfaces/qdrant.md) interfaces.

## Verification

Test language and size bounds, history disposal, ambiguity preservation, relationship direction and evidence rules, semantic-scale ordering, release compatibility, same-release hydration, explicit degradation, and rejection of private or persistence fields.
