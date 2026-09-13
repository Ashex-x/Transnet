# Transnet conventions

Version: 1.0.1

These rules apply to the entire repository. A later versioned conventions file supersedes this one only after repository references are updated. When a component-specific document under docs conflicts with this file, that component document wins for its scope only.

## Inline documentation

Every public item needs a short note so readers and generated documentation can understand its intent without opening the body. One sentence is sufficient when the name is clear, but document non-obvious design choices, usage constraints, caller responsibilities, units, error behavior, and invariants.

Code notes live beside the symbol in source. Product, architecture, operations, and contract documentation live under docs according to ownership.

Document every module or file header, public type, public function or method, and unsafe API. Also document public fields and private items when they are easy to misuse. Write for a reader who did not write the code: explain role, contract, lifecycle, boundaries, units, errors, panics, timeouts, ownership, validity, and idempotency when relevant. Do not repeat an identifier, and fix notes that no longer match the code.

For Rust, use `//!` for module or crate prose at the top of a file and `///` for items. Start with a one-sentence summary. Link public types where useful, add `# Errors` to fallible public APIs, add `# Safety` to unsafe APIs, and state complexity or lock scope when it affects callers.

All new public items must be documented before merge. When editing a file, add or correct notes on changed public items and nearby stale notes. The file checklist is: header note, every public type and function documented, non-obvious fields documented, and no stale or boilerplate comments.

## Markdown documentation

Markdown prose may exceed the code line-width target. Keep each paragraph and simple list item on one physical line unless it contains a nested block or fenced code. Use short paragraphs and paragraph breaks in preference to bullets; use bullets for genuine enumerations, checklists, and procedures.

Use Mermaid for relational diagrams, flows, and architecture. Do not use ASCII art for diagrams. Avoid small two-column tables for simple mappings; write source -> target or key: value inline instead. Write as internal engineering notes: one idea per paragraph, no filler, and selective bold emphasis only.

Use inline backticks sparingly in documentation. Reserve them for exact identifiers, wire names, commands, paths, values, and syntax; write ordinary prose, headings, and explanatory terms as plain text. This limit does not apply inside fenced code blocks, where exact code must remain intact.

Keep doc comments on public items. Docs contains architecture, contracts, operations, and product documentation; do not edit generated documentation directly. Update source comments or hand-written Markdown, then regenerate generated output.

Each public module has one hand-written Markdown document when it needs one. Simple documents may be free-form and minimal. The suggested Contents, Overview, Architecture, API, Design, and Testing structure is a reference, not a template to impose blindly: adapt headings, order, and depth to the repository and document's purpose.

### Documentation structure

Use `docs/documentation-index.md` as the repository-wide entry point. It links to the authoritative system design, interface catalog, module-reference catalog, and task-oriented guides; it does not repeat their content. Every documentation directory with multiple independently useful pages has one index or catalog that states the directory's scope and links each owned page with a one-sentence description.

Place documents by authority:

- `docs/transnet.md`: product semantics and system architecture.
- `docs/product/`: focused consumer-visible behavior that expands the system design without redefining contracts.
- `docs/interfaces/`: normative HTTP, UDS, and storage-adapter wire contracts, grouped by service boundary or route domain.
- `docs/reference/<module>/`: implementation-oriented module references, grouped by stable ownership area such as `runtime`, `transport`, `application`, `domain`, `ports`, `adapters`, or `operations`.
- `docs/guides/`: task-oriented procedures such as development, configuration, publication, and quality assurance.

An important module or stable subsystem owns its own directory. A small module has one substantive document that covers all of its concerns. Split inner pages only when the module has multiple independently searchable concerns and each page contains enough unique design, lifecycle, invariants, or verification guidance to stand on its own. Keep closely coupled concerns together; do not create one page per source file, type, route, or hypothetical future module.

Do not keep short placeholder pages, repeated boundary boilerplate, or pages that merely name responsibilities already obvious from an index or source module. Merge such material into the module's main document or source comments. Page count and file-to-module symmetry are not goals; clear ownership and useful search results are.

Each fact has one authoritative home. A reference page explains module ownership, dependencies, lifecycle, invariants, current-versus-target status, and verification; it links to interface documents for exact fields and examples, guides for procedures, the system design for product semantics, and rustdoc for item-level API detail. Summaries should be short and must not copy whole contracts, field catalogs, test lists, or standard boundary language across sibling pages.

