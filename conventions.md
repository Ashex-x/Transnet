# Coding conventions

These conventions apply to the entire Transnet repository.

## Rust

Follow the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html), with repository configuration taking precedence. Use two-space indentation, no tabs, the repository `rustfmt.toml`, and a 100-column target.

Every Rust source file starts with a `//!` module comment. Every public item has a concise `///` comment that explains its purpose and contract. Fallible public APIs include `# Errors`; unsafe APIs include `# Safety`.

Use `anyhow` with context at process boundaries and `thiserror` for typed library errors. Avoid `unwrap` and `expect` outside tests. Use structured `tracing` fields, never log source text or credentials, and do not block Tokio executor threads.

Unit tests live beside the code they cover. HTTP and provider integration tests live in `tests/`. Name tests as short behavior statements.

## Repository

Transnet is one Cargo package. Rust code lives in `src/`, integration tests in `tests/`, runtime configuration in `config/`, and hand-written documentation in `docs/`.

Use brief, descriptive filenames instead of repeated generic names so repository searches are unambiguous. `docs/transnet.md` owns the system design, `docs/guides/` owns task-oriented instructions, and `docs/reference/` owns external contracts. Keep Rust API detail in source comments and rustdoc.

Keep each prose paragraph and simple list item on one physical line. Use Mermaid for architecture and flows. Documentation describes implemented behavior; update it with every public API, configuration, or process-boundary change.

Keep generated files, build output, logs, runtime state, credentials, and editor-local state out of version control. Commit `Cargo.lock` because Transnet is deployable.

## Checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## Git

Use concise Conventional Commit subjects with `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, or `chore:`. Branches use an intent prefix and kebab-case description, such as `refactor/core-only`. Inspect status and diff before staging explicit paths, and do not commit unrelated changes.
