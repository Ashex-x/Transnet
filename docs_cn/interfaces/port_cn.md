# Island-port 接口

English: [Island-port interface](../../docs/interfaces/port.md)

本文件是受信任的 Island-port 与 Transnet 之间的完整中文 HTTP 合同。Transnet 不认证调用者、不管理会话，也不接受 Cookie 或 Bearer token；Island-port 在转发内部请求前完成认证和授权。

## 传输约定

请求和响应使用 JSON。每个响应包含 `X-Request-Id`。有状态路由要求 Island-port 提供 `X-Learner-Id`；修改操作要求 `Idempotency-Key`。匿名异步任务使用 `Lookup-Capability` 或 `Privacy-Capability`，它们只返回一次且不出现在 URL 中。`If-Match` 用于并发替换，游标是不透明且绑定版本的字符串。

未知字段和查询参数必须拒绝；时间使用 UTC RFC 3339，ID 使用不透明字符串，语言使用 BCP-47。`/v1` 错误使用 `application/problem+json`，`/translate` 保留 `{"error":"description"}` 旧格式。

## 端点清单

健康探针为 GET /health、GET /livez 和 GET /readyz；直接翻译为 POST /translate；学习卡计算和异步轮询为 POST /v1/lookups 与 GET /v1/lookup-jobs/{job_id}。其余路由按下列领域分组。

## 进程与翻译

健康、存活和就绪路由不需要学习者上下文。翻译只接受非空文本以及源、目标语言标记。

## 查词与任务

POST /v1/lookups 计算学习卡；GET /v1/lookup-jobs/{job_id} 轮询异步工作。匿名任务以一次性的 Lookup-Capability 受限访问。

## 规范词义与图

GET /v1/senses/{sense_id}、GET /v1/graph 和 GET /v1/graph/nodes/{kind}/{id}/neighbors 读取规范内容和有界图。

## 反馈

POST /v1/graph-edges/{edge_id}/feedback 记录个人反馈，要求学习者上下文和幂等键。

## 历史与保存词义

GET 和 DELETE /v1/history 及其 lookup ID 路由读取或删除历史。GET、PUT 和 DELETE /v1/saved-senses 及其词义路由保存学习状态。

## 练习与进度

POST /v1/practice/sessions、next、attempts，以及 current 和 GET /v1/progress 管理自适应练习与掌握度。

## 已保存图视图

GET、POST、PUT 和 DELETE /v1/graph-views 管理私有图布局；替换操作使用 If-Match。

## 学习者与隐私

GET /v1/me、PATCH /v1/me/preferences、POST /v1/me/export、DELETE /v1/me 和隐私任务路由管理偏好、导出与删除。

## 错误

```json
{
  "query": "caliente",
  "source_language": "es",
  "target_language": "en",
  "explanation_language": "zh-CN",
  "detail": "full",
  "include": ["relations", "word_history"]
}
```

```json
{
  "schema_version": "1.0",
  "lookup_id": "01JLOOKUP",
  "coverage": {"definitions": "available", "relations": "partial"},
  "warnings": []
}
```

分页使用不透明游标；私有资源的缺失和非所有者访问均返回 `404`。重复幂等请求重放原结果，不同请求复用同一键返回 `409`。服务错误返回 `503`，超过请求体限制返回 `413`，语义校验失败返回 `422`。
