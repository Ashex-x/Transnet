# Coding Conventions

These conventions apply to the entire Transnet repository. More specific guidance in a module document under `docs/` may override this file for that module only.

```mermaid
flowchart LR
  code["Write code"] --> format["Format"]
  format --> lint["Lint"]
  lint --> test["Test"]
  test --> docs["Update related docs"]
  docs --> commit["Commit"]
```

## Rust documentation

Every public item must have a short `///` comment that explains its purpose and any contract that is not clear from the signature. Start each Rust source file with a `//!` module comment that states the module's responsibility and boundary.

Write for a reader who did not implement the code. Prefer why, contracts, units, lifecycle, error behavior, concurrency guarantees, and ownership boundaries over a restatement of the identifier. Link Rust types with ``[`TypeName`]``. Fallible public APIs should include `# Errors`; unsafe APIs must include `# Safety`. Document panics when a public function may panic under normal caller control.

When editing a file, document every public item changed and repair stale nearby comments. Private helpers need comments only when their invariants or behavior are easy to misuse.

```rust
//! Parses and validates structured responses returned by the language model.

/// Parses JSON from a raw model response, including a Markdown code fence.
///
/// # Errors
/// Returns an error when the response does not contain valid JSON.
pub fn parse_llm_response(content: &str) -> anyhow::Result<serde_json::Value> {
  // ...
}
```

## Markdown documentation

Hand-written documentation lives under `docs/` and starts at `docs/README.md`. System-wide design stays at the top level, task-oriented instructions live in `docs/guides/`, and code-facing service contracts live in `docs/reference/<service>/`. Keep Rust API detail in source comments and generate it with rustdoc; do not edit generated output.

Keep each prose paragraph and simple list item on one physical line. Markdown is exempt from the Rust line-width limit. Use Mermaid for architecture, flows, and state relationships; do not use ASCII art for diagrams. Compact directory trees and literal terminal output may use fenced text blocks.

Write concise internal engineering notes: one idea per paragraph, with bullets only for genuine enumerations. Prefer prose or a small example to a two-column table that merely maps names. When English and Chinese documents both exist, use `_cn` before the extension, keep their headings and API names aligned, and link the translations to each other.

Update the relevant document in the same change whenever a public API, configuration field, persistent format, deployment step, or process boundary changes. Do not describe planned behavior as if it is already implemented; label roadmaps explicitly.

### Module documents

Each service has a `docs/reference/<service>/README.md` entry point. Each non-trivial public module should have one document beside that index. A small adapter or single-purpose utility may use a short free-form document. A module with state, concurrency, multiple dependencies, non-obvious algorithms, or a public surface used by other modules should cover:

- **Overview**: purpose, ownership boundaries, and dependencies.
- **Architecture**: data flow and process boundaries.
- **API**: inputs, outputs, types, errors, timing, and caller responsibilities.
- **Design**: important choices, state transitions, assumptions, and known limits.
- **Testing**: test method, command, and pass criteria.

Complex documents begin with a **Contents** list linking to those sections. Add focused sections such as Security, Configuration, or Operations when the module needs them.

## HTTP API contracts

Hand-written HTTP contracts live below the owning service at `docs/reference/<service>/api.md`. When the contract has multiple domains, use `docs/reference/<service>/api/README.md` for shared rules and place one file per domain beside it. Update a contract in the same change as its handler.

Each route must state authentication, request fields, success status and payload, and exact error statuses and wire codes. Use field names and enum literals exactly as serialized by the server. Prefer one minimal JSON request and response over tables that repeat type declarations. Document shared envelopes, timestamp formats, identifiers, content types, and validation limits once in a common interface section.

Write for a caller who does not have the implementation open. Make it clear who may call a route, what to send, what is returned, and how validation, upstream-service, and internal failures differ.

## Rust style

Follow the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html), with repository configuration taking precedence. Use two spaces for indentation, no tabs, a 100-column target, and the repository `rustfmt.toml`. Run rustfmt instead of aligning code manually.

