# Model ports

中文：[模型 port](../../../docs_cn/reference/ports/models_cn.md)

This module defines narrow application operations for plain translation and bounded structured generation.

The port accepts validated domain inputs, explicit deadlines, and versioned operation identifiers. It returns validated candidate output or closed failure categories. Provider URLs, credentials, HTTP envelopes, prompts, role messages, JSON Schema mechanics, retry headers, and model-specific payloads remain adapter concerns.

Application code chooses an operation, not a provider brand. Model output is never canonical evidence. Callers must validate structure and references before use and discard request content and generated material when the request ends.

Tests use fakes at this boundary to cover routing, timeouts, malformed output, unsupported schema behavior, and safe error classification without external provider calls.
