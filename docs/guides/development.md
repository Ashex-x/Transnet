# Development and operations

## Prerequisites

Install a current stable Rust toolchain with Cargo, rustfmt, and Clippy. Provide one or both OpenAI-compatible model endpoints configured in `island-transnet/config/transnet_llm.toml`.

## Verify the workspace

Run the complete local gate from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

## Run locally

Start the translation core first:

```bash
cargo run -p transnet
```

In another terminal, start the optional gateway:

```bash
cargo run -p transnet-server
```

Check the core directly:

```bash
curl http://127.0.0.1:35792/health
curl --request POST http://127.0.0.1:35792/translate \
  --header 'content-type: application/json' \
  --data '{"text":"hello","source_lang":"en","target_lang":"zh","mode":"basic","input_type":"word"}'
```

Check translation through the gateway by sending the same body to `http://127.0.0.1:8080/translate`.

## Deployment notes

Run each binary under a process supervisor and send logs to standard output. Both binaries handle Ctrl-C and Unix termination for graceful shutdown.

The current services have no authentication and use permissive CORS. Bind them to a trusted interface or put them behind a reverse proxy that provides TLS, authentication, request limits, and an explicit origin policy. Do not expose placeholder account routes as a real identity service.

The gateway is API-only. Deploy any browser interface as a separate application; `/`, `/index.html`, `/assets/*`, and `/resource/*` intentionally return `404 Not Found`.

Related: [configuration](configuration.md), [service reference](../reference/README.md), and [architecture](../architecture.md).
