# Transnet HTTP API

Requests and responses use JSON. No implemented route requires authentication.

## GET /health

Returns HTTP 200 when the process can serve requests. This does not probe model providers.

```json
{"status":"ok"}
```

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

Malformed JSON returns HTTP 400. Blank text or malformed language codes return HTTP 422. An exhausted model request returns HTTP 503. Errors have one shape:

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

Successful and error responses include `Cache-Control: no-store` and `X-Request-Id`. Malformed JSON returns HTTP 400, invalid fields return HTTP 422, unavailable models return HTTP 503, and invalid structured model output after one repair returns HTTP 502. Errors use `application/problem+json` with stable `code`, `request_id`, `retryable`, and field errors.

All other paths return HTTP 404.
