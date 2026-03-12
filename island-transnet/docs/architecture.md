# Architecture

## System Overview

Transnet should be built around a single translation core with multiple transports on top of it. The purpose of this architecture is to ship a small, reliable translation service first and then grow it without coupling every future feature to the request path.

## Design Goals

- Keep the first production path small and testable
- Avoid forcing auth, embeddings, and advanced NLP into the MVP
- Let REST and IPC call the same core service
- Treat llama-server as an OpenAI-compatible provider, not as a special case spread across the codebase
- Make structured, validated LLM output part of the design from day one

## High-Level Architecture

```text
Clients
  ├─ Go gateway / web UI
  ├─ direct HTTP clients
  └─ Island OS IPC clients
           |
           v
Transport Layer
  ├─ REST adapter (Axum)
  └─ IPC adapter (FlatBuffers over Unix socket)
           |
           v
Translation Core
  ├─ request validation
  ├─ lightweight input classification
  ├─ prompt construction
  ├─ response parsing
  └─ domain response shaping
           |
           v
Infrastructure
  ├─ OpenAI-compatible LLM client
  └─ SQLite persistence/cache (later stage)
```

## Recommended Layering

### Transport Layer

This layer should stay thin.

- Parse HTTP or FlatBuffers requests
- Convert them into shared Rust request types
- Call the translation core
- Convert results into transport-specific responses

Neither REST handlers nor IPC handlers should own translation logic.

### Translation Core

This is the center of the system.

- Normalizes input
- Applies simple classification rules such as word, phrase, sentence, or text
- Builds strict prompts for the model
- Parses model output into stable Rust types
- Applies feature flags or mode selection such as `basic`, `explain`, or later richer analysis

This layer is what both `POST /translate` and IPC translation requests should call.

### Infrastructure

This layer contains integrations and persistence concerns.

- LLM client against an OpenAI-compatible base URL
- Configuration loading
- Logging and tracing
- SQLite history/cache when that stage is added

Optional systems such as Qdrant belong here only after the core service is already stable.

## Why This Is Better Than The Original Draft

The previous design made advanced features part of the default request path: auth, cache lookup, semantic analysis, Qdrant writes, and rich NLP. That is a good long-term roadmap, but it is not a good first delivery shape.

The revised design improves this by:

- reducing moving parts in the first request path
- making the core logic reusable across transports
- letting the Go gateway remain a thin integration layer
- deferring operational complexity until after the MVP works

## Initial Request Flow

### REST Translation Flow

```text
1. Client sends POST /translate
2. Axum handler validates the request
3. Handler maps request into a shared translation command
4. Translation core builds an OpenAI-compatible prompt
5. LLM client calls llama-server at http://localhost:13595/v1
6. Response is parsed into typed Rust output
7. API returns normalized JSON
```

### IPC Translation Flow

```text
1. Island OS sends a FlatBuffers TranslationRequest over Unix socket
2. IPC adapter decodes the message
3. IPC adapter maps it into the shared translation command
4. Translation core processes the request exactly as REST does
5. IPC adapter encodes the TranslationResponse as FlatBuffers
6. Island OS receives the typed response
```

## Module Responsibilities

### `transnet-types`

- shared request and response models
- config models
- typed domain errors
- transport-neutral translation result types

### `transnet-llm`

- OpenAI-compatible HTTP client
- provider configuration such as base URL, API key, model, timeout
- prompt builders
- strict response parsing

### `transnet-api`

- Axum route registration
- HTTP extractors and response formatting
- health and translation endpoints for the MVP
- later: auth, history, favorites, IPC adapter

### `transnet-db`

- SQLite persistence and migrations
- translation history
- cache tables
- deferred semantic indexing support if later needed

### `transnet-auth`

- JWT and password hashing
- only added after the service already works as a simple translator

### `transnet-nlp`

- heuristic-first classification
- language utilities
- optional future richer analysis

## Provider Strategy

Transnet should not assume a cloud OpenAI dependency. Instead, it should assume an OpenAI-compatible provider interface with configurable values:

- `base_url`
- `api_key`
- `model`
- `timeout_seconds`

For local development, the expected provider is llama-server on `http://localhost:13595/v1`.

## Persistence Strategy

SQLite should be the first persistence layer because it is easy to operate and enough for:

- translation history
- simple request/result caching
- favorites later on

Qdrant is useful only if semantic indexing proves necessary after the core translator is already working.

## Error Handling

The architecture should separate user-facing failures from internal errors.

- validation errors should return clear client messages
- provider failures should include context in logs
- transport adapters should map domain errors consistently
- typed errors should be used in library crates
- application startup and orchestration can use `anyhow`

## Observability

From the start, the service should include:

- request IDs
- structured logs with `tracing`
- latency logging around LLM calls
- clear distinction between transport errors and translation-core errors

## Staged Growth Plan

### Stage 1

- `GET /health`
- `POST /translate`
- shared translation core
- llama-server integration

### Stage 2

- `.env` and launcher scripts
- local operational workflow

### Stage 3

- SQLite history and cache
- auth
- additional REST routes
- FlatBuffers IPC server

### Later

- Qdrant
- richer linguistic analysis
- streaming responses
- advanced user features
