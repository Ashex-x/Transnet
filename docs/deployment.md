# Backend Translation Service API

This document describes the API of the **backend translation service**. It is called by the web server gateway (see [api.md](api.md)). The backend performs translation only (with optional RAG) and exposes no user or account endpoints.

---

## Overview

**Protocol**: HTTP  
**Content-Type**: `application/json` for request and response bodies  
**Authentication**: None. The backend is invoked by the web server or other trusted callers; auth is handled by the gateway.

**Related**:
- **Web server (gateway) API**: [api.md](api.md) — public API, account, history, favorites, profile.
- **Architecture**: [architecture.md](architecture.md) — input analysis, translation types, workflows.
- **Translation types**: [types.md](types.md) — canonical definitions of `input_type` and `mode`.
- **Databases**: [database.md](database.md) — web server (MySQL), backend (RAG).

---

## Endpoint Summary

| Method | Path         | Description                           |
|--------|--------------|---------------------------------------|
| POST   | `/translate` | Translate text                        |
| GET    | `/health`    | Liveness + dependency readiness check |

---

## Translation

### POST `/translate`

Translate text. Request and response format MUST match the translation contract used by the web server (see [api.md — POST /translate](api.md#post-translate)). The backend does not persist history or attach `user_id`; the gateway does that when it calls this service.

Translation behavior is determined by **input type** and **translation mode** as defined in [types.md](types.md) and [architecture.md](architecture.md).

**Request Body**:
```json
{
  "text": "The quick brown fox",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic"
}
```

**Fields**:
- `text` (required): Text to translate
- `source_lang` (required): Source language code (ISO 639-1)
- `target_lang` (required): Target language code (ISO 639-1)
- `mode` (required): Translation mode, one of:
  - `basic`
  - `explain`
  - `full_analysis`
  Backend will automatically detect `input_type` and validate that this `mode` is supported for the detected input type. See [types.md](types.md) for detection criteria and per-input-type behavior.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "translation": {
      // Translation field type specific data
    }
  }
}
```

**Translation field type**: The `translation` field contains type-specific data based on the detected `input_type` and requested `mode`. There are **9 possible types** (see [types.md](types.md)):

| Input Type | Mode               | Translation Type            |
|------------|---------------------|----------------------------|
| `word`     | `basic`              | `TranslationWordBasic`     |
| `word`     | `explain`             | `TranslationWordExplain`    |
| `word`     | `full_analysis`       | `TranslationWordFullAnalysis`|
| `phrase`    | `basic`              | `TranslationPhraseBasic`    |
| `phrase`    | `explain`             | `TranslationPhraseExplain`   |
| `phrase`    | `full_analysis`       | `TranslationPhraseFullAnalysis`|
| `sentence`   | `basic`              | `TranslationSentenceBasic`   |
| `sentence`   | `explain`             | `TranslationSentenceExplain`  |
| `paragraph`  | `basic`              | `TranslationParagraphEssayBasic`|
| `essay`     | `basic`              | `TranslationParagraphEssayBasic`|

Clients should deserialize the `translation` field based on the detected `input_type` and the `mode` used.

**Note**: Backend response does not include `user_id`. The gateway adds `user_id` from the JWT when returning the response to the client and persists to history when the user is authenticated.

**Error Responses**:
- `400`: Invalid request data
- `422`: Semantically invalid request
- `500`: Internal server error
- `503`: LLM API or dependency unavailable

---

## Health

### GET `/health`

Combined liveness and readiness check.

- **Liveness**: The HTTP server is up and responding.
- **Readiness**: Core dependencies (e.g. Qdrant, LLM API) are reachable and healthy.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "status": "ready",
    "service": "transnet-backend",
    "checks": {
      "qdrant": "connected",
      "llm_api": "reachable"
    }
  }
}
```

**Error Responses**:
- `503`: Server shutting down or unable to serve, or required dependency check failed. The response body SHOULD include `checks` with per-dependency status when available.

---

## Standard Response Format

**Success**:
```json
{
  "success": true,
  "data": { /* ... */ }
}
```

**Error**:
```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message"
  }
}
```

**HTTP Status Codes**: `200` OK, `400` Bad Request, `422` Unprocessable Entity, `500` Internal Server Error, `503` Service Unavailable
