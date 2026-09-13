# Provider adapters

中文：[Provider adapter](../../../docs_cn/reference/adapters/providers_cn.md)

This module implements model ports over OpenAI-compatible endpoints while keeping protocol mechanics separate from role-specific policy.

The shared client owns HTTP construction, authentication, response-size bounds, strict structured-output decoding, safe error mapping, and resilience integration. Gemma 4 owns short-text and bounded structured-composition request policy. TranslateGemma owns longer connected-text request policy. The current runtime selects between them at `translation.long_text_chars`; target application orchestration may add request-local planning without moving that policy into HTTP handlers.

Adapters never log prompts, source text, history, provider bodies, credentials, or generated content. Errors expose only closed dependency and operation categories.

Test exact request envelopes, strict schemas, status classification, response bounds, redaction, deadline propagation, retry eligibility, and circuit behavior with local stubs.
