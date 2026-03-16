# Transnet API Documentation

## Overview

**Protocol**: HTTPS
**Content-Type**: `application/json`
**Authentication**: JWT Bearer Token (for protected endpoints only)
**ID Format**: UUID v7. Primary identifiers: `user_id` (users), `translation_id` (history/favorites).

## Routes
[ ] `GET api/about`
[ ] `GET api/stats`
[ ] `GET api/health`
[x] `POST api/account/register`
[x] `POST api/account/login`
[x] `POST api/account/logout`
[x] `POST api/account/refresh`
[x] `POST api/account/change-password`
[x] `POST api/transnet/translate`
[x] `GET api/transnet/history`
[x] `GET api/transnet/history/:translation_id`
[x] `DELETE api/transnet/history/:translation_id`
[x] `POST api/transnet/favorites`
[x] `GET api/transnet/favorites`
[x] `PUT api/transnet/favorites/:translation_id`
[x] `DELETE api/transnet/favorites/:translation_id`
[x] `GET api/profile`
[x] `PUT api/profile`

## Architecture

```
Client (TypeScript)
    ↓
Gateway (Rust) ← JWT Validation (for protected routes)
    ↓
Core Backend (Rust) ← SQLite + Qdrant
```

**Authentication Strategy**:
- `/translate`: No authentication required (anonymous access)
- `/history`, `/favorites`, `/profile`: JWT authentication required

---

## System Information

### GET `/api/about`
Get system version and information.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "name": "Transnet",
    "version": "1.0.0",
    "description": "AI-powered translation service with history and favorites",
    "features": [
      "translation",
      "history",
      "favorites",
      "user_profiles",
      "multi_language_support"
    ],
    "supported_languages": ["en", "es", "fr", "de", "zh", "ja", "cn"],
    "max_text_length": 5000
  }
}
```

### GET `/api/stats`
Get real-time system statistics and health status.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translations_today": 14253,
    "active_users": 387,
    "translations_this_hour": 1247,
    "llm_api_status": "healthy",
    "database_status": "connected",
    "requests_per_minute": 245,
    "database_size_mb": 142.3
  }
}
```

### GET `/api/health`

Combined liveness and readiness check.

Liveness: The HTTP server is up and responding.
Readiness: Core dependencies (e.g. Qdrant, LLM API) are reachable and healthy.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "status": "ready",
    "service": "transnet",
    "checks": {
      "qdrant": "connected",
      "llm_api": "reachable"
    }
  }
}
```

---

## Authentication & Authorization

### JWT Token Structure

**Header**: `Authorization: Bearer <token>`

**Token Payload**:
```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",  // user_id (UUID v7)
  "iat": 1234567890,                              // issued at (Unix timestamp)
  "exp": 1234571490,                              // expires at (Unix timestamp)
  "type": "access"                                // token type: "access" or "refresh"
}
```

### Token Lifecycle

- **Access Token**: Valid for 1 hour
- **Refresh Token**: Valid for 7 days
- **Token Rotation**: On password change, invalidate all tokens

---

## Account Management

### POST `/api/account/register`
Register a new account.

**Request Body**:
```json
{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "SecurePass123!"
}
```

**Constraints**:
- `username`: 3-50 characters, alphanumeric + underscore
- `email`: Valid email format
- `password`: Minimum 8 characters, at least one uppercase, one lowercase, one number

**Response (201 Created)**:
```json
{
  "success": true,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `409`: Username or email already exists

---

### POST `/api/account/login`
Login to account.

**Request Body**:
```json
{
  "email": "john@example.com",
  "password": "SecurePass123!"
}
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com"
    }
  }
}
```

**Error Responses**:
- `400`: Invalid credentials format
- `401`: Invalid email or password
- `423`: Account inactive

---

### POST `/api/account/logout`
Logout from account (invalidate tokens).

**Headers**:
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  }
}
```

**Behavior**:
- Deletes all sessions for user
- Invalidates both access and refresh tokens

---

### POST `/api/account/refresh`
Refresh access token using refresh token.

**Request Body**:
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
}
```

