# Observability contract

中文：[可观测性合同](../../docs_cn/reference/observability_cn.md)

This document defines the target whole-system telemetry contract for the Transnet process, its adapters, and the offline publisher. It governs structured logs, traces, metrics, and audit events. The current Rust foundation implements a closed content-free event envelope, strict internal `traceparent` admission and HTTP propagation, closed metric dimensions, bounded non-blocking metric dispatch, and local drop counters. Complete route instrumentation, event export, collectors, audit persistence, retention, and deployment policy remain target work.

## Goals and failure rule

Telemetry answers whether a request was admitted, which bounded path ran, where time was spent, which dependency failed, whether degradation occurred, and whether publication state changed. It must do so without recording the material being translated or any end-user identity.

Online telemetry is non-blocking and best effort. A full buffer, unavailable collector, serialization error, or export timeout increments a local dropped-event counter and cannot fail, delay, retry, or change a business response. Only an explicitly configured mandatory audit sink may block an offline publication transition; it never affects online translation readiness.

The current dispatcher accounts locally for capacity exhaustion and an unavailable asynchronous runtime. Exporter-side serialization, timeout, and collector-failure accounting will be added with the production exporter; no external collector is configured by this foundation.

## Signal ownership

| Signal | Purpose | Retention and cardinality |
|---|---|---|
| Structured log | diagnosable lifecycle and closed failure events | short operational retention; no bodies |
| Trace | request and dependency timing across island-port, Transnet, model, and data boundaries | sampled; static span names and bounded attributes |
| Metric | rates, latency, saturation, availability, and quality-policy counters | aggregate only; closed low-cardinality labels |
| Audit event | offline publication and security-relevant control transitions | append-only restricted sink; no request content |

Island-port creates or validates the internal request and trace identifiers. Transnet does not accept arbitrary internet trace baggage and never treats telemetry correlation as user identity. Every dependency span remains a child of the admitted request and shares its deadline.

The current HTTP boundary accepts exactly one canonical W3C version-00 `traceparent`, normalizes hexadecimal digits, stores the validated value in request extensions, and propagates it in the response. It drops malformed, repeated, unsupported-version, and all-zero identifiers. `tracestate` and arbitrary baggage are not admitted.

## Common event schema

Every structured log, trace event, and audit event uses a versioned envelope with `event_schema`, `timestamp`, `severity`, `service`, `service_version`, `environment`, and `event_name`. Request-path events may add `request_id`, `trace_id`, `span_id`, `operation`, static `route`, `outcome`, safe `error_code`, `duration_ms`, `deadline_remaining_bucket`, `request_size_bucket`, `response_size_bucket`, and `content_release`.

The implemented envelope deliberately starts with the required common fields plus closed route, outcome, and dependency enums. It has no free-form message or attribute map. Request identifiers, duration and size buckets, content releases, and broader execution dimensions remain omitted until their owning request-context and route instrumentation land.

Only the following closed execution dimensions are allowed: `input_kind` (`text`, `segments`, or `image_regions`), `inference_profile` (`none`, `fast`, or `reasoning`), `reasoning_escalated`, `retrieval_mode` (`offline`, `allowed`, or `required`), `retrieval_used`, `degraded`, `dependency`, `attempt`, `retry_count`, and circuit or bulkhead outcome. Exact text lengths, image dimensions, segment counts, citation URLs, model tokens, SQL text, canonical labels, and IDs not explicitly listed here are not generic telemetry fields. Metrics use coarser buckets than logs and traces.

Events are emitted at admission completion, application completion, each dependency logical-call completion, reasoning escalation, live-retrieval completion, degradation, cancellation, shutdown, and publication transition. Retries are child events of one logical dependency call; the request completion event is emitted exactly once.

## Content prohibition

No signal may contain request or response bodies, text or substrings, segments, protected ranges, terminology, history, images, OCR text, translations, alternatives, prompts, system instructions, provider bodies, hidden reasoning, embeddings, vector values, live-search queries, result titles or snippets, fetched pages, citation URLs, canonical content bodies, raw SQL, credentials, authorization material, cookies, socket peer details, user/account/session identifiers, or stable fingerprints derived from any of them.

Errors use closed codes and redacted dependency classes. Debug formatting and panic paths follow the same rule. Hashing forbidden content does not make it safe: deterministic hashes, query fingerprints, and per-request content digests are also prohibited because they permit correlation or dictionary recovery.

## Logs and traces

Production logs are newline-delimited JSON written to standard output or a configured local collector. Compact text and bounded rolling files are development-only. Export uses a bounded memory queue, never an unbounded channel or request-content spool. Rotation, compression, retention, and access control are deployment responsibilities and must be verified before production.

Span names are static operations such as `translation.execute`, `model.generate`, `embedding.search`, `live_retrieval.search`, `canonical.read`, and `publication.activate`; paths use matched templates, never raw URLs. Success traces are sampled at a configured low rate. Failures, reasoning escalation, live retrieval, and degradation may use higher bounded sampling, but sampling cannot inspect content. Sampling decisions and dropped-event counts are metrics.

Hidden model reasoning is opaque provider execution. Telemetry may record that the reasoning profile was selected and its total duration; it cannot request, decode, retain, or export chain-of-thought.

## Metrics

Required metric families cover admitted and rejected requests, completed outcomes, latency, in-flight work, deadline exhaustion, model profile calls, reasoning escalations, embedding calls, live-retrieval use and failure, dependency attempts, retries, circuit and bulkhead outcomes, degradation, response validation failure, telemetry drops, and publication transitions. Histograms use fixed buckets. Labels are enumerated in code and must not include request IDs, releases, language tags, model strings, domains, relation types, or error messages.

Release-specific diagnosis uses bounded logs or traces rather than an unbounded metric label. Quality evaluation metrics are produced by offline licensed test datasets, not by recording production request content or model responses.

## Audit events

Audit events are limited to offline publication and control-plane actions: stage creation, validation, review decision, quarantine, activation, rollback, removal, configuration acceptance, and rejected security boundary events. They may identify the service actor or job, release and schema versions, transition, validation outcome, aggregate counts, manifest hash, reason code, and timestamp.

Audit events never contain source bodies, generated candidates, evidence text, prompts, query data, or reviewer free text. The publisher writes the audit event and state transition atomically when the deployment supports it; otherwise activation fails closed before becoming visible.

## Verification

Contract tests capture every signal sink and search for seeded secrets and request fragments across success, validation failure, dependency failure, timeout, cancellation, reasoning, vision, live retrieval, and panic-safe paths. Tests also enforce closed metric labels, static span names, one completion event, trace-parent continuity, queue bounds, drop behavior, audit/state ordering, and the rule that telemetry failure cannot alter an online response.

Current repository tests prove strict trace-parent parsing and propagation, redacted trace-context debug output, content-free envelope construction and debug output, closed metric labels, bounded dispatch, and monotonic capacity/runtime drop accounting. The broader scenario matrix above remains an acceptance target for the components that are not yet implemented.

Deployment acceptance verifies collector transport, access control, rotation, retention, backup behavior, provider-side telemetry, and deletion policy. Repository tests alone cannot prove those external controls.

## Related documents

- [Operations module](operations.md)
- [Request dataflow](application/request-dataflow.md)
- [Model runtime](model-runtime.md)
- [Configuration](../guides/configuration.md)
- [Quality assurance](../guides/quality-assurance.md)
