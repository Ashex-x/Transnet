# GPU-server deployment

中文：[GPU 服务器部署](../../docs_cn/guides/deployment_cn.md)

This guide deploys Transnet from GitHub Actions to the GPU server through the existing FRP SSH endpoint at `139.224.103.112:16004`. The workflow tests pull requests without deploying them. A push to `master` or a reviewed manual dispatch must pass every repository check before the production job atomically activates an immutable release and restarts `island.transnet.service`.

## One-time GPU-server setup

Run these commands once on the GPU server as a sudo-capable administrator. The service deploys under `/home/ashex/services/island.transnet`; it does not require a production Git checkout.

```bash
sudo install -d -o ashex -g ashex -m 0755 /home/ashex/services/island.transnet/releases
sudo install -d -o root -g root -m 0755 /etc/island
sudo install -o root -g root -m 0600 /dev/null /etc/island/transnet.env
sudo install -m 0644 deploy/island.transnet.service /etc/systemd/system/island.transnet.service
sudo systemctl daemon-reload
sudo systemctl enable island.transnet
```

The unit starts after and weakly wants `island-port.service`. This preserves independent restart behavior: stopping or restarting island-port does not automatically stop Transnet, while a normal boot orders the canonical authority first. The unit creates `/run/transnet` with mode `0770` and a process umask of `0007`. The runtime binds only `/run/transnet/transnet.sock` inside that directory; grant only the island-port runtime access through the configured group.

The deployed production configuration enables canonical reads over `/run/island-port/island-port.sock` and enables knowledge in optional bootstrap mode. Put a stable 32-to-4,096-byte cursor key in the root-owned environment file without shell interpolation, for example `TRANSNET_KNOWLEDGE_CURSOR_SECRET=<random-value>`. The file is loaded by systemd, not parsed by Transnet as TOML, and must never enter a release artifact or Git. Provider credentials and any other production overrides also remain outside version control. Never add credentials, request text, generated output, logs, or runtime state to this repository. See the [configuration guide](configuration.md) for the current and target settings.

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

The Linux test job checks formatting, runs Clippy for all targets and features, runs all-target tests, builds rustdoc, and produces the locked release binary. A separate Windows job runs `cargo check --bin transnet --locked` to keep the non-Unix fail-closed compile boundary healthy; it neither starts a production runtime nor produces a deployment artifact. Pull requests stop after these checks. A successful non-pull-request run downloads the exact Linux artifact, uploads it with the production configuration into a commit-SHA directory, atomically switches `/home/ashex/services/island.transnet/current`, restarts `island.transnet`, and probes the target UDS. A failed restart or probe restores the previous release when one exists. Releases older than seven days are removed only after successful activation and never include the current or rollback target.

The production configuration uses the GPU host's actual OpenAI-compatible model identifier `Gemma4-26B`. `TRANSNET_CONFIG` selects the deployed configuration at runtime, so an uploaded binary never depends on the CI runner's checkout path.

The production provider timeout is 105 seconds. Streaming callers may admit up to 120 seconds;
the shorter provider bound leaves time for validation, enrichment, and the terminal SSE result
without reverting to the legacy 30-second buffered limit. Generation requests use llama.cpp's
JSON-object response format so the strict translation contract is not wrapped in Markdown prose.

Knowledge is intentionally optional during the first deployment against an empty canonical authority. Transnet still requires canonical readiness, but starts without knowledge routes when island-port has no complete active canonical/node/edge release tuple. Publish and reconcile both immutable collections, submit the activation candidate through island-port, and verify `POST /api/v1/knowledge-releases/active`. Then copy the deployed TOML to an operator-owned configuration, change `[knowledge].required` to `true`, point `TRANSNET_CONFIG` at that file with a systemd drop-in, and restart Transnet. Required mode rejects startup if the active tuple is absent or incompatible and makes projection drift fail readiness. Do not enable required mode by editing an immutable release directory.

## Release check

Verify the target UDS probe and inspect the service:

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

Related: [development and operations](development.md), [configuration](configuration.md), and [Transnet service interface](../interfaces/transnet.md).
