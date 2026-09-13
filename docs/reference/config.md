# Runtime configuration

中文：[运行时配置](../../docs_cn/reference/config_cn.md)

`src/config.rs` owns typed runtime settings, defaults, validation, and secret references. It never reads request data.

Status: partially implemented. Current types cover the loopback listener, HTTP body and CORS policy, translation routing, two providers, and provider resilience. Target UDS, island-port, release, retrieval, model-role, and evaluation settings are not yet implemented.

## Contract

The process reads `config/transnet.toml` relative to the Cargo manifest. Cross-field validation must fail startup before a listener is admitted. Secret values are provisioned outside Git, used only at their outbound boundary, and redacted from diagnostics.

The target listener uses a configurable socket path and mode while API namespaces remain fixed. Current `server.host`, `server.port`, and browser CORS settings belong only to the transitional loopback runtime and disappear when UDS serving lands.

Provider-specific resilience overrides inherit legacy translation defaults only where documented. Zero limits, unsafe listener exposure, invalid origins, unusable deadlines, and incompatible release settings are configuration errors rather than request errors.

## Verification

Tests should cover defaults, deserialization, redaction, every validation boundary, and cross-field incompatibility. Exact fields and defaults belong to the [configuration guide](../guides/configuration.md); target process boundaries are in the [system design](../transnet.md).

