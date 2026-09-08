# Transnet HTTP API

Requests and responses use JSON. No route requires authentication.

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

All other paths return HTTP 404.
