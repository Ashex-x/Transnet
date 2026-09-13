# Version 1 sense reads

中文：[版本 1 词义读取](../../../../docs_cn/reference/api/v1/sense_cn.md)

`src/api/v1/sense.rs` is the target thin handler for `POST /transnet/v1/senses/get`, a follow-up read by a canonical sense ID returned from translation.

Status: target route. Existing canonical sense-detail services and legacy HTTP routes are reusable foundations, but the default launcher does not production-compose this UDS endpoint.

## Handler boundary

The handler strictly decodes the canonical sense and release identifiers required by the contract, preserves the request deadline, invokes the operation-focused structured-data application service, and serializes a release-pinned result. It does not perform free-text search, infer user intent, or accept user identity.

A missing sense maps to the closed not-found problem. Release mismatch or incompatibility fails safely rather than silently reading another version. Returned canonical content is reviewed data; request-local model output and vector similarity cannot establish it.

The route is a read only. It never changes publication state, records viewing history, saves a graph, or writes the request to a cache or database.

## Verification

Contract tests should cover exact release pinning, found and not-found outcomes, release conflicts, strict fields, dependency failure, response projection, and no durable writes. The normative route is in the [Transnet interface](../../../interfaces/transnet.md).

