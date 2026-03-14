# Transnet API Documentation

## Overview

**Base URL**: `/api`
**Protocol**: HTTP/HTTPS
**Content-Type**: `application/json`
**Authentication**: JWT Bearer Token (for protected endpoints only)
**ID Format**: UUID v7

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

### GET `/about`
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

### GET `/stats`
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

### GET `/health`
Health check endpoint.

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "service": "transnet"
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
  "sub": "550e8400-e29b-41d4-a716-446655440000",  // uuid (UUID v7) - user_id
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

### POST `/account/register`
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
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `409`: Username or email already exists

---

### POST `/account/login`
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
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
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

### POST `/account/logout`
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

### POST `/account/refresh`
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

### POST `/account/change-password`
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

### POST `/translate`
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
  "mode": "basic",
  "input_type": "auto"
}
```

**Fields**:
- `text` (required): Text to translate
- `source_lang` (required): Source language code (ISO 639-1)
- `target_lang` (required): Target language code (ISO 639-1)
- `mode` (optional): `basic`, `explain`
- `input_type` (optional): `auto`, `word`, `phrase`, `sentence`, `text`

**Authenticated Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "translation": "El rápido zorro marrón",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "created_at": "2024-01-15T10:30:00Z",
    "user_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**Anonymous Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "translation": "El rápido zorro marrón",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

**Behavior**:
- Translations are saved to history ONLY when JWT is provided
- Returns translation ID for favorites reference
- Translation ID is always returned (can be used by favorites even without JWT)

**Error Responses**:
- `400`: Invalid request data
- `422`: Semantically invalid request
- `500`: Internal server error
- `503`: Service unavailable (LLM backend down)

---

## Translation History

### GET `/history`
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
- `sort_by` (optional, default: created_at): Sort field (`created_at`, `source_lang`)
- `sort_order` (optional, default: desc): `asc` or `desc`

**Example Request**:
```
GET /history?page=1&limit=20&source_lang=en&target_lang=es&sort_by=created_at&sort_order=desc
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translations": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "text": "The quick brown fox",
        "translation": "El rápido zorro marrón",
        "source_lang": "en",
        "target_lang": "es",
        "input_type": "sentence",
        "provider": "openai-compatible",
        "model": "ACTION",
        "created_at": "2024-01-15T10:30:00Z"
      },
      {
        "id": "660e8400-e29b-41d4-a716-446655440001",
        "text": "Hello world",
        "translation": "Hola mundo",
        "source_lang": "en",
        "target_lang": "es",
        "input_type": "text",
        "provider": "openai-compatible",
        "model": "ACTION",
        "created_at": "2024-01-15T09:15:00Z"
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

### GET `/history/:id`
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
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "translation": "El rápido zorro marrón",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "created_at": "2024-01-15T10:30:00Z",
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

### DELETE `/history/:id`
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

### POST `/favorites`
Add a translation to favorites.

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
- `translation_id` (required): Translation UUID from history
- `note` (optional, max: 500 characters): User note

**Response (201 Created)**:
```json
{
  "success": true,
  "data": {
    "id": "880e8400-e29b-41d4-a716-446655440002",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "note": "Useful for travel",
    "created_at": "2024-01-15T11:00:00Z"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `401`: Authentication required
- `404`: Translation not found
- `409`: Already favorited (unique constraint)

---

### GET `/favorites`
Get user's favorites.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Query Parameters**:
- `page` (optional, default: 1): Page number
- `limit` (optional, default: 20, max: 100): Items per page
- `sort_by` (optional, default: created_at): Sort field
- `sort_order` (optional, default: desc): `asc` or `desc`

**Example Request**:
```
GET /favorites?page=1&limit=20&sort_by=created_at&sort_order=desc
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "favorites": [
      {
        "id": "880e8400-e29b-41d4-a716-446655440002",
        "translation_id": "550e8400-e29b-41d4-a716-446655440000",
        "word_id": "770e8400-e29b-41d4-a716-446655440001",
        "note": "Useful for travel",
        "created_at": "2024-01-15T11:00:00Z",
        "translation": {
          "id": "550e8400-e29b-41d4-a716-446655440000",
          "text": "The quick brown fox",
          "translation": "El rápido zorro marrón",
          "source_lang": "en",
          "target_lang": "es",
          "input_type": "sentence",
          "provider": "openai-compatible",
          "model": "ACTION",
          "created_at": "2024-01-15T10:30:00Z"
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

### PUT `/favorites/:id`
Update a favorite note.

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
    "id": "880e8400-e29b-41d4-a716-446655440002",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "word_id": "770e8400-e29b-41d4-a716-446655440001",
    "note": "Updated note for context",
    "created_at": "2024-01-15T11:00:00Z",
    "updated_at": "2024-01-15T12:30:00Z"
  }
}
```

**Error Responses**:
- `400`: Validation error
- `401`: Authentication required
- `403`: Favorite not found or access denied
- `404`: Favorite not found

---

### DELETE `/favorites/:id`
Delete a favorite.

**Headers** (Required):
```
Authorization: Bearer <access_token>
```

**Response (200 OK)**:
```json
{
  "success": true,
  "data": {
    "message": "Favorite deleted successfully"
  }
}
```

**Error Responses**:
- `401`: Authentication required
- `403`: Favorite not found or access denied
- `404`: Favorite not found

---

## User Profile

### GET `/profile`
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
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "created_at": "2024-01-01T00:00:00Z",
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

### PUT `/profile`
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
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
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

1. **Validate JWT tokens** on protected routes (`/history`, `/favorites`, `/profile`)
2. **Forward user_id** to translate requests from JWT claim (for history tracking)
3. **Do not require authentication** for `/translate` (anonymous access)
4. **Add user_id** to translation response only when JWT is provided (authenticated user)
5. **Cache responses** for public routes (`/about`, `/health`, `/stats`)

### Authentication Logic for `/translate`

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

### Tables

- **users**: User accounts with `uuid` as primary key
- **sessions**: Active JWT sessions
- **translations**: Translation history with `user_id` (nullable)
- **favorites**: Saved translations/words with `user_id` (required)
- **word_meanings**: Word definitions with cached embeddings
- **translation_parts**: Individual word/phrase translations

### Write Pattern

- **Authentication**: Insert into `users` + `sessions` tables
- **Translation**: Insert into `translations` + `translation_parts` + `word_meanings` (with Qdrant cache check)
  - `user_id` is included when JWT is provided (authenticated user)
  - `user_id` is NULL when JWT is NOT provided (anonymous user)
- **Favorites**: Insert into `favorites` table
- **History**: Query `translations` with pagination and filters

### Read Pattern

- **History**: Join `translations` + `users` with pagination (where `user_id` matches JWT `sub`, or all for admin)
- **Favorites**: Join `favorites` + `translations` + `word_meanings` (where `user_id` matches JWT `sub`)

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
