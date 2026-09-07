# Translation core reference

The `island-transnet` crate owns translation behavior and the API-only core process. Use this page as the entry point for its hand-written reference.

## Interfaces

- [Core HTTP API](api.md): wire envelopes, routes, validation, response shapes, and error codes.
- [Translation types](types.md): classification rules, domain contracts, and translation identifiers.

## Implementation modules

- [LLM translation service](llm.md): request orchestration, provider selection, retries, and logging policy.
- [Translation prompts](prompt.md): prompt and response-schema selection.
- [Provider response format](format.md): JSON extraction and structural validation.
- [Core server process](server.md): configuration loading, tracing, listener startup, and shutdown.

For system context, see the [architecture](../../architecture.md). For local use, see the [development guide](../../guides/development.md) and [configuration guide](../../guides/configuration.md).
