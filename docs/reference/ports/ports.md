# Ports module

中文：[Port 模块](../../../docs_cn/reference/ports/ports_cn.md)

The ports module defines narrow operations that application services require from model and data dependencies. Ports use validated domain types, explicit deadlines, closed outcomes, and release identifiers; they do not expose provider or database protocols.

## Model operations

Model ports provide bounded plain translation and structured generation. Application code selects an operation, not a provider brand. Provider URLs, credentials, HTTP envelopes, prompts, role messages, JSON Schema mechanics, retry headers, and model-specific payloads belong to adapters.

Model output is never canonical evidence. Callers validate structure and references before use and discard request content and generated material when the request ends.

## Data operations

Structured reads resolve active releases, canonical candidates, complete sense details, domain inventories, facts, evidence, and provenance. Vector reads retrieve release-filtered node and edge candidates and bounded shallow neighborhoods. Methods express these use cases rather than SQL, Qdrant-native requests, or generic repository access.

Every read carries the request deadline and exact release identifiers. Closed outcomes distinguish missing, incompatible, unavailable, invalid, and permission-filtered data without disclosing withheld content.

The online composition receives no mutation methods. Separate publication ports may stage, validate, project, activate, quarantine, remove, and roll back reviewed canonical content, and are never passed to request handlers. Exact operations belong to the [SQL](../../interfaces/mysql.md) and [vector](../../interfaces/qdrant.md) contracts.

## Verification

Use fakes to test operation selection, deadline propagation, release consistency, malformed model output, closed failure mapping, filtered data, and the absence of online mutation access.
