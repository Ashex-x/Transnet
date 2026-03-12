# REST API

This document describes the revised REST API strategy for Transnet. The API is intentionally staged: the first release only exposes the minimum set of endpoints needed to run a useful translator, while later phases add auth, persistence, and richer features.

## Overview

- **Base URL**: `/transnet/api`
- **Protocol**: HTTP/HTTPS
- **Content-Type**: `application/json`
- **Primary target for v1**: health + translation
- **Authentication**: not required for the MVP

## API Philosophy

The original API draft was broad, but the implementation should start smaller.

### MVP endpoints

- `GET /health`
- `POST /translate`

### Later endpoints

- auth endpoints
- translation history
- favorites
- user profile

This keeps the first public API consistent with a deliverable backend instead of documenting a large surface area before the core service exists.

## Response Format

### Success Response

```json
{
  "success": true,
  "data": {}
}
```

### Error Response

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message"
  }
}
```

### HTTP Status Codes

- `200 OK`: successful request
- `400 Bad Request`: invalid request data
- `404 Not Found`: route not found
- `422 Unprocessable Entity`: semantically invalid request
- `500 Internal Server Error`: server or provider failure
- `503 Service Unavailable`: downstream model is unavailable

## MVP Endpoints

### 1. Health Check

**Endpoint**: `GET /health`

**Purpose**:

- verifies that the Transnet process is running
- can later expose provider and database readiness

**Response**:

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "service": "transnet"
  }
}
```

### 2. Translate

**Endpoint**: `POST /translate`

**Purpose**:

Translate text using the shared translation core and an OpenAI-compatible model backend.

**Request Body**:

```json
{
  "text": "The quick brown fox",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic",
  "input_type": "auto"
}
```

### Request Fields

- `text` required: text to translate
- `source_lang` required: source language code
- `target_lang` required: target language code
- `mode` optional: `basic` for the MVP, with room for later modes such as `explain`
- `input_type` optional: `auto`, `word`, `phrase`, `sentence`, or `text`

### MVP Response

```json
{
  "success": true,
  "data": {
    "translation": "El rapido zorro marron",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION"
  }
}
```

### Notes

- The MVP response is intentionally small and stable.
- Rich word analysis is deferred until the core translation path is working.
- The server should parse the model output into typed Rust data before returning JSON.

## Validation Rules

- `text` must not be empty
- `source_lang` and `target_lang` must not be empty
- `source_lang` and `target_lang` should be normalized consistently
- `mode` must be one of the supported values
- `input_type` should default to `auto` if omitted

## Provider Behavior

The REST API should not assume a cloud OpenAI account. The expected provider interface is OpenAI-compatible HTTP, with local development targeting:

```text
http://localhost:13595/v1
```

This allows Transnet to use llama-server while keeping the client abstraction provider-agnostic.

## Roadmap Endpoints

These are intentionally not part of the MVP implementation.

### Auth

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`

### User Data

- `GET /history`
- `POST /favorites`
- `GET /favorites`
- `GET /user/profile`

### Why They Are Deferred

- They increase surface area before the core translator exists
- They add storage and permission complexity
- They are better built on top of a working translation core and SQLite schema

## Gateway Usage

The Go gateway should stay thin and proxy `/transnet/api/*` routes to the Rust service. It should not reimplement translation logic.

## Future Extensions

- richer response modes such as `explain`
- persisted translation history
- favorites and user accounts
- request streaming
- rate limiting and auth middleware
