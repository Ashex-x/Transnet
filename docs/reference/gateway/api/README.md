# Gateway HTTP API

Parent: [Gateway reference](../README.md).

The gateway exposes JSON APIs only. It does not serve HTML, browser assets, or an SPA fallback; unknown paths return 404.

Requests with bodies use `Content-Type: application/json`. Successful JSON responses use `{"success":true,"data":...}`. Errors use `{"success":false,"error":{"code":"CODE","message":"description"}}`.

Authentication is not enforced in the current implementation. Account endpoints return mock tokens, profile endpoints return `401 UNAUTHORIZED`, and account/history/favorites handlers do not persist data. Do not treat this compatibility surface as production authentication.

Route contracts are grouped by domain:

- [System and health](system.md)
- [Account](account.md)
- [Translation, history, and favorites](transnet.md)
- [Profile](profile.md)

Language fields use short identifiers such as `en` and `zh`. `input_type` accepts `auto`, `word`, `phrase`, `sentence`, `paragraph`, or `essay`. Pagination is one-based and contains `page`, `limit`, `total`, and `total_pages`.