Use concise, descriptive filenames that name the owned concept. Within one module directory, use `README.md` or a clearly named catalog only when navigation is needed; do not add empty indexes or generic files such as `overview.md` whose purpose is unclear in search results. When deleting or merging a page, update every inbound link and preserve unique content in the new authoritative owner.

Start a document with its purpose, owner or boundary, and intended reader when those are not obvious from its title. Explain prerequisites before procedures, put the smallest working example before advanced variants, and define project-specific terms on first use. End related documents with relative links. Keep a concise status note when a design is proposed, partial, deprecated, or not yet implemented.

Write changes so a new contributor can answer: where the behavior lives, how to run or verify it, which inputs and outputs matter, and which adjacent document or source file is authoritative. Keep examples runnable or explicitly mark them illustrative; never include credentials, user data, or production-only identifiers.

English documentation lives in docs. Each translation uses a localized docs_<language> tree and the locale suffix on every inner filename. The Chinese tree is docs_cn and uses the _cn suffix. Localized pages mirror the English path, headings, diagrams, API names, and internal links; link a document to its translation near the top when both exist. Update the corresponding English and localized page in the same change when an API, configuration field, or process boundary changes.

Use fenced json blocks for endpoint and interface requests and responses. Keep examples simple, valid, consistently indented, and limited to relevant fields. Use other fenced blocks only when syntax highlighting or a multiline non-JSON artifact materially improves clarity.

## REST API contracts

Keep hand-written HTTP and related WebSocket wire contracts below docs/interfaces and update the matching contract in the same change as the client or server handler.

Each contract states the owning boundary, authentication requirements, shared wire rules, validation limits, success and error envelopes, timestamp and identifier formats, and route-specific behavior. Use one plain-text `## METHOD /path` heading per route; do not put code formatting in route headings. Each route states authentication, request body or query, a minimal success example, and error codes.

Document exceptions to the standard envelope, including probes, legacy routes, and streaming responses. Use exact server wire field names and error strings in examples. A client author must be able to determine who may call a route, what to send, what success returns, and which codes mean validation, authorization, not-found, and server failure without opening the server source.

Prefer one minimal request and response example per route over tables that restate fields. Cross-link related domains at the bottom using relative links. Keep distinct models in their own documents and link between them rather than merging terminology.

## Code style and editor setup

Use two-space indentation, no tabs, the repository rustfmt configuration, and a 100-column target. Keep functions focused and files separated by responsibility. Prefer explicit types when they materially improve readability.

Repository configuration is the source of truth. Commit formatter, linter, and indentation settings at the root; CI enforces them. VS Code and Cursor use the checked-in .vscode settings and extensions. Other editors must use the same repository configuration rather than a second style guide. AI coding tools follow this file, repository configuration, and applicable module documentation.

## Rust

Follow the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html), then repository configuration. Start every Rust file with a `//!` module comment and document every public item with concise `///` contract text.

Use contextual `anyhow` errors at process boundaries and `thiserror` for typed library errors. Prefer Result over panics; use context when propagating failures. Avoid `unwrap` and `expect` outside tests.

Use Tokio. Never block executor threads; use `tokio::task::spawn_blocking` for blocking work. Use structured tracing and tracing-subscriber. Prefer structured fields and never log source text, request content, credentials, capabilities, provider bodies, or other secrets.

Keep unit tests beside their code in `#[cfg(test)]` modules and integration tests in tests. Add tests for non-trivial behavior and logic that is easy to get wrong; use short behavior-based test names.

## Repository and documentation

Keep Transnet as one Cargo package: Rust in src, integration tests in tests, configuration in config, and hand-written documentation in docs. Use brief, descriptive, preferably unique filenames; avoid repeated generic names and duplicate documents.

Keep system design in docs/transnet.md, task instructions in docs/guides, wire and storage contracts in docs/interfaces, module references in docs/reference, and Rust API detail in source comments and rustdoc. Give each contract and module concern one authoritative owner; other documents summarize and link instead of copying it.

Document implemented behavior and update documentation with every public API, configuration, or process-boundary change. Exclude generated files, build output, logs, runtime state, credentials, and editor state from version control; commit Cargo.lock.

## Verification

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## Git

Use concise Conventional Commit subjects with `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, or `chore:`. Name branches with an intent prefix and kebab-case description. Inspect status and diff, stage explicit paths, and never commit unrelated changes.
