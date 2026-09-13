# Request dataflow

中文：[请求数据流](../../../docs_cn/reference/application/request-dataflow_cn.md)

This page traces one request across Transnet and identifies what each module receives, decides, returns, and must discard. Exact routes and payloads remain authoritative in the [Transnet service interface](../../interfaces/transnet.md).

Status: the target flow below is not fully composed. The current executable uses loopback HTTP and directly wires transitional translation and model-backed lookup paths; [current runtime](../runtime/startup.md) identifies what is available today.

## End-to-end flow

~~~mermaid
sequenceDiagram
  participant IP as island-port
  participant T as Transport
  participant O as Orchestrator
  participant D as Domain
  participant P as Ports
  participant A as Adapters
  participant X as External dependencies

  IP->>T: UDS HTTP/1.1 JSON request
  T->>T: Admit, decode, validate, establish request context
  T->>O: Validated request + request ID + deadline
  O->>P: Read compatible active release
  P->>A: Operation-focused data call
  A->>X: island-port data endpoint
  X-->>A: Release or closed failure
  A-->>P: Domain result
  P-->>O: Pinned release trio
  O->>D: Normalize and classify input
  D-->>O: Word, phrase, or passage decision
  alt Connected passage
    O->>P: Translate with request-local context
    P->>A: Model operation
    A->>X: Gemma provider call
    X-->>A: Candidate translation
    A-->>O: Bounded model result
  else Word or established phrase
    O->>P: Resolve canonical candidates
    P->>A: Structured/vector reads
    A->>X: island-port data calls
    X-->>A: Release-filtered candidates and facts
    A-->>O: Hydrated canonical bundle
    O->>P: Optional bounded composition
    P->>A: Structured model operation
    A->>X: Gemma provider call
    X-->>A: Candidate organization
    A-->>O: Bounded model result
  end
  O->>D: Build, project, and validate result
  D-->>O: Validated response model
  O-->>T: Application outcome
  T-->>IP: JSON response
~~~

One deadline and one release trio follow the request through every downstream call. No module may replace either value or extend the deadline.

## Admission

Island-port removes end-user identity and sends only the service request plus optional minimal chronological translation history. The transport module verifies the UDS-only boundary, request size, media type, UTF-8, method and path, strict JSON shape, request ID, deadline, and concurrency permit.

Malformed or inadmissible requests stop here. No model or data dependency is called. Transport maps the closed failure to the normative response and records only content-free telemetry.

## Orchestration and routing

The application orchestrator pins the compatible release once, then asks domain logic to normalize and classify the input. The result selects connected-text translation or lexical-knowledge processing; callers never choose the path.

The orchestrator owns call order, remaining-time allocation, cancellation, and degraded-result policy. It does not parse HTTP, implement provider payloads, issue SQL or vector-native queries, or change canonical facts.

## Connected-text branch

The translation application derives the model operation. Short input uses the configured Gemma 4 role; longer input uses TranslateGemma and may use a request-local chunk plan and terminology ledger. The model port carries validated input and the remaining deadline to the provider adapter.

The adapter creates the provider-specific HTTP request, applies resilience policy, bounds and decodes the result, and returns a closed outcome. Application logic checks coverage, order, terminology consistency, and output validity before adding the translation to the superset result.

## Lexical-knowledge branch

The knowledge application derives lookup forms and requests canonical candidates through data ports. Exact and alias matches outrank weaker retrieval signals. When domain expansion is useful, it resolves against the published domain inventory.

Vector reads nominate release-filtered candidates. Structured reads hydrate the selected nodes, facts, relationships, evidence, and revisions from authoritative same-release data. Similarity never establishes a fact. The application filters and ranks the hydrated bundle, then may ask the model port to organize only those supplied facts into concise sections.

Deterministic validation rejects unknown references, invalid directions, incompatible releases, unsupported evidence, and out-of-scope content. A basic structured card may survive vector failure with explicit degradation; missing authoritative content or an incompatible release fails safely.

## Projection and response

Domain and application logic assemble one superset result. Response projection derives brief, standard, or full without another retrieval or model call. Final validation checks required fields, release consistency, evidence labels, graph bounds, language constraints, and the absence of private or persistence fields.

Transport serializes the validated application outcome using the interface envelope. It adds the request ID header and records a static route, outcome class, byte counts, and duration without bodies.

## Module responsibility map

- **Runtime:** constructs dependencies, registers only supported routes, exposes readiness, owns listener lifetime, and coordinates shutdown.
- **Configuration:** loads and validates typed settings once; supplies redacted values to bootstrap.
- **Transport:** admits UDS HTTP/JSON, establishes request context, invokes one application operation, and maps the result.
- **Orchestrator:** owns the request sequence, release pin, deadline budget, branch selection, cancellation, degradation, and final outcome.
- **Translation application:** preserves connected-text meaning and structure; owns request-local chunk and terminology planning.
- **Knowledge application:** resolves senses and domains, retrieves and ranks canonical material, and composes relationship sections.
- **Translation domain:** validates language, history, response-level, and translation-result invariants.
- **Lexical-knowledge domain:** owns canonical identity, typed relationships, evidence semantics, and graph invariants.
- **Release domain:** owns compatible immutable release identity and valid degraded states.
- **Model ports:** expose bounded translation and structured-generation operations without provider protocol.
- **Data ports:** expose use-case-specific canonical and vector reads without database-native requests.
- **Provider adapters:** implement OpenAI-compatible requests, model roles, response decoding, and dependency failure mapping.
- **Island-port adapters:** implement UDS data calls while preserving deadline, release, and closed outcomes.
- **Observability and resilience:** record safe aggregate signals and enforce timeouts, retries, concurrency, and circuits.
- **Offline publication:** creates reviewed releases outside the request path; it is never reachable from an online request.

## Request-lifetime data

Request text, history, normalized forms derived from private input, chunk plans, terminology ledgers, provider input and output, intermediate candidates, inferred explanations, and proposed domains live only for the bounded request. They are dropped on success, failure, timeout, or cancellation and never enter durable caches, queues, MySQL, Qdrant, logs, metrics, or traces.

Published canonical IDs, release identifiers, reviewed facts, and aggregate operational counters are not request-content persistence. Cacheable data must be canonical, release-pinned, and independent of private request influence.

## Verification

End-to-end tests should prove early rejection before dependency calls, one release and deadline across every branch, automatic routing, provider and data-port call order, vector degradation, authoritative-data failure, cancellation propagation, deterministic response projection, safe error mapping, telemetry redaction, and request-state disposal.
