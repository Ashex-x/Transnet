# 请求数据流

English: [Request dataflow](../../../docs/reference/application/request-dataflow.md)

本页跟踪一个请求如何经过 Transnet，并说明每个模块接收什么、决定什么、返回什么以及必须丢弃什么。精确路由与 payload 仍以 [Transnet 服务接口](../../interfaces/transnet_cn.md)为权威。

状态：下述目标流程尚未完成组合。当前可执行文件使用回环 HTTP，并直接连接过渡期翻译与模型驱动查询路径；[当前运行时](../runtime/startup_cn.md)说明今天实际可用的行为。

## 端到端流程

~~~mermaid
sequenceDiagram
  participant IP as island-port
  participant T as Transport
  participant O as Orchestrator
  participant D as Domain
  participant P as Ports
  participant A as Adapters
  participant X as External dependencies

  IP->>T: UDS HTTP/1.1 JSON 请求
  T->>T: 准入、解码、校验、建立请求上下文
  T->>O: 已校验请求 + 请求 ID + deadline
  O->>P: 读取兼容活动发布
  P->>A: 面向操作的数据调用
  A->>X: island-port 数据 endpoint
  X-->>A: 发布或闭合失败
  A-->>P: Domain 结果
  P-->>O: 固定发布三元组
  O->>D: 规范化并分类输入
  D-->>O: 单词、短语或篇章决策
  alt 连续篇章
    O->>P: 使用请求级上下文翻译
    P->>A: 模型操作
    A->>X: Gemma provider 调用
    X-->>A: 候选译文
    A-->>O: 有界模型结果
  else 单词或固定短语
    O->>P: 解析规范候选
    P->>A: 结构化/向量读取
    A->>X: island-port 数据调用
    X-->>A: 按发布过滤的候选与事实
    A-->>O: 已补全规范 bundle
    O->>P: 可选有界组织
    P->>A: 结构化模型操作
    A->>X: Gemma provider 调用
    X-->>A: 候选组织结果
    A-->>O: 有界模型结果
  end
  O->>D: 构造、投影并校验结果
  D-->>O: 已校验响应模型
  O-->>T: Application 结果
  T-->>IP: JSON 响应
~~~

一个 deadline 和一组发布三元组贯穿请求的每次下游调用。任何模块都不能替换这些值或延长 deadline。

## 准入

Island-port 移除终端用户身份，只发送服务请求以及可选的最小按时间排序翻译历史。传输模块验证 UDS-only 边界、请求大小、媒体类型、UTF-8、method 与 path、严格 JSON 结构、请求 ID、deadline 和并发 permit。

畸形或不可准入请求在此终止，不调用模型或数据依赖。传输模块把闭合失败映射为规范响应，并只记录不含内容的遥测。

## 编排与路由

Application 编排器只固定兼容发布一次，再要求 domain 逻辑规范化并分类输入。结果选择连续文本翻译或词汇知识处理；调用方绝不选择路径。

编排器负责调用顺序、剩余时间分配、取消和降级结果策略。它不解析 HTTP、不实现 provider payload、不发出 SQL 或 vector-native 查询，也不修改规范事实。

## 连续文本分支

翻译 application 推导模型操作。短输入使用已配置的 Gemma 4 角色；更长输入使用 TranslateGemma，并可使用请求级分块计划与术语台账。模型 port 把已校验输入及剩余 deadline 传给 provider adapter。

Adapter 创建 provider 专属 HTTP 请求、应用容错策略、限制并解码结果，再返回闭合结果。Application 逻辑在把译文加入超集结果前检查覆盖、顺序、术语一致性与输出有效性。

## 词汇知识分支

知识 application 推导查询形式，并通过数据 port 请求规范候选。精确与别名匹配优先于较弱检索信号。领域展开有用时，它对照已发布领域清单完成解析。

向量读取只提名按发布过滤的候选。结构化读取从同一发布的权威数据补全所选节点、事实、关系、证据与修订。相似度绝不建立事实。Application 过滤并排序已补全 bundle，随后可以要求模型 port 仅把这些已提供事实组织成精简分区。

确定性校验拒绝未知引用、无效方向、不兼容发布、不合格证据与越界内容。向量失败时，基础结构化卡片可带显式降级继续返回；权威内容缺失或发布不兼容必须安全失败。

## 投影与响应

Domain 与 application 逻辑组装一个超集结果。响应投影无需再次检索或调用模型即可派生 brief、standard 或 full。最终校验检查必需字段、发布一致性、证据标签、图边界、语言限制及不存在私有或持久化字段。

传输模块使用接口 envelope 序列化已校验 application 结果，添加请求 ID header，并记录静态路由、结果分类、byte 数与持续时间，不记录 body。

## 模块职责地图

- **运行时：** 构造依赖、只注册受支持路由、暴露 readiness、拥有 listener 生命周期并协调停机。
- **配置：** 一次性加载并校验有类型设置；向 bootstrap 提供脱敏值。
- **传输：** 准入 UDS HTTP/JSON、建立请求上下文、调用一次 application 操作并映射结果。
- **编排器：** 负责请求顺序、发布固定值、deadline 预算、分支选择、取消、降级与最终结果。
- **翻译 application：** 保留连续文本含义与结构；负责请求级分块和术语规划。
- **知识 application：** 解析词义与领域、检索并排序规范材料、组织关系分区。
- **翻译 domain：** 校验语言、历史、响应级别与翻译结果不变量。
- **词汇知识 domain：** 负责规范身份、有类型关系、证据语义与图不变量。
- **发布 domain：** 负责兼容不可变发布身份与有效降级状态。
- **模型 port：** 暴露有界翻译与结构化生成操作，不包含 provider 协议。
- **数据 port：** 暴露面向用例的规范与向量读取，不包含数据库原生请求。
- **Provider adapter：** 实现 OpenAI-compatible 请求、模型角色、响应解码与依赖失败映射。
- **Island-port adapter：** 实现 UDS 数据调用，并保留 deadline、发布与闭合结果。
- **可观测性与容错：** 记录安全聚合信号，并强制 timeout、重试、并发和断路器。
- **离线发布：** 在请求路径之外创建已审核发布；在线请求绝不能到达它。

## 请求生命周期数据

请求文本、历史、从私有输入派生的规范形式、分块计划、术语台账、provider 输入与输出、中间候选、推断解释和候选领域只在有界请求内存在。成功、失败、timeout 或取消时都会丢弃，绝不进入持久 cache、queue、MySQL、Qdrant、日志、指标或 trace。

已发布规范 ID、发布标识符、已审核事实和聚合运维计数不属于请求内容持久化。可缓存数据必须是规范、发布固定且不受私有请求影响。

## 验证

端到端测试应证明：依赖调用前提前拒绝、所有分支共享一组发布与 deadline、自动路由、provider 与数据 port 调用顺序、向量降级、权威数据失败、取消传播、确定性响应投影、安全错误映射、遥测脱敏及请求状态丢弃。
