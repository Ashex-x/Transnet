# Ports module

中文：[Port 模块](../../docs_cn/reference/ports_cn.md)

The ports module defines narrow operations that application services require from model and data dependencies. Ports use validated domain types, explicit deadlines, closed outcomes, and release identifiers; they do not expose provider or database protocols.

## Model operations

The generation port provides bounded translation, visual reading, and structured generation through `fast` or `reasoning` profiles of the same configured VLM. Application code selects an operation and the orchestrator applies the closed escalation policy; neither selects a provider brand. The embedding port separately exposes publication-time canonical embedding and request-local candidate nomination. Provider URLs, credentials, HTTP envelopes, prompts, reasoning controls, image encoding, JSON Schema mechanics, retry headers, and model-specific payloads belong to adapters.

Model output is never canonical evidence. Callers validate structure and references before use and discard request content and generated material when the request ends.

The [model-runtime reference](model-runtime.md) owns call budgets, escalation, vision, and embedding lifecycle. Hidden reasoning is never a domain value or response field.

## Data operations

Structured reads resolve active releases, canonical candidates, complete sense details, domain inventories, facts, evidence, and provenance. Vector reads retrieve release-filtered node and edge candidates and bounded shallow neighborhoods. Methods express these use cases rather than SQL, Qdrant-native requests, or generic repository access.

Every read carries the request deadline and exact release identifiers. Closed outcomes distinguish missing, incompatible, unavailable, invalid, and permission-filtered data without disclosing withheld content.

The online composition receives no mutation methods. Separate publication ports may stage, validate, project, activate, quarantine, remove, and roll back reviewed canonical content, and are never passed to request handlers. Exact operations belong to the [canonical-data](../interfaces/canonical-data.md) and [retrieval-data](../interfaces/retrieval-data.md) contracts.

## Verification

Use fakes to test operation selection, deadline propagation, release consistency, malformed model output, closed failure mapping, filtered data, and the absence of online mutation access.
