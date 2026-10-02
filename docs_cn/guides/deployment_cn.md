# GPU 服务器部署

English: [GPU-server deployment](../../docs/guides/deployment.md)

本指南说明如何由 GitHub Actions 通过现有 FRP SSH endpoint `139.224.103.112:16004` 将 Transnet 部署到 GPU 服务器。工作流会测试 pull request，但不会部署它。推送到 `master` 或经审核的手动触发必须通过全部仓库检查，production job 才会原子激活不可变 release 并重启 `island.transnet.service`。

## GPU 服务器一次性设置

由具备 sudo 权限的管理员在 GPU 服务器上一次性运行以下命令。服务部署在 `/home/ashex/services/island.transnet`，不需要生产 Git checkout。

```bash
sudo install -d -o ashex -g ashex -m 0755 /home/ashex/services/island.transnet/releases
sudo install -m 0644 deploy/island.transnet.service /etc/systemd/system/island.transnet.service
sudo systemctl daemon-reload
sudo systemctl enable island.transnet
```

该 unit 以 `0770` 模式创建 `/run/transnet`，并将进程 umask 设为 `0007`。运行时只在其中绑定 `/run/transnet/transnet.sock`；应仅通过配置的组向 island-port 运行时授予访问权。

生产配置和所有 provider 凭据都必须留在版本控制之外。不得向本仓库加入凭据、请求文本、生成输出、日志或运行时状态。当前及目标设置见[配置指南](configuration_cn.md)。

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

测试 job 会检查格式、对全部 target 与 feature 运行 Clippy、运行 all-target 测试、构建 rustdoc，并生成锁定依赖的 release binary。Pull request 在该 job 后结束。成功的非 pull-request 运行会下载该精确 artifact，把它和生产配置上传到 commit-SHA 目录，原子切换 `/home/ashex/services/island.transnet/current`，重启 `island.transnet`，再探测目标 UDS。重启或探测失败时，只要存在先前 release，就会恢复它。只有成功激活后才删除七天以前的 release，且绝不删除 current 或 rollback target。

生产配置使用 GPU 主机实际提供的 OpenAI-compatible model identifier `Gemma4-26B`。`TRANSNET_CONFIG` 在运行时选择已部署配置，因此上传的 binary 不依赖 CI runner checkout 路径。

## 发布检查

检查目标 UDS 探针和服务状态：

```bash
curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'
```

相关文档：[开发与运维](development_cn.md)、[配置](configuration_cn.md)及 [Transnet 服务接口](../interfaces/transnet_cn.md)。
