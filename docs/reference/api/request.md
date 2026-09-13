# API request models

中文：[API 请求模型](../../../docs_cn/reference/api/request_cn.md)

`src/api/request.rs` is the target owner of exact incoming wire requests, including a translation turn and its optional minimal history items.

Status: target module design. The current runtime uses legacy request types and paths; they do not implement the target `/transnet/v1` request contract.

## Shapes and validation

The translation request contains only `text`, `source_language`, `target_language`, `response_level`, and optional chronological `history`. Initial language selectors are `auto`, `en`, and `zh-CN` where the contract permits them; response level is one of `brief`, `standard`, or `full`. Unknown fields are rejected.

Each history item contains only prior source text, translated text, and their languages. History has no separate item-count limit, but the common body limit applies. It carries no user, turn, time, feedback, preference, domain, model, or saved-item metadata and is discarded with the request.

Request models perform structural and closed-value validation, then map into domain input. They do not choose intent, meanings, domains, retrieval, providers, or persistence.

## Verification

Round-trip and rejection tests should cover every closed value, unknown fields, chronological history shape, empty or invalid text, and the body-boundary interaction. The normative schema is the [Transnet interface](../../interfaces/transnet.md).

