# Transnet 前端 API 文档

## 概述
- **协议**: HTTPS
- **内容类型**: `application/json`
- **认证方式**: JWT Bearer Token（仅用于受保护的端点）
- **ID 格式**: UUID v7。主标识符：`user_id`（用户）、`translation_id`（历史/收藏）。

## 路由
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

## 架构
```
客户端 (TypeScript)
    ↓
网关 (Rust) ← JWT 验证（用于受保护的路由）
    ↓
核心后端 (Rust) ← SQLite + Qdrant
```

**认证策略**:
- `/translate`: 无需认证（匿名访问）
- `/history`, `/favorites`, `/profile`: 需要 JWT 认证

---

## 系统信息

### GET `/api/about`
获取系统版本和信息。

**响应 (200 OK):**
```json
{
   "success": true,
   "data": {
     "name": "Transnet",
     "version": "1.0.0",
     "description": "具有历史记录和收藏功能的 AI 驱动翻译服务",
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
获取实时系统统计信息和健康状态。

**响应 (200 OK):**
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

组合的健康检查和就绪检查。

- **健康检查 (Liveness)**: HTTP 服务器正在运行并响应
- **就绪检查 (Readiness)**: 核心依赖（如 Qdrant、LLM API）可访问且健康

**响应 (200 OK):**
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

## 认证与授权

### JWT Token 结构
- **Header**: `Authorization: Bearer <token>`
- **Token Payload**:
```json
{
   "sub": "550e8400-e29b-41d4-a716-446655440000",  // user_id (UUID v7)
   "iat": 1234567890,                              // 签发时间 (Unix 时间戳)
   "exp": 1234571490,                              // 过期时间 (Unix 时间戳)
   "type": "access"                                // token 类型: "access" 或 "refresh"
}
```

### Token 生命周期
- **Access Token**: 有效期为 1 小时
- **Refresh Token**: 有效期为 7 天
- **Token 轮换**: 密码更改时，使所有 token 失效

---

## 账户管理

### POST `/api/account/register`
注册新账户。

**请求体:**
```json
{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "SecurePass123!"
}
```

**约束条件:**
- `username`: 3-50 个字符，允许字母数字和下划线
- `email`: 有效的邮箱格式
- `password`: 最少 8 个字符，至少包含一个大写字母、一个小写字母和一个数字

**响应 (201 Created):**
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

**错误响应:**
- `400`: 验证错误
- `409`: 用户名或邮箱已存在

---

### POST `/api/account/login`
登录账户。

**请求体:**
```json
{
  "email": "john@example.com",
  "password": "SecurePass123!"
}
```

**响应 (200 OK):**
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

**错误响应:**
- `400`: 凭据格式无效
- `401`: 邮箱或密码无效
- `423`: 账户未激活

---

### POST `/api/account/logout`
登出账户（使 token 失效）。

**Headers:**
```
Authorization: Bearer <access_token>
```

**响应 (200 OK):**
```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  }
}
```

**行为:**
- 删除用户的所有会话
- 使 access token 和 refresh token 都失效

---

### POST `/api/account/refresh`
使用 refresh token 刷新 access token。

**请求体:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**响应 (200 OK):**
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

**错误响应:**
- `401`: refresh token 无效或已过期
- `403`: refresh token 已被使用（一次性使用）

---

### POST `/api/account/change-password`
更改用户密码。

**Headers:**
```
Authorization: Bearer <access_token>
```

**请求体:**
```json
{
  "current_password": "OldPass123!",
  "new_password": "NewSecurePass456!"
}
```

**响应 (200 OK):**
```json
{
  "success": true,
  "data": {
    "message": "Password changed successfully"
  }
}
```

**行为:**
- 使所有现有会话/token 失效
- 使用更新后的密码创建新会话

---

## 翻译

### POST `/api/transnet/translate`
翻译文本。

**认证**: 可选（JWT Bearer token）

- **当提供 JWT 时（已认证用户）**:
  - 翻译保存到用户的历史记录
  - 响应中包含 `user_id`
  - 需要 `Authorization: Bearer <token>` 头

- **当未提供 JWT 时（匿名用户）**:
  - 返回翻译但**不保存**到历史记录
  - 响应中**不包含** `user_id`
  - 无需认证

**请求体:**
```json
{
  "text": "The quick brown fox",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic"
}
```

**字段说明:**
- `text` (必需): 要翻译的文本
- `source_lang` (必需): 源语言代码 (ISO 639-1)
- `target_lang` (必需): 目标语言代码 (ISO 639-1)
- `mode` (必需): 翻译模式，可选值：
  - `basic`
  - `explain`
  - `full_analysis`
  后端会自动检测 `input_type` 并验证该 `mode` 对检测到的输入类型是否支持。参见 `types.md` 了解检测标准和每个输入类型的行为。

**翻译字段类型**: 响应中的 `translation` 字段包含基于检测到的 `input_type` 和请求的 `mode` 的特定类型数据。`types.md` 中定义了 **9 种可能的类型**：

| 输入类型    | 模式            | 翻译类型                          |
| ---------- | --------------- | -------------------------------- |
| `word`     | `basic`         | `TranslationWordBasic`           |
| `word`     | `explain`       | `TranslationWordExplain`         |
| `word`     | `full_analysis` | `TranslationWordFullAnalysis`    |
| `phrase`   | `basic`         | `TranslationPhraseBasic`         |
| `phrase`   | `explain`       | `TranslationPhraseExplain`       |
| `phrase`   | `full_analysis` | `TranslationPhraseFullAnalysis`  |
| `sentence` | `basic`         | `TranslationSentenceBasic`       |
| `sentence` | `explain`       | `TranslationSentenceExplain`     |
| `paragraph`| `basic`         | `TranslationParagraphEssayBasic` |
| `essay`    | `basic`         | `TranslationParagraphEssayBasic` |

关于每种翻译字段类型的详细 JSON 模式和示例，请参见 `types.md`。

**已认证响应 (200 OK):**
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
      // 翻译字段类型特定数据
    }
  }
}
```

