# Transnet conventions

Version: 1.0.1

These rules apply to the entire repository. A later versioned conventions file supersedes this one only after repository references are updated.

## Rust

- Follow the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html), then repository configuration.
- Use two-space indentation, no tabs, `rustfmt.toml`, and a 100-column target.
- Start every Rust file with a `//!` module comment and document every public item with concise `///` contract text.
- Add `# Errors` to fallible public APIs and `# Safety` to unsafe APIs.
- Use contextual `anyhow` errors at process boundaries and `thiserror` for typed library errors; avoid `unwrap` and `expect` outside tests.
- Use structured `tracing`; never log source text, learner content, credentials, capabilities, or provider bodies.
- Never block Tokio executor threads.
- Keep unit tests beside their code and HTTP/provider integration tests in `tests/`; use short behavior-based test names.

## Repository and documentation

- Keep Transnet as one Cargo package: Rust in `src/`, integration tests in `tests/`, configuration in `config/`, and hand-written documentation in `docs/`.
- Use brief, descriptive, preferably unique filenames; avoid repeated generic names and duplicate documents.
- Keep system design in `docs/transnet.md`, task instructions in `docs/guides/`, human contracts in `docs/interfaces/`, machine contracts in `docs/reference/`, and Rust API detail in source comments and rustdoc.
- Give each contract one authoritative owner; other documents summarize and link instead of copying it.
- Keep prose paragraphs and simple list items on one physical line; use Mermaid for architecture and flows.
- Use fenced `json` blocks for endpoint and interface request/response examples. Keep examples simple, valid, and consistently indented; omit unrelated fields.
- Document implemented behavior and update documentation with every public API, configuration, or process-boundary change.
- Exclude generated files, build output, logs, runtime state, credentials, and editor state from version control; commit `Cargo.lock`.

## Verification

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## Git

- Use concise Conventional Commit subjects with `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, or `chore:`.
- Name branches with an intent prefix and kebab-case description, such as `refactor/core-only`.
- Inspect status and diff, stage explicit paths, and never commit unrelated changes.
