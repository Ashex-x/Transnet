# Project Structure

This document describes the folder organization and crate responsibilities in the Transnet project.

## Root Directory Structure

```
island-transnet/
├── Cargo.toml                      # Workspace definition
├── README.md                       # Main project documentation
├── docs/                           # Documentation folder
│   ├── README.md                   # Documentation index
│   ├── architecture.md             # System architecture
│   ├── database.md                 # Database schemas
│   ├── api.md                      # REST API documentation
│   ├── ipc.md                      # IPC communication
│   ├── project_structure.md        # This file
│   ├── development.md              # Development guide
│   └── integration.md              # Island OS integration
├── crates/                         # Library crates
│   ├── transnet-types/             # Shared types and errors
│   ├── transnet-db/                # Database layer
│   ├── transnet-llm/               # LLM integration
│   ├── transnet-nlp/               # NLP processing
│   ├── transnet-auth/              # Authentication
│   └── transnet-api/               # REST API server
└── bins/                           # Binary crates
    └── transnet-server/            # Main server application
```

## Crate Responsibilities

### 1. transnet-types

**Purpose**: Defines shared types, errors, and data structures used across all crates.

**Structure**:
```
crates/transnet-types/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── user.rs                     # User-related types
    ├── translation.rs              # Translation types
    ├── word.rs                     # Word and meaning types
    ├── error.rs                    # Error types
    └── config.rs                   # Configuration types
```

**Key Types**:
- `User`: User account information
- `Translation`: Translation record
- `WordMeaning`: Word meaning and related words
- `TranslationRequest`: API request
- `TranslationResponse`: API response
- `TransnetError`: Enum of all error types
- `Config`: Application configuration

**Dependencies**:
- `serde`: Serialization
- `uuid`: Unique IDs
- `chrono`: Date/time handling
- `thiserror`: Error derivation

### 2. transnet-db

**Purpose**: Handles all database operations (SQLite + Qdrant).

**Structure**:
```
crates/transnet-db/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── sqlite.rs                   # SQLite database manager
    ├── qdrant.rs                   # Qdrant vector database
    ├── models.rs                   # Database models
    ├── migrations.rs               # Database migrations
    └── pool.rs                     # Connection pool management
```

**Key Components**:
- `DbManager`: Main database manager
- `SqliteClient`: SQLite operations
- `QdrantClient`: Qdrant operations
- `Pool`: Connection pool
- `MigrationRunner`: Run database migrations

**Dependencies**:
- `rusqlite`: SQLite client
- `qdrant-client`: Qdrant client
- `transnet-types`: Shared types
- `tokio`: Async runtime
- `tracing`: Logging

### 3. transnet-llm

**Purpose**: Manages LLM interactions with OpenAI API.

**Structure**:
```
crates/transnet-llm/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── client.rs                   # OpenAI client wrapper
    ├── prompts.rs                  # Prompt templates
    ├── translation.rs              # Translation logic
    ├── word_analysis.rs           # Word relationship analysis
    └── parser.rs                   # Response parsing
```

**Key Components**:
- `OpenAIClient`: OpenAI API client
- `TranslationService`: Translation orchestration
- `WordAnalyzer`: Word analysis logic
- `PromptBuilder`: Build prompts for different input types

**Dependencies**:
- `async-openai`: OpenAI API client
- `transnet-types`: Shared types
- `tokio`: Async runtime
- `tracing`: Logging

### 4. transnet-nlp

**Purpose**: Natural language processing for input classification and tokenization.

**Structure**:
```
crates/transnet-nlp/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── classifier.rs               # Input type classifier
    ├── tokenizer.rs               # Sentence/word tokenizer
    ├── pos_tagger.rs              # Part-of-speech tagging
    ├── language_detector.rs       # Language detection
    └── utils.rs                   # NLP utilities
```

**Key Components**:
- `InputClassifier`: Classify input as text/sentence/phrase/word
- `Tokenizer`: Split sentences and words
- `POSTagger`: Identify parts of speech
- `LanguageDetector`: Detect input language

**Dependencies**:
- `rusty-nlp`: NLP library
- `transnet-types`: Shared types
- `tracing`: Logging

### 5. transnet-auth

**Purpose**: Handles authentication and authorization.

