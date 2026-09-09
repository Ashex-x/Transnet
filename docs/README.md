# Transnet documentation

## Design and planning

- [System design and architecture](transnet.md): authoritative current boundary and target English-learning architecture.
- [English-learning experience](product/learning-experience.md): learner behavior, learning cards, graph exploration, practice, and personalization.
- [Basic-core overall plan](todo.md): decisions, phases, dependencies, exit criteria, and deferred work.

## Reference

- [Implemented HTTP API](reference/transnet-api.md): current public routes and JSON shapes.
- [Proposed learning HTTP API](reference/learning-api.md): target `/v1` contract, graph payloads, feedback, practice, and privacy routes.
- [Proposed MySQL schema](reference/mysql-schema.md): table catalog, DDL, encryption, retention, and transactional invariants.

## Guides

- [Configuration](guides/configuration.md): current listener, routing, and provider settings.
- [Development](guides/development.md): current local startup, checks, and deployment notes.
- [Content publishing](guides/content-publishing.md): proposed lexical source ingestion, validation, vector build, publication, removal, and rollback.
- [Quality assurance](guides/quality-assurance.md): proposed benchmarks, failure tests, release gates, and monitoring.

## Repository rules

- [Coding conventions](../conventions.md): Rust, repository, documentation, checks, and Git conventions.

Use descriptive filenames. Implemented documentation stays aligned with current behavior. Target documents label proposed behavior and retain a clearly separated implemented baseline.
