<p align="center">
  <img src="assets/transnet-logo.png" alt="Transnet logo" width="168">
</p>

<h1 align="center">Transnet</h1>

<p align="center">Private, stateless translation and relationship knowledge.</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?logo=rust"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-2563eb.svg"></a>
  <a href="docs/transnet.md"><img alt="Status: active development" src="https://img.shields.io/badge/Status-active_development-0f766e.svg"></a>
</p>

<p align="center">
  <a href="docs/documentation-index.md">Documentation</a> ·
  <a href="README_cn.md">中文</a>
</p>

Transnet is a private, stateless translation and relationship-knowledge service. It translates connected text and, for a resolved lexical sense or domain concept, builds a concise relationship-centered translation-wiki page. The checked-in executable currently provides the loopback translation and structured-lookup subset; the [system design](docs/transnet.md) defines the target service.

```mermaid
flowchart LR
  client["WebUI / internet client"] -->|"HTTPS or WSS"| island["island-port"]
  island -->|"Transnet UDS: api/v1"| service["Transnet"]
  service -->|"island-port UDS: api/v1 structured/vector data"| island
  island --> databases["MySQL / Qdrant"]
  service -->|"at most 4,000 characters"| gemma4["Gemma 4 :18011"]
  service -->|"over 4,000 characters"| translate["TranslateGemma :18007"]
```

## Prerequisites

Install a current stable Rust toolchain with Cargo, rustfmt, and Clippy. Start OpenAI-compatible Gemma 4 and TranslateGemma servers at the endpoints in `config/transnet.toml`. **Gemma 4** is the general-purpose provider used for short-text translation and the current structured lookup; **TranslateGemma** is the translation-specialized provider selected for longer text. The Gemma 4 endpoint used by structured lookup must support OpenAI-compatible strict JSON Schema output.

This two-provider setup describes the current executable only. The target architecture uses one Gemma4-27B vision-language model with fast and bounded reasoning profiles plus one embedding model; see the [model-runtime reference](docs/reference/model-runtime.md).

## Configure

The process always reads `config/transnet.toml` relative to the Cargo manifest. `[server]` selects the target Unix socket with `socket_path` and `socket_mode`; omitting `socket_path` explicitly selects the transitional loopback `host` and `port`. `[http]` sets the body limit and transitional CORS policy, `[translation]` sets routing and legacy retry defaults, `[gemma4]` and `[translate_gemma]` identify those provider endpoints, and `[provider_resilience.*]` sets independent timeout, retry, concurrency, and circuit-breaker bounds. See the [configuration guide](docs/guides/configuration.md). Do not commit real provider credentials.

`RUST_LOG` overrides `server.log_level`. `server.log_format = "json"` writes newline-delimited JSON; any other value writes compact text. Debug builds log to `logs/debug/transnet.log`, release builds log to `logs/release/transnet.log`; each file is replaced on startup.

## Build and verify

```bash
cargo build
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

The release binary is `target/release/transnet`.

## Run

Configure the listener and model servers in `config/transnet.toml`, then run:

```bash
cargo run
```

For a release build:

```bash
cargo run --release
```

Target UDS calls:

```bash
curl --unix-socket /run/transnet/transnet.sock --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' --data '{}'
curl --unix-socket /run/transnet/transnet.sock --request POST http://localhost/api/v1/livez \
  --header 'content-type: application/json' --data '{}'
curl --unix-socket /run/transnet/transnet.sock --request POST http://localhost/api/v1/readyz \
  --header 'content-type: application/json' --data '{}'
curl --unix-socket /run/transnet/transnet.sock --request POST http://localhost/api/v1/translations \
  --header 'content-type: application/json' \
  --data '{"text":"Hello","source_language":"auto","target_language":"zh-CN","response_level":"standard"}'
```

The checked-in configuration uses the target UDS listener. MySQL canonical cards and releases plus Qdrant knowledge nodes and edges remain target capabilities until their status is advanced in the interface and guide documents. The process handles Ctrl-C and Unix termination signals for graceful shutdown.

For transitional loopback compatibility, omit `server.socket_path`, then verify with `curl http://127.0.0.1:16002/health`.

See the [design](docs/transnet.md), [island-port-to-Transnet service interface and UDS transport](docs/interfaces/transnet.md), [canonical-data endpoints](docs/interfaces/canonical-data.md), [retrieval-data endpoints](docs/interfaces/retrieval-data.md), [configuration reference](docs/guides/configuration.md), and [deployment guide](docs/guides/deployment.md).

## License

Transnet is licensed under the [MIT License](LICENSE).
