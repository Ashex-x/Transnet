# Translation API

## Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Interfaces](#interfaces)
- [Routes](#routes)
- [Testing](#testing)

## Overview

Translation routes forward requests to the backend. History and favorites routes preserve the existing wire surface but do not persist data.

## Authentication

These routes require no credentials. Translation responses omit `user_id` until authentication is implemented.

## Interfaces

Responses use the [gateway envelopes and shared wire rules](../api.md).

## Routes

## POST /translate

## POST /api/transnet/translate

Both paths forward the same request to the backend.

```json
{
  "text": "Hello world",
  "source_lang": "en",
  "target_lang": "zh",
  "mode": null,
  "input_type": "sentence"
}
```

Success returns the backend translation with a gateway `translation_id`. Invalid backend input returns `422 VALIDATION_ERROR`. Backend transport, parsing, and other status failures return `503 BACKEND_ERROR`.

## GET /api/transnet/history

Accepts optional `page`, `limit`, `source_lang`, `target_lang`, and `input_type` query parameters. Returns an empty page because persistence is not implemented.

## GET /api/transnet/history/:id

Always returns `404 NOT_FOUND` because persistence is not implemented.

## DELETE /api/transnet/history/:id

Returns a static success message and does not delete persisted data.

## POST /api/transnet/favorites

Accepts `translation_id` and optional `note`, then returns a mock favorite result.

## GET /api/transnet/favorites

Accepts optional `page` and `limit` query parameters. Returns an empty page because persistence is not implemented.

## PUT /api/transnet/favorites/:id

Accepts `note` and returns a mock updated favorite.

## DELETE /api/transnet/favorites/:id

Returns a static success message and does not delete persisted data.

## Testing

Router unit tests assert that `/` and `/assets/app.js` return 404, preventing accidental restoration of the removed WebUI fallback.
