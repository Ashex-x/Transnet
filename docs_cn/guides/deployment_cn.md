# GPU 服务器部署

English: [GPU-server deployment](../../docs/guides/deployment.md)

本指南说明如何由 GitHub Actions 通过现有 FRP SSH endpoint `139.224.103.112:16004` 将 Transnet 部署到 GPU 服务器。工作流会测试 pull request，但不会部署它。推送到 `master` 或经审核的手动触发必须通过全部仓库检查，production job 才会原子激活不可变 release 并重启 `island.transnet.service`。

## GPU 服务器服务配置

当前生产服务是 `ashex` 用户级单元 `island-transnet.service`。它从 `/home/ashex/services/island.transnet/current` 运行，绑定 `/home/ashex/services/island.transnet/transnet.sock`，由 GitHub 工作流通过 `systemctl --user` 重启。部署工作流在激活不可变 release 前只改写 listener socket 路径。应保持用户级单元启用，并允许 `ashex` 写入服务目录；另一套系统级 `island.transnet.service` 不是部署目标。
如果仍安装旧的系统级单元，管理员必须先执行 `sudo systemctl disable --now island.transnet.service` 停止并禁用它，再启动用户级单元；否则两者会争夺同一 socket。部署 gate 会核对运行中的用户级单元 PID 是否拥有 socket 并执行当前 release binary；旧单元占用的可连接 socket 不会被误判为部署成功。

仓库中的生产配置当前采用纯模型翻译。由于尚未发布经审核的 canonical release，`[canonical]` 与 `[knowledge]` 均关闭。该配置无需 canonical socket 或知识游标密钥，不公告知识视角，并将相关就绪组件报告为 `disabled`。成功翻译仍需模型 endpoint 和 generation provider；目标就绪只报告已配置依赖的状态，并非虚构的模型输出测试。

获得获批的 canonical 内容和完整投影 tuple 后，通过经过审核的配置变更启用 canonical 与 knowledge，在用户级单元的外部环境文件中提供稳定的 32 至 4,096 byte 游标密钥，并在修改 required knowledge readiness 前验证 active release。不得把密钥、provider 凭据、请求文本、生成输出、日志或运行时状态写入 Git。详见[配置指南](configuration_cn.md)。

## GitHub 配置

创建专用 Ed25519 部署密钥，将公钥加入 GPU 服务器的 `/home/ashex/.ssh/authorized_keys`，再配置以下 GitHub Actions 仓库 secret：

- `DEPLOY_SSH_KEY`：对应的私钥。
- `DEPLOY_KNOWN_HOSTS`：`[139.224.103.112]:16004` 对应的完整、可信 host-key 行。

应通过可信的管理连接获取 host-key 行，而不是未经验证的网络扫描。为 GitHub `production` environment 配置所需 reviewer 和分支限制。工作流使用只读仓库权限、拒绝未知 host key，并通过 `transnet-production` concurrency group 串行执行部署。

## 部署行为

Linux 测试 job 会检查格式、对全部 target 与 feature 运行 Clippy、运行 all-target 测试、构建 rustdoc，并生成锁定依赖的 release binary。独立的 Windows job 会运行 `cargo check --bin transnet --locked`，以守护 non-Unix fail-closed 编译边界；它既不会启动生产 runtime，也不会生成部署 artifact。Pull request 在这些检查后结束。成功的非 pull-request 运行会下载该精确 Linux artifact，把它和生产配置上传到 commit-SHA 目录，原子切换 `/home/ashex/services/island.transnet/current`，重启用户级单元 `island-transnet.service`，再探测其 UDS。重启或探测失败时，只要存在先前 release，就会恢复它。只有成功激活后才删除七天以前的 release，且绝不删除 current 或 rollback target。

生产配置使用 GPU 主机实际提供的 OpenAI-compatible model identifier `Gemma4-26B`。`TRANSNET_CONFIG` 在运行时选择已部署配置，因此上传的 binary 不依赖 CI runner checkout 路径。

生产 provider timeout 为 105 秒。Streaming caller 最多可准入 120 秒；较短的 provider
上限为严格验证、enrichment 与终止 SSE result 留出时间，同时避免退回旧的 30 秒
buffered 限制。Generation request 使用 llama.cpp JSON-object response format，防止严格
translation contract 被 Markdown prose 包裹。

canonical 数据库尚无获批 release 时，knowledge 保持关闭。翻译使用 LLM，不虚构 canonical 事实或知识视角。发布经审核的 canonical release 并 reconcile node/edge collection 后，通过 island-port 提交 activation candidate，并验证 `POST /api/v1/knowledge-releases/active`。只有在此之后，才通过经过审核的生产配置启用 canonical 与 knowledge 依赖。Required knowledge 模式在活动 tuple 缺失或不兼容时拒绝启动，并使 projection drift 导致 readiness 失败。

## 发布检查

检查目标 UDS 探针和服务状态：

```bash
curl --fail --unix-socket /home/ashex/services/island.transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'

curl --fail --unix-socket /home/ashex/services/island.transnet/transnet.sock \
  --request POST http://localhost/api/v1/readyz \
  --header 'content-type: application/json' \
  --data '{}'
```

相关文档：[开发与运维](development_cn.md)、[配置](configuration_cn.md)及 [Transnet 服务接口](../interfaces/transnet_cn.md)。
