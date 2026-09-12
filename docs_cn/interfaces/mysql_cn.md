# MySQL 适配器接口

English: [MySQL adapter interface](../../docs/interfaces/mysql.md)

MySQL 8 保存规范内容、学习者私有状态、反馈、练习、任务和幂等记录。JSON 仅表示适配器的逻辑载荷，不是网络协议或数据库 JSON 表结构。

## 通用合同

```json
{
  "operation": "saved_sense.upsert",
  "request_id": "01JREQUEST",
  "learner_id": "learner_01",
  "idempotency": {
    "key_digest": "sha256:BASE64",
    "request_fingerprint": "sha256:BASE64",
    "expires_at": "2026-09-13T10:00:00Z"
  },
  "input": {"sense_id": "01JHOT", "state": "learning"}
}
```

结果限定为 `ok`、`missing`、`conflict`、`replayed`、`expired`、`lease_lost` 或 `unavailable`。所有权、版本和幂等比较必须与写入处于同一事务。

## 规范内容与图

活动内容、词法搜索、候选加载、词义详情、图节点、邻居分页、暂存版本、门禁、向量构建、校验、发布、回滚和来源隔离都属于规范内容与图操作。

## 学习者状态与隐私

档案、偏好、历史、保存词义、隐私清单和隐私任务必须在同一所有权和版本检查模型下运行。

## 反馈、视图与练习

反馈事件和投影、私有图视图、练习会话、冻结题目、提交和掌握度是学习者私有状态；写入需要原子所有权、版本与幂等检查。

## 任务与幂等

查词任务、持久任务队列、租约、失败重试、重放和幂等预留在执行外部工作前持久化，并返回封闭结果。

## 存储规则

历史有明确过期时间且不保存上下文；学习者文本使用带版本密钥的认证加密，等值检索使用外部密钥 HMAC。日志和错误不得暴露 SQL、凭据、密文、哈希、学习者内容或连接信息。
