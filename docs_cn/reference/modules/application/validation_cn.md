# 应用校验

English: [Application validation](../../../../docs/reference/modules/application/validation.md)

应用校验模块对目标请求、provider 候选、超集聚合和最终投影结果强制语义不变量。线上解码与安全 HTTP problem 映射仍属于 API 职责。

状态：由合同与已有 domain 不变量构成的目标设计。当前可执行文件会校验过渡路由，但尚未组合这一完整目标校验边界。

## 职责

输入检查强制支持的语言、源/目标兼容、文本与 body 限制、按时间排序的最小历史结构及闭合响应级别。Provider 与组织检查拒绝畸形结构化输出、未知引用、无支持断言和不安全的大小增长。

聚合检查强制含义归属、稳定规范 ID、一个兼容发布三元组、关系方向与适用性、图边界、证据与验证标签、语义尺度规则、降级一致性及最终响应限制。

## 依赖与不变量

校验是确定性的，不产生存储变更或 provider 副作用。它通过已校验配置或 domain 策略接收版本化 registry 与限制，而非由调用方控制关系语义。

故障映射为闭合安全 application 错误，不回显请求文本、历史、provider body、凭据、向量或存储内部信息。校验绝不通过虚构事实修复响应；允许的有界 schema 修复属于 provider adapter 策略，修复后仍须完整重新校验。

## 验证

测试应覆盖每个闭合值与边界、API 边界未知字段、多含义一致性、发布不匹配、分类方向与环、语义尺度、证据标签、图深度与大小、投影限制、无效 provider 输出及内容安全错误。Fuzz 与注入测试应把全部外部文本视为不可信数据。

## 相关文档

- [服务模块参考](../../modules_cn.md)
- [Transnet 服务接口](../../../interfaces/transnet_cn.md)
- [SQL 数据 endpoint](../../../interfaces/mysql_cn.md)
- [向量数据 endpoint](../../../interfaces/qdrant_cn.md)
- [质量保证](../../../guides/quality-assurance_cn.md)
