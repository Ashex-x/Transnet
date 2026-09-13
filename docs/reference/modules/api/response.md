# API response models

中文：[API 响应模型](../../../../docs_cn/reference/modules/api/response_cn.md)

`src/api/response.rs` is the target owner of success metadata and wire serialization for `TranslationResult`, meaning-specific details, canonical sense reads, and graph reads.

Status: target module design. Current legacy response types and existing canonical foundations do not constitute the shared target response contract or default production composition.

## Contract

Successful application responses use `data` and `meta`; `meta.request_id` matches `X-Request-Id`, and canonical reads identify the immutable content release. Translation success places one result at `data.translation`, with an ordered `translations` array so materially plausible meanings remain distinct.

`brief`, `standard`, and `full` serialize deterministic projections of one validated superset result. Projection may remove supporting fields and excess low-value items, but cannot change fact identity, relationship direction, evidence state, release identity, or hide a materially plausible meaning when that would mislead.

Canonical markers appear only for reviewed content in the named release. Inferred and exploratory material remains request-local and explicitly labeled. Response models never serialize request history, provider details, storage internals, credentials, or user-owned state.

## Verification

Golden JSON tests should cover every response level, multiple meanings, canonical and degraded reads, graph bounds, release metadata, omitted optional fields, and unknown enum prevention. The normative shapes are in the [Transnet interface](../../../interfaces/transnet.md).