**Error Responses**:
- `401`: Invalid or expired refresh token
- `403`: Refresh token already used (one-time use)

---

### POST `/api/account/change-password`
Change user password.

**Headers**:
```
Authorization: Bearer <access_token>
```

**Request Body**:
```json
{
  "current_password": "OldPass123!",
  "new_password": "NewSecurePass456!"
}
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "message": "Password changed successfully"
  }
}
```

**Behavior**:
- Invalidates all existing sessions/tokens
- Creates new session with updated password

---

## Translation

### POST `/api/transnet/translate`
Translate text.

**Authentication**: Optional (JWT Bearer token)

When JWT is provided (authenticated user):
- Translation is saved to user's history
- `user_id` is included in response
- Requires `Authorization: Bearer <token>` header

When JWT is NOT provided (anonymous user):
- Translation is returned but NOT saved to history
- `user_id` is NOT included in response
- No authentication required

**Request Body**:
```json
{
  "text": "The quick brown fox",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic"
}
```

**Fields**:
- `text` (required): Text to translate
- `source_lang` (required): Source language code (ISO 639-1)
- `target_lang` (required): Target language code (ISO 639-1)
- `mode` (required): Translation mode, one of:
  - `basic`
  - `explain`
  - `full_analysis`
  Backend automatically detects `input_type` and validates that this `mode` is supported for the detected input type. See `types.md` for detection criteria and per-input-type behavior.

**Translation Field Types**: The `translation` field in the response contains type-specific data based on the detected `input_type` and requested `mode`. There are **9 possible types** defined in `types.md`:

| Input Type  | Mode            | Translation Type                 |
| ----------- | --------------- | -------------------------------- |
| `word`      | `basic`         | `TranslationWordBasic`           |
| `word`      | `explain`       | `TranslationWordExplain`         |
| `word`      | `full_analysis` | `TranslationWordFullAnalysis`    |
| `phrase`    | `basic`         | `TranslationPhraseBasic`         |
| `phrase`    | `explain`       | `TranslationPhraseExplain`       |
| `phrase`    | `full_analysis` | `TranslationPhraseFullAnalysis`  |
| `sentence`  | `basic`         | `TranslationSentenceBasic`       |
| `sentence`  | `explain`       | `TranslationSentenceExplain`     |
| `paragraph` | `basic`         | `TranslationParagraphEssayBasic` |
| `essay`     | `basic`         | `TranslationParagraphEssayBasic` |

For detailed JSON schemas and examples of each translation field type, see `types.md`.

**Authenticated Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "translation": {
      // Translation field type specific data
    }
  }
}
```

**Anonymous Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "translation": {
      // Translation field type specific data
    }
  }
}
```

**Behavior**:
- Translations are saved to history ONLY when JWT is provided
- Returns `translation_id` for history and favorites (e.g. POST `/favorites`); always returned even when anonymous

**Error Responses**:
- `400`: Invalid request data
- `422`: Semantically invalid request
- `500`: Internal server error
- `503`: Service unavailable (LLM backend down)

---

## Translation History

### GET `/api/transnet/history`
Get translation history for authenticated user.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Query Parameters**:
- `page` (optional, default: 1): Page number
- `limit` (optional, default: 20, max: 100): Items per page
- `source_lang` (optional): Filter by source language
- `target_lang` (optional): Filter by target language
- `input_type` (optional): Filter by input type

**Example Request**:
```
GET /api/history?page=1&limit=20&source_lang=en&target_lang=es
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translations": [
      {
        "translation_id": "550e8400-e29b-41d4-a716-446655440000",
        "text": "The quick brown fox",
        "source_lang": "en",
        "target_lang": "es",
        "input_type": "sentence",
        "provider": "openai-compatible",
        "model": "ACTION",
        "translation": {
          // Translation field type specific data
        }
      },
      {
        "translation_id": "660e8400-e29b-41d4-a716-446655440001",
        "text": "Hello world",
        "source_lang": "en",
        "target_lang": "es",
        "input_type": "sentence",
        "provider": "openai-compatible",
        "model": "ACTION",
        "translation": {
          // Translation field type specific data
        }
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total": 42,
      "total_pages": 3
    }
  }
}
```

