# GPU-server deployment

中文：[GPU 服务器部署](../../docs_cn/guides/deployment_cn.md)

This guide deploys Transnet from GitHub Actions to the GPU server through the existing FRP SSH endpoint at `139.224.103.112:16004`. The workflow tests pull requests without deploying them. A push to `master` or a reviewed manual dispatch must pass every repository check before the production job updates and restarts the service.

## One-time GPU-server setup

Run these commands on the GPU server as a sudo-capable administrator. Replace `<repository-ssh-url>` with the GitHub SSH clone URL. Do not overwrite another checkout or copy secrets into the repository.

```bash
sudo -u ashex git clone <repository-ssh-url> /home/ashex/projects/Transnet
sudo cp /home/ashex/projects/Transnet/deploy/transnet.service /etc/systemd/system/transnet.service
sudo systemctl daemon-reload
sudo -u ashex bash -lc 'cd /home/ashex/projects/Transnet && cargo build --release --locked'
sudo systemctl enable transnet
sudo systemctl restart transnet
```

The unit creates `/run/transnet` with mode `0770` and a process umask of `0007`. The target runtime binds `/run/transnet/transnet.sock` inside that directory; grant only the island-port runtime access through the configured group. The current executable still uses its configured loopback TCP listener until the inbound UDS migration is complete.

Production configuration and any provider credentials remain outside version control. Never add credentials, request text, generated output, logs, or runtime state to this repository. See the [configuration guide](configuration.md) for the current and target settings.

The `ashex` user needs narrowly scoped passwordless permission to restart and inspect this service. Add it with `sudo visudo -f /etc/sudoers.d/transnet`:

```sudoers
ashex ALL=(root) NOPASSWD: /bin/systemctl restart transnet, /bin/systemctl is-active --quiet transnet
```

Adjust `/bin/systemctl` if `command -v systemctl` reports another path.

## GitHub configuration

Create a dedicated Ed25519 deployment key. Add its public key to `/home/ashex/.ssh/authorized_keys` on the GPU server, then configure these GitHub Actions repository secrets:

- `DEPLOY_SSH_KEY`: the corresponding private key.
- `DEPLOY_KNOWN_HOSTS`: the exact trusted host-key line for `[139.224.103.112]:16004`.

Obtain the host-key line through a trusted administrative connection, not an unverified network scan. Configure GitHub's `production` environment with the required reviewers and branch restrictions. The workflow uses read-only repository permissions, refuses unknown host keys, and serializes deployments through the `transnet-production` concurrency group.

## Deployment behavior

The test job checks formatting, runs Clippy for all targets and features, runs all-target tests, and builds rustdoc with the lockfile enforced. Pull requests stop after that job. A successful non-pull-request run connects to the GPU server, resets only `/home/ashex/projects/Transnet` to `origin/master`, builds the locked release, restarts `transnet`, and verifies that systemd reports it active.

The remote hard reset intentionally makes the dedicated production checkout match the reviewed `master` branch. Do not use that checkout for development or uncommitted operational changes.

## Release check

Until the UDS transport migration is complete, verify the configured transitional loopback listener and inspect the service:

```bash
curl --fail http://127.0.0.1:16002/health
sudo systemctl status transnet
```

After the target listener is implemented, use the documented UDS probe instead:

```bash
curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'
```

Related: [development and operations](development.md), [configuration](configuration.md), and [Transnet service interface](../interfaces/transnet.md).
