# AGENTS.md - Transnet Development Guide

This document provides essential information for agents working on the Transnet codebase.

## Project Overview

Transnet is a translation service with the following architecture:

```
Client (TypeScript Frontend)
    ↓
Gateway/Rust Backend ← SQLite + LLM
```

### Key Components

| Component | Location | Technology |
|-----------|----------|------------|
| Backend Core | `island-transnet/` | Rust (Axum) |
| API Crate | `island-transnet/crates/transnet-api/` | Axum HTTP |
| Types Crate | `island-transnet/crates/transnet-types/` | Shared types |
| LLM Crate | `island-transnet/crates/transnet-llm/` | LLM client |
| Frontend | `static/` | TypeScript + SCSS |

---

## Essential Commands

### Rust Backend

```bash
# Build all crates
cd island-transnet && cargo build

# Run tests
cd island-transnet && cargo test

# Run the server
cd island-transnet && cargo run --bin transnet-server

# Run a specific crate's tests
cd island-transnet && cargo test -p transnet-types
cd island-transnet && cargo test -p transnet-api
```

### Frontend

```bash
# Build frontend (compiles TypeScript + SCSS)
cd static && npm run build

# Development mode (watch)
cd static && npm run watch

# Individual tasks
cd static && npm run build:ts      # Compile TypeScript
cd static && npm run build:scss    # Compile SCSS
cd static && npm run fix-imports   # Fix import paths
```

---

## Project Structure

```
Transnet/
├── island-transnet/           # Rust workspace
│   ├── Cargo.toml            # Workspace definition
│   ├── config/               # TOML config files
│   │   ├── transnet.toml    # Main server config
│   │   ├── transnet_api.toml
│   │   ├── transnet_db.toml
│   │   └── transnet_llm.toml
│   ├── crates/
│   │   ├── transnet-types/  # Shared types, errors, config
│   │   ├── transnet-api/     # Axum HTTP handlers
│   │   ├── transnet-llm/     # LLM client wrapper
│   │   └── transnet-auth/    # (future) Auth/JWT
│   └── bins/
│       └── transnet-server/  # Main entry point
├── static/                   # TypeScript frontend
│   ├── src/
│   │   ├── app.ts           # Main app
│   │   ├── services/       # API services
│   │   ├── metaland/       # 3D rendering (metaland)
│   │   ├── transnet/       # Translation UI
│   │   └── styles/         # SCSS stylesheets
│   ├── package.json
│   └── tsconfig.json
└── docs/                     # Documentation
    ├── api.md               # API specification
    ├── architecture.md      # System architecture
    ├── types.md             # Translation types
    ├── database.md           # Database schema
    └── deployment.md         # Deployment guide
```

---

## Code Patterns

### Rust - Error Handling

Use `thiserror` for error types with `TransnetError` enum:

```rust
#[derive(Debug, Error)]
pub enum TransnetError {
    #[error("{0}")]
    Validation(String),
    #[error("llm request failed: {0}")]
    Llm(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("internal error: {0}")]
    Internal(String),
}
```

### Rust - API Handlers

Return `Result<Json<SuccessResponse<T>>, TransnetError>` or use `impl IntoResponse`:

```rust
async fn translate(
    State(state): State<AppState>,
    Json(request): Json<TranslateRequest>,
) -> impl IntoResponse {
    match state.translation_service.translate(request).await {
        Ok(response) => (StatusCode::OK, Json(SuccessResponse::new(response))).into_response(),
        Err(error) => map_error(error),
    }
}
```

### Rust - State Management

Use `Arc` for shared state:

```rust
#[derive(Clone)]
pub struct AppState {
    translation_service: Arc<TranslationService>,
}
```

### TypeScript - Response Format

Standard response wrapper:

```typescript
interface SuccessResponse<T> {
  success: true;
  data: T;
}

interface ErrorResponse {
  success: false;
  error: {
    code: string;
    message: string;
  };
}
```

---

## Configuration

### Server Config (`transnet.toml`)

```toml
[server]
host = "127.0.0.1"
port = 35792
workers = 4

[logging]
level = "info"
format = "json"
file = "logs/transnet.log"

[ipc]
socket_path = "/tmp/transnet_ipc.sock"
enable = true
```

### LLM Config (`transnet_llm.toml`)

```toml
[openai]
api_key = "your-api-key"
base_url = "http://localhost:13595/v1"
model = "ACTION"
timeout_seconds = 60
max_retries = 3
```

---

## API Patterns

### Translation Endpoint

```
POST /translate
```

Request:
```json
{
  "text": "Hello world",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic"
}
```

### Health Endpoint

```
GET /health
```

### Authenticated Endpoints

Require `Authorization: Bearer <token>` header:
- `/api/history`
- `/api/favorites`
- `/api/profile`

### Public Endpoints (no auth):
- `/api/transnet/translate`
- `/api/about`
- `/api/health`

---

## Testing

### Rust Tests

Tests are co-located in source files with `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_name() {
        // Test code
    }
}
```

Run with: `cargo test`

### Frontend Tests

No test framework configured yet. Build verification via:
```bash
npm run build
```

---

## Key Dependencies

### Rust (from workspace Cargo.toml)

- `tokio` - Async runtime
- `axum` - HTTP framework
- `tower`, `tower-http` - Middleware
- `serde`, `serde_json` - Serialization
- `thiserror` - Error handling
- `tracing`, `tracing-subscriber` - Logging

### Frontend

- `typescript` - Type safety
- `sass` - SCSS compilation
- `flatbuffers` - IPC serialization

---

## Important Gotchas

1. **LLM Endpoint**: Default points to local `http://localhost:13595/v1` (llama-server). Ensure local LLM is running for translation to work.

2. **Response Format**: Always wrap responses in `SuccessResponse<T>` with `success: true` field.

3. **Error Mapping**: Map `TransnetError` variants to HTTP status codes:
   - `Validation` → 422
   - `Llm` → 503
   - `Config` → 500
   - `Internal` → 500

4. **Input Type Detection**: The backend automatically detects input type (word/phrase/sentence/text) based on heuristics. This affects which translation fields are returned.

5. **Frontend Build Order**: TypeScript compiles first, then imports are fixed, then SCSS compiles. The `fix-imports` script handles path adjustments.

6. **JWT Auth**: Tokens have 1-hour expiry for access tokens, 7 days for refresh tokens. Protected routes require `Authorization: Bearer <token>`.

7. **Anonymous Translation**: `/api/transnet/translate` works without auth but doesn't save to history. Add JWT to enable history tracking.

---

## Documentation

Refer to these files for detailed information:

- `docs/api.md` - Complete API specification
- `docs/architecture.md` - System design and workflows
- `docs/types.md` - Translation input/output types
- `docs/database.md` - Database schema
- `docs/setup.md` - Development setup
- `docs/deployment.md` - Production deployment
- `island-transnet/TODO.md` - Implementation roadmap
