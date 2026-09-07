# HTTP API index

Transnet exposes two JSON-over-HTTP APIs. Both use the success envelope `{"success":true,"data":...}` and the error envelope `{"success":false,"error":{"code":"...","message":"..."}}` unless a route explicitly says otherwise.

## Translation core

The core API is the production implementation of the current translation feature. It listens on `127.0.0.1:35792` by default.

- `GET /health`: report core service availability.
- `POST /translate`: validate, classify, and translate text with a configured OpenAI-compatible provider.

See [the core API contract](core/transnet/api.md) for request fields, response variants, validation rules, and exact errors.

## Gateway

The optional gateway listens on `0.0.0.0:8080` by default and forwards `POST /translate` and `GET /health` to the core. Its `/api/*` account, history, favorites, profile, about, and statistics routes are compatibility placeholders without authentication or persistence.

See [the gateway API contract](gateway/api.md) for the complete route inventory and the implementation status of each route.

## Security

Neither service authenticates callers. Both currently enable permissive CORS. Treat them as trusted-network services or place them behind an authenticated, policy-enforcing edge before public deployment.

Related: [architecture](architecture.md), [configuration](configuration.md), and [development](development.md).
