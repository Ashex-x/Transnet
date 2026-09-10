# Configuration

The process reads `config/transnet.toml` relative to the Cargo manifest, independent of the shell working directory.

`[server]` configures `host`, `port`, `log_level`, and `log_format`. `RUST_LOG` overrides `log_level`; `log_format = "json"` selects JSON output and other values select compact logs.

`[http]` configures `max_request_body_bytes`, `allowed_origins`, and `allow_credentials`. The body limit applies before JSON is buffered and defaults to 1,048,576 bytes. `allowed_origins` is an exact list of `http` or `https` origins without a path, query, fragment, or trailing slash; an empty list disables CORS. Wildcard origins are rejected. When origins are configured, the service allows `GET`, `POST`, and `OPTIONS`, accepts `Content-Type` and `X-Request-Id`, and exposes `X-Request-Id`. `allow_credentials = true` is safe only because every allowed origin is exact.

`[translation]` configures the Unicode-character routing boundary, per-attempt timeout, retry count after the first attempt, and delay between attempts.

`[gemma4]` and `[translate_gemma]` each configure an OpenAI-compatible `base_url`, `model`, and `api_key`. Defaults target Gemma 4 on port 18011 and TranslateGemma on port 18007. Real credentials must be provisioned without committing them to Git.

The structured `/v1/lookups` slice uses the `[gemma4]` provider and requests strict JSON Schema output. The configured server must support the OpenAI-compatible `response_format.type = "json_schema"` request field and return JSON text in the first assistant message.

Related: [design](../transnet.md), [API contract](../reference/transnet-api.md), and [development](development.md).
