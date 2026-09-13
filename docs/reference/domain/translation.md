# Translation domain

中文：[翻译 domain](../../../docs_cn/reference/domain/translation_cn.md)

This module owns transport-independent vocabulary for translation requests and results.

## Model

A request contains source text, source and target language selectors, response level, and optional chronological minimal history. Language tags are canonicalized and bounded. Source language may be automatic only where the service contract permits it. Unknown wire fields are a transport concern; domain construction still rejects invalid or oversized values.

A translation result separates the primary translated text from optional ambiguity, register, terminology, or cultural notes. A response-level value is one of `brief`, `standard`, or `full`; it controls deterministic breadth after the full result is assembled.

## Invariants

History is linguistic context, not identity or durable state. It contains only the minimum prior source/translation pairs needed by the current request and is discarded afterward. Domain values must not carry user IDs, persistence policy, provider selection, or storage instructions.

Exact public shapes and limits belong to the [Transnet service interface](../../interfaces/transnet.md).
