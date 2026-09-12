# Transnet 计算服务计划

English: [compute-service plan](../docs/todo.md)

## 固定边界

Transnet 始终是纯回环计算服务。Island-port 拥有身份、权限、用户数据、隐私、加密、持久化、缓存、幂等、持久任务、MySQL、Qdrant 和所有有状态公共 API。

## 当前基线

- [x] 使用短/长提供商路由的 `/translate` 计算。
- [x] 仅模型驱动的结构化 `/v1/lookups` 计算。
- [x] 有界请求、请求 ID、脱敏 tracing、优雅关闭和提供商容错。
- [x] 计算接口排除调用者身份、凭据、持久化策略和持久任务字段。

## 后续计算工作

- [ ] 定义由 Island-port 提供、可用于证据卡片组装的规范候选快照。
- [ ] 在交付快照前由 Island-port 完成来源许可和显示过滤。
- [ ] 定义基于有界版本化输入的纯排序与组装操作。
- [ ] 定义基于给定评分标准的练习生成和答案评估。
- [ ] 增加拒绝身份、存储、数据库和持久化字段的合同测试。
- [ ] 增加负载、超时、取消、无效模型输出和 prompt injection 基准。

## 边界清理

- [ ] 将有状态和数据库基础设施移到 Island-port，或缩减为对调用者值进行纯计算的算法。
- [ ] 移除暗示 Transnet 拥有轮询、私有状态、规范 repository 或存储的条件路由。
- [ ] 保持部署二进制不依赖 MySQL、SQLx、Qdrant、会话、加密和身份。

## 完成标准

- Transnet 无数据库或向量服务也能启动并服务每个路由。
- 数据包或 trace 审查显示没有调用者身份、token、cookie、角色、持久化指令、数据库凭据、幂等键或任务能力跨越回环边界。
- 重试 Transnet 请求不会重复或修改持久状态。
- Island-port 独立测试权限、事务、保留、删除和持久化结果。

## 相关文档

- [计算服务架构](transnet_cn.md)
- [Island-port 接口](interfaces/port_cn.md)
- [MySQL 接口](interfaces/mysql_cn.md)
- [Qdrant 接口](interfaces/qdrant_cn.md)
