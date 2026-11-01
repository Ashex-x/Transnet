# GPU-server deployment

中文：[GPU 服务器部署](../../docs_cn/guides/deployment_cn.md)

This guide deploys Transnet from GitHub Actions to the GPU server through the existing FRP SSH endpoint at `139.224.103.112:16004`. The workflow tests pull requests without deploying them. A push to `master` or a reviewed manual dispatch must pass every repository check before the production job atomically activates an immutable release and restarts `island.transnet.service`.

## GPU-server service setup

The deployed production service is the `ashex` user unit `island-transnet.service`. It runs from `/home/ashex/services/island.transnet/current`, binds `/home/ashex/services/island.transnet/transnet.sock`, and is restarted by the GitHub workflow with `systemctl --user`. The deployment workflow rewrites only the listener socket path in the immutable release configuration before activation. Keep the user unit enabled and the service directory writable by `ashex`; the separate system-level `island.transnet.service` is not the deployment target.

The checked-in production configuration currently runs model-only translation. `[canonical]` and `[knowledge]` are disabled because no reviewed canonical release is published. This configuration needs no canonical socket or knowledge cursor secret, advertises no knowledge lenses, and reports those readiness components as `disabled`. The model endpoint and generation provider remain required for successful translations; target readiness reports configured dependency readiness, not a synthetic test of model output.

When approved canonical content and a complete projection tuple exist, enable canonical and knowledge through a reviewed configuration change, supply a stable 32-to-4,096-byte cursor key through the user unit's external environment file, and verify the active release before changing required knowledge readiness. Never put the key, provider credentials, request text, generated output, logs, or runtime state in Git. See the [configuration guide](configuration.md).

## GitHub configuration

Create a dedicated Ed25519 deployment key. Add its public key to `/home/ashex/.ssh/authorized_keys` on the GPU server, then configure these GitHub Actions repository secrets:

- `DEPLOY_SSH_KEY`: the corresponding private key.
- `DEPLOY_KNOWN_HOSTS`: the exact trusted host-key line for `[139.224.103.112]:16004`.

Obtain the host-key line through a trusted administrative connection, not an unverified network scan. Configure GitHub's `production` environment with the required reviewers and branch restrictions. The workflow uses read-only repository permissions, refuses unknown host keys, and serializes deployments through the `transnet-production` concurrency group.

## Deployment behavior

The Linux test job checks formatting, runs Clippy for all targets and features, runs all-target tests, builds rustdoc, and produces the locked release binary. A separate Windows job runs `cargo check --bin transnet --locked` to keep the non-Unix fail-closed compile boundary healthy; it neither starts a production runtime nor produces a deployment artifact. Pull requests stop after these checks. A successful non-pull-request run downloads the exact Linux artifact, uploads it with the production configuration into a commit-SHA directory, atomically switches `/home/ashex/services/island.transnet/current`, restarts the user unit `island-transnet.service`, and probes its UDS. A failed restart or probe restores the previous release when one exists. Releases older than seven days are removed only after successful activation and never include the current or rollback target.

The production configuration uses the GPU host's actual OpenAI-compatible model identifier `Gemma4-26B`. `TRANSNET_CONFIG` selects the deployed configuration at runtime, so an uploaded binary never depends on the CI runner's checkout path.

The production provider timeout is 105 seconds. Streaming callers may admit up to 120 seconds;
the shorter provider bound leaves time for validation, enrichment, and the terminal SSE result
without reverting to the legacy 30-second buffered limit. Generation requests use llama.cpp's
JSON-object response format so the strict translation contract is not wrapped in Markdown prose.

Knowledge remains disabled while the canonical database has no approved release. Translation uses the LLM without inventing canonical facts or knowledge lenses. Once publication has produced a reviewed canonical release and reconciled node/edge collections, submit the activation candidate through island-port and verify `POST /api/v1/knowledge-releases/active`. Only then enable the canonical and knowledge dependencies in a reviewed production configuration. Required knowledge mode rejects startup if the active tuple is absent or incompatible and makes projection drift fail readiness.

## Release check

Verify the target UDS probe and inspect the service:

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

Related: [development and operations](development.md), [configuration](configuration.md), and [Transnet service interface](../interfaces/transnet.md).
