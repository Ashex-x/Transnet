# Metrics

中文：[指标](../../../../docs_cn/reference/modules/observability/metrics_cn.md)

This page defines closed aggregate counters and latency outcomes for service health at `src/observability/metrics.rs`. It is for contributors implementing or reviewing the target service boundary.

Status: partial current foundation; the target boundary and production composition are not complete.

## Contract

A checked-in foundation already defines redacted domain events, a metrics port, and an in-memory recorder, but the executable does not yet compose the target exporter and route-level catalog. Target labels stay low-cardinality and closed. Export, retention, sampling, alerts, and dashboards are operational policy outside domain and port types.

## Ownership and dependencies

This module contains domain vocabulary or implements only the boundary named above. It must preserve Transnet's user-agnostic, request-stateless design. Wire shapes remain authoritative in the interface contracts, publication policy in the publishing guide, and cross-module semantics in the system design; this page does not create an additional API.

## Verification

Add unit tests beside implemented code for validation and invariants, plus integration or contract tests where values cross a process boundary. Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `cargo doc --no-deps` from the repository root.

## Related documents

- [Service module reference](../../modules.md)
- [System design](../../../transnet.md)
- [Transnet service interface](../../../interfaces/transnet.md)
- [Content publishing](../../../guides/content-publishing.md)

