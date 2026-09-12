# Configuration

The process reads `config/transnet.toml` relative to the Cargo manifest, independent of the shell working directory.

`[server]` configures `host`, `port`, `log_level`, and `log_format`. `host` must be a loopback IP address such as `127.0.0.1` or `::1`; Island-port owns any public-edge deployment. `RUST_LOG` overrides `log_level`; `log_format = "json"` selects newline-delimited JSON and other values select compact text. Debug builds write `logs/debug/transnet.log`; release builds write `logs/release/transnet.log`. The non-blocking logger replaces the applicable file on startup and includes tracing targets.

`[http]` configures `max_request_body_bytes`, `allowed_origins`, and `allow_credentials`. The body limit applies before JSON is buffered and defaults to 1,048,576 bytes. Production keeps `allowed_origins` empty because browsers call Island-port, not Transnet. Exact origins remain available only for isolated local development; wildcard origins are rejected.

`[translation]` configures the Unicode-character routing boundary plus legacy defaults for a provider's per-attempt timeout, retry count after the first attempt, and retry delay. Provider-specific overrides take precedence.

`[gemma4]` and `[translate_gemma]` each configure an OpenAI-compatible `base_url`, `model`, and `api_key`. Defaults target Gemma 4 on port 18011 and TranslateGemma on port 18007. Real credentials must be provisioned without committing them to Git; parsed credentials are redacted from Rust `Debug` diagnostics and are used only for outbound provider requests.

`[provider_resilience.gemma4]` and `[provider_resilience.translate_gemma]` configure independent provider bounds. `timeout_seconds`, `max_retries`, and `retry_delay_ms` are optional overrides of `[translation]`; `max_retry_delay_ms` caps a provider-supplied `Retry-After` delay, `max_concurrent_requests` is a fail-fast bulkhead, `circuit_failure_threshold` is the number of consecutive transient logical-call failures that opens the circuit, and `circuit_open_ms` is the open interval before one half-open probe. Omitted tables use the Rust defaults of 8 concurrent attempts, a threshold of 5, a 30-second open interval, and a five-second retry-delay cap; the checked-in TranslateGemma policy tightens concurrency to 4.

Only request timeouts, connection-establishment failures, `429`, `500`, `502`, `503`, `504`, and unusable successful envelopes are retried. A valid `Retry-After` header is used instead of the configured retry delay, subject to `max_retry_delay_ms`; other client, server, and ambiguous transport failures fail without another request. An open circuit and full bulkhead fail the affected provider request promptly and map to the existing unavailable response.

Provider traces contain only the static provider boundary, operation name, attempt number, outcome class, status code when available, elapsed time, and retry delay. The process exposes in-memory redacted provider counter snapshots through Rust service APIs; no raw query, context, generated answer, provider body, credential, or identity is included in provider telemetry.

The structured `/v1/lookups` slice uses the `[gemma4]` provider and requests strict JSON Schema output. The configured server must support the OpenAI-compatible `response_format.type = "json_schema"` request field and return JSON text in the first assistant message.

MySQL, Qdrant, encryption, shared-cache, telemetry-exporter, and worker-role configuration does not belong to Transnet. Island-port owns those settings and adapters; see the [MySQL](../interfaces/mysql.md), [Qdrant](../interfaces/qdrant.md), and [Rust port](../interfaces/port.md) contracts.

Related: [design](../transnet.md), [Island-port interface](../interfaces/port.md), and [development](development.md).
