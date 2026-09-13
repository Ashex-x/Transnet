# Runtime configuration

中文：[运行时配置](../../../docs_cn/reference/runtime/configuration_cn.md)

This module owns typed loading, defaults, validation, secret references, and redacted diagnostics for process configuration.

## Ownership

The process reads `config/transnet.toml` relative to the Cargo manifest. `RUST_LOG` may override the configured filter. Provider credentials are secrets and must never appear in checked-in files, debug output, logs, metrics, or errors.

Configuration parsing rejects unknown fields. Validation checks individual bounds and incompatible cross-field combinations before clients or listeners are created. Code outside the configuration and bootstrap modules receives typed settings rather than rereading files or environment variables.

## Current and target settings

Current listener, provider routing, CORS, logging, resilience, and default values are listed in the [configuration guide](../../guides/configuration.md). That guide also owns target UDS and island-port settings. This page does not duplicate the field catalog.

## Verification

Test defaults, unknown fields, override precedence, redaction, exact validation boundaries, and cross-field incompatibilities. Any public setting change updates the guide and its Chinese mirror in the same change.
