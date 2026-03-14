# 数据库

本文描述 Transnet 所使用的数据库。

**数据流**：客户端 → Web 服务器 → 后端服务器

- **Web 服务器**：MySQL — 用户账号、资料及按用户维度的数据（历史、收藏）。
- **后端服务器**：RAG 数据库 — 用于翻译与检索；另行文档说明。

---

## Web 服务器 (MySQL)

Web 服务器使用 MySQL 存储用户身份、资料及与每个用户关联的数据。

### 用户 (Users)

存储用户账号与资料信息。

| 字段           | 类型      | 说明                     |
| -------------- | --------- | ------------------------ |
| user_id        | uuid      | 主键（如 UUID v7）       |
| username       | string    | 3–50 字符，唯一          |
| email          | string    | 合法邮箱，唯一           |
| password_hash  | string    | 哈希存储（如 bcrypt）    |
| updated_at     | timestamp | 最近一次资料更新时间     |
| active         | boolean   | 可选；用于账号停用状态   |

### 历史 (History)

按用户的翻译历史。每个用户对应一条翻译记录列表，对应 `/history` 接口。**收藏**即同一批记录中 `is_favorite = true` 的项；获取收藏列表时，在用户历史中按该布尔值筛选即可。`note` 字段为可选备注，默认空，在收藏时可填写（最多 500 字符）。

| 字段           | 类型      | 说明                             |
| -------------- | --------- | -------------------------------- |
| translation_id | uuid      | 主键                             |
| user_id        | uuid      | 外键 → Users                     |
| text           | string    | 原文                             |
| translation    | string    | 译文                             |
| source_lang    | string    | 语言代码（如 en）                |
| target_lang    | string    | 语言代码                         |
| input_type     | string    | 如 word, phrase, sentence, text  |
| provider       | string    | 如 openai-compatible             |
| model          | string    | 模型名称                         |
| created_at     | timestamp | 翻译创建时间                     |
| is_favorite    | boolean   | 默认 false；为 true 表示已收藏   |
| note           | string?   | 可选，最多 500 字符，默认空（收藏时使用） |
| updated_at     | timestamp | 可选；备注最近更新时间           |
