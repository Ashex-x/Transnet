# Transnet compute-service plan

中文：[计算服务计划](../docs_cn/todo_cn.md)

## Fixed boundary

Transnet remains a pure loopback compute service. Island-port owns identity, permissions, user data, privacy, encryption, persistence, cache policy, idempotency, durable work, MySQL, Qdrant, and every stateful public API.

## Current baseline

- [x] Direct `/translate` computation with short/long provider routing.
- [x] Structured model-only `/v1/lookups` computation.
- [x] Bounded requests, request IDs, redacted tracing, graceful shutdown, and provider resilience.
- [x] Compute interfaces exclude caller identity, credentials, persistence policy, and durable-work fields.

## Next compute work

- [ ] Define an Island-port-supplied canonical candidate snapshot for evidence-backed card assembly.
- [ ] Keep source licensing and display filtering in Island-port before snapshot delivery.
- [ ] Define pure ranking and assembly operations over bounded versioned inputs.
- [ ] Define optional pure exercise generation and answer evaluation over supplied rubrics.
- [ ] Add contract tests proving identity, storage, database, and persistence fields are rejected.
- [ ] Add load, timeout, cancellation, invalid-model-output, and prompt-injection benchmarks.

## Boundary cleanup

- [ ] Move stateful and database-oriented foundations to Island-port or reduce them to pure algorithms over caller-supplied values.
- [ ] Remove conditional HTTP routes that imply Transnet owns polling, private state, canonical repositories, or storage.
- [ ] Keep the deployed binary free of MySQL, SQLx, Qdrant, sessions, encryption, and identity dependencies.

## Definition of done

- Transnet can start and serve every route without a database or vector service.
- A packet or trace review shows no caller identity, token, cookie, role, persistence instruction, database credential, idempotency key, or task capability crossing the loopback boundary.
- Retrying a Transnet request cannot duplicate or mutate durable state.
- Island-port independently tests permissions, transactions, retention, deletion, and persistence outcomes.

## Related documents

- [Compute-service architecture](transnet.md)
- [Island-port interface](interfaces/port.md)
- [MySQL interface](interfaces/mysql.md)
- [Qdrant interface](interfaces/qdrant.md)
