# Version 1 translations

中文：[版本 1 翻译](../../../../../docs_cn/reference/modules/api/v1/translations_cn.md)

`src/api/v1/translations.rs` is the target thin handler for `POST /transnet/v1/translations`, the only entry point that begins a new translation turn.

Status: target route. The current executable exposes a transitional legacy translation route and does not compose the target automatic orchestration, release-pinned retrieval, or shared response.

## Handler boundary

The handler strictly decodes the simple request, applies request context and deadline, calls the request orchestrator once, and maps its validated aggregate to the success or problem envelope. It never asks callers to select a mode, domain, dialect, audience, purpose, retrieval filter, or model.

Optional history is passed as request-scoped linguistic context and discarded after completion. The handler never writes request content, publishes model output, or creates user-owned state. Canonical reads remain pinned to the orchestrator's one compatible release trio.

The application layer decides word, phrase, or passage processing; meaning resolution; domain assessment; retrieval; provider selection; relationship composition; and response projection. The handler performs no such policy.

## Verification

Contract tests should cover strict decoding, all response levels, materially ambiguous meanings, history disposal, deadline and dependency failures, safe errors, and absence of writes. The wire contract is the [Transnet interface](../../../../interfaces/transnet.md).

