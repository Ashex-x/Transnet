# Adapters module

中文：[Adapter 模块](../../docs_cn/reference/adapters_cn.md)

The adapters module implements model and data ports. It owns external protocol mechanics while preserving domain deadlines, release pins, closed outcomes, and privacy rules.

## Model providers

The shared OpenAI-compatible client owns HTTP construction, authentication, response-size bounds, strict structured-output decoding, safe error mapping, and resilience integration. Gemma 4 owns short-text and bounded structured-composition request policy. TranslateGemma owns longer connected-text policy. The current runtime selects between them at translation.long_text_chars.

Provider adapters never log prompts, source text, history, provider bodies, credentials, or generated content. Errors expose only closed dependency and operation categories.

## Island-port data access

The island-port client maps data-port operations to versioned HTTP/1.1 JSON calls over island-port's owned Unix socket. It owns connection lifecycle, content-type and body bounds, schema-version handling, deadlines, and safe transport errors.

The Stage 3 client implements outbound-only canonical reads for translation candidates, lexical candidate resolution, and sense details. Its private strict DTOs reconstruct the existing `CanonicalTranslationRevision`, `CanonicalCandidate`, and `CanonicalSenseDetails` types. Ranking, fusion, ambiguity resolution, and coverage remain request-local application work. Unix builds provide the production socket transport; tests inject a bounded fake transport without adding an inbound listener or database client.

Stage 4 adds the active canonical release read to the same outbound transport. It maps the strict response into a canonical-only release pin, with no vector collection or local ranking-policy version. The application obtains that pin once and passes it to every later authority read. The existing full hybrid content tuple remains unchanged for vector retrieval and publication compatibility.

The executable now optionally constructs this outbound transport and canonical-only read service from validated runtime configuration. Only the read-only active-release operation participates in canonical dependency readiness. This does not add a public BasicCard route or an inbound UDS listener.

The corresponding island-port server is maintained outside this repository and must implement the current delta in `interfaces/mysql.md`. Until that peer is upgraded, incompatible or incomplete responses fail closed and real island-port/MySQL end-to-end operation is not considered verified.

Structured and vector mapping preserves release identifiers and closed outcomes. Island-port owns MySQL and Qdrant drivers, queries, pooling, transactions, collection selection, and credentials. Transnet does not expose SQL or Qdrant-native requests. Filesystem permissions authenticate processes; JSON never forwards end-user identity or credentials.

Online adapters are read-only. A separately authorized publisher composition uses mutation-capable operations. Exact payloads remain in the [SQL](../interfaces/mysql.md) and [vector](../interfaces/qdrant.md) interfaces.

## Verification

Test exact provider and island-port envelopes, strict decoding, response bounds, redaction, deadline propagation, release preservation, status classification, retry eligibility, circuit behavior, and local-socket failures.
