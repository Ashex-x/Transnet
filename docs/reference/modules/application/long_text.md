# Long-text translation support

中文：[长文本翻译支持](../../../../docs_cn/reference/modules/application/long_text_cn.md)

The long-text application module provides target request-local planning for connected text that cannot be handled as one bounded provider input.

Status: target design. The current runtime selects TranslateGemma for input over the configured 4,000-character routing threshold, but no separate chunk planner or terminology-ledger module is currently composed.

## Responsibilities

The module derives a bounded chunk plan that respects paragraphs, protected spans, and semantic boundaries. It maintains a terminology ledger for names, abbreviations, repeated terms, and resolved choices, then combines translated chunks without changing their order or dropping content.

The plan and ledger are implementation details, not wire fields or canonical records. The module returns connected-text aggregate data to the translation path and does not create a durable job, queue, translation memory, or resumable user workflow.

## Dependencies and invariants

Chunk translation uses the translation-model port under the request's single deadline and resilience budget. Every allocation is owned by the request and released before return; current text, chunks, ledger entries, and provider bodies never enter logs, metrics, caches, MySQL, Qdrant, or durable queues.

Chunking must preserve structure and global terminology consistency. An accepted history array is used whole within the body and deadline bounds rather than silently truncated by this module.

## Verification

Tests should cover boundary-safe splitting, protected spans, paragraph reconstruction, repeated terminology, names and abbreviations, empty or malformed provider chunks, cancellation, and total-size enforcement. Privacy tests should demonstrate that plans and ledgers disappear on success and every failure path.

## Related documents

- [Service module reference](../../modules.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [System design](../../../transnet.md)
- [Quality assurance](../../../guides/quality-assurance.md)
