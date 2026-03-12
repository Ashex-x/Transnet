# Transnet Implementation TODO

This file tracks the revised implementation order for Transnet. The goal is to get a small working translator online first, then expand the service carefully.

## Phase 1: Project Setup ✅

- [x] Create workspace structure
- [x] Create configuration files
- [x] Write and revise initial documentation

## Phase 2: Translation MVP

### `transnet-types`

- [ ] Create shared request and response models for translation
- [ ] Create typed config models
- [ ] Create typed domain errors
- [ ] Add serde derives where needed
- [ ] Write unit tests

### `transnet-llm`

- [ ] Create OpenAI-compatible client wrapper
- [ ] Support configurable `base_url`, `api_key`, `model`, and timeout
- [ ] Target local llama-server at `http://localhost:13595/v1`
- [ ] Implement prompt builder for simple translation
- [ ] Implement strict response parser
- [ ] Add retry logic for provider failures
- [ ] Write unit tests with mocked provider responses

### `transnet-api`

- [ ] Create Axum server setup
- [ ] Add `GET /health`
- [ ] Add `POST /translate`
- [ ] Validate request payloads
- [ ] Return stable JSON response shapes
- [ ] Add API endpoint tests

### `transnet-server`

- [ ] Create application entry point
- [ ] Load config and `.env`
- [ ] Wire shared translation core into Axum routes
- [ ] Add graceful shutdown
- [ ] Verify startup and shutdown behavior

## Phase 3: Local Developer Workflow

- [ ] Write `island-transnet/.env` for local llama-server usage
- [ ] Write `transnet.bash` to launch only the Transnet backend
- [ ] Update `server.bash` to launch only the Go server
- [ ] Document expected local ports and paths

## Phase 4: Persistence

### `transnet-db`

- [ ] Create SQLite client and migrations
- [ ] Add translation history storage
- [ ] Add simple translation cache
- [ ] Create schema versioning
- [ ] Write unit and integration tests for SQLite behavior

## Phase 5: IPC API

### Transnet IPC Server

- [ ] Add FlatBuffers request and response schema for translation
- [ ] Implement Unix socket server inside Transnet
- [ ] Map IPC requests into the shared translation core
- [ ] Encode typed FlatBuffers responses
- [ ] Write IPC roundtrip tests

### Island OS Client

- [ ] Create `island-transnet-client` crate when needed
- [ ] Implement connection and request helpers
- [ ] Parse FlatBuffers translation responses
- [ ] Add example usage from Island-side code

## Phase 6: Auth And User Features

### `transnet-auth`

- [ ] Add password hashing
- [ ] Add JWT creation and validation
- [ ] Add auth middleware
- [ ] Add register and login flows
- [ ] Write auth tests

### Additional REST Endpoints

- [ ] Add history endpoints
- [ ] Add favorites endpoints
- [ ] Add user profile endpoints

## Phase 7: Input Enrichment

### `transnet-nlp`

- [ ] Add heuristic-first input classification
- [ ] Add normalization helpers
- [ ] Add optional sentence and phrase utilities
- [ ] Expand only when real use cases require richer analysis

## Backlog

- [ ] Qdrant or other semantic indexing
- [ ] Rich word relationship graphs
- [ ] Streaming translation responses
- [ ] Translation memory
- [ ] Glossary support
- [ ] Voice and image translation
- [ ] Rate limiting by user tier
- [ ] Redis or distributed caching

## Notes

- The MVP is translation first, not full platform parity
- REST and IPC should share one translation core
- Go should remain a thin gateway layer
- Qdrant is explicitly deferred until the simpler system proves insufficient