**Structure**:
```
crates/transnet-auth/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── jwt.rs                      # JWT token management
    ├── password.rs                 # Password hashing/verification
    ├── service.rs                  # Authentication service
    └── middleware.rs               # Auth middleware for API
```

**Key Components**:
- `JwtManager`: Generate and validate JWT tokens
- `PasswordHasher`: Hash and verify passwords (bcrypt)
- `AuthService`: Login, register, session management
- `AuthMiddleware`: Axum middleware for request auth

**Dependencies**:
- `jsonwebtoken`: JWT library
- `bcrypt`: Password hashing
- `transnet-types`: Shared types
- `tokio`: Async runtime

### 6. transnet-api

**Purpose**: Implements the REST API and IPC server.

**Structure**:
```
crates/transnet-api/
├── Cargo.toml
└── src/
    ├── lib.rs                      # Public exports
    ├── server.rs                   # Axum server setup
    ├── routes/
    │   ├── mod.rs                  # Route module
    │   ├── auth.rs                 # Auth endpoints
    │   ├── translation.rs          # Translation endpoints
    │   ├── history.rs              # History endpoints
    │   ├── favorites.rs            # Favorites endpoints
    │   └── user.rs                 # User endpoints
    ├── handlers/
    │   ├── mod.rs                  # Handler module
    │   ├── auth.rs                 # Auth handlers
    │   ├── translation.rs          # Translation handlers
    │   ├── history.rs              # History handlers
    │   ├── favorites.rs            # Favorites handlers
    │   └── user.rs                 # User handlers
    ├── ipc/
    │   ├── mod.rs                  # IPC module
    │   ├── server.rs               # IPC server (FlatBuffers)
    │   └── handlers.rs             # IPC request handlers
    ├── middleware/
    │   ├── mod.rs                  # Middleware module
    │   ├── auth.rs                 # Auth middleware
    │   ├── cors.rs                 # CORS middleware
    │   └── logging.rs              # Logging middleware
    └── utils.rs                    # API utilities
```

**Key Components**:
- `ApiServer`: Main API server
- `IpcServer`: IPC server for Island OS
- `Routes`: Route definitions
- `Handlers`: Request handlers
- `Middleware`: Request/response middleware

**Dependencies**:
- `axum`: Web framework
- `tower`: Middleware stack
- `tokio`: Async runtime
- `flatbuffers`: IPC serialization
- `transnet-types`: Shared types
- `transnet-db`: Database operations
- `transnet-llm`: LLM integration
- `transnet-nlp`: NLP processing
- `transnet-auth`: Authentication

### 7. transnet-server

**Purpose**: Main application entry point.

**Structure**:
```
bins/transnet-server/
├── Cargo.toml
└── src/
    ├── main.rs                     # Application entry point
    ├── cli.rs                      # Command-line arguments
    ├── config.rs                   # Configuration loading
    └── bootstrap.rs                # Service bootstrap
```

**Key Components**:
- `main()`: Application entry point
- `CLI`: Command-line argument parsing
- `ConfigLoader`: Load configuration from files
- `Bootstrap`: Initialize all services

**Dependencies**:
- `clap`: CLI argument parsing
- `tokio`: Async runtime
- `tracing`: Logging
- `transnet-api`: API server
- `transnet-db`: Database
- `transnet-llm`: LLM
- `transnet-nlp`: NLP
- `transnet-auth`: Auth
- `transnet-types`: Types

## Module Structure

### lib.rs Pattern

Each crate follows this pattern:

```rust
// lib.rs - Public exports
pub mod user;
pub mod translation;
pub mod error;

pub use user::User;
pub use translation::{Translation, TranslationRequest, TranslationResponse};
pub use error::{TransnetError, Result};

// Re-export commonly used types
pub use transnet_types::*;
```

### Error Handling Pattern

```rust
// error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransnetError {
    #[error("Database error: {0}")]
    Database(#[from] DbError),
    
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, TransnetError>;
```

### Service Pattern