**Error Responses**:
- `401`: Authentication required

---

### GET `/api/transnet/history/:translation_id`
Get a specific translation by ID.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "translation": {
      // Translation field type specific data
    },
    "word_meaning_id": "770e8400-e29b-41d4-a716-446655440001",
    "translation_parts": [
      {
        "original_text": "The",
        "translated_text": "El",
        "position": 0
      },
      {
        "original_text": "quick",
        "translated_text": "rápido",
        "position": 1
      }
    ]
  }
}
```

**Error Responses**:
- `401`: Authentication required
- `403`: Translation not found or access denied
- `404`: Translation not found

---

### DELETE `/api/transnet/history/:translation_id`
Delete a translation from history.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "message": "Translation deleted successfully"
  }
}
```

**Behavior**:
- Deletes translation and associated translation_parts
- Only owner can delete their own translations

**Error Responses**:
- `401`: Authentication required
- `403`: Translation not found or access denied
- `404`: Translation not found

---

## Favorites

Favorites are history records with `is_favorite = true` and an optional `note`. There is no separate favorite ID; use `translation_id` to identify a favorited item.

### POST `/api/transnet/favorites`
Mark a translation as favorite (sets `is_favorite = true` on the history record and optionally stores a note).

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Request Body**:
```json
{
  "translation_id": "550e8400-e29b-41d4-a716-446655440000",
  "note": "Useful for travel"
}
```

**Fields**:
- `translation_id` (required): Translation ID from the user's history
- `note` (optional, max: 500 characters): User note (default empty)

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "note": "Useful for travel",
    "updated_at": "2024-01-15T12:30:00Z"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `401`: Authentication required
- `404`: Translation not found
- `409`: Already favorited (`is_favorite` already true for this translation)

---

### GET `/api/transnet/favorites`
Get user's favorites (history records where `is_favorite = true`).

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Query Parameters**:
- `page` (optional, default: 1): Page number
- `limit` (optional, default: 20, max: 100): Items per page

**Example Request**:
```
GET /favorites?page=1&limit=20
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "favorites": [
      {
        "translation_id": "550e8400-e29b-41d4-a716-446655440000",
        "note": "Useful for travel",
        "updated_at": "2024-01-15T12:30:00Z",
        "translation": {
          "translation_id": "550e8400-e29b-41d4-a716-446655440000",
          "text": "The quick brown fox",
          "source_lang": "en",
          "target_lang": "es",
          "input_type": "sentence",
          "provider": "openai-compatible",
          "model": "ACTION",
          "translation": {
            // Translation field type specific data
          }
        },
        "word_meaning": {
          "id": "770e8400-e29b-41d4-a716-446655440001",
          "word": "fox",
          "pos": "noun",
          "meaning": "A small carnivorous mammal of the dog family",
          "related_words": ["dog", "canine", "pup", "kit"],
          "source_lang": "en",
          "target_lang": "es",
          "word_type": "word"
        }
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total": 15,
      "total_pages": 1
    }
  }
}
```

**Error Responses**:
- `401`: Authentication required

---

### PUT `/api/transnet/favorites/:translation_id`
Update the note for a favorited translation.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Request Body**:
```json
{
  "note": "Updated note for context"
}
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "note": "Updated note for context",
    "updated_at": "2024-01-15T12:30:00Z"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `401`: Authentication required
- `403`: Translation not in favorites or access denied
- `404`: Translation not found

---

### DELETE `/api/transnet/favorites/:translation_id`
Remove a translation from favorites (sets `is_favorite = false` on the history record).

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "message": "Favorite removed successfully"
  }
}
```

**Error Responses**:
- `401`: Authentication required
- `403`: Translation not in favorites or access denied
- `404`: Translation not found

---

## User Profile

