# Database Schema

This document describes the database schemas used by Transnet.

## Overview

Transnet uses two databases:

1. **SQLite** - Structured data storage (users, translations, cache)
2. **Qdrant** - Vector embeddings for semantic word relationships

## SQLite Schema

### Database File
- **Path**: `data/transnet.db`
- **Mode**: WAL (Write-Ahead Logging) for better concurrency
- **Connection Pool**: Configurable pool size (default: 10)

### Tables

#### 1. users
Stores user account information.

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,                    -- UUID
    username TEXT UNIQUE NOT NULL,          -- Unique username
    email TEXT UNIQUE NOT NULL,              -- Unique email
    password_hash TEXT NOT NULL,             -- bcrypt hashed password
    created_at TEXT NOT NULL,                -- ISO 8601 timestamp
    updated_at TEXT NOT NULL,                -- ISO 8601 timestamp
    is_active INTEGER DEFAULT 1              -- 0 = inactive, 1 = active
);

-- Indexes
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
```

**Indexes**:
- `idx_users_username`: Fast username lookup during login
- `idx_users_email`: Fast email lookup
- `idx_users_created_at`: For analytics and user statistics

#### 2. translations
Stores translation history for users.

```sql
CREATE TABLE translations (
    id TEXT PRIMARY KEY,                    -- UUID
    user_id TEXT NOT NULL,                  -- Foreign key to users
    source_text TEXT NOT NULL,              -- Original text
    source_lang TEXT NOT NULL,              -- Source language (ISO 639-1)
    target_lang TEXT NOT NULL,              -- Target language (ISO 639-1)
    translation TEXT NOT NULL,              -- Translated text
    input_type TEXT NOT NULL,               -- text, sentence, phrase, word
    created_at TEXT NOT NULL,                -- ISO 8601 timestamp
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_translations_user_id ON translations(user_id);
CREATE INDEX idx_translations_created_at ON translations(created_at DESC);
CREATE INDEX idx_translations_source_lang ON translations(source_lang);
CREATE INDEX idx_translations_target_lang ON translations(target_lang);
CREATE INDEX idx_translations_input_type ON translations(input_type);
```

**Indexes**:
- `idx_translations_user_id`: Fast history lookup for a user
- `idx_translations_created_at`: For chronological ordering
- `idx_translations_source_lang`: For language filtering
- `idx_translations_target_lang`: For language filtering
- `idx_translations_input_type`: For filtering by input type

**Foreign Key Constraints**:
- `ON DELETE CASCADE`: Deleting a user deletes all their translations

#### 3. favorites
Stores favorited translations/words for users.

```sql
CREATE TABLE favorites (
    id TEXT PRIMARY KEY,                    -- UUID
    user_id TEXT NOT NULL,                  -- Foreign key to users
    word_id TEXT NOT NULL,                  -- Foreign key to word_meanings
    note TEXT,                              -- Optional user note
    created_at TEXT NOT NULL,                -- ISO 8601 timestamp
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (word_id) REFERENCES word_meanings(id) ON DELETE CASCADE,
    UNIQUE(user_id, word_id)                -- Prevent duplicates
);

-- Indexes
CREATE INDEX idx_favorites_user_id ON favorites(user_id);
CREATE INDEX idx_favorites_created_at ON favorites(created_at DESC);
```

**Indexes**:
- `idx_favorites_user_id`: Fast favorites lookup for a user
- `idx_favorites_created_at`: For chronological ordering

**Unique Constraints**:
- `UNIQUE(user_id, word_id)`: A user can only favorite a word once

#### 4. word_meanings
Caches word meanings and related words to avoid repeated LLM calls.

```sql
CREATE TABLE word_meanings (
    id TEXT PRIMARY KEY,                    -- UUID
    word TEXT NOT NULL,                    -- The word (normalized to lowercase)
    pos TEXT,                              -- Part of speech: noun, verb, adj, etc.
    meaning TEXT NOT NULL,                 -- Word definition/meaning
    related_words TEXT,                    -- JSON array of related words/phrases
    word_type TEXT NOT NULL,               -- text, sentence, phrase, word
    source_lang TEXT NOT NULL,             -- Source language
    target_lang TEXT NOT NULL,             -- Target language
    embeddings_id TEXT,                    -- Reference to Qdrant embedding (if any)
    created_at TEXT NOT NULL,              -- ISO 8601 timestamp
    updated_at TEXT NOT NULL,               -- ISO 8601 timestamp
    access_count INTEGER DEFAULT 0,         -- Number of times accessed (for cache popularity)
    last_accessed_at TEXT                  -- ISO 8601 timestamp
);

-- Indexes
CREATE INDEX idx_word_meanings_word ON word_meanings(word);
CREATE INDEX idx_word_meanings_pos ON word_meanings(pos);
CREATE INDEX idx_word_meanings_word_type ON word_meanings(word_type);
CREATE INDEX idx_word_meanings_source_lang ON word_meanings(source_lang);
CREATE INDEX idx_word_meanings_target_lang ON word_meanings(target_lang);
CREATE INDEX idx_word_meanings_access_count ON word_meanings(access_count DESC);
CREATE INDEX idx_word_meanings_created_at ON word_meanings(created_at DESC);
```

**Indexes**:
- `idx_word_meanings_word`: Fast word lookup (primary cache lookup)
- `idx_word_meanings_pos`: For filtering by part of speech
- `idx_word_meanings_word_type`: For filtering by input type
- `idx_word_meanings_source_lang`: For language filtering
- `idx_word_meanings_target_lang`: For language filtering
- `idx_word_meanings_access_count`: For cache eviction (least frequently used)
- `idx_word_meanings_created_at`: For cache eviction (oldest first)

**Fields**:
- `related_words`: JSON array, example: `["good", "excellent", "superb"]`
- `access_count`: Used for LRU cache management
- `embeddings_id`: ID in Qdrant collection

#### 5. translation_parts
Stores individual word/phrase translations from sentence translation.

```sql
CREATE TABLE translation_parts (
    id TEXT PRIMARY KEY,                    -- UUID
    translation_id TEXT NOT NULL,           -- Foreign key to translations
    original_text TEXT NOT NULL,            -- Original word/phrase
    translated_text TEXT NOT NULL,          -- Translated word/phrase
    pos TEXT,                              -- Part of speech
    word_meaning_id TEXT,                  -- Foreign key to word_meanings (optional)
    position INTEGER NOT NULL,              -- Position in sentence
    FOREIGN KEY (translation_id) REFERENCES translations(id) ON DELETE CASCADE,
    FOREIGN KEY (word_meaning_id) REFERENCES word_meanings(id) ON DELETE SET NULL
);

-- Indexes
CREATE INDEX idx_translation_parts_translation_id ON translation_parts(translation_id);
CREATE INDEX idx_translation_parts_position ON translation_parts(position);
```

**Indexes**:
- `idx_translation_parts_translation_id`: Fast lookup for sentence parts
- `idx_translation_parts_position`: For maintaining order

**Use Case**:
When translating a sentence like "The quick brown fox", store each part:
1. "The" → "Le" (position 0)
2. "quick" → "rapide" (position 1)
3. "brown" → "marron" (position 2)
4. "fox" → "renard" (position 3)

#### 6. sessions
Stores active user sessions for JWT token management.

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,                    -- UUID
    user_id TEXT NOT NULL,                  -- Foreign key to users
    token_hash TEXT NOT NULL,               -- Hash of JWT token
    expires_at TEXT NOT NULL,               -- ISO 8601 timestamp
    created_at TEXT NOT NULL,               -- ISO 8601 timestamp
    last_used_at TEXT NOT NULL,             -- ISO 8601 timestamp
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
```

**Indexes**:
- `idx_sessions_user_id`: Fast lookup for user's active sessions
- `idx_sessions_token_hash`: Fast token validation
- `idx_sessions_expires_at`: For cleanup of expired sessions

**Use Case**:
- Token revocation: Check if token is in sessions table
- Session management: Track active sessions per user
- Security: Invalidate sessions on password change

## Qdrant Schema

### Collections

#### 1. word_embeddings
Stores vector embeddings for words and their semantic relationships.

**Collection Configuration**:
```rust
Collection {
    name: "word_embeddings",
    vectors: {
        size: 1536,  // OpenAI text-embedding-ada-002 dimension
        distance: Cosine,
        index_type: HNSW,
    },
    params: {
        hnsw_config: {
            m: 16,
            ef_construct: 100,
        }
    }
}
```

**Payload Structure**:
```json
{
    "word_id": "uuid",
    "word": "example",
    "pos": "noun",
    "source_lang": "en",
    "target_lang": "es",
    "related_words": ["ejemplo", "caso", "instancia"],
    "created_at": "2024-01-01T00:00:00Z",
    "word_type": "word"
}
```

**Fields**:
- `word_id`: UUID matching SQLite word_meanings.id
- `word`: The word (normalized)
- `pos`: Part of speech
- `source_lang`: Source language
- `target_lang`: Target language
- `related_words`: List of related words
- `created_at`: Timestamp
- `word_type`: text, sentence, phrase, word

**Indexing**:
- **Payload Index**: Create indexes on `word`, `pos`, `source_lang`, `target_lang` for fast filtering

**Use Cases**:
1. **Semantic Search**: Find similar words by vector similarity
2. **Word Relationships**: Hypernyms, hyponyms, synonyms
3. **Intensity Levels**: For adjectives (good → excellent → superb)
4. **Related Phrases**: For phrase translation

### Collection Operations

#### Insert Embedding
```rust
client.upsert_points_blocking(&word_embeddings_collection_name, vec![
    PointStruct {
        id: PointId::Uuid(word_id),
        vector: embedding_vec,  // 1536-dimensional vector
        payload: payload_map,
    }
])
```

#### Search Similar Words
```rust
let search_result = client.search_points(&SearchPoints {
    collection_name: word_embeddings_collection_name.to_string(),
    vector: query_embedding,
    limit: 10,
    with_payload: Some(true),
    filter: Some(Filter::must([
            Condition::matches("source_lang", "en"),
            Condition::matches("pos", "noun"),
        ])),
        params: None,
        score_threshold: Some(0.7),
    }).await?;
```

#### Delete Embedding
```rust
client.delete_points_blocking(
    &word_embeddings_collection_name,
    &[PointId::Uuid(word_id)]
).await?;
```

## Database Relationships

### ER Diagram

```
┌─────────────┐       ┌─────────────────┐       ┌─────────────────┐
│    users    │       │   translations  │       │ translation_   │
│             │1    N  │                 │1    N  │     parts      │
│ - id        │──────▶│ - id            │──────▶│                 │
│ - username  │       │ - user_id       │       │ - id           │
│ - email     │       │ - source_text   │       │ - translation_  │
│ - password  │       │ - translation   │       │   id           │
│ - created_at│       │ - created_at    │       │ - original_text │
└─────────────┘       └─────────────────┘       └─────────────────┘
       │                                              │
       │1                                            │N
       ▼                                              ▼
┌─────────────┐                               ┌─────────────────┐
│  favorites  │                               │  word_meanings  │
│             │N    1                         │                 │
│ - id        │──────▶│ - id            │       │ - word         │
│ - user_id   │       │ - word_id       │       │ - meaning      │
│ - word_id   │       │ - note          │       │ - related_words│
│ - note      │       │ - created_at    │       │ - embeddings_id│
└─────────────┘       └─────────────────┘       └─────────────────┘
                                                       │
                                                       │1
                                                       ▼
                                                ┌─────────────────┐
                                                │  Qdrant: word_  │
                                                │   embeddings    │
                                                │                 │
                                                │ - vector        │
                                                │ - payload       │
                                                └─────────────────┘
```

### Foreign Key Relationships

| Table | Foreign Key | References | On Delete |
|-------|-------------|------------|-----------|
| translations | user_id | users.id | CASCADE |
| translation_parts | translation_id | translations.id | CASCADE |
| translation_parts | word_meaning_id | word_meanings.id | SET NULL |
| favorites | user_id | users.id | CASCADE |
| favorites | word_id | word_meanings.id | CASCADE |
| sessions | user_id | users.id | CASCADE |

## Migration Strategy

### Versioning
- Use version numbers in table names or a `schema_migrations` table
- Track applied migrations
- Support rollback for development

### Initial Schema (v1.0.0)
- Create all tables
- Create all indexes
- Initialize Qdrant collections

### Future Migrations
- Add columns for new features
- Modify indexes for performance
- Update Qdrant collection parameters
- Data transformation scripts

## Performance Considerations

### SQLite Optimizations
1. **WAL Mode**: Enable for better concurrency
2. **Connection Pooling**: Reuse connections
3. **Prepared Statements**: Use parameterized queries
4. **Indexes**: Create indexes on frequently queried columns
5. **Vacuum**: Periodically vacuum to reclaim space
6. **Cache Size**: Adjust SQLite cache size

### Qdrant Optimizations
1. **HNSW Parameters**: Tune `m` and `ef_construct` for recall/speed trade-off
2. **Payload Indexes**: Create indexes on frequently filtered fields
3. **Batch Operations**: Batch insert/upsert for better performance
4. **Search Limit**: Limit search results to top N
5. **Score Threshold**: Use score threshold to filter irrelevant results

### Cache Strategy
1. **LRU Eviction**: Evict least recently used word_meanings
2. **Access Count**: Track access frequency for cache decisions
3. **TTL**: Expire old word_meanings (optional)
4. **Preload**: Preload common words on startup (optional)

## Backup and Recovery

### SQLite Backup
```bash
# Online backup using .backup
sqlite3 data/transnet.db ".backup data/transnet_backup_$(date +%Y%m%d).db"

# Or copy the database file (when no active transactions)
cp data/transnet.db data/transnet_backup_$(date +%Y%m%d).db
```

### Qdrant Backup
```bash
# Create snapshot
curl -X PUT 'http://localhost:6333/collections/word_embeddings/snapshots'

# Download snapshot
curl -X GET 'http://localhost:6333/collections/word_embeddings/snapshots' \
  --output snapshot.tar
```

### Recovery
- Restore SQLite from backup file
- Restore Qdrant collection from snapshot
- Rebuild indexes if needed
- Verify data integrity

## Data Cleanup

### Periodic Cleanup Tasks
1. **Expired Sessions**: Delete sessions past expiration
2. **Old Translations**: Archive old translations (optional)
3. **Unused Embeddings**: Delete Qdrant points for words not in SQLite
4. **Orphaned Records**: Clean up orphaned word_meanings

### SQL Cleanup Queries
```sql
-- Delete expired sessions
DELETE FROM sessions WHERE expires_at < datetime('now');

-- Delete old translations (older than 1 year)
DELETE FROM translations WHERE created_at < datetime('now', '-1 year');

-- Find orphaned word_meanings
SELECT id, word FROM word_meanings
WHERE id NOT IN (SELECT word_id FROM favorites)
  AND id NOT IN (SELECT word_meaning_id FROM translation_parts)
  AND access_count = 0
  AND created_at < datetime('now', '-6 months');
```