```rust
// service.rs
pub struct TranslationService {
    db: Arc<DbManager>,
    llm: Arc<OpenAIClient>,
    nlp: Arc<NlpProcessor>,
}

impl TranslationService {
    pub async fn translate(&self, req: TranslationRequest) -> Result<TranslationResponse> {
        // 1. Classify input
        let input_type = self.nlp.classify(&req.text).await?;
        
        // 2. Check cache
        if let Some(cached) = self.db.get_cached_word(&req.text, &req.target_lang).await? {
            return Ok(cached);
        }
        
        // 3. Call LLM
        let translation = self.llm.translate(&req).await?;
        
        // 4. Store in cache
        self.db.store_word(&translation).await?;
        
        Ok(translation)
    }
}
```

## File Naming Conventions

### Source Files
- `snake_case.rs` for Rust modules
- `mod.rs` for module exports

### Configuration Files
- `snake_case.toml` for configuration
- `Cargo.toml` for crate dependencies

### Documentation Files
- `kebab-case.md` for documentation

### Test Files
- `mod_test.rs` for inline tests
- `tests/` directory for integration tests

## Import Patterns

### Internal Imports
```rust
// Prefer absolute imports
use crate::error::{TransnetError, Result};
use transnet_types::{User, Translation};

// Or relative imports within crate
use super::db::DbManager;
```

### External Imports
```rust
// Group imports logically
use std::sync::Arc;
use tokio::net::UnixListener;

use anyhow::Result;
use serde::{Deserialize, Serialize};
```

## Testing Structure

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translation() {
        let service = setup_test_service().await;
        let result = service.translate(test_request()).await.unwrap();
        assert_eq!(result.translation, "expected");
    }
}
```

### Integration Tests
```
crates/transnet-api/
└── tests/
    ├── api_tests.rs
    └── ipc_tests.rs
```

## Documentation

### Inline Documentation
```rust
/// Translates text from source language to target language.
///
/// # Arguments
///
/// * `text` - The text to translate
/// * `source_lang` - Source language code (ISO 639-1)
/// * `target_lang` - Target language code (ISO 639-1)
///
/// # Returns
///
/// Returns a `TranslationResponse` containing the translation and related words.
///
/// # Errors
///
/// Returns `TransnetError` if:
/// - The text is empty
/// - LLM API fails
/// - Database operation fails
pub async fn translate(&self, text: &str, source_lang: &str, target_lang: &str) 
    -> Result<TranslationResponse>
{
    // Implementation
}
```

### Module Documentation
```rust
//! Translation service
//!
//! This module provides translation functionality using LLM integration
//! and semantic word analysis.
//!
//! # Example
//!
//! ```rust,no_run
//! use transnet_llm::TranslationService;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let service = TranslationService::new(config).await?;
//! let result = service.translate("hello", "en", "es").await?;
//! println!("{}", result.translation);
//! # Ok(())
//! # }
//! ```
```

## Configuration

### Config Files
```
island-transnet/
└── config/
    ├── transnet.toml               # Main configuration
    ├── transnet_db.toml            # Database configuration
    ├── transnet_llm.toml           # LLM configuration
    └── transnet_api.toml           # API configuration
```

### Environment Variables
```
# .env
OPENAI_API_KEY=local-llama-server
OPENAI_BASE_URL=http://localhost:13595/v1
OPENAI_MODEL=ACTION
DATABASE_PATH=data/transnet.db
JWT_SECRET=change-me-later
RUST_LOG=info
```

## Build Artifacts

### Target Directory
```
target/
├── debug/
│   ├── transnet-server            # Debug binary
│   └── deps/                       # Dependencies
└── release/
    └── transnet-server            # Release binary
```

## Shared Resources

### FlatBuffers Schema
```
flatbuffers/
└── transnet_fb.fbs                 # IPC schema definition
```

### Static Assets
```
static/
└── src/components/
    └── transnet.ts                 # Frontend component
```

## Summary

The Transnet project follows a modular architecture with a shared translation core and thin transport adapters:

1. **transnet-types**: Shared types and errors
2. **transnet-llm**: OpenAI-compatible provider integration
3. **transnet-api**: REST and later IPC transport adapters
4. **transnet-server**: Main application
5. **transnet-db**: Added when persistence is introduced
6. **transnet-auth**: Added when auth is introduced
7. **transnet-nlp**: Added when richer classification is needed

This structure enables:
- Clear dependencies between components
- Easy testing and development
- Reusable libraries
- Shared logic across REST and IPC
- Clear code organization
