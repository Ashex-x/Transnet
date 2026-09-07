# Transnet roadmap

This roadmap records work that remains outside the current translation core. Completed implementation details belong in module documentation rather than this backlog.

## Current hardening

- [ ] Replace process-local translation identifiers with durable IDs when persistence is introduced.
- [ ] Add stubbed provider integration tests covering retry and malformed provider responses.
- [ ] Replace permissive CORS with configuration before exposing the server outside a trusted local environment.
- [ ] Decide whether the reserved `logging.file` setting should be implemented or removed in the next configuration-breaking release.

## Deferred adapters

- [ ] Add SQLite history and cache behind a storage interface.
- [ ] Add FlatBuffers IPC over a Unix socket as a thin adapter over `TranslationService`.
- [ ] Add authentication, authorization, and rate limiting only when the service boundary requires them.

## Deferred language features

- [ ] Add glossary and translation-memory support.
- [ ] Add streaming provider responses.
- [ ] Evaluate semantic indexing only after a concrete retrieval use case exists.
- [ ] Add voice and image translation through separate input adapters.

The files `config/transnet_api.toml` and `config/transnet_db.toml` are examples for deferred work; the current server intentionally does not load them.