### GET `/api/profile`
Get current user profile.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "updated_at": "2024-01-15T09:30:00Z",
    "stats": {
      "total_translations": 142,
      "total_favorites": 15,
      "languages_used": ["en", "es", "fr", "de"]
    }
  }
}
```

**Error Responses**:
- `401`: Authentication required

---

### PUT `/api/profile`
Update user profile.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Request Body**:
```json
{
  "username": "johndoe_new",
  "email": "john_new@example.com"
}
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe_new",
    "email": "john_new@example.com",
    "updated_at": "2024-01-15T13:00:00Z"
  }
}
```

**Constraints**:
- `username`: 3-50 characters, alphanumeric + underscore
- `email`: Valid email format

**Error Responses**:
- `400`: Validation error
- `401`: Authentication required
- `409`: Username or email already exists

---

## Standard Response Format

### Success Response

```json
{
  "success": true,
  "data": { /* ... */ }
}
```

### Error Response

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message"
  }
}
```

### HTTP Status Codes

- `200 OK`: Successful request
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request data
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Access denied to resource
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (duplicate)
- `422 Unprocessable Entity`: Semantically invalid request
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server or provider failure
- `503 Service Unavailable`: Downstream model is unavailable

---

## Gateway Behavior

The gateway (transnet-server) should:

1. **Validate JWT tokens** on protected routes (`/api/history`, `/api/favorites`, `/api/profile`)
2. **Forward user_id** to translate requests from JWT claim (for history tracking)
3. **Do not require authentication** for `/api/transnet/translate` (anonymous access)
4. **Add user_id** to translation response only when JWT is provided (authenticated user)
5. **Cache responses** for public routes (`/api/about`, `/api/health`, `/api/stats`)

### Authentication Logic for `/api/transnet/translate`

**If JWT is NOT provided (empty/null):**
- Return translation WITHOUT `user_id` field
- Translation is NOT saved to history
- Translation ID can still be used for favorites

**If JWT IS provided (authenticated user):**
- Return translation WITH `user_id` field (from JWT `sub` claim)
- Translation IS saved to user's history
- Returns full user context

---

## Database Integration

For the **web server (MySQL)** schema (users, history, favorites as part of history), see `docs/database.md`.

Below describes backend storage patterns. Identifiers align with the API: `user_id` (users), `translation_id` (history).

### Tables (conceptual)

- **users**: User accounts; primary key `user_id`
- **sessions**: Active JWT sessions
- **history** (translations): Translation history; primary key `translation_id`, foreign key `user_id`. Includes `is_favorite` (boolean) and `note` (optional) for favorites; no separate favorites table
- **word_meanings**: Word definitions with cached embeddings (backend)
- **translation_parts**: Individual word/phrase translations (backend)

### Write Pattern

- **Authentication**: Insert into `users` + `sessions`
- **Translation**: Insert into history with `translation_id`, `user_id` (when JWT provided, else nullable for anonymous). Optionally `translation_parts` + `word_meanings` (with Qdrant cache check)
- **Favorites**: Update history record: set `is_favorite = true` and optional `note` (POST). Update `note` (PUT). Set `is_favorite = false` (DELETE)

### Read Pattern

- **History**: Query history by `user_id` with pagination and filters
- **Favorites**: Query history where `user_id` matches JWT `sub` and `is_favorite = true`

### Cache Strategy

- **Word Meanings**: Check Qdrant first, then SQLite
- **Translations**: No caching (user-specific data)
- **Embeddings**: Lazy load on first translation

---

## Security Considerations

### Password Security
- Minimum 8 characters
- At least one uppercase, one lowercase, one number
- bcrypt hashing (cost factor: 12)
- No password reuse history

### Token Security
- JWT signed with HS256
- Token expiration enforced
- Token rotation on password change
- Refresh tokens are one-time use

### Rate Limiting
- 100 requests per minute per IP
- 1000 requests per hour per user
- Header: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

### CORS
- Frontend domain whitelisted
- `GET`, `POST`, `PUT`, `DELETE` methods allowed
- Authorization header allowed
