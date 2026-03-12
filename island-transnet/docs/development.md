# Development Guide

This guide covers the revised development workflow for Transnet. The project now prioritizes a small translation MVP first, then layers in persistence, auth, and IPC.

## MVP First

The first working target is intentionally small:

1. `GET /health`
2. `POST /translate`
3. OpenAI-compatible llama-server on `localhost:13595`

Qdrant, advanced NLP, and full auth are not required for initial development.

## Prerequisites

### Required Software

1. **Rust**: 1.75 or later
   ```bash
   rustc --version
   ```

2. **SQLite**: bundled with `rusqlite`

3. **llama-server**: running locally with an OpenAI-compatible endpoint

4. **Git**
   ```bash
   git --version
   ```

### Optional Tools

- `cargo-watch`
- `ripgrep`
- editor Rust tooling

## Local Model Setup

Transnet should be developed against a local OpenAI-compatible backend exposed by llama-server:

```text
http://localhost:13595/v1
```

The client layer should treat this as an OpenAI-compatible provider, not as a hardcoded cloud-only integration.

## Environment Setup

### 1. Enter The Project

```bash
cd /home/ashex/projects/Island/island-transnet
```

### 2. Build The Workspace

```bash
cargo build
cargo build --release
```

### 3. Set Up Environment Variables

Create `.env` in `island-transnet/`:

```bash
OPENAI_API_KEY=local-llama-server
OPENAI_BASE_URL=http://localhost:13595/v1
OPENAI_MODEL=ACTION
DATABASE_PATH=data/transnet.db
JWT_SECRET=change-me-later
RUST_LOG=info
```

Notes:

- `OPENAI_API_KEY` can be a placeholder if the local server does not enforce a real key
- `OPENAI_BASE_URL` is the important value for the MVP
- keep `.env` out of version control

### 4. Config Shape

Keep the configuration small for the MVP.

**`config/transnet.toml`**:

```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4

[logging]
level = "info"
format = "json"
```

**`config/transnet_llm.toml`**:

```toml
[openai]
api_key = "${OPENAI_API_KEY}"
base_url = "${OPENAI_BASE_URL}"
model = "${OPENAI_MODEL}"
timeout_seconds = 30
max_retries = 3
```

**`config/transnet_db.toml`**:

```toml
[sqlite]
path = "data/transnet.db"
pool_size = 10
enable_wal = true
```

For the MVP, Qdrant config should be considered optional and deferred.

## Running

### Development Mode

```bash
cargo run --bin transnet-server
```

### With Logging

```bash
RUST_LOG=debug cargo run --bin transnet-server
```

### With Explicit Config

```bash
cargo run --bin transnet-server -- --config config/transnet.toml
```

## Testing Strategy

The first tests should focus on the smallest useful path.

### MVP Tests

- config parsing
- health endpoint
- translate endpoint with mocked provider behavior where possible
- response parsing for model output

### Later Tests

- SQLite history and cache
- auth flows
- FlatBuffers IPC roundtrip

## Development Priorities

### First

- shared request and response types
- translation core
- provider client with configurable base URL
- Axum health and translate routes

### Then

- launcher scripts
- `.env` workflow
- SQLite persistence
- IPC transport

### Later

- auth
- favorites
- advanced language analysis
- optional semantic indexing

## Operational Advice

- prefer strict JSON output from the model when possible
- log provider latency and parse failures
- keep transport adapters thin
- do not block the MVP on Qdrant or complex NLP

## Building

### Build Commands

```bash
# Build all crates in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Build specific binary
cargo build --bin transnet-server

# Build specific crate
cargo build -p transnet-api

# Clean build artifacts
cargo clean
```

### Build Features

```bash
# Build with default features
cargo build

# Build without some features
cargo build --no-default-features

# Build with custom features
cargo build --features "test-mode"
```

## Running

### Development Mode

```bash
# Run the server
cargo run --bin transnet-server

# Run with logging
RUST_LOG=debug cargo run --bin transnet-server

# Run with custom config
cargo run --bin transnet-server -- --config config/transnet.toml
```

### Production Mode

```bash
# Build optimized binary
cargo build --release

# Run optimized binary
./target/release/transnet-server --config config/transnet.toml
```

### Auto-reload Development

Install cargo-watch for auto-reload during development:

```bash
cargo install cargo-watch

# Watch for changes and rebuild
cargo watch -x run --bin transnet-server

# Run tests on changes
cargo watch -x test
```

## Testing

### Run All Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode (faster)
cargo test --release
```

### Run Specific Tests

```bash
# Run tests for specific crate
cargo test -p transnet-db

# Run specific test
cargo test test_translation

# Run tests matching pattern
cargo test translation

# Run tests in specific file
cargo test --lib transnet_db::tests
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test api_tests
```

### Code Coverage

Install cargo-tarpaulin:

```bash
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# Open coverage report
open tarpaulin-report.html
```

### Test Organization

```
crates/transnet-db/
├── src/
│   ├── lib.rs
│   └── models.rs
└── tests/                           # Integration tests
    └── db_integration_test.rs

src/
├── main.rs
└── tests/                           # Binary integration tests
    └── server_test.rs
```

## Debugging

### Using Rust Debugger

```bash
# Install rust-analyzer for VS Code
code --install-extension matklad.rust-analyzer

# Or use rust-gdb
rust-gdb target/debug/transnet-server
```

### Using Log Levels

```bash
# Debug logging
RUST_LOG=debug cargo run

# Trace logging (very verbose)
RUST_LOG=trace cargo run

