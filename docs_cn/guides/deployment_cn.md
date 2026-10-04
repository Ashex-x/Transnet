# GPU 服务器部署

English: [GPU-server deployment](../../docs/guides/deployment.md)

本指南说明如何由 GitHub Actions 通过现有 FRP SSH endpoint `139.224.103.112:16004` 将 Transnet 部署到 GPU 服务器。工作流会测试 pull request，但不会部署它。推送到 `master` 或经审核的手动触发必须通过全部仓库检查，production job 才会原子激活不可变 release 并重启 `island.transnet.service`。

## GPU 服务器一次性设置

由具备 sudo 权限的管理员在 GPU 服务器上一次性运行以下命令。服务部署在 `/home/ashex/services/island.transnet`，不需要生产 Git checkout。

```bash
sudo install -d -o ashex -g ashex -m 0755 /home/ashex/services/island.transnet/releases
sudo install -d -o root -g root -m 0755 /etc/island
sudo install -o root -g root -m 0600 /dev/null /etc/island/transnet.env
sudo install -m 0644 deploy/island.transnet.service /etc/systemd/system/island.transnet.service
sudo systemctl daemon-reload
sudo systemctl enable island.transnet
```

该 unit 在 `island-port.service` 之后启动，并对它使用弱 `Wants` 依赖。这保留独立重启行为：停止或重启 island-port 不会自动停止 Transnet，而正常启动会先排序 canonical authority。该 unit 以 `0770` 模式创建 `/run/transnet`，并将进程 umask 设为 `0007`。运行时只在其中绑定 `/run/transnet/transnet.sock`；应仅通过配置的组向 island-port 运行时授予访问权。

部署的生产配置通过 `/run/island-port/island-port.sock` 启用 canonical 读取，并以可选 bootstrap 模式启用 knowledge。在 root-owned environment 文件中放置一个稳定的 32 至 4,096 byte cursor key，不要使用 shell interpolation，例如 `TRANSNET_KNOWLEDGE_CURSOR_SECRET=<random-value>`。该文件由 systemd 加载，不会被 Transnet 当作 TOML 解析，也绝不能进入 release artifact 或 Git。Provider credential 与其他生产 override 同样留在版本控制之外。不得向本仓库加入凭据、请求文本、生成输出、日志或运行时状态。当前及目标设置见[配置指南](configuration_cn.md)。

`ashex` 用户需要仅限重启和检查该服务的免密码权限。先运行 `sudo visudo -cf deploy/island.transnet.sudoers`，再运行 `sudo install -m 0440 deploy/island.transnet.sudoers /etc/sudoers.d/island-transnet` 来安装仓库中的规则。安装文件名刻意不含点号，因为 sudoers 会忽略该目录中的 dotted file。其精确内容是：

```sudoers
ashex ALL=(root) NOPASSWD: /usr/bin/systemctl restart island.transnet, /usr/bin/systemctl is-active --quiet island.transnet
```

若 `command -v systemctl` 返回其他路径，请相应调整 `/usr/bin/systemctl`。

## GitHub 配置

创建专用 Ed25519 部署密钥，将公钥加入 GPU 服务器的 `/home/ashex/.ssh/authorized_keys`，再配置以下 GitHub Actions 仓库 secret：

- `DEPLOY_SSH_KEY`：对应的私钥。
- `DEPLOY_KNOWN_HOSTS`：`[139.224.103.112]:16004` 对应的完整、可信 host-key 行。

应通过可信的管理连接获取 host-key 行，而不是未经验证的网络扫描。为 GitHub `production` environment 配置所需 reviewer 和分支限制。工作流使用只读仓库权限、拒绝未知 host key，并通过 `transnet-production` concurrency group 串行执行部署。

## 部署行为

Linux 测试 job 会检查格式、对全部 target 与 feature 运行 Clippy、运行 all-target 测试、构建 rustdoc，并生成锁定依赖的 release binary。独立的 Windows job 会运行 `cargo check --bin transnet --locked`，以守护 non-Unix fail-closed 编译边界；它既不会启动生产 runtime，也不会生成部署 artifact。Pull request 在这些检查后结束。成功的非 pull-request 运行会下载该精确 Linux artifact，把它和生产配置上传到 commit-SHA 目录，原子切换 `/home/ashex/services/island.transnet/current`，重启 `island.transnet`，再探测目标 UDS。重启或探测失败时，只要存在先前 release，就会恢复它。只有成功激活后才删除七天以前的 release，且绝不删除 current 或 rollback target。

生产配置使用 GPU 主机实际提供的 OpenAI-compatible model identifier `Gemma4-26B`。`TRANSNET_CONFIG` 在运行时选择已部署配置，因此上传的 binary 不依赖 CI runner checkout 路径。

生产 provider timeout 为 105 秒。Streaming caller 最多可准入 120 秒；较短的 provider
上限为严格验证、enrichment 与终止 SSE result 留出时间，同时避免退回旧的 30 秒
buffered 限制。Generation request 使用 llama.cpp JSON-object response format，防止严格
translation contract 被 Markdown prose 包裹。

首次针对空 canonical authority 部署时，knowledge 刻意保持 optional。Transnet 仍要求 canonical readiness，但当 island-port 尚无完整的活动 canonical/node/edge release tuple 时，会在不安装 knowledge route 的情况下启动。发布并 reconcile 两个不可变 collection，通过 island-port 提交 activation candidate，并验证 `POST /api/v1/knowledge-releases/active`。随后把已部署 TOML 复制为 operator-owned 配置，将 `[knowledge].required` 改为 `true`，通过 systemd drop-in 令 `TRANSNET_CONFIG` 指向该文件，再重启 Transnet。Required 模式会在活动 tuple 缺失或不兼容时拒绝启动，并使 projection drift 导致 readiness 失败。不要通过编辑不可变 release directory 启用 required 模式。

## 发布检查

检查目标 UDS 探针和服务状态：

```bash
curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'

curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/readyz \
  --header 'content-type: application/json' \
  --data '{}'
```

相关文档：[开发与运维](development_cn.md)、[配置](configuration_cn.md)及 [Transnet 服务接口](../interfaces/transnet_cn.md)。
