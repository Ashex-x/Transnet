# Configuration

## Translation core

The core binary reads checked-in TOML from `island-transnet/config/`, resolving paths from the crate manifest so startup does not depend on the current directory.

`transnet.toml` configures `[server]` fields `host`, `port`, and `workers`, plus `[logging]` fields `level`, `format`, and the currently reserved `file`. The `format` value selects compact output by default and JSON output when set to `json`. The server reports `workers` in startup metadata; Tokio runtime sizing is not currently customized by this value.

`transnet_llm.toml` configures the `[openai]` provider. `api_key`, `base_url`, `model`, `timeout_seconds`, and `max_retries` are required. `normal_lang_base_url` and `normal_lang_model` are optional overrides used when both source and target match the common-language allowlist.

The checked-in API key is a local placeholder. For deployments that use credentials, provision the file through a secret-aware deployment mechanism and keep the real value out of Git. The current core does not override TOML provider values from environment variables.

`transnet_api.toml` and `transnet_db.toml` describe deferred API-policy and persistence settings. No current process loads them; changing them has no runtime effect.

## Gateway

The gateway reads environment variables at startup:

- `GATEWAY_HOST`: listener address, default `0.0.0.0`.
- `GATEWAY_PORT`: listener port, default `8080`.
- `WORKERS`: compatibility metadata, default `4`; it does not resize the Tokio runtime.
- `RUST_LOG`: tracing filter, default `info`.
- `BACKEND_HOST`: core service host, default `127.0.0.1`.
- `BACKEND_PORT`: core service port, default `35792`.

The binary attempts to load `.env` from the repository root for local development. `.env` is ignored by Git. Invalid numeric values stop startup with a contextual configuration error.

Related: [architecture](../architecture.md), [development](development.md), [core server](../reference/core/server.md), and [gateway](../reference/gateway/README.md).
