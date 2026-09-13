# Connected-text translation

中文：[连续文本翻译](../../../../docs_cn/reference/modules/application/translation_cn.md)

The translation application module owns target short connected-text translation and constructs the primary passage portion of the shared translation aggregate.

Status: target boundary with current behavior to migrate. The current runtime already translates over a transitional loopback route and sends text at or below the configured routing threshold to Gemma 4, but it does not produce the target shared aggregate through this module.

## Responsibilities

The module preserves meaning, tone, register, terminology, paragraph structure, protected spans, names, numbers, negation, and relevant formatting while producing natural target-language text. It may use request-scoped history for reference resolution, terminology continuity, tone, and follow-up instructions.

The translated text is always primary. It may attach at most two one-sentence tips for material ambiguity, idiom, consequential register choice, or cultural context, and may include one clearly labeled alternative when context is insufficient. Detailed lexical relationship expansion belongs to the lexical path.

## Dependencies and invariants

The module calls the connected-text operation of the translation-model port under the orchestrator's deadline and provider-resilience bounds. It accepts validated language selectors and returns domain translation types rather than wire envelopes.

Provider output is request-local and is not canonical unless an exact reviewed translation was separately resolved from the pinned release. Live traffic never publishes, caches, or labels a translation important. Response level cannot change the chosen translation text.

## Verification

Tests should cover meaning and completeness, structure, terminology, names, numbers, negation, idioms, dialect, tone, formatting, the two-tip limit, invalid provider output, timeout, and bounded fallback. Multi-turn cases should verify context use and complete disposal after return.

## Related documents

- [Service module reference](../../modules.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [System design](../../../transnet.md)
- [Quality assurance](../../../guides/quality-assurance.md)
