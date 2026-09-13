# Bootstrap composition

中文：[Bootstrap 组合](../../docs_cn/reference/bootstrap_cn.md)

`src/bootstrap.rs` is the target composition root for the online service. It is intended for implementers wiring infrastructure to application ports.

Status: target design; this module does not exist in the current runtime. Composition remains embedded in `src/main.rs`, which exposes only transitional loopback translation and model-backed lookup.

## Responsibilities

Bootstrap loads and validates configuration, initializes observability, constructs provider and island-port clients, composes application services, registers readiness checks, binds the owned Unix socket, and coordinates shutdown. Online composition receives read-only structured and vector ports and must never receive publication mutations.

Startup order is configuration -> observability -> provider clients -> island-port client -> application services -> readiness registry -> UDS bind -> accept loop. Readiness becomes true only after dependencies required by enabled routes report compatible schemas and an active compatible release.

Shutdown stops admission, drains accepted work within deadlines, closes clients, unlinks only the socket this process owns, flushes safe telemetry, and exits. Partial startup must clean up only resources whose ownership bootstrap established.

## Verification

Composition tests should prove route registration follows dependency availability, failures do not claim readiness, and shutdown respects ownership and deadlines. See the [configuration module](config.md), [UDS server](transport/uds_server.md), and [Transnet interface](../interfaces/transnet.md).