Keep modules and functions focused. Prefer explicit types when they clarify a boundary, but do not annotate obvious locals. File and module names use concise `snake_case`; type and trait names use `UpperCamelCase`; functions, variables, and test names use `snake_case`; constants use `SCREAMING_SNAKE_CASE`.

Use `anyhow` with contextual errors in binaries and application boundaries. Use `thiserror` for typed errors exposed by libraries. Return `Result` instead of panicking for recoverable failures, and avoid `unwrap` and `expect` outside tests unless an invariant is documented at the call site.

Use `tracing` for structured logging. Prefer fields, for example `tracing::info!(address = %address, "server started")`. Never use `println!` as operational logging and never log secrets, credentials, or complete sensitive request bodies.

Use Tokio for asynchronous work. Do not perform blocking file, network, or CPU-heavy work on an async executor thread; use an async API or `tokio::task::spawn_blocking`. Keep lock scope small and never hold a synchronous lock across `.await`.

Configuration belongs in the appropriate typed config structure and checked-in example TOML file. Give new settings sensible defaults where compatibility requires them, document purpose and valid values, and never commit credentials or local `.env` files.

## Repository layout

`island-transnet/` owns the translation library and its service binary. `transnet-server/` owns the HTTP gateway or adapter that depends on `island-transnet`. Shared translation behavior belongs in `island-transnet`; transport handlers stay thin and call the shared service rather than duplicating business logic.

Rust unit tests live in `#[cfg(test)]` modules at the bottom of the source file they cover. Crate-level integration tests live in `<crate>/tests/`. Name tests like a sentence using `test_<behavior>_<condition>_<result>`. Add tests for regressions, parsing, validation, state transitions, and other logic that is costly to verify manually; trivial wiring need not have a dedicated test.

Keep generated files, build output, logs, runtime state, credentials, and editor-local state out of version control. Commit `Cargo.lock` for deployable applications; do not blanket-ignore lockfiles.

## Local checks

Run checks for the complete Cargo workspace from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Formatting, linting, and tests must pass before pushing. Generate rustdoc when public interfaces or their comments change.

## Editor setup

Repository configuration is the source of truth. Editors should honor `.editorconfig` and `rustfmt.toml`, format Rust with rustfmt on save, and use rust-analyzer for diagnostics. Do not maintain a separate editor-only style guide.

## Development workflow

For a new or redesigned module:

1. Define the problem, success criteria, constraints, and non-goals.
2. Define the public interface and process boundary before selecting implementation details.
3. Write or update the module document, including its testing approach.
4. Implement the smallest useful behavior with focused tests.
5. Add public API comments and update configuration and HTTP contracts.
6. Run format, lint, tests, and relevant documentation checks.
7. Commit code, tests, configuration, and documentation as one reviewable logical change.

For a small change, update affected API comments, implement the behavior, add regression tests when useful, update only the relevant documentation, and run the package checks above.

## Git

Use an imperative, concise Conventional Commit subject with one of `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, or `chore:`. Keep one logical change per commit. Documentation-only work uses `docs:`; behavior plus documentation uses the behavior prefix.

Branch names use an intent prefix and kebab-case description, such as `feat/translation-cache`, `fix/response-parser`, `docs/api-contract`, `refactor/remove-webui`, or `chore/update-dependencies`. Keep the default branch deployable and do not force-push a shared branch without agreement.

Inspect `git status` and `git diff` before staging. Stage explicit paths rather than `git add .`, and do not commit unrelated user changes, generated output, logs, local databases, secrets, or temporary files.

## Review checklist

- [ ] The workspace passes rustfmt, Clippy with warnings denied, and tests.
- [ ] Public items and changed modules have accurate Rust documentation.
- [ ] Errors carry context and recoverable failures do not panic.
- [ ] Async code does not block executor threads or hold locks across `.await`.
- [ ] Logging is structured and contains no secrets or unnecessary payloads.
- [ ] API, configuration, architecture, and operations docs match the code.
- [ ] New non-trivial behavior has focused tests.
- [ ] The diff contains no WebUI assets, generated output, credentials, logs, or unrelated changes.
