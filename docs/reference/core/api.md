# Core HTTP API

Parent: [Translation core reference](README.md).

## Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Interfaces](#interfaces)
- [Routes](#routes)
- [Testing](#testing)

## Overview

The `api` module exposes translation core behavior over HTTP. It owns routing, JSON envelopes, CORS, and domain-error status mapping. Translation logic remains in `TranslationService`; persistence and authentication are outside the current service.

## Authentication

The current routes do not require credentials. CORS is permissive for compatibility with existing local clients, so the listener should remain on a trusted interface until access controls and configured origins are implemented.

## Interfaces

Requests and responses use `application/json`. Enum values use snake case: input types are `auto`, `word`, `phrase`, `sentence`, `paragraph`, and `essay`; modes are `basic`, `explain`, and `full_analysis`.

Success responses use `{"success":true,"data":{...}}`. Errors use `{"success":false,"error":{"code":"VALIDATION_ERROR","message":"..."}}`.

## Routes

## GET /health

No authentication or request body is required. A successful readiness probe returns HTTP 200:

```json
{
  "success": true,
  "data": {
    "status": "ready",
    "service": "transnet-backend"
  }
}
```

The endpoint reports process readiness and does not probe the provider. Provider availability is established when a translation request is made.

## POST /translate

No authentication is required. Submit nonblank source text and language names or codes. `mode` defaults to `basic`; `input_type` defaults to `auto`.

```json
{
  "text": "hello",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic",
  "input_type": "word"
}
```

A successful response returns HTTP 200. The `translation` object shape depends on the resolved input type and mode:

```json
{
  "success": true,
  "data": {
    "translation_id": 1,
    "text": "hello",
    "translation": {
      "headword": "hello",
      "part_of_speech": "interjection",
      "phonetic": "/həˈloʊ/",
      "translations": ["hola"],
      "synonyms": [],
      "antonyms": [],
      "examples": []
    },
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "word"
  }
}
```

HTTP 422 with `VALIDATION_ERROR` means the input is blank or the mode is unsupported. `full_analysis` is unsupported for sentences, and `explain` and `full_analysis` are unsupported for paragraphs and essays. HTTP 503 with `LLM_ERROR` means all provider attempts failed. HTTP 500 uses `CONFIG_ERROR` or `INTERNAL_ERROR` for the corresponding domain categories.

## Testing

Unit tests use Axum's in-process service adapter. Run `cargo test -p transnet api::tests` from the repository root. Passing tests establish route and validation mapping behavior without contacting a provider.

Related: [LLM orchestration](llm.md), [shared types](types.md).
