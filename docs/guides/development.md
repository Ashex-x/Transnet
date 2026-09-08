# Development and operations

## Prerequisites

Install a current stable Rust toolchain with Cargo, rustfmt, and Clippy. Start OpenAI-compatible Gemma 4 and TranslateGemma servers at the configured endpoints.

## Verify

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## Run locally

```bash
cargo run
```

The default listener is `127.0.0.1:35792`. See the [root README](../../README.md) for curl examples.

## Deploy

Run the binary under a process supervisor and collect standard output. The process handles Ctrl-C and Unix termination for graceful shutdown.

Transnet has no authentication, TLS, CORS policy, or request-size limit. Keep it on a trusted interface or place it behind a policy-enforcing edge before public exposure.

Related: [configuration](configuration.md), [design](../transnet.md), and [API contract](../reference/transnet-api.md).
