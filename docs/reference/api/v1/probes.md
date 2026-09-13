# Version 1 probes

中文：[版本 1 探针](../../../../docs_cn/reference/api/v1/probes_cn.md)

`src/api/v1/probes.rs` is the target thin-handler owner for health, liveness, and readiness probes.

Status: target route split. The current runtime exposes transitional probe behavior, but the three target `POST /transnet/v1/...` routes over UDS are not implemented.

## Routes

`POST /transnet/v1/health` reports that the process can serve HTTP and parse its empty `{}` request. `POST /transnet/v1/livez` reports whether the event loop and mandatory internal control state are functioning. Neither claims that external dependencies or canonical data are ready.

`POST /transnet/v1/readyz` reports ready only when dependencies required by enabled routes can serve compatible schemas and, for canonical routes, a compatible active release. Optional dependency degradation is represented exactly as the interface contract permits and must not turn missing mandatory capability into readiness.

Handlers validate `{}`, consult process or readiness state, and serialize the result. They contain no dependency probing strategy, retries, translation logic, or persistence.

## Verification

Tests should distinguish health, liveness, and readiness across startup, dependency degradation, incompatible releases, admission shutdown, and drain. Exact schemas and statuses belong to the [Transnet interface](../../../interfaces/transnet.md).

