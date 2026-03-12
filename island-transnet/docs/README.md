# Transnet Documentation

This directory contains comprehensive documentation for the Transnet project.

The documentation has been revised around a staged build strategy:

- MVP first
- one shared translation core
- REST and IPC as adapters
- local OpenAI-compatible llama-server support
- advanced systems deferred until the simpler service works

## Documentation Index

### Core Documentation

1. **[Architecture](./architecture.md)**
   - System architecture overview
   - Shared translation core
   - REST and IPC layering
   - Technology decisions

2. **[Database](./database.md)**
   - SQLite schema design
   - Future persistence design
   - Migration guide

3. **[API](./api.md)**
   - MVP endpoints
   - Request/response formats
   - API roadmap
   - Error handling

4. **[IPC Communication](./ipc.md)**
   - FlatBuffers schema definitions
   - Island OS ↔ Transnet communication
   - Message types and formats
   - Implementation examples

5. **[Project Structure](./project_structure.md)**
   - Folder organization
   - Crate responsibilities
   - Module structure
   - File naming conventions

### Development Documentation

6. **[Development Guide](./development.md)**
   - Environment setup
   - Building and running
   - Testing
   - Code style guidelines
   - Contributing

7. **[Integration with Island OS](./integration.md)**
   - Go gateway role
   - IPC integration path
   - Island client strategy
   - Integration order

## Quick Reference

### Key Commands

```bash
# Build the workspace
cargo build

# Run the server
cargo run --bin transnet-server

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin transnet-server
```

### Configuration

Configuration files should be placed in `island-transnet/config/`:

- `transnet.toml` - Main configuration
- `transnet_db.toml` - Database settings
- `transnet_llm.toml` - OpenAI-compatible provider settings

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENAI_API_KEY` | Provider API key or placeholder for local llama-server | Required |
| `OPENAI_BASE_URL` | OpenAI-compatible base URL | `http://localhost:13595/v1` |
| `OPENAI_MODEL` | Model alias exposed by llama-server | `ACTION` |
| `DATABASE_PATH` | SQLite database path | `data/transnet.db` |
| `JWT_SECRET` | JWT signing secret for later auth phase | Optional for MVP |
| `RUST_LOG` | Logging level | `info` |

## Getting Started

1. Read [Architecture](./architecture.md) to understand the system
2. Follow [Development Guide](./development.md) to set up your environment
3. Review [API](./api.md) to understand available endpoints
4. Check [Integration](./integration.md) for Island OS integration

## Support

For questions or issues, refer to the main README.md or contact the Island Team.
