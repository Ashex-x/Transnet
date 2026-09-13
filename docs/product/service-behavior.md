# Transnet service behavior

中文：[Transnet 服务行为](../../docs_cn/product/service-behavior_cn.md)

This guide describes consumer-visible behavior of the stateless Transnet service. Product applications own their own user experience and may use these responses in any workflow without sending user data to Transnet.

## Translation

`POST /translate` accepts bounded source text and request-scoped language or register options. It returns natural target-language text that preserves meaning, tone, register, and paragraph structure. The service adds no more than two concise tips, and only when an ambiguity, idiom, consequential register choice, or cultural context materially affects the translation.

## Lexical and concept lookup

`POST /v1/lookups` resolves a word, established lexical phrase, or specialist term to one selected canonical sense or concept. It presents a concise card first: canonical and translated forms, aliases, part of speech where applicable, definitions, pronunciation, morphology, examples, domain IDs, and release metadata. Homographs, parts of speech, phrase-level meanings, and field-specific senses remain separate. Ambiguity produces ranked candidates or a clarification result; a fragment that cannot be confidently treated as a lexical unit routes to translation.

The optional context field is used only to disambiguate the current request. It is not retained. Request language, dialect, register, and detail options change only the response presentation.

## Relationship and domain exploration

Canonical cards anchor bounded Qdrant relationship reads. The page exposes purpose-ranked groups of typed relationships, explanations, conditions, provenance, evidence state, and release metadata. For general vocabulary, the neighborhood can include taxonomy, intensity, contrasts, syntax, collocation, register, morphology, and cultural extension. For a domain-specific concept, it can additionally include multilingual terminology, fields and subfields, mechanisms, prerequisites, phenomena, equations, technologies, applications, measurements, standards, and professional usage conventions.

The LLM classifies the resolved meaning as `general`, `domain_specific`, `mixed`, or `uncertain` and selects useful relationship groups. Canonical data validates its candidates. Verified relationships, evidence-grounded request-local inferences, and exploratory vector or model associations remain visibly separate. Selecting a root can expand one shallow neighborhood or a short path of named relations; Transnet does not present a similarity chain as a factual path.

## Content and safety

All answers are pinned to a canonical content release where relevant. MySQL supplies concise canonical cards and Qdrant supplies relationships; unavailable graph retrieval degrades explicitly to the basic card. The service never stores request text, context, a caller identity, or user-related state.

## Related documents

- [System design](../transnet.md)
- [Service interface](../interfaces/port.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
