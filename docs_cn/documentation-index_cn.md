# Transnet 中文文档

本目录是 `docs/` 的中文镜像。英文文档是规范来源；本目录中的页面与英文页面保持相同的主题、路径层级和链接关系。

## 设计与规划

- [系统设计与架构](transnet_cn.md)：运行时边界、信任合同和未来计算扩展。
- [英语学习体验](product/learning-experience_cn.md)：查词卡片、关系图、练习和个性化。
- [总体计划](todo_cn.md)：当前基线、后续工作和完成标准。

## 接口

- [Island-port 接口](interfaces/port_cn.md)：无认证的完整 HTTP 合同、请求头、错误和 JSON 示例。
- [MySQL 适配器接口](interfaces/mysql_cn.md)：持久化操作、事务约束和请求载荷。
- [Qdrant 适配器接口](interfaces/qdrant_cn.md)：集合、向量点、检索、校验、发布和清理。

## 指南

- [配置](guides/configuration_cn.md)
- [开发与运维](guides/development_cn.md)
- [内容发布](guides/content-publishing_cn.md)
- [质量保证](guides/quality-assurance_cn.md)

## 仓库规则

- [仓库约定 v1.0.1](conventions-v1.0.1_cn.md)：Rust、文档、验证和 Git 规则。

接口文档是规范合同；Rust trait 的细节仍以源代码注释和 rustdoc 为准。
