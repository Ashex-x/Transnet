# Adapters module

中文：[Adapter 模块](../../docs_cn/reference/adapters_cn.md)

The adapters module implements model and data ports. It owns external protocol mechanics while preserving domain deadlines, release pins, closed outcomes, and privacy rules.

## Model providers

The target OpenAI-compatible generation adapter owns HTTP construction, authentication, response-size bounds, image encoding, strict structured-output decoding, safe error mapping, and resilience integration for one Gemma4-27B VLM. It maps the domain-neutral `fast` and `reasoning` profiles to provider settings without exposing provider-native reasoning fields. A separate embedding adapter serves only ephemeral online candidate nomination. Publication embedding and lexical encoding are island-port execution responsibilities and return attested receipts through the publication adapter.

The runtime invokes one resilient Gemma4-27B endpoint for both neutral profiles, caps calls by the remaining request deadline, observes cooperative cancellation, and validates bounded output and version metadata. The separate OpenAI-compatible embedding adapter validates a strict one-vector response, finite fixed dimensions, and its configured artifact version; it is composed only by features that truthfully support semantic nomination.

Provider adapters never log prompts, source text, history, provider bodies, credentials, or generated content. Errors expose only closed dependency and operation categories.

## Island-port data access

The island-port client maps data-port operations to versioned HTTP/1.1 JSON calls over island-port's owned Unix socket. It owns connection lifecycle, content-type and body bounds, schema-version handling, deadlines, and safe transport errors.

The canonical-data client implements outbound-only `canonical-data-v1` reads for active-release selection, translation candidates, lexical candidate resolution, and sense details. Its private strict DTOs reconstruct the existing `CanonicalTranslationRevision`, `CanonicalCandidate`, and `CanonicalSenseDetails` types. Ranking, fusion, ambiguity resolution, and coverage remain request-local application work. Unix builds provide the production socket transport; tests inject a bounded fake transport without adding an inbound listener or database client.

Candidate reads now require release-bound authoritative source records, reviewed nonempty attribution, and matching source/evidence permissions; strict DTO mapping fails closed on missing or conflicting lineage. Structured `content_release_unavailable` is distinct from `schema_incompatible`, and pinned sense reads verify the returned canonical schema against the caller's pin. No error classification reads peer message text.

Stage 4 adds the active canonical release read to the same outbound transport. It maps the strict response into a canonical-only release pin, with no vector collection or local ranking-policy version. The application obtains that pin once and passes it to every later authority read. The existing full hybrid content tuple remains unchanged for vector retrieval and publication compatibility.

The executable optionally constructs this outbound transport and canonical-only read service from validated runtime configuration. Only the read-only active-release operation participates in canonical dependency readiness. Public BasicCard lookup and release-pinned sense follow-up consume the service through the application boundary; no inbound UDS listener or database client is added.

The publication adapter is a separate outbound-only client over the same injectable transport. Private strict DTOs map `KnowledgePublicationPort` operations to `knowledge-publication-v1` begin, node/edge batch, freeze, reconcile, status, and abort calls. Its pure node/edge batch inspection and final send use one DTO/JSON/base64 serializer; inspection reserves the maximum legal request-ID/deadline representation without transport I/O, while send repeats the 1 MiB defensive check. It also enforces request/deadline/release echoing, the 256-point bound, 256 KiB control/response bounds, closed outcome/code combinations, exact execution receipts, and cross-artifact hashes. A second offline-only adapter maps explicit candidate submission and rollback selection to `release-control-v1`, validates proof and audit-sequence echoes, and remains absent from runtime routing. Fake transports exercise both contracts without adding Qdrant/MySQL drivers, an island-port server, authority mutation, embedding execution, or online handler wiring.

The corresponding island-port server is maintained outside this repository and must implement the current delta in `interfaces/canonical-data.md`. Until that peer is upgraded, incompatible or incomplete responses fail closed and real island-port/MySQL end-to-end operation is not considered verified.

Structured and vector mapping preserves release identifiers and closed outcomes. Island-port owns MySQL and Qdrant drivers, queries, pooling, transactions, collection selection, and credentials. Transnet does not expose SQL or Qdrant-native requests. Filesystem permissions authenticate processes; JSON never forwards end-user identity or credentials.

Online adapters are read-only. A separately authorized publisher composition uses mutation-capable operations. Exact payloads remain in the [canonical-data](../interfaces/canonical-data.md) and [retrieval-data](../interfaces/retrieval-data.md) interfaces.

## Verification

Test exact provider and island-port envelopes, strict decoding, response bounds, redaction, deadline propagation, release preservation, status classification, retry eligibility, circuit behavior, and local-socket failures.
