# Transnet

Transnet is a Rust translation service for Island OS. The design goal is to ship a small, reliable translation backend first, then add richer language features without making the core request path fragile.

## Current Direction

The project now follows a staged architecture:

- **V1**: health check, translation endpoint, shared translation core, OpenAI-compatible local LLM backend
- **V1.1**: SQLite-backed translation history and cache
- **V1.2**: FlatBuffers IPC server so Island OS can call the same translation core over a Unix socket
- **Later**: auth, favorites, richer word analysis, optional semantic indexing

This keeps the first delivery small while preserving a clean path to the larger vision.

## Core Principles

- **One translation core**: REST and IPC should both call the same service logic
- **Thin transports**: Axum REST handlers and FlatBuffers IPC handlers are adapters, not business logic
- **Provider-agnostic LLM layer**: treat llama-server as an OpenAI-compatible endpoint, not as a hardcoded cloud dependency
- **Defer optional systems**: Qdrant, advanced NLP, and auth should not block a working translator
- **Stable machine-readable output**: LLM responses should be parsed into strict Rust types instead of free-form prose

## Revised Architecture

```text
Go Gateway / Web UI ----HTTP----+
                                |
Island OS --------FlatBuffers---+--> Transnet Transports --> Translation Core --> OpenAI-compatible LLM
                                                        |
                                                        +--> SQLite (later stage)
```

The most important design choice is that the transport layer and the translation logic are separate. This keeps REST, IPC, and future integration paths consistent and easier to test.

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 2021 |
| API | Axum |
| IPC | FlatBuffers over Unix socket |
| Storage | SQLite |
| LLM Provider Interface | OpenAI-compatible HTTP API |
| Local LLM Runtime | llama-server on `http://localhost:13595/v1` |
| Frontend/Gateway | Go + TypeScript |

## What Is Deferred

These are still valuable, but they are no longer part of the first usable build:

- Qdrant-based semantic indexing
- Rich word graph generation
- Full auth and user management
- Advanced NLP pipeline
- Favorites and profile management

## Quick Start Target

The initial runnable target is:

1. `GET /health`
2. `POST /translate`
3. Local llama-server via OpenAI-compatible API
4. Optional Go gateway proxying `/transnet/api/*`

## Documentation

See the [docs](./docs) folder for details:

- [Architecture](./docs/architecture.md) - revised system architecture and layering
- [API](./docs/api.md) - MVP API and later API roadmap
- [IPC](./docs/ipc.md) - FlatBuffers transport design
- [Development](./docs/development.md) - local setup with llama-server
- [Integration](./docs/integration.md) - Island OS and Go gateway integration
- [Project Structure](./docs/project_structure.md) - workspace structure

## Project Structure

```text
island-transnet/
├── Cargo.toml
├── README.md
├── docs/
├── config/
├── crates/
│   ├── transnet-types/
│   ├── transnet-llm/
│   ├── transnet-api/
│   ├── transnet-db/      # introduced when persistence is added
│   ├── transnet-auth/    # introduced when auth is added
│   └── transnet-nlp/     # introduced when richer classification is added
└── bins/
    └── transnet-server/
```

## License

MIT
