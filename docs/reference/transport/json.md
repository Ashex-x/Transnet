# JSON transport

中文：[JSON 传输](../../../docs_cn/reference/transport/json_cn.md)

`src/transport/json.rs` is the target codec boundary for all Transnet HTTP bodies.

Status: target module design. The current Axum API has body limiting and JSON handling, but the shared strict target codec is not separated into this file.

## Contract

Requests and responses use UTF-8 JSON with `Content-Type: application/json`; clients send `Accept: application/json`. Every operation carries one JSON object, including `{}` for probes. The body limit defaults to 1,048,576 bytes and is enforced before buffering or decoding.

Decoding rejects malformed JSON, duplicate or unknown fields according to the owning wire type, non-object top-level values, unsupported media types, chunked request bodies, and bodies beyond the configured limit. Encoding uses the route's success or problem envelope and never logs or traces a body.

The codec distinguishes transport rejection from valid application outcomes and leaves semantic validation to API request types and handlers. It adds no user identity, persistence metadata, or hidden compatibility fields.

## Verification

Contract tests should cover UTF-8, media types, exact size boundaries, unknown fields, `{}` probes, and safe encoding. The normative rules and examples are in the [Transnet interface](../../interfaces/transnet.md).

