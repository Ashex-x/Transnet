# GPU 服务器部署

English: [GPU-server deployment](../../docs/guides/deployment.md)

本指南说明如何由 GitHub Actions 通过现有 FRP SSH endpoint `139.224.103.112:16004` 将 Transnet 部署到 GPU 服务器。工作流会测试 pull request，但不会部署它。推送到 `master` 或经审核的手动触发必须通过全部仓库检查，production job 才会更新并重启服务。

## GPU 服务器一次性设置

由具备 sudo 权限的管理员在 GPU 服务器上运行以下命令。将 `<repository-ssh-url>` 替换为 GitHub SSH clone URL。不要覆盖其他 checkout，也不要把凭据复制进仓库。

```bash
sudo -u ashex git clone <repository-ssh-url> /home/ashex/projects/Transnet
sudo cp /home/ashex/projects/Transnet/deploy/transnet.service /etc/systemd/system/transnet.service
sudo systemctl daemon-reload
sudo -u ashex bash -lc 'cd /home/ashex/projects/Transnet && cargo build --release --locked'
sudo systemctl enable transnet
sudo systemctl restart transnet
```

该 unit 以 `0770` 模式创建 `/run/transnet`，并将进程 umask 设为 `0007`。目标运行时会在其中绑定 `/run/transnet/transnet.sock`；应仅通过配置的组向 island-port 运行时授予访问权。在入站 UDS 迁移完成前，当前可执行文件仍使用配置的回环 TCP listener。

生产配置和所有 provider 凭据都必须留在版本控制之外。不得向本仓库加入凭据、请求文本、生成输出、日志或运行时状态。当前及目标设置见[配置指南](configuration_cn.md)。

`ashex` 用户需要仅限重启和检查该服务的免密码权限。通过 `sudo visudo -f /etc/sudoers.d/transnet` 添加：

```sudoers
ashex ALL=(root) NOPASSWD: /bin/systemctl restart transnet, /bin/systemctl is-active --quiet transnet
```

若 `command -v systemctl` 返回其他路径，请相应调整 `/bin/systemctl`。

## GitHub 配置

创建专用 Ed25519 部署密钥，将公钥加入 GPU 服务器的 `/home/ashex/.ssh/authorized_keys`，再配置以下 GitHub Actions 仓库 secret：

- `DEPLOY_SSH_KEY`：对应的私钥。
- `DEPLOY_KNOWN_HOSTS`：`[139.224.103.112]:16004` 对应的完整、可信 host-key 行。

应通过可信的管理连接获取 host-key 行，而不是未经验证的网络扫描。为 GitHub `production` environment 配置所需 reviewer 和分支限制。工作流使用只读仓库权限、拒绝未知 host key，并通过 `transnet-production` concurrency group 串行执行部署。

## 部署行为

测试 job 会检查格式、对全部 target 与 feature 运行 Clippy、运行 all-target 测试，并在强制使用 lockfile 的情况下构建 rustdoc。Pull request 在该 job 后结束。成功的非 pull-request 运行会连接 GPU 服务器，仅将 `/home/ashex/projects/Transnet` 重置到 `origin/master`，构建锁定依赖的 release，重启 `transnet`，并确认 systemd 报告其处于 active 状态。

远程 hard reset 的目的，是使专用 production checkout 与经审核的 `master` 分支保持一致。不要用该 checkout 进行开发或保存未提交的运维修改。

## 发布检查

在 UDS 传输迁移完成前，检查配置的过渡性回环 listener 和服务状态：

```bash
curl --fail http://127.0.0.1:16002/health
sudo systemctl status transnet
```

目标 listener 实现后，改用文档规定的 UDS 探针：

```bash
curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'
```

相关文档：[开发与运维](development_cn.md)、[配置](configuration_cn.md)及 [Transnet 服务接口](../interfaces/transnet_cn.md)。
