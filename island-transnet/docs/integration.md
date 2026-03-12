# Integration with Island OS

This document describes the revised integration strategy for Transnet inside Island OS. The main principle is to keep Transnet's core translation logic in Rust and let both HTTP and IPC call that same core.

## Overview

Transnet integrates with Island OS through two transport paths:

1. **REST** for gateway and frontend access
2. **FlatBuffers IPC** for internal Island OS clients that need low-overhead local communication

The transports should be different adapters over the same shared translation core.

## Integration Principles

- keep the Go gateway thin
- avoid duplicating translation logic in Go
- use IPC for Island-native consumers
- use REST for external or browser-facing consumers
- do not require auth, favorites, or Qdrant before translation works

## Revised Architecture

```text
Island OS client ----FlatBuffers IPC----+
                                        |
Go gateway -----------HTTP--------------+--> Transnet adapters --> Translation core --> LLM provider
```

## Workspace Strategy

### Option A: Independent Workspace

Keep `island-transnet` as an independent workspace. This is the simplest way to keep Transnet isolated while it is still being built.

### Option B: Add To Root Workspace Later

Once the crates stabilize, the Transnet crates can be added to the Island root workspace if shared development workflows make that worthwhile.

## IPC Strategy

The IPC layer should be exposed by Transnet itself as a FlatBuffers server API.

This means:

- Island OS clients build FlatBuffers request messages
- Transnet listens on a Unix socket
- Transnet decodes the request
- Transnet routes it through the same translation core used by REST
- Transnet returns a FlatBuffers response

The Go gateway should not be responsible for FlatBuffers translation handling.

## Recommended Data Flow

### HTTP Path

```text
Browser or gateway -> /transnet/api/translate -> Axum handler -> Translation core -> LLM client
```

### IPC Path

```text
Island OS client -> Unix socket -> FlatBuffers handler -> Translation core -> LLM client
```

## FlatBuffers Scope

The first IPC scope should stay small.

### Initial message types

- `TranslationRequest`
- `TranslationResponse`
- `ErrorResponse`
- optional `HealthRequest` and `HealthResponse`

Do not design the initial IPC schema around all future features. Start with translation only.

## Go Gateway Role

The Go gateway should act as an HTTP integration layer for the existing frontend and other web-facing consumers.

Recommended responsibilities:

- proxy `/transnet/api/*`
- preserve request and response payloads
- add gateway-level logging
- avoid business logic duplication

Not recommended for the first implementation:

- reimplementing translation logic
- building a separate Go-side Transnet client unless there is a proven need
- owning IPC behavior

## Island OS Client Shape

The Island side can later add a small Rust client crate for IPC usage.

Suggested responsibilities:

- connect to the Unix socket
- build FlatBuffers translation requests
- parse FlatBuffers responses
- expose a small ergonomic Rust API to Anima and other internal systems

## Suggested Integration Order

1. Build Transnet REST MVP
2. Add Go proxy routes if needed
3. Add Transnet FlatBuffers IPC server
4. Add Island-side Rust IPC client
5. Add persistence and richer user-level features

## Why This Order Works

- it validates the translation core before adding multiple integration points
- it makes debugging easier because REST can be tested first with normal tooling
- it avoids coupling Island internals to an unstable early API

## Future Integration Areas

These are intentionally later-stage:

- shared auth across Island OS and Transnet
- user translation history access from Island systems
- richer language-learning workflows for Anima
- favorites and profile synchronization
