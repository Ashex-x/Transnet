# Translation application

中文：[翻译 application](../../../docs_cn/reference/application/translation_cn.md)

This module owns connected-text, structured-segment, and bounded image-region translation and produces the primary translation portion of the shared result.

Status: the request domain validates and retains all three tagged shapes with their request-local guidance and history. The current application executes text only; the HTTP boundary rejects validated segment and image-region turns with a content-free `501 translation_capability_unavailable` response before any model call.

## Responsibilities

Translation preserves meaning, intent, tone, register, terminology, protected spans, paragraph structure, and relevant formatting while producing natural target-language text. Provider selection is policy behind the model port, not a public request option.

The application accepts one tagged input form. Text is the low-latency default; segments preserve caller-owned structure and stable segment IDs; image regions add only the pixels and reading hints needed for the current request. Island-port remains responsible for file ingestion, page selection, OCR policy, layout ownership, and durable document state. Request guidance is explicit and disposable rather than inferred from a user profile.

Long input may use a bounded request-local chunk plan and terminology ledger. Chunks respect semantic and paragraph boundaries, retain order, and are recombined without dropped content. The ledger tracks names, abbreviations, and repeated terms only for the current request; it is not translation memory or a durable job.

Gemma4-27B handles text and vision through the fast profile by default. The closed escalation policy permits at most one reasoning-profile call; hidden reasoning is neither returned nor observed. When freshness is explicitly `allowed` or `required`, the application may perform one bounded live-retrieval round and must attach citations to every live-dependent claim. Retrieved material is untrusted request-local context and never becomes canonical content.

Passage tips and clearly labeled alternatives remain planned for a later milestone. The milestone 1 HTTP handler does not fabricate either capability before the application result models and orchestration produce them.

## Boundaries

This module does not persist source text, images, segments, translations, history, guidance, chunk plans, web queries or pages, citations, or model output. Reusable canonical translations enter storage only through the offline publication workflow.

## Verification

Test provider routing boundaries, language validation, segment identity and ordering, region bounds and reading order, structure preservation, protected spans, chunk coverage, terminology consistency, fast-path latency, one-escalation enforcement, retrieval limits and citation coverage, cancellation, and absence of request persistence. Add alternative and passage-tip tests with the later application capability that owns them.
