# Island-port 接口

English: [Island-port interface](../../docs/interfaces/port.md)

本文件是受信任的 Island-port 与 Transnet 之间的完整中文 HTTP 合同。Transnet 不认证调用者、不管理会话，也不接受 Cookie 或 Bearer token；Island-port 在转发内部请求前完成认证和授权。

## 传输约定

请求和响应使用 JSON。每个响应包含 `X-Request-Id`。有状态路由要求 Island-port 提供 `X-Learner-Id`；修改操作要求 `Idempotency-Key`。匿名异步任务使用 `Lookup-Capability` 或 `Privacy-Capability`，它们只返回一次且不出现在 URL 中。`If-Match` 用于并发替换，游标是不透明且绑定版本的字符串。

未知字段和查询参数必须拒绝；时间使用 UTC RFC 3339，ID 使用不透明字符串，语言使用 BCP-47。`/v1` 错误使用 `application/problem+json`，`/translate` 保留 `{"error":"description"}` 旧格式。

## 路由

- `GET /health`、`GET /livez`、`GET /readyz`：进程状态、存活和就绪。
- `POST /translate`：直接翻译。
- `POST /v1/lookups`、`GET /v1/lookup-jobs/{job_id}`：学习卡计算和异步轮询。
- `GET /v1/senses/{sense_id}`、`GET /v1/graph`、`GET /v1/graph/nodes/{kind}/{id}/neighbors`：规范内容和图读取。
- `POST /v1/graph-edges/{edge_id}/feedback`：记录个人反馈。
- `GET/DELETE /v1/history`、`GET/DELETE /v1/history/{lookup_id}`：历史读取和删除。
- `GET/PUT/DELETE /v1/saved-senses` 及其 ID 路由：保存词义和学习状态。
- `POST /v1/practice/sessions`、`next`、`current`、`attempts`、`GET /v1/progress`：练习与掌握度。
- `GET/POST/PUT/DELETE /v1/graph-views`：私人图视图。
- `GET /v1/me`、`PATCH /v1/me/preferences`、`POST /v1/me/export`、`DELETE /v1/me`：学习者偏好和隐私操作。
- `GET /v1/privacy-requests/{request_id}`、`POST .../result`：隐私任务。

## 请求示例

    {
      "query": "caliente",
      "source_language": "es",
      "target_language": "en",
      "explanation_language": "zh-CN",
      "detail": "full",
      "include": ["relations", "word_history"]
    }

    {
      "schema_version": "1.0",
      "lookup_id": "01JLOOKUP",
      "coverage": {"definitions": "available", "relations": "partial"},
      "warnings": []
    }

分页使用不透明游标；私有资源的缺失和非所有者访问均返回 `404`。重复幂等请求重放原结果，不同请求复用同一键返回 `409`。服务错误返回 `503`，超过请求体限制返回 `413`，语义校验失败返回 `422`。
