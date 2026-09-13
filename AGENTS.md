# Agent instructions

Read the repository before writing code. Start with `README.md`, `conventions.v1.0.1.md`, and `docs/documentation-index.md`; then read the authoritative design or interface document for the area you will change and its Chinese mirror under `docs_cn`. Inspect `git status` and the relevant diff before editing because the worktree may contain user changes.

## Repository map

- `src`: the single Rust package and its public service boundaries.
- `tests`: integration and HTTP contract tests.
- `config`: checked-in, non-secret runtime configuration.
- `docs/transnet.md`: authoritative target system design and product semantics.
- `docs/interfaces`: normative wire and storage-adapter contracts.
- `docs/reference`: detailed module and machine-contract references.
- `docs/guides`: development, publishing, configuration, and quality workflows.
- `docs_cn`: path-mirrored Chinese documentation; every inner filename has the `_cn` suffix.

Use `rg` and `rg --files` to find the implementation, tests, and documentation that already own a concept. Follow existing names and module boundaries instead of creating a competing abstraction or document.

## Invariants

Transnet is a private, user-agnostic service. Do not add user identity, profiles, saved-item state, durable history, or request-content persistence to its reachable APIs, logs, metrics, caches, queues, MySQL data, or Qdrant data. Island-port may send a chronological list of minimal prior translation turns as request-scoped linguistic context; Transnet must discard it with the request. Product-owned state belongs to island-port. Only reviewed canonical content enters Transnet storage through the publication workflow; a live request never publishes itself.

Keep current runtime behavior distinct from target contracts. The current executable uses transitional loopback HTTP, while target internal interfaces use HTTP/1.1 JSON over Unix domain sockets. Do not describe a target route as implemented until its handler, composition, tests, and documentation exist.

MySQL is authoritative for canonical structured content and immutable releases. Qdrant is a rebuildable, release-pinned projection for canonical nodes and typed edges. Vector similarity proposes candidates and never establishes a fact.

## Change rules

Follow `conventions.v1.0.1.md` in full. In particular:

- begin every Rust source file with a module comment and document every public item and field;
- reject unknown wire fields and keep request/response examples valid JSON;
- update an English API, configuration, process-boundary, or workflow document together with its Chinese mirror;
- preserve unrelated worktree changes and stage or commit only explicit paths when asked;
- never commit credentials, request text, generated output, logs, or runtime state.

For contract changes, trace the shape through the owning Markdown contract, Rust types and handlers, adapters, tests, examples, and adjacent summaries. Prefer shared named wire types when endpoints represent the same concept, but do not force unrelated graph or probe operations into a translation-specific shape.

## Verification

Run the narrowest relevant tests while iterating, then run the repository checks from the root before handoff:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

For documentation-only changes, also validate every edited JSON example and inspect English and Chinese heading/link parity. Report checks that could not be run and why.
