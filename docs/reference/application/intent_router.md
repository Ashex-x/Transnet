# Intent router

中文：[意图路由器](../../../docs_cn/reference/application/intent_router_cn.md)

The intent router is the target application boundary that classifies a new turn as a word, established lexical phrase, or connected passage and selects the least intrusive useful processing path. The caller supplies no mode switch.

Status: target design. The current runtime routes plain translation by text length and exposes a separate legacy lookup path; it does not yet implement this automatic shared router.

## Responsibilities

The router uses current text, requested languages, and request-scoped history to derive unit and intent. A word, term, idiom, phrasal verb, or established phrase enters lexical and possible domain processing; a clause, sentence, or passage enters connected-text translation. Ambiguous short fragments must take a safe path that preserves useful translation without fabricating lexical certainty.

The result is an internal routing decision, not a new public request field. Provider choice, domain selection, retrieval filters, dialect, register, and formatting remain derived decisions owned by later application services.

## Dependencies and invariants

The router may use bounded model classification behind an application port, but deterministic validation owns the closed decision. It must not call storage mutation operations or persist the input.

History may affect reference resolution and intent but never canonical identity. Routing is independent of response level so `brief`, `standard`, and `full` remain projections of the same selected workflow and superset.

## Verification

Tests should cover words, specialist terms, idioms, phrases, sentences, passages, symbols such as `C++`, mixed-language text, and ambiguous fragments. Evaluation should confirm that no WebUI mode is required, accepted history is not silently truncated, and routing errors create no durable state.

## Related documents

- [Service module reference](../modules.md)
- [Transnet service interface](../../interfaces/transnet.md)
- [System design](../../transnet.md)
- [Quality assurance](../../guides/quality-assurance.md)