**匿名响应 (200 OK):**
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
      // 翻译字段类型特定数据
    }
  }
}
```

**行为:**
- 仅在提供 JWT 时将翻译保存到历史记录
- 始终返回 `translation_id`，用于历史与收藏（如 POST `/favorites`）；匿名时也会返回

**错误响应:**
- `400`: 请求数据无效
- `422`: 语义上无效的请求
- `500`: 内部服务器错误
- `503`: 服务不可用（LLM 后端宕机）

---

## 翻译历史

### GET `/api/transnet/history`
获取已认证用户的翻译历史。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**查询参数:**
- `page` (可选, 默认: 1): 页码
- `limit` (可选, 默认: 20, 最大: 100): 每页项目数
- `source_lang` (可选): 按源语言过滤
- `target_lang` (可选): 按目标语言过滤
- `input_type` (可选): 按输入类型过滤

**示例请求:**
```
GET /api/history?page=1&limit=20&source_lang=en&target_lang=es
```

**响应 (200 OK):**
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
          // 翻译字段类型特定数据
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
          // 翻译字段类型特定数据
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

**错误响应:**
- `401`: 需要认证

---

### GET `/api/transnet/history/:translation_id`
通过 ID 获取特定翻译。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**响应 (200 OK):**
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
      // 翻译字段类型特定数据
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

**错误响应:**
- `401`: 需要认证
- `403`: 翻译未找到或访问被拒绝
- `404`: 翻译未找到

---

### DELETE `/api/transnet/history/:translation_id`
从历史记录中删除翻译。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**响应 (200 OK):**
```json
{
  "success": true,
  "data": {
    "message": "Translation deleted successfully"
  }
}
```

**行为:**
- 删除翻译及关联的 translation_parts
- 只有所有者可以删除自己的翻译

**错误响应:**
- `401`: 需要认证
- `403`: 翻译未找到或访问被拒绝
- `404`: 翻译未找到

---

## 收藏

收藏即历史记录中 `is_favorite = true` 且带有可选 `note` 的项。无单独的收藏 ID；用 `translation_id` 标识一条收藏。

### POST `/api/transnet/favorites`
将某条翻译标记为收藏（在对应历史记录上设置 `is_favorite = true` 并可选保存备注）。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**请求体:**
```json
{
  "translation_id": "550e8400-e29b-41d4-a716-446655440000",
  "note": "Useful for travel"
}
```

**字段:**
- `translation_id` (必需): 用户历史中的翻译 ID
- `note` (可选, 最多 500 字符): 用户备注（默认空）

**响应 (200 OK):**
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

**错误响应:**
- `400`: 验证错误
- `401`: 需要认证
- `404`: 翻译未找到
- `409`: 已收藏（该翻译的 `is_favorite` 已为 true）

---

### GET `/api/transnet/favorites`
获取用户收藏（即历史记录中 `is_favorite = true` 的项）。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**查询参数:**
- `page` (可选, 默认: 1): 页码
- `limit` (可选, 默认: 20, 最大: 100): 每页项目数

**示例请求:**
```
GET /api/favorites?page=1&limit=20
```

**响应 (200 OK):**
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
            // 翻译字段类型特定数据
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

**错误响应:**
- `401`: 需要认证

---

### PUT `/api/transnet/favorites/:translation_id`
更新某条收藏翻译的备注。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**请求体:**
```json
{
  "note": "Updated note for context"
}
```

**响应 (200 OK):**
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

**错误响应:**
- `400`: 验证错误
- `401`: 需要认证
- `403`: 该翻译未在收藏中或访问被拒绝
- `404`: 翻译未找到

---

### DELETE `/api/transnet/favorites/:translation_id`
取消收藏（将对应历史记录的 `is_favorite` 设为 false）。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**响应 (200 OK):**
```json
{
  "success": true,
  "data": {
    "message": "Favorite removed successfully"
  }
}
```

**错误响应:**
- `401`: 需要认证
- `403`: 该翻译未在收藏中或访问被拒绝
- `404`: 翻译未找到

---

## 用户资料

### GET `/api/profile`
获取当前用户资料。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**响应 (200 OK):**
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

**错误响应:**
- `401`: 需要认证

---

### PUT `/api/profile`
更新用户资料。

**Headers (必需):**
```
Authorization: Bearer <access_token>
```

**请求体:**
```json
{
  "username": "johndoe_new",
  "email": "john_new@example.com"
}
```

**响应 (200 OK):**
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

**约束条件:**
- `username`: 3-50 个字符，允许字母数字和下划线
- `email`: 有效的邮箱格式

**错误响应:**
- `400`: 验证错误
- `401`: 需要认证
- `409`: 用户名或邮箱已存在

---

## 标准响应格式

### 成功响应
```json
{
  "success": true,
  "data": { /* ... */ }
}
```

### 错误响应
```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "人类可读的错误消息"
  }
}
```

---

## HTTP 状态码
- `200 OK`: 请求成功
- `201 Created`: 资源创建成功
- `400 Bad Request`: 请求数据无效
- `401 Unauthorized`: 缺少或无效的认证
- `403 Forbidden`: 拒绝访问资源
- `404 Not Found`: 资源未找到
- `409 Conflict`: 资源冲突（重复）
- `422 Unprocessable Entity`: 语义上无效的请求
- `429 Too Many Requests`: 超过速率限制
- `500 Internal Server Error`: 服务器或提供者故障
- `503 Service Unavailable`: 下游模型不可用

---

## 网关行为

网关 (transnet-server) 应该：

1. **验证 JWT token**：在受保护的路由 (`/api/history`, `/api/favorites`, `/api/profile`) 上
2. **转发 user_id**：从 JWT claim 转发到翻译请求（用于历史记录跟踪）
3. **不要求认证**：对于 `/api/transnet/translate`（匿名访问）
4. **添加 user_id**：仅在提供 JWT 时（已认证用户）在翻译响应中添加 `user_id` 字段
5. **缓存响应**：对于公共路由 (`/api/about`, `/api/health`, `/api/stats`)

## `/api/transnet/translate` 的认证逻辑

**如果未提供 JWT (空/null)**:
- 返回翻译，**不包含** `user_id` 字段
- 翻译**不保存**到历史记录
- 翻译 ID 仍可用于收藏

**如果提供了 JWT (已认证用户)**:
- 返回翻译，**包含** `user_id` 字段（来自 JWT 的 `sub` claim）
- 翻译**保存**到用户的历史记录
- 返回完整的用户上下文

---

## 数据库集成

**Web 服务器 (MySQL)** 的表结构（用户、历史、收藏作为历史的一部分）见 `docs/database.md`。

以下描述后端存储约定。标识符与 API 一致：`user_id`（用户）、`translation_id`（历史）。

### 表（概念）
- `users`: 用户账户；主键 `user_id`
- `sessions`: 活跃的 JWT 会话
- `history`（translations）: 翻译历史；主键 `translation_id`，外键 `user_id`。包含 `is_favorite`（布尔）和 `note`（可选）表示收藏；无单独收藏表
- `word_meanings`: 单词定义与缓存嵌入（后端）
- `translation_parts`: 单词/短语级翻译（后端）

### 写入模式
- **认证**: 插入 `users` + `sessions`
- **翻译**: 插入历史，带 `translation_id`、`user_id`（提供 JWT 时必填，匿名可为空）。可选写入 `translation_parts` + `word_meanings`（先查 Qdrant 缓存）
- **收藏**: 更新历史记录：设置 `is_favorite = true` 及可选 `note`（POST）；更新 `note`（PUT）；设置 `is_favorite = false`（DELETE）

### 读取模式
- **历史**: 按 `user_id` 查询历史，分页与过滤
- **收藏**: 查询历史中 `user_id` 匹配 JWT `sub` 且 `is_favorite = true` 的项

### 缓存策略
- **单词含义**: 先检查 Qdrant，然后 SQLite
- **翻译**: 无缓存（用户特定数据）
- **嵌入**: 首次翻译时懒加载

---

## 安全考虑

### 密码安全
- 最少 8 个字符
- 至少包含一个大写字母、一个小写字母和一个数字
- bcrypt 哈希（成本因子：12）
- 无密码重用历史

### Token 安全
- JWT 使用 HS256 签名
- 强制执行 token 过期
- 密码更改时轮换 token
- Refresh token 为一次性使用

### 速率限制
- 每个 IP 每分钟 100 次请求
- 每个用户每小时 1000 次请求
- Header: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

### CORS
- 前端域名白名单
- 允许 `GET`, `POST`, `PUT`, `DELETE` 方法
- 允许 Authorization header
