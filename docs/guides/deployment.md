# GPU-server deployment

中文：[GPU 服务器部署](../../docs_cn/guides/deployment_cn.md)

This guide deploys Transnet from GitHub Actions to the GPU server through the existing FRP SSH endpoint at `139.224.103.112:16004`. The workflow tests pull requests without deploying them. A push to `master` or a reviewed manual dispatch must pass every repository check before the production job atomically activates an immutable release and restarts `island.transnet.service`.

## One-time GPU-server setup

Run these commands once on the GPU server as a sudo-capable administrator. The service deploys under `/home/ashex/services/island.transnet`; it does not require a production Git checkout.

```bash
sudo install -d -o ashex -g ashex -m 0755 /home/ashex/services/island.transnet/releases
sudo install -m 0644 deploy/island.transnet.service /etc/systemd/system/island.transnet.service
sudo systemctl daemon-reload
sudo systemctl enable island.transnet
```

The unit creates `/run/transnet` with mode `0770` and a process umask of `0007`. The runtime binds only `/run/transnet/transnet.sock` inside that directory; grant only the island-port runtime access through the configured group.

Production configuration and any provider credentials remain outside version control. Never add credentials, request text, generated output, logs, or runtime state to this repository. See the [configuration guide](configuration.md) for the current and target settings.

The `ashex` user needs narrowly scoped passwordless permission to restart and inspect this service. Install the checked-in rule with `sudo visudo -cf deploy/island.transnet.sudoers` followed by `sudo install -m 0440 deploy/island.transnet.sudoers /etc/sudoers.d/island-transnet`. The installed filename deliberately contains no dot because sudoers ignores dotted files in that directory. Its exact content is:

```sudoers
ashex ALL=(root) NOPASSWD: /usr/bin/systemctl restart island.transnet, /usr/bin/systemctl is-active --quiet island.transnet
```

Adjust `/usr/bin/systemctl` if `command -v systemctl` reports another path.

## GitHub configuration

Create a dedicated Ed25519 deployment key. Add its public key to `/home/ashex/.ssh/authorized_keys` on the GPU server, then configure these GitHub Actions repository secrets:

- `DEPLOY_SSH_KEY`: the corresponding private key.
- `DEPLOY_KNOWN_HOSTS`: the exact trusted host-key line for `[139.224.103.112]:16004`.

Obtain the host-key line through a trusted administrative connection, not an unverified network scan. Configure GitHub's `production` environment with the required reviewers and branch restrictions. The workflow uses read-only repository permissions, refuses unknown host keys, and serializes deployments through the `transnet-production` concurrency group.

## Deployment behavior

The test job checks formatting, runs Clippy for all targets and features, runs all-target tests, builds rustdoc, and produces the locked release binary. Pull requests stop after that job. A successful non-pull-request run downloads that exact artifact, uploads it with the production configuration into a commit-SHA directory, atomically switches `/home/ashex/services/island.transnet/current`, restarts `island.transnet`, and probes the target UDS. A failed restart or probe restores the previous release when one exists. Releases older than seven days are removed only after successful activation and never include the current or rollback target.

The production configuration uses the GPU host's actual OpenAI-compatible model identifier `Gemma4-26B`. `TRANSNET_CONFIG` selects the deployed configuration at runtime, so an uploaded binary never depends on the CI runner's checkout path.

## Release check

Verify the target UDS probe and inspect the service:

```bash
curl --fail --unix-socket /run/transnet/transnet.sock \
  --request POST http://localhost/api/v1/health \
  --header 'content-type: application/json' \
  --data '{}'
```

Related: [development and operations](development.md), [configuration](configuration.md), and [Transnet service interface](../interfaces/transnet.md).
