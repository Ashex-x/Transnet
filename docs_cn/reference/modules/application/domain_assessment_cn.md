# 领域评估

English: [Domain assessment](../../../../docs/reference/modules/application/domain_assessment.md)

领域评估 application 模块判断已解析含义是否适合领域专属展开，并记录一个闭合的请求级解析结果。

状态：目标设计。领域类型与存储合同已存在，但当前可执行文件尚未检索生产领域清单或组合此评估路径。

## 职责

模块先检索有界且发布固定的清单，其中包含稳定领域 ID、多语言名称与别名、定义、纳入与排除范围、上层领域和知识覆盖。模型只能选择 allowlist 中的 ID。

确定性结果为 `existing`、`proposed_new`、`general` 或 `uncertain`。未选择任何已提供 ID 且给出有效结构化说明时，可产生请求级提案；清单故障始终产生 `uncertain`。提案没有规范领域 ID，且只在 full 投影允许时出现。

## 依赖与不变量

模块使用结构化数据读 port，并可在有用时调用有界结构化模型操作。它既不搜索不受限的模型生成目录，也不调用实时领域创建操作。

技术语境中的常用词可以选择领域，支持薄弱、形似术语的字符串则保持 general 或 uncertain。提案与模型推理随请求丢弃，除非进入单独的已审核发布流程，否则绝不进入 MySQL 或 Qdrant。

## 验证

测试应覆盖 allowlist 内已有选择、一般含义、正当提案、弱证据、重叠范围、多语言标签及不可用或不兼容清单。测试必须证明未知模型 ID 会被拒绝，依赖故障不会伪装成 `proposed_new`。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [SQL 数据 endpoint](../../../interfaces/mysql_cn.md)
- [系统设计](../../../transnet_cn.md)
- [质量保证](../../../guides/quality-assurance_cn.md)
