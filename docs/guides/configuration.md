# Configuration

中文：[配置](../../docs_cn/guides/configuration_cn.md)

The process reads `config/transnet.toml` relative to the Cargo manifest, independent of the shell working directory.

The target listener configuration uses `socket_path = "/run/transnet/transnet.sock"`, `socket_mode = "0660"`, and an operator-managed socket group. Structured and vector data clients use island-port at `/run/island-port/island-port.sock`. Socket paths are deployment settings; API namespaces are fixed. The current `[server] host` and `port` fields configure only the transitional loopback runtime and are removed when UDS serving lands. `RUST_LOG` overrides `log_level`; `log_format = "json"` selects newline-delimited JSON and other values select compact text. Debug builds write `logs/debug/transnet.log`; release builds write `logs/release/transnet.log`.

`[http]` configures `max_request_body_bytes` and transitional CORS fields. The body limit applies before JSON is buffered and defaults to 1,048,576 bytes. UDS has no browser origin and the target runtime ignores and ultimately removes CORS configuration; browsers call a product gateway, not Transnet.

`http.allow_credentials` must be false; true is rejected at startup. Transitional CORS permits only configured exact origins and the Content-Type and X-Request-Id request headers. End-user authentication remains owned by island-port.

`[translation]` configures the Unicode-character routing boundary plus legacy defaults for a provider's per-attempt timeout, retry count after the first attempt, and retry delay. Provider-specific overrides take precedence.

`[gemma4]` and `[translate_gemma]` each configure an OpenAI-compatible `base_url`, `model`, and `api_key`. Defaults target Gemma 4 on port 18011 and TranslateGemma on port 18007. Real credentials must be provisioned without committing them to Git; parsed credentials are redacted from Rust `Debug` diagnostics and are used only for outbound provider requests.

These tables describe current transitional runtime behavior. The target configuration replaces them with one generation endpoint and one embedding endpoint. The generation settings name one Gemma4-27B model and provider-specific fast/reasoning controls; application policy, not a second endpoint, selects the profile. Separate bounded settings cover fast inference, reasoning escalation, embedding calls, and optional live retrieval. The checked-in configuration does not adopt those target keys until the matching Rust types and composition exist.

Provider clients connect directly to their configured endpoints and do not inherit operating-system or environment proxy settings. This keeps loopback and private model traffic, including bearer credentials, out of unrelated proxy processes.

`[provider_resilience.gemma4]` and `[provider_resilience.translate_gemma]` configure independent provider bounds. `timeout_seconds`, `max_retries`, and `retry_delay_ms` are optional overrides of `[translation]`; `max_retry_delay_ms` caps a provider-supplied `Retry-After` delay, `max_concurrent_requests` is a fail-fast bulkhead, `circuit_failure_threshold` is the number of consecutive transient logical-call failures that opens the circuit, and `circuit_open_ms` is the open interval before one half-open probe. Omitted tables use the Rust defaults of 8 concurrent attempts, a threshold of 5, a 30-second open interval, and a five-second retry-delay cap; the checked-in TranslateGemma policy tightens concurrency to 4.

Only request timeouts, connection-establishment failures, `429`, `500`, `502`, `503`, `504`, and unusable successful envelopes are retried. A valid `Retry-After` header is used instead of the configured retry delay, subject to `max_retry_delay_ms`; other client, server, and ambiguous transport failures fail without another request. An open circuit and full bulkhead fail the affected provider request promptly and map to the existing unavailable response.

Provider traces contain only the static provider boundary, operation name, attempt number, outcome class, status code when available, elapsed time, and retry delay. The process exposes in-memory redacted provider counter snapshots through Rust service APIs; no raw query, context, generated answer, provider body, credential, or identity is included in provider telemetry.

Target telemetry additionally records only the closed inference profile (`fast` or `reasoning`), input-kind category, and whether a reasoning escalation occurred. It never records image data, prompts, hidden reasoning, embeddings, live-search queries, fetched content, or generated output.

The structured `/v1/lookups` slice uses the `[gemma4]` provider and requests strict JSON Schema output. The configured server must support the OpenAI-compatible `response_format.type = "json_schema"` request field and return JSON text in the first assistant message.

`[canonical]` is an opt-in canonical-only production read dependency. The checked-in configuration sets `enabled = false`: the process constructs no island-port client, model translation behaves as before, and readiness does not claim canonical availability. To enable it on a Unix host, set `enabled = true`, an absolute `socket_path` for island-port (for example `/run/island-port/island-port.sock`), and `timeout_ms` from 1 through 30,000. The Unix socket path must be at most 107 bytes, contain no whitespace or `..` component, and is redacted from configuration Debug and errors. Missing or invalid enabled settings reject startup; enabled operation on a non-Unix host rejects startup because the production UDS transport is unavailable there.

When enabled, the process constructs the strict outbound island-port client and request-local `CanonicalReadService` used by `POST /api/v1/basic-cards/lookup` and release-pinned `POST /api/v1/senses/get`; existing translation and lookup responses remain unchanged. `GET /readyz` uses the existing response contract and performs only the bounded `/api/v1/releases/active` read: no active release, transport failure, incompatible schema, malformed response, or timeout means not ready. A missing or outdated island-port server may leave the process running with `/readyz` returning `503`; it is never treated as a canonical miss or a ready capability. The socket is not a MySQL connection, and no vector/ranking version is invented. The adapter wire schema and response bound remain the fixed, strict Stage 3 contract, not caller-configurable versions.

Transnet-side canonical public delivery is implemented by the two routes above, including release/schema failure mapping and attribution/evidence projection. The island-port canonical server, production MySQL migrations, publisher/write operations, real publication, activation, rollback and quarantine, immutable old-release serving verification, and island-port/MySQL end-to-end acceptance remain external target capabilities. Embeddings, candidate retrieval, and their evaluation remain later milestones; see the [canonical-data endpoints](../interfaces/canonical-data.md), [retrieval-data endpoints](../interfaces/retrieval-data.md), and [Transnet](../transnet.md) contracts. Never place credentials or request content in the checked-in file.

Related: [design](../transnet.md), [Transnet service interface](../interfaces/transnet.md), and [development](development.md).
