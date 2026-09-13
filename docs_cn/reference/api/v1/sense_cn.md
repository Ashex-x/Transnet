# 版本 1 词义读取

English: [Version 1 sense reads](../../../../docs/reference/api/v1/sense.md)

`src/api/v1/sense.rs` 是 `POST /transnet/v1/senses/get` 的目标薄 Handler；该路由按翻译返回的规范词义 ID 执行后续读取。

状态：目标路由。现有规范词义详情服务与旧版 HTTP 路由是可复用基础，但默认启动器未生产组合此 UDS Endpoint。

## Handler 边界

Handler 严格解码合同要求的规范词义与发布标识符，保留请求 Deadline，调用面向操作的结构化数据应用服务，并序列化发布固定结果。它不执行自由文本搜索、推断用户意图或接受用户身份。

缺失词义映射为封闭 not-found 问题。发布不匹配或不兼容时安全失败，而非静默读取其他版本。返回的规范内容是已审核数据；请求级模型输出与向量相似度不能确立其规范性。

该路由只读，绝不更改发布状态、记录查看历史、保存图或把请求写入缓存或数据库。

## 验证

合同测试应覆盖准确发布固定、找到与未找到结果、发布冲突、严格字段、依赖失败、响应投影及不存在持久写入。规范路由见[Transnet 接口](../../../interfaces/transnet_cn.md)。

