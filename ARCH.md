# Transnet Architecture

A comprehensive technical reference for the Transnet translation system architecture.

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Project Structure](#project-structure)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Backend Crates](#backend-crates)
6. [Frontend Architecture](#frontend-architecture)
7. [Configuration](#configuration)
8. [API Reference](#api-reference)
9. [Database Schema](#database-schema)
10. [Build & Development](#build--development)

---

## System Overview

Transnet is a translation service with a layered architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Browser Client                            │
│                   (TypeScript + SCSS + WebGL)                    │
└───────────────────────────┬─────────────────────────────────────┘
                            │ HTTP/REST
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Gateway Server                              │
│                    (transnet-server/)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Routes    │  │   Static    │  │    Backend Client       │  │
│  │  (Axum)     │  │   Files     │  │    (reqwest)            │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │ HTTP (internal)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Backend Server                              │
│                  (island-transnet/bins/)                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  transnet-  │──│  transnet-  │──│     transnet-llm        │  │
│  │    api      │  │   types     │  │   (OpenAI-compatible)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
    ┌──────────┐      ┌──────────┐      ┌──────────┐
    │  SQLite  │      │  Qdrant  │      │   LLM    │
    │  (local) │      │ (vector) │      │ (local/  │
    │          │      │          │      │  cloud)  │
    └──────────┘      └──────────┘      └──────────┘
```

### Design Principles

- **Separation of Concerns**: Gateway handles routing and static files; Backend handles translation logic
- **Shared Types**: Common type definitions in `transnet-types` crate
- **Configuration-Driven**: All behavior configurable via TOML files
- **LLM-Agnostic**: OpenAI-compatible interface supports any provider

---

## Project Structure

```
Transnet/
├── transnet-server/              # Gateway Server (standalone crate)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # Entry point
│       ├── lib.rs               # Routes, types, state
│       └── backend_client.rs    # Backend HTTP client
│
├── island-transnet/              # Backend Core (workspace)
│   ├── Cargo.toml               # Workspace definition
│   ├── config/                  # Configuration files
│   │   ├── transnet.toml        # Server config
│   │   ├── transnet_api.toml    # API config (CORS, auth, ratelimit)
│   │   ├── transnet_db.toml     # Database config
│   │   └── transnet_llm.toml    # LLM config
│   ├── crates/
│   │   ├── transnet-types/      # Shared types and errors
│   │   ├── transnet-llm/        # LLM translation service
│   │   └── transnet-api/        # HTTP handlers (Axum)
│   └── bins/
│       └── transnet-server/     # Backend entry point
│
├── static/                       # Frontend
│   ├── src/
│   │   ├── app.ts               # App bootstrap
│   │   ├── router.ts            # SPA router
│   │   ├── main/                # Main pages (home, auth, about)
│   │   ├── transnet/            # Translation UI pages
│   │   ├── metaland/            # 3D world (WebGL/ECS)
│   │   ├── services/            # API services
│   │   ├── shared/              # Shared components
│   │   └── styles/              # SCSS stylesheets
│   ├── package.json
│   └── tsconfig.json
│
└── docs/                         # Documentation
    ├── api.md                    # API specification
    ├── architecture.md           # Translation workflows
    ├── database.md               # Database schema
    ├── types.md                  # Type definitions
    └── deployment.md             # Deployment guide
```

---

## Component Architecture

### Gateway Server (`transnet-server/`)

| Responsibility | Implementation |
|----------------|----------------|
| Serve static files | `tower-http::ServeDir` |
| SPA routing | Fallback to `index.html` |
| Proxy to backend | `BackendClient` (reqwest) |
| CORS | `tower-http::CorsLayer` |

**Routes:**
- `GET /health` → Proxy to backend
- `POST /translate` → Proxy to backend
- `POST /api/transnet/translate` → Proxy to backend
- `GET /assets/*` → Static files
- `GET /resource/*` → Static resources
- `*` → `index.html` (SPA fallback)

### Backend Server (`island-transnet/`)

| Crate | Responsibility |
|-------|----------------|
| `transnet-types` | Shared types, errors, config models |
| `transnet-llm` | LLM client, translation logic, retry handling |
| `transnet-api` | Axum routes, handlers, middleware |
| `transnet-server` (bin) | Config loading, tracing, server startup |

---

## Data Flow

### Translation Request Flow

```
┌──────────┐    POST /translate     ┌───────────────┐
│  Client  │ ──────────────────────▶│    Gateway    │
└──────────┘                        └───────┬───────┘
                                            │
                          POST /translate   │
                                            ▼
                                    ┌───────────────┐
                                    │  transnet-api │
                                    │   (handler)   │
                                    └───────┬───────┘
                                            │
                                            ▼
                                    ┌───────────────┐
                                    │  transnet-llm │
                                    │ (Translation  │
                                    │   Service)    │
                                    └───────┬───────┘
                                            │
                      POST /chat/completions│
                                            ▼
                                    ┌───────────────┐
                                    │  LLM Provider │
                                    │ (OpenAI-comp) │
                                    └───────────────┘
```

### Input Type Detection

The system automatically classifies input:

| Input Type | Detection Criteria |
|------------|-------------------|
| `word` | Single word, no spaces |
| `phrase` | 2-5 words, no sentence markers |
| `sentence` | Contains sentence-ending punctuation |
| `text` | Multiple sentences or long text |
| `auto` | Let system decide (default) |

---

## Backend Crates

### `transnet-types`

**Purpose:** Shared type definitions and error handling

**Key Types:**
```rust
// Input classification
pub enum InputType { Auto, Word, Phrase, Sentence, Text }

// Request/Response
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub mode: Option<String>,
    pub input_type: Option<InputType>,
}

pub struct TranslateResponse {
    pub text: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub input_type: InputType,
    pub provider: String,
    pub model: String,
}

// Standard response wrappers
pub struct SuccessResponse<T> { pub success: bool, pub data: T }
pub struct ErrorResponse { pub success: bool, pub error: ErrorInfo }

// Error types
pub enum TransnetError {
    Validation(String),
    Llm(String),
    Config(String),
    Internal(String),
}
```

**Functions:**
- `infer_input_type(text: &str) -> InputType` - Auto-detect input type

### `transnet-llm`

**Purpose:** Translation service using OpenAI-compatible APIs

**Key Types:**
```rust
pub struct TranslationService {
    client: reqwest::Client,
    config: LlmConfig,
}
```

**Key Functions:**
- `TranslationService::new(config)` - Create service with timeout/retry config
- `translate(request: TranslateRequest) -> Result<TranslateResponse>` - Main translation
- `validate_request()` - Validate non-empty text and languages
- `build_user_prompt()` - Construct LLM prompt
- `parse_model_response()` - Extract JSON from LLM response

**Behavior:**
- Supports configurable `base_url`, `api_key`, `model`
- Handles markdown code fences in LLM responses
- Implements retry logic for transient failures

### `transnet-api`

**Purpose:** HTTP API handlers (Axum)

**Key Types:**
```rust
pub struct AppState {
    pub translation_service: Arc<TranslationService>,
}
```

**Routes:**
```rust
Router::new()
    .route("/health", get(health))
    .route("/translate", post(translate))
    .layer(CorsLayer::permissive())
    .layer(TraceLayer::new_for_http())
```

**Error Mapping:**
| Error | HTTP Status |
|-------|-------------|
| `Validation` | 422 UNPROCESSABLE_ENTITY |
| `Llm` | 503 SERVICE_UNAVAILABLE |
| `Config` | 500 INTERNAL_SERVER_ERROR |
| `Internal` | 500 INTERNAL_SERVER_ERROR |

---

## Frontend Architecture

### Core Modules

| Module | Path | Purpose |
|--------|------|---------|
| App | `app.ts` | Bootstrap, route registration, lifecycle |
| Router | `router.ts` | SPA routing with `navigate()`, `handleRoute()` |

### Pages

| Route | File | Description |
|-------|------|-------------|
| `/home` | `main/home.ts` | Landing page with 3D sphere |
| `/login` | `main/auth.ts` | Login form |
| `/register` | `main/auth.ts` | Registration form |
| `/about` | `main/about.ts` | About page |
| `/setting` | `main/setting.ts` | Settings page |
| `/transnet` | `transnet/transnet.ts` | Main translation UI |
| `/transnet/history` | `transnet/history.ts` | Translation history |
| `/transnet/favorites` | `transnet/favorites.ts` | Favorited translations |
| `/transnet/profile` | `transnet/profile.ts` | User profile |
| `/metaland/house` | `metaland/house.ts` | 3D world zone |

### Services

| Service | File | Purpose |
|---------|------|---------|
| API | `services/tran_api/api.ts` | Full typed API client |
| Auth | `services/auth.ts` | JWT management, login/logout |
| Language | `services/language.ts` | i18n (en/zh), localStorage |

### Shared Components

| Component | File | Purpose |
|-----------|------|---------|
| PageShell | `shared/page-shell.ts` | Page wrapper with auth guard, header, footer |
| Header | `shared/header.ts` | Navigation header |
| Footer | `shared/footer.ts` | Footer component |
| Toast | `shared/toast.ts` | Toast notifications |
| ParticleBackground | `shared/particle-background.ts` | Canvas particle animation |

### Metaland (3D)

| Module | File | Purpose |
|--------|------|---------|
| ECS | `metaland/ecs.ts` | Entity-Component-System |
| Renderer | `metaland/renderer.ts` | WebGL renderer |
| Camera | `metaland/camera.ts` | 3D camera |
| Types | `metaland/types.ts` | 3D type definitions |

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
format = "json"  # or "compact"
file = "logs/transnet.log"
```

### API Config (`transnet_api.toml`)

```toml
[auth]
token_expiry_hours = 24
refresh_token_expiry_days = 7
password_min_length = 8

[ratelimit]
requests_per_minute = 100

[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PATCH", "DELETE"]
```

### Database Config (`transnet_db.toml`)

```toml
[sqlite]
path = "data/transnet.db"
pool_size = 10
enable_wal = true

[qdrant]
url = "http://localhost:6333"
collection_name = "word_embeddings"
vector_size = 1536
distance = "Cosine"
```

### LLM Config (`transnet_llm.toml`)

```toml
[openai]
api_key = "${OPENAI_API_KEY}"
base_url = "${OPENAI_BASE_URL}"
model = "${OPENAI_MODEL}"
timeout_seconds = 30
max_retries = 3
```

**Environment Variables:**
- `OPENAI_API_KEY` - API key for LLM provider
- `OPENAI_BASE_URL` - Base URL (default: local llama-server)
- `OPENAI_MODEL` - Model name

---

## API Reference

### Implemented Endpoints

| Method | Path | Description | Auth |
|--------|------|-------------|------|
| `GET` | `/health` | Health check | None |
| `POST` | `/translate` | Translate text | None |

### Planned Endpoints

| Method | Path | Description | Auth |
|--------|------|-------------|------|
| `GET` | `/api/about` | System info | None |
| `GET` | `/api/stats` | System stats | None |
| `POST` | `/api/account/register` | Register user | None |
| `POST` | `/api/account/login` | Login | None |
| `POST` | `/api/account/logout` | Logout | JWT |
| `POST` | `/api/account/refresh` | Refresh token | None |
| `POST` | `/api/account/change-password` | Change password | JWT |
| `POST` | `/api/transnet/translate` | Translate | Optional JWT |
| `GET` | `/api/history` | Get history | JWT |
| `GET` | `/api/history/:id` | Get translation | JWT |
| `DELETE` | `/api/history/:id` | Delete translation | JWT |
| `POST` | `/api/favorites` | Add favorite | JWT |
| `GET` | `/api/favorites` | Get favorites | JWT |
| `PUT` | `/api/favorites/:id` | Update note | JWT |
| `DELETE` | `/api/favorites/:id` | Remove favorite | JWT |
| `GET` | `/api/profile` | Get profile | JWT |
| `PUT` | `/api/profile` | Update profile | JWT |

### Response Format

**Success:**
```json
{
  "success": true,
  "data": { ... }
}
```

**Error:**
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Text cannot be empty"
  }
}
```

---

## Database Schema

### Users Table

| Field | Type | Notes |
|-------|------|-------|
| `user_id` | UUID | Primary key (UUID v7) |
| `username` | VARCHAR(50) | Unique, 3-50 chars |
| `email` | VARCHAR(255) | Unique |
| `password_hash` | VARCHAR(255) | bcrypt |
| `created_at` | TIMESTAMP | |
| `updated_at` | TIMESTAMP | |
| `active` | BOOLEAN | Account status |

### History Table

| Field | Type | Notes |
|-------|------|-------|
| `translation_id` | UUID | Primary key |
| `user_id` | UUID | FK → Users |
| `text` | TEXT | Source text |
| `translation` | TEXT | Translated text |
| `source_lang` | VARCHAR(10) | ISO 639-1 |
| `target_lang` | VARCHAR(10) | ISO 639-1 |
| `input_type` | VARCHAR(20) | word/phrase/sentence/text |
| `provider` | VARCHAR(50) | openai-compatible |
| `model` | VARCHAR(100) | Model name |
| `created_at` | TIMESTAMP | |
| `is_favorite` | BOOLEAN | Default false |
| `note` | VARCHAR(500) | Optional note |
| `updated_at` | TIMESTAMP | |

### Backend Stores

| Store | Type | Purpose |
|-------|------|---------|
| SQLite | SQL | Local data storage |
| Qdrant | Vector | Word embeddings (1536 dims, cosine) |

---

## Build & Development

### Backend Commands

```bash
# Build all crates
cd island-transnet && cargo build

# Run tests
cd island-transnet && cargo test

# Run server
cd island-transnet && cargo run --bin transnet-server

# Test specific crate
cd island-transnet && cargo test -p transnet-types
```

### Frontend Commands

```bash
# Build (TypeScript + SCSS)
cd static && npm run build

# Development watch
cd static && npm run watch

# Individual tasks
cd static && npm run build:ts      # Compile TypeScript
cd static && npm run build:scss    # Compile SCSS
cd static && npm run fix-imports   # Fix import paths
```

### Dependencies

**Rust (Workspace):**
| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime |
| `axum` | 0.7 | HTTP framework |
| `tower` / `tower-http` | 0.5 | Middleware |
| `serde` / `serde_json` | 1.0 | Serialization |
| `anyhow` / `thiserror` | 1.0 | Error handling |
| `tracing` | 0.1 | Logging |
| `reqwest` | 0.13 | HTTP client |
| `toml` | 1.0 | Config parsing |

**Frontend:**
| Package | Version | Purpose |
|---------|---------|---------|
| `typescript` | ^5.9 | TypeScript compiler |
| `sass` | ^1.97 | SCSS compilation |
| `flatbuffers` | ^25.9 | Binary serialization |

---

## Implementation Status

| Phase | Status | Description |
|-------|--------|-------------|
| Project Setup | ✅ Complete | Workspace, config, docs |
| Translation MVP | 🔄 In Progress | Core translation working |
| Local Dev Workflow | ⏳ Pending | Scripts and environment |
| Persistence | ⏳ Pending | SQLite, caching |
| IPC API | ⏳ Pending | FlatBuffers, Unix socket |
| Auth & Users | ⏳ Pending | JWT, history, favorites |
| Input Enrichment | ⏳ Pending | NLP, classification |

---

## Related Documentation

- [docs/api.md](docs/api.md) - Full API specification
- [docs/architecture.md](docs/architecture.md) - Translation workflows and types
- [docs/database.md](docs/database.md) - Database schema details
- [docs/types.md](docs/types.md) - Translation input/output types
- [docs/deployment.md](docs/deployment.md) - Deployment guide
