# Transnet service behavior

中文：[Transnet 服务行为](../../docs_cn/product/service-behavior_cn.md)

This guide describes consumer-visible behavior of the stateless Transnet service. Product applications own their user experience and state. They may send current text and selected prior translation turns, but never user identity or product-owned state.

## Translation

`POST /transnet/v1/translations` is the only new-turn entry point. Besides the text, the user chooses only source language, target language, and `brief`, `standard`, or `full` response level. Island-port may append a chronological list of minimal prior source/translation pairs; there is no separate history-count limit within the common body bound. Transnet automatically selects lexical lookup, domain expansion, or passage translation and derives all other options.

The response contains an ordered translation list. A word or phrase returns several meaning-specific translations when materially different senses remain plausible; history influences their ranking. A sentence or passage keeps natural translated text primary and preserves meaning, tone, terminology, and paragraph structure. Response level selects fields from one canonical superset: brief keeps essential translations and meaning labels, standard adds concise supporting detail, and full adds bounded lexical, domain, relationship, evidence, taxonomy, and intensity detail.

## Lexical and concept detail

The service automatically resolves words, established lexical phrases, and specialist terms to canonical senses or concepts. Homographs, parts of speech, phrase-level meanings, and field-specific senses remain separate. Direct sense and graph reads support follow-up navigation from returned canonical IDs; they are not choices the WebUI asks a user to make for the initial translation.

## Relationship and domain exploration

Canonical cards anchor bounded Qdrant relationship reads. The page exposes purpose-ranked groups of typed relationships, explanations, conditions, provenance, evidence state, and release metadata. For general vocabulary, the neighborhood can include taxonomy, intensity, contrasts, syntax, collocation, register, morphology, and cultural extension. For a domain-specific concept, it can additionally include multilingual terminology, fields and subfields, mechanisms, prerequisites, phenomena, equations, technologies, applications, measurements, standards, and professional usage conventions.

Transnet retrieves existing domain names, scope, and RAG coverage before assessment. The LLM may select only supplied domain IDs; if none fits, it may return a request-local new-domain proposal without calling a create endpoint. Catalog failure yields uncertainty, not a new domain. For selected domains, Qdrant retrieves candidate basic facts and MySQL hydrates their exact evidence and provenance before composition.

Verified relationships, evidence-grounded request-local inferences, and exploratory vector or model associations remain visibly separate. Taxonomy parents and children are distinct from named intensity scales such as `warm → hot → sweltering → scorching`. Selecting a root can expand one shallow neighborhood or a short path of named relations; Transnet does not present a similarity chain as a factual path.

## Content and safety

All answers are pinned to a canonical content release where relevant. MySQL supplies concise canonical cards and Qdrant supplies relationships; unavailable graph retrieval degrades explicitly to the basic card. The service never stores live request text, context, a caller identity, or user-related state. Only reviewed, rights-cleared translations selected by the content-publication workflow enter the canonical MySQL release. User-saved translations remain product-owned data in island-port.

## Related documents

- [System design](../transnet.md)
- [Transnet service interface](../interfaces/transnet.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
