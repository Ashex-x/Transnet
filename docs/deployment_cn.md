# 后端翻译服务 API

本文档描述 **后端翻译服务** 的 API。它由 Web 服务器网关调用（参见 [api.md](api.md)）。后端仅执行翻译（可选 RAG），不暴露用户或账户端点。

---

## 概述

**协议**: HTTP  
**内容类型**: 请求和响应体使用 `application/json`  
**认证**: 无。后端由 Web 服务器或其他可信调用方调用；认证由网关处理。

**相关文档**:
- **Web 服务器（网关）API**: [api.md](api.md) — 公共 API、账户、历史、收藏、个人资料。
- **架构**: [architecture.md](architecture.md) — 输入分析、翻译类型、工作流。
- **翻译类型**: [types.md](types.md) — `input_type` 和 `mode` 的规范定义。
- **数据库**: [database.md](database.md) — Web 服务器 (MySQL)、后端 (RAG)。

---

## 端点摘要

| 方法 | 路径         | 描述                           |
|------|--------------|---------------------------------|
| POST | `/translate` | 翻译文本                        |
| GET  | `/health`    | 健康检查 + 依赖就绪检查         |

---

## 翻译

### POST `/translate`

翻译文本。请求和响应格式必须与 Web 服务器使用的翻译合约匹配（参见 [api.md — POST /translate](api.md#post-translate)）。后端不持久化历史记录或附加 `user_id`；网关在调用此服务时处理这些内容。

翻译行为由 [types.md](types.md) 和 [architecture.md](architecture.md) 中定义的 **输入类型** 和 **翻译模式** 决定。

**请求体**:
```json
{
  "text": "The quick brown fox",
  "source_lang": "en",
  "target_lang": "es",
  "mode": "basic"
}
```

**字段**:
- `text` (必需): 要翻译的文本
- `source_lang` (必需): 源语言代码 (ISO 639-1)
- `target_lang` (必需): 目标语言代码 (ISO 639-1)
- `mode` (必需): 翻译模式，可选值：
  - `basic`
  - `explain`
  - `full_analysis`
  后端会自动检测 `input_type` 并验证该 `mode` 对检测到的输入类型是否支持。参见 [types.md](types.md) 了解检测标准和每个输入类型的行为。

**响应 (200 OK)**:
```json
{
  "success": true,
  "data": {
    "translation_id": "550e8400-e29b-41d4-a716-446655440000",
    "text": "The quick brown fox",
    "translation": "El rápido zorro marrón",
    "source_lang": "en",
    "target_lang": "es",
    "input_type": "sentence",
    "provider": "openai-compatible",
    "model": "ACTION",
    "translation": {
      "tone": "neutral",
      "rephrasing": "El astuto zorro marrón salta por encima del perro perezoso."
    }
  }
}
```

**翻译字段类型**: `translation` 字段包含基于检测到的 `input_type` 和请求的 `mode` 的特定类型数据。**9 种可能的类型**（参见 [types.md](types.md)）：

| 输入类型  | 模式               | 翻译类型                    |
|----------|--------------------|----------------------------|
| `word`   | `basic`            | `TranslationWordBasic`     |
| `word`   | `explain`          | `TranslationWordExplain`    |
| `word`   | `full_analysis`    | `TranslationWordFullAnalysis`|
| `phrase` | `basic`            | `TranslationPhraseBasic`    |
| `phrase` | `explain`          | `TranslationPhraseExplain`   |
| `phrase` | `full_analysis`    | `TranslationPhraseFullAnalysis`|
| `sentence`| `basic`           | `TranslationSentenceBasic`   |
| `sentence`| `explain`         | `TranslationSentenceExplain`  |
| `paragraph`| `basic`          | `TranslationParagraphEssayBasic`|
| `essay`   | `basic`           | `TranslationParagraphEssayBasic`|

客户端应根据检测到的 `input_type` 和使用的 `mode` 反序列化 `translation` 字段。

**注意**: 后端响应不包含 `user_id`。当网关向客户端返回响应并在用户已认证时持久化到历史记录时，会从 JWT 中添加 `user_id`。

**错误响应**:
- `400`: 请求数据无效
- `422`: 语义上无效的请求
- `500`: 内部服务器错误
- `503`: LLM API 或依赖不可用

---

## 健康检查

### GET `/health`

组合的健康检查和就绪检查。

- **健康检查 (Liveness)**: HTTP 服务器正在运行并响应
- **就绪检查 (Readiness)**: 核心依赖（如 Qdrant、LLM API）可访问且健康

**响应 (200 OK)**:
```json
{
  "success": true,
  "data": {
    "status": "ready",
    "service": "transnet-backend",
    "checks": {
      "qdrant": "connected",
      "llm_api": "reachable"
    }
  }
}
```

**错误响应**:
- `503`: 服务器关闭或无法提供服务，或必需的依赖检查失败。响应体应在可用时包含 `checks` 以及每个依赖的状态。

---

## 标准响应格式

**成功**:
```json
{
  "success": true,
  "data": { /* ... */ }
}
```

**错误**:
```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "人类可读的错误消息"
  }
}
```

**HTTP 状态码**: `200` OK、`400` Bad Request、`422` Unprocessable Entity、`500` Internal Server Error、`503` Service Unavailable
