# Translation application

中文：[翻译 application](../../../docs_cn/reference/application/translation_cn.md)

This module owns connected-text, structured-segment, and bounded image-region translation and produces the primary translation portion of the shared result.

Status: the request domain validates and retains all three tagged shapes with their request-local guidance and history. The validated result superset covers word, phrase, passage, ordered segment, and ordered image-region outcomes with typed annotations, terminology decisions, review state, response-local citation references, and deterministic breadth projection. The application executes guided text, structured segments, and bounded image-region turns. It deterministically checks required and forbidden terminology plus paragraph structure and permits at most one reasoning-profile repair. Preferred terminology remains a prompt preference rather than a hard postcondition. Image execution rejects execution-dependent guidance that it cannot enforce, locally decodes each validated image, submits only one bounded crop per declared region, makes one bounded fast-profile call, and strictly maps attachment indices, exact reading-order IDs, and detected languages into the image-region result.

## Responsibilities

Translation preserves meaning, intent, tone, register, terminology, protected spans, paragraph structure, and relevant formatting while producing natural target-language text. Provider selection is policy behind the model port, not a public request option.

The application accepts one tagged input form. Text is the low-latency default; segments preserve caller-owned structure and stable segment IDs; image regions add only the pixels and reading hints needed for the current request. Island-port remains responsible for file ingestion, page selection, OCR policy, layout ownership, and durable document state. Request guidance is explicit and disposable rather than inferred from a user profile.

Long input may use a bounded request-local chunk plan and terminology ledger. Chunks respect semantic and paragraph boundaries, retain order, and are recombined without dropped content. The ledger tracks names, abbreviations, and repeated terms only for the current request; it is not translation memory or a durable job.

Structured segments use bounded parallel fast calls while preserving deterministic request order in the assembled result. Each prompt binds the segment role, format, source and target language, protected scalar ranges, request guidance, and prompt contract; caller IDs are not sent to the model. Required and forbidden terminology is checked with script-appropriate matching, including terms embedded in unsegmented CJK text, and the result records the complete ordered terminology decision list. A request has one shared reasoning-repair budget. Deterministic postconditions reject missing, reordered, or duplicated protected values, changed paragraph breaks, changed Markdown structural delimiters, or changed HTML tags. Successful results are validated against the request before return and use caller segment IDs, zero-based segment order, `translation_0`, and optional closed format annotations; no segment content is retained after completion.

Gemma4-27B handles text through the fast profile by default. The closed escalation policy permits at most one reasoning-profile call for invalid, ambiguous, or guidance-violating output; hidden reasoning is neither returned nor observed. `offline` never performs retrieval. Because no production search authority is configured, `allowed` currently degrades to ordinary translation without claiming live support, while `required` fails explicitly before generation. Once a search authority is configured, the existing bounded live-retrieval service remains the only admissible retrieval path and every live-dependent claim must carry response-local citations.

Passage tips and clearly labeled alternatives remain planned for a later milestone. The HTTP handler does not fabricate either capability before the application result models, deterministic usefulness evaluator, and orchestration produce them. Empty external-source collections on the current offline path are omitted.

## Boundaries

This module does not persist source text, images, segments, translations, history, guidance, chunk plans, web queries or pages, citations, or model output. Reusable canonical translations enter storage only through the offline publication workflow.

## Verification

Test provider routing boundaries, language validation, segment identity and ordering, region bounds and reading order, structure preservation, protected spans, chunk coverage, terminology consistency, fast-path latency, one-escalation enforcement, retrieval limits and citation coverage, cancellation, and absence of request persistence. Add alternative and passage-tip tests with the later application capability that owns them.
