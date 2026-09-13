# Request orchestration

中文：[请求编排](../../../docs_cn/reference/application/orchestration_cn.md)

This module owns one translation turn after transport validation and before wire projection.

## Flow

The orchestrator receives the validated request, request ID, one deadline, and one compatible active release pin. It classifies the input as a word, established lexical phrase, or connected passage, invokes the applicable translation or knowledge path, builds one superset result, projects the requested response level, and performs final invariant validation.

Callers do not select a mode, provider, domain, retrieval filter, dialect, register, or presentation section. Those are bounded internal decisions.

## Invariants

All canonical and vector reads use the same release trio. Minimal chronological history may influence reference resolution, terminology continuity, sense ranking, and wording, but it cannot change canonical identity or facts. Text, history, intermediate output, and proposals are discarded when the request ends.

The orchestrator owns sequencing and budget allocation, not wire parsing, provider protocol, storage queries, or formatting. Degraded results remain explicit and may never be padded with invented facts.

## Verification

Test routing ambiguity, one-deadline propagation, release consistency, cancellation, degraded dependencies, response-level monotonicity, and request-state disposal. See [translation](translation.md), [knowledge](knowledge.md), and the [service interface](../../interfaces/transnet.md).
