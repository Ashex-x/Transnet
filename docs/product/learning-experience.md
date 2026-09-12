# Transnet service behavior

中文：[Transnet 服务行为](../../docs_cn/product/learning-experience_cn.md)

This guide describes consumer-visible behavior of the stateless Transnet service. Product applications own their own user experience and may use these responses in any workflow without sending user data to Transnet.

## Translation

`POST /translate` accepts bounded source text and request-scoped language or register options. It returns natural target-language text that preserves meaning, tone, register, and paragraph structure. The service adds no more than two concise tips, and only when an ambiguity, idiom, consequential register choice, or cultural context materially affects the translation.

## Lexical lookup

`POST /v1/lookups` resolves a word or established lexical phrase to one or more canonical senses. It presents a concise card first: canonical form, part of speech, definitions, translations, pronunciation, morphology, examples, CEFR metadata, domain IDs, and release metadata. Homographs and parts of speech remain separate. Misspellings and ambiguous fragments produce explicit alternatives or a translation-mode result.

The optional context field is used only to disambiguate the current request. It is not retained. Request language, dialect, register, and detail options change only the response presentation.

## Knowledge exploration

Canonical cards link to bounded Qdrant graph reads. Graph responses expose typed relationships, explanations, evidence state, and release metadata. Verified relationships are separate from exploratory vector associations. Selecting a root can expand one shallow neighborhood; Transnet does not present a similarity chain as a factual path.

## Content and safety

All answers are pinned to a canonical content release where relevant. MySQL supplies concise canonical cards and Qdrant supplies relationships; unavailable graph retrieval degrades explicitly to the basic card. The service never stores request text, context, a caller identity, or user-related state.

## Related documents

- [System design](../transnet.md)
- [Service interface](../interfaces/port.md)
- [MySQL interface](../interfaces/mysql.md)
- [Qdrant interface](../interfaces/qdrant.md)
