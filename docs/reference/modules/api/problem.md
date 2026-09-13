# API problem responses

中文：[API 问题响应](../../../../docs_cn/reference/modules/api/problem_cn.md)

`src/api/problem.rs` owns closed, display-safe failure responses for versioned routes.

Status: existing foundation, not the default target contract. The current module emits RFC 9457-style `application/problem+json` for some versioned routes, while the target Transnet interface currently specifies an `error` envelope. This shape mismatch must be resolved when the target handlers are implemented; neither document should be treated as proof of runtime availability.

## Mapping rules

Problem mapping converts known validation, capacity, dependency, release, provider-output, not-found, and deadline failures into stable status and code pairs. Field errors name safe wire fields only. Retryability is explicit and conservative.

Responses include the request ID and a no-store policy. Detail text never echoes current text or history and never reveals provider bodies, vectors, canonical text, credentials, socket paths, peer details, SQL, collection names, or internal error chains.

Unexpected failures map to one generic internal outcome and retain diagnostic context only in redacted server telemetry. Domain and application errors remain typed until this boundary.

## Verification

Tests should exhaust each closed mapping, content type, request-ID propagation, cache policy, retryability, and redaction. The target status list and envelope are normative in the [Transnet interface](../../../interfaces/transnet.md).

