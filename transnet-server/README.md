# Transnet Gateway Server

Rust HTTP gateway that exposes the Transnet JSON API and forwards translation requests to the backend service. The process does not build, bundle, or serve a WebUI; deploy browser clients independently.

## Run

```bash
cargo run -p transnet-server
```

The gateway reads a root `.env` file when present, then uses process environment variables. `GATEWAY_HOST` defaults to `0.0.0.0`, `GATEWAY_PORT` to `8080`, `BACKEND_HOST` to `127.0.0.1`, `BACKEND_PORT` to `35792`, `WORKERS` to `4`, and `RUST_LOG` to `info`.

See [the gateway reference](../docs/reference/gateway/README.md) for architecture and verification, and [the HTTP contract](../docs/reference/gateway/api/README.md) for routes and response envelopes.
