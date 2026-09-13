# Translation application

中文：[翻译 application](../../../docs_cn/reference/application/translation_cn.md)

This module owns connected-text translation and produces the primary translation portion of the shared result.

## Responsibilities

Translation preserves meaning, intent, tone, register, terminology, protected spans, paragraph structure, and relevant formatting while producing natural target-language text. Provider selection is policy behind the model port, not a public request option.

Long input may use a bounded request-local chunk plan and terminology ledger. Chunks respect semantic and paragraph boundaries, retain order, and are recombined without dropped content. The ledger tracks names, abbreviations, and repeated terms only for the current request; it is not translation memory or a durable job.

At most two concise notes may explain important ambiguity, idiom, register, or cultural context. Insufficient context may produce a clearly labeled alternative rather than false certainty.

## Boundaries

This module does not persist source text, translations, history, chunk plans, or model output. Reusable canonical translations enter storage only through the offline publication workflow.

## Verification

Test provider routing boundaries, language validation, structure preservation, protected spans, chunk coverage and ordering, terminology consistency, alternatives, cancellation, and absence of request persistence.
