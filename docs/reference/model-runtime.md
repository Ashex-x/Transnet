# Model runtime

中文：[模型运行时](../../docs_cn/reference/model-runtime_cn.md)

This subsystem owns target inference policy for the one configured Gemma4-27B vision-language model and the separate embedding model. It does not expose provider brands, reasoning controls, prompts, or embedding payloads through the service interface.

Status: the executable and public Rust boundary use one provider-neutral generation port with closed `fast` and `reasoning` profiles, bounded redacted values, deadlines, cooperative cancellation, and an atomic one-reasoning-call guard. The legacy direct translation API and two-provider length router are removed.

## Generation profiles

The generation port exposes `fast` and `reasoning` invocation profiles backed by the same Gemma4-27B endpoint and model identity. Fast is the default for translation, structured extraction, visual reading, classification, and grounded composition. Reasoning is an internal bounded escalation, not a caller-selected model and not a separate provider.

Canonical exact matches require no generation call. Ordinary text, segment, and image-region translation use fast inference. Input length never selects another model: the translation application splits accepted long input into bounded semantic chunks, executes independent chunks with bounded concurrency, carries a request-local terminology ledger, and reassembles deterministically.

One request may escalate to reasoning at most once, and only after deterministic work identifies unresolved material sense ambiguity, conflicting terminology or formatting constraints, a verified multi-hop explanation that cannot be rendered safely by the fast profile, or one invalid fast structured result. A valid fast result survives an optional reasoning failure with reduced-detail metadata. If correctness depends on the reasoning result, the operation returns clarification or a closed dependency failure instead of guessing.

Neither prompts nor hidden reasoning are response data. The service may return concise conclusions, safe decision codes, and evidence references, but never chain-of-thought, scratch work, reasoning tokens, or provider-native reasoning fields.

## Embedding operations

The Transnet embedding port has one bounded use: online requests may embed a query or an explicitly permitted live-retrieval fragment in memory to nominate candidates. Online vectors are discarded with the request and never enter logs, traces, metrics, caches, canonical storage, or a later release. Offline publication instead sends frozen canonical inputs to island-port; island-port executes dense embedding and lexical encoding and attests the exact revisions and dimensions in publication receipts. Edge inputs contain structured endpoints, typed relation, scope, and verified evidence metadata, never newly generated relationship prose.

Vector similarity is a ranking signal only. It cannot establish translation equivalence, synonymy, taxonomy, causality, mechanism, cultural meaning, evidence, or truth.

The implemented embedding port accepts only a bounded request-local input and returns a finite vector whose dimension and immutable artifact version are validated. Its OpenAI-compatible adapter maps failures to closed, content-free errors. It is not yet wired into online candidate retrieval, and it is deliberately separate from publication input preparation and island-port-owned publication embedding execution.

## Deadlines and call budget

Every model and embedding operation consumes the caller deadline. Configuration supplies separate fast, reasoning, embedding, and optional live-retrieval sub-deadlines, each capped by the remaining request time. Independent canonical and vector reads run concurrently when their dependencies permit it.

The default call budget is zero generation calls for a sufficient canonical match, one fast logical operation for ordinary input, bounded parallel fast chunk operations for long structured input, and at most one reasoning escalation for the entire request. The application does not issue one model call per response section or relationship.

Retries apply only to transport-level retryable attempts and do not create a second semantic generation after a valid result. Optional enrichment that misses its sub-deadline degrades explicitly without extending the caller deadline. Required generation or embedding failures use closed outcomes.

## Vision boundary

The VLM accepts only validated inline PNG, JPEG, or WebP data and bounded image regions from the Transnet request contract. Island-port owns file upload, PDF rendering, decompression, malware checks, page selection, and document reconstruction. Transnet never fetches a caller-supplied image URL and never persists image bytes or visual intermediate output.

## Verification

Test zero-call canonical answers, fast-path selection, the closed escalation triggers, the one-escalation limit, long-input chunk coverage and ordering, terminology consistency, image bounds, structured-output repair, deadline accounting, cancellation, concurrency, redaction, and disposal of text, images, reasoning output, and online vectors.

Current coverage verifies bounded values, redacted debug output, fixed finite embedding dimensions, strict embedding envelopes, closed errors, one-time reasoning-budget claims, deterministic lexical-versus-connected routing, bounded parallel chunk calls, ordered reassembly, terminology-ledger disposal, deadline and cancellation propagation, first-participation version ordering, and the single deterministic invalid-or-ambiguous output repair. Valid guidance that the runtime cannot yet enforce fails explicitly before generation. Structured segment, image-region, guidance execution, retrieval, and canonical zero-call composition remain later slices.
