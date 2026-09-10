# Transnet HTTP API

Requests and responses use JSON. No implemented route requires authentication. The hand-maintained [OpenAPI 3.1 contract](transnet-openapi.json) covers only the routes wired by the default Rust process; optional dependency-injected routes are intentionally absent.

## Request correlation and browser access

Every response, including CORS preflights, route misses, and payload-limit failures, contains `X-Request-Id`. A caller may supply one safe value to correlate an upstream request; safe values are 1 through 128 ASCII letters, digits, hyphens, underscores, or periods. Unsafe, repeated, or absent values are replaced with a generated ULID. The service logs only the request ID, HTTP method, matched route template, outcome, and latency; it does not log request bodies, credentials, or provider responses.

Cross-origin access is disabled unless `[http].allowed_origins` contains the caller's exact origin. Configured origins may use `GET`, `POST`, and `OPTIONS`, send `Content-Type` and `X-Request-Id`, and read `X-Request-Id`. The service never accepts a wildcard origin, including for credentialed requests.

Request bodies are bounded by `[http].max_request_body_bytes`. An oversized `/translate` payload returns HTTP 413 with the existing direct error shape. An oversized `/v1` payload returns the versioned problem shape below.

## GET /health

Returns HTTP 200 when the process can serve requests. This does not probe model providers.

```json
{"status":"ok"}
```

## GET /livez

Returns the same HTTP 200 process-liveness response as `GET /health`. It is suitable for a supervisor that only needs to know whether the HTTP process is alive.

## GET /readyz

Returns HTTP 200 with `{"status":"ok"}` when the injected readiness probe permits traffic. It returns HTTP 503 with `{"status":"unavailable"}` when a required dependency is unavailable. The current model-only service uses an always-ready probe, so model-provider availability does not change readiness; future runtime wiring can inject a probe for required dependencies.

## POST /translate

Accepts nonblank text and BCP-47-shaped source and target language codes.

```json
{"text":"Hello","source_lang":"en","target_lang":"zh-CN"}
```

Success returns HTTP 200:

```json
{"translation":"你好"}
```

Text containing at most 4,000 Unicode characters uses Gemma 4. Longer text is sent intact to TranslateGemma.

Malformed JSON returns HTTP 400. Blank text or malformed language codes return HTTP 422. An exhausted model request returns HTTP 503. An oversized body returns HTTP 413. These `/translate` errors preserve one direct shape:

```json
{"error":"description"}
```

## POST /v1/lookups

Accepts a word or short expression and produces a structured English-learning result. The current implementation is a synchronous, model-only bridge; it is not yet the canonical RAG lookup described in the [learning API](learning-api.md).

```json
{
  "query": "caliente",
  "source_language": "es",
  "target_language": "en",
  "context": "La sopa está caliente.",
  "explanation_language": "zh-CN",
  "english_dialect": "en-US",
  "learner_level": "B1",
  "detail": "full",
  "include": ["relations", "word_history"],
  "history_mode": "incognito"
}
```

`query` is required and limited to 100 Unicode characters. `context` is optional and limited to 1,000. `source_language` defaults to `auto`; `target_language` defaults to and currently must be `en`; `explanation_language` defaults to `en`; `english_dialect` is `en-US` or `en-GB`; `learner_level` is A1 through C2; and `detail` is `brief` or `full`.

The response separates ranked meanings and parts of speech, and can contain definitions, localized glosses, pronunciations, forms, usage notes, examples, etymology, and typed related words. Because the canonical lexicon is not implemented, every returned assertion has `generated: true`, canonical IDs are null, evidence lists are empty, related words have `canonical: false`, and top-level provenance reports `evidence_backed: false`.

Successful and error responses include `Cache-Control: no-store` and `X-Request-Id`. Malformed JSON returns HTTP 400, invalid fields return HTTP 422, unavailable models return HTTP 503, invalid structured model output after one repair returns HTTP 502, and oversized bodies return HTTP 413. `/v1` route misses and unsupported methods also use a problem response. Problems use `application/problem+json` and RFC 9457-style `type`, `title`, `status`, and `detail` fields plus stable `code`, `request_id`, `retryable`, and field errors.

```json
{
  "type": "about:blank",
  "title": "Invalid lookup request",
  "status": 422,
  "code": "validation_error",
  "detail": "One or more lookup fields are invalid.",
  "request_id": "01J...",
  "retryable": false,
  "errors": [{"field":"query","message":"must not be blank"}]
}
```

All other paths return HTTP 404.