# Module-specific logging
RUST_LOG=transnet_db=debug,transnet_llm=info cargo run
```

### Using VS Code

Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug transnet-server",
      "cargo": {
        "args": [
          "build",
          "--bin=transnet-server"
        ],
        "filter": {
          "name": "transnet-server",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}/island-transnet"
    }
  ]
}
```

## Code Style

### Rustfmt

```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt --check

# Format specific file
cargo fmt -- crates/transnet-db/src/lib.rs
```

### Clippy (Linter)

```bash
# Run clippy on all code
cargo clippy

# Run clippy with all features
cargo clippy --all-features

# Fix clippy warnings automatically
cargo clippy --fix
```

### Code Style Guidelines

1. **Naming Conventions**
   - Functions: `snake_case`
   - Structs: `PascalCase`
   - Constants: `SCREAMING_SNAKE_CASE`
   - Modules: `snake_case`

2. **Order of Imports**
   ```rust
   // Standard library
   use std::sync::Arc;
   
   // External crates
   use tokio::net::TcpListener;
   
   // Internal crates
   use transnet_types::User;
   
   // Current module
   use super::db::DbManager;
   ```

3. **Error Handling**
   - Use `anyhow::Result<T>` for application code
   - Use `thiserror` for library error types
   - Use `?` operator for error propagation

4. **Documentation**
   - Document all public APIs
   - Use `///` for documentation comments
   - Include examples in documentation
   - Document errors in function docs

## Contributing

### Workflow

1. **Create a Branch**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make Changes**
   - Follow code style guidelines
   - Add tests for new features
   - Update documentation

3. **Test Changes**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat: add new translation feature"
   ```

5. **Push and Create PR**
   ```bash
   git push origin feature/my-feature
   ```

### Commit Message Format

Follow Conventional Commits:

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance tasks

Examples:
```
feat(api): add translation endpoint

Add new POST /translate endpoint that supports
text, sentence, phrase, and word translation.

Closes #123
```

```
fix(db): resolve connection pool issue

Fixed issue where connections were not being
properly returned to the pool after use.
```

### Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing performed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review performed
- [ ] Documentation updated
- [ ] No new warnings generated
```

## Useful Commands

### Cargo Aliases

Add to `~/.cargo/config`:

```toml
[alias]
t = "test"
b = "build"
r = "run"
br = "build --release"
rr = "run --release"
c = "check"
clippy = "clippy -- -D warnings"
fmt = "fmt -- --check"
doc = "doc --no-deps --open"
```

### Common Tasks

```bash
# Check if code compiles
cargo check

# Check specific crate
cargo check -p transnet-api

# Generate documentation
cargo doc --no-deps

# Open documentation in browser
cargo doc --no-deps --open

# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Audit dependencies for security issues
cargo audit
```

## Troubleshooting

### Common Issues

**Issue**: "Failed to connect to Qdrant"
```bash
# Check if Qdrant is running
curl http://localhost:6333/healthz

# Start Qdrant
docker run -p 6333:6333 qdrant/qdrant:latest
```

**Issue**: "OPENAI_API_KEY not found"
```bash
# Check .env file exists
cat .env

# Export environment variable
export OPENAI_API_KEY=sk-...
```

**Issue**: "Database locked"
```bash
# Check for existing processes
lsof data/transnet.db

# Enable WAL mode in config
[sqlite]
enable_wal = true
```

**Issue**: "Compilation errors after dependency update"
```bash
# Clean and rebuild
cargo clean
cargo build

# Update lockfile
rm Cargo.lock
cargo update
```

## Performance Tips

### Build Time

```bash
# Use cargo-chef for faster Docker builds
cargo install cargo-chef

# Use sccache for caching compilation artifacts
cargo install sccache

# Enable sccache
export RUSTC_WRAPPER=sccache
```

### Runtime Performance

```bash
# Use release mode for benchmarking
cargo build --release

# Use flamegraph for profiling
cargo install flamegraph
cargo flamegraph --bin transnet-server

# Use perf for Linux
perf record -g ./target/release/transnet-server
perf report
```

## Documentation

### Generate Documentation

```bash
# Generate all documentation
cargo doc --all-features

# Generate and open documentation
cargo doc --open

# Document private items
cargo doc --document-private-items
```

### Write Documentation

```rust
/// Translates text from source language to target language.
///
/// This function provides translation with optional word relationship analysis.
///
/// # Arguments
///
/// * `text` - The text to translate
/// * `source_lang` - Source language (ISO 639-1 code)
/// * `target_lang` - Target language (ISO 639-1 code)
/// * `options` - Translation options
///
/// # Returns
///
/// Returns a `TranslationResponse` containing:
/// - The translated text
/// - Related words (if `include_related` is true)
/// - Parts of speech (if input is a sentence)
///
/// # Errors
///
/// This function will return an error if:
/// - The text is empty
/// - LLM API request fails
/// - Database operation fails
///
/// # Example
///
/// ```rust,no_run
/// use transnet_llm::TranslationService;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let service = TranslationService::new(config).await?;
/// let response = service.translate("hello", "en", "es", Default::default()).await?;
/// println!("Translation: {}", response.translation);
/// # Ok(())
/// # }
/// ```
pub async fn translate(
    &self,
    text: &str,
    source_lang: &str,
    target_lang: &str,
    options: TranslationOptions,
) -> Result<TranslationResponse> {
    // Implementation
}
```

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Tokio Documentation](https://docs.rs/tokio/)
- [Axum Documentation](https://docs.rs/axum/)
- [OpenAI API Documentation](https://platform.openai.com/docs)
- [Qdrant Documentation](https://qdrant.tech/documentation/)

## Getting Help

- Check existing issues on GitHub
- Ask questions in team chat
- Review documentation in `docs/` folder
- Look at test files for usage examples
