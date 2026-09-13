# 模块参考

English: [Module reference](../../docs/reference/modules.md)

本目录记录重要服务模块及其所有权边界。先读本索引，再打开负责该行为的模块页。精确线上与存储 Schema 保留在[接口](../interfaces/README_cn.md)，产品语义保留在[系统设计](../transnet_cn.md)，操作步骤保留在[指南](../documentation-index_cn.md#指南)，Rust 公共项细节保留在源码注释与 rustdoc。

状态：当前可执行文件使用过渡期回环 HTTP。除非状态段明确说明行为已组合进默认运行时，否则目标页面描述的是预期边界。

## 模块地图

- [运行时](runtime/startup_cn.md)：进程启动、组合、就绪、停机及[配置](runtime/configuration_cn.md)。
- [传输](transport/server_cn.md)：UDS server、JSON 准入、middleware 与薄 API 映射。
- [Application](application/orchestration_cn.md)：请求流程与用例所有权；另见[翻译](application/translation_cn.md)和[知识](application/knowledge_cn.md)。
- [Domain](domain/translation_cn.md)：翻译词汇；另见[词汇知识](domain/lexical-knowledge_cn.md)和[发布](domain/releases_cn.md)。
- [Port](ports/models_cn.md)：模型操作；另见[数据访问](ports/data_cn.md)。
- [Adapter](adapters/providers_cn.md)：provider client；另见 [island-port](adapters/island-port_cn.md)。
- [运维](operations/observability_cn.md)：安全遥测与容错；另见[发布流程](operations/publication_cn.md)。

## 依赖规则

依赖向内指向：`transport -> application -> domain <- ports <- adapters`。API 代码校验并映射线上数据；application 代码编排用例；domain 代码拥有与传输无关的规则；port 描述 application 所需操作；adapter 实现外部 I/O。

在线服务只接收只读数据 port。只有离线 publisher 可以接收可变更 port。请求文本、历史、模型输出和请求级提案绝不跨越持久写边界。

## 查找所有者

先按概念搜索本索引。如果概念是请求或响应字段、路由、错误、SQL 操作或向量操作，应使用所属接口文档。如果它是当前 Rust 符号，应使用 `rg` 和 rustdoc。只有当稳定边界需要源码注释无法无重复表达的设计、生命周期或跨文件指导时，才新增模块页。

## 变更规则

模块页解释所有权、依赖、不变量、当前/目标状态与验证，并链接权威合同，而不复制字段、示例或操作步骤。英文页与中文镜像必须同时更新。
